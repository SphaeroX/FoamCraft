// SPDX-License-Identifier: GPL-3.0-or-later
//! Result fields in the real runtime (needs Docker and the runtime image):
//!     cargo test --workspace -- --ignored runtime_
//!     cargo test --workspace -- --ignored validation_

use std::path::PathBuf;

use foamcraft_core::command::{Registry, Session};
use serde_json::{Value, json};

/// Documented budget of one result cache (surface plus three grid levels).
const CACHE_BUDGET_BYTES: u64 = 40 * 1024 * 1024;

fn clean(s: &mut Session) {
    if let Ok(project) = s.project() {
        let store = foamcraft_core::job::JobStore::new(project.root.join("runs"));
        let runner = foamcraft_runtime::DockerRunner::local();
        for record in store.list().unwrap_or_default() {
            let _ = foamcraft_runtime::jobs::remove_job_resources(&runner, &record);
        }
    }
}

fn run(reg: &Registry, s: &mut Session, name: &str, params: Value) -> Value {
    reg.execute(s, name, params)
        .unwrap_or_else(|e| panic!("{name}: {e}"))
}

/// The cube of the test data (20 units) in `unit`, wind tunnel at 20 m/s, meshed and solved.
fn solved_cube(name: &str, unit: &str, ground: &str) -> (Registry, Session, PathBuf) {
    let reg = foamcraft_api::registry();
    let base = std::env::temp_dir().join(format!("fc-results-rt-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&base);
    let mut s = Session::new(base.join("data"));
    s.events().subscribe(|e| {
        if e.name == "results.failed" {
            eprintln!("results.failed: {}", e.payload);
        }
    });
    run(
        &reg,
        &mut s,
        "project.new",
        json!({"path": base.join("p"), "name": name}),
    );
    let model =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/data/geometry/cube_binary.stl");
    run(
        &reg,
        &mut s,
        "geometry.import",
        json!({"path": model, "unit": unit}),
    );
    run(
        &reg,
        &mut s,
        "scenario.set_wind_tunnel",
        json!({"speed_m_s": 20.0, "direction_deg": 0.0, "ground": ground,
               "medium": {"kind": "air", "temperature_c": 20.0}, "quality": "draft"}),
    );
    let out = reg
        .execute(&mut s, "mesh.generate", json!({"max_cells": 200000}))
        .and_then(|_| {
            reg.execute(
                &mut s,
                "run.start",
                json!({"wait": true, "iterations": 600}),
            )
        });
    if let Err(e) = out {
        clean(&mut s);
        panic!("run failed: {e}");
    }
    (reg, s, base.join("p"))
}

#[test]
#[ignore = "needs Docker and the runtime image"]
fn runtime_cube_results_are_cached_and_probed() {
    let (reg, mut s, root) = solved_cube("cache", "mm", "fixed");
    let fields = run(&reg, &mut s, "results.list_fields", json!({}));
    clean(&mut s);
    let names: Vec<(String, String)> = fields["fields"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| {
            (
                f["name"].as_str().unwrap().into(),
                f["location"].as_str().unwrap().into(),
            )
        })
        .collect();
    for (name, location) in [
        ("p", "surface"),
        ("Cp", "surface"),
        ("wall_shear_stress", "surface"),
        ("y_plus", "surface"),
        ("p", "volume"),
        ("p_total", "volume"),
        ("Ux", "volume"),
        ("k", "volume"),
    ] {
        assert!(
            names.contains(&(name.into(), location.into())),
            "{name} {location}: {names:?}"
        );
    }
    assert_eq!(fields["levels"].as_array().unwrap().len(), 3);
    let bytes = fields["bytes"].as_u64().unwrap();
    assert!(bytes < CACHE_BUDGET_BYTES, "{bytes} bytes");
    // The top of the 20 mm cube (z = 20 mm) and a point in the flow above it.
    let on_top = run(
        &reg,
        &mut s,
        "results.probe",
        json!({"point": [0.01, 0.01, 0.02], "field": "p"}),
    );
    assert_eq!(on_top["unit"], "Pa");
    assert_eq!(on_top["location"], "surface");
    assert!(on_top["distance_m"].as_f64().unwrap() < 0.002);
    let above = run(
        &reg,
        &mut s,
        "results.probe",
        json!({"point": [0.01, 0.01, 0.05], "field": "Ux"}),
    );
    assert_eq!(above["unit"], "m/s");
    assert!(above["value"].as_f64().unwrap() > 5.0, "{above}");
    // Inside the cube there is no flow.
    let inside = reg
        .execute(
            &mut s,
            "results.probe",
            json!({"point": [0.01, 0.01, 0.01], "field": "Ux"}),
        )
        .unwrap_err();
    assert_eq!(inside.code, "results.outside");
    // Colouring, and the cache survives closing and opening the project.
    run(
        &reg,
        &mut s,
        "view.set_surface_field",
        json!({"field": "p"}),
    );
    assert_eq!(
        reg.execute(&mut s, "view.set_surface_field", json!({"field": "nope"}))
            .unwrap_err()
            .code,
        "results.unknown_field"
    );
    run(&reg, &mut s, "project.close", json!({}));
    run(&reg, &mut s, "project.open", json!({"path": root}));
    let again = run(&reg, &mut s, "results.list_fields", json!({}));
    assert_eq!(again["job_id"], fields["job_id"]);
    let info = run(&reg, &mut s, "project.info", json!({}));
    assert!(info.get("schema_version").is_some());
}

/// Stagnation point of a cube facing the flow: Cp = 1 (Bernoulli: the flow comes to rest, the
/// pressure rises by the dynamic pressure). Tolerance 0.1 (fixed in Task_14; never widen it).
#[test]
#[ignore = "slow CFD reference case; needs Docker and the runtime image"]
fn validation_cube_stagnation_pressure_coefficient_is_one() {
    // 0.2 m cube from (0, 0, 0) to (0.2, 0.2, 0.2), free stream along +x.
    let (reg, mut s, _) = solved_cube("stagnation", "cm", "none");
    let cp = reg.execute(
        &mut s,
        "results.probe",
        json!({"point": [0.0, 0.1, 0.1], "field": "Cp"}),
    );
    clean(&mut s);
    let cp = cp.unwrap();
    let value = cp["value"].as_f64().unwrap();
    eprintln!("stagnation Cp {value} (distance {} m)", cp["distance_m"]);
    assert!((value - 1.0).abs() <= 0.1, "Cp {value}");
}
