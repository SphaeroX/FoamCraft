// SPDX-License-Identifier: GPL-3.0-or-later
//! Wind tunnel runs in the real runtime (needs Docker and the runtime image):
//!     cargo test --workspace -- --ignored runtime_
//!     cargo test --workspace -- --ignored validation_

use std::path::PathBuf;

use foamcraft_core::command::{Registry, Session};
use serde_json::{Value, json};

/// Removes the volumes and containers of all jobs of the project (records stay).
fn clean(reg: &Registry, s: &mut Session) {
    let _ = reg;
    if let Ok(project) = s.project() {
        let store = foamcraft_core::job::JobStore::new(project.root.join("runs"));
        let runner = foamcraft_runtime::DockerRunner::local();
        for record in store.list().unwrap_or_default() {
            let _ = foamcraft_runtime::jobs::remove_job_resources(&runner, &record);
        }
    }
}

/// The 20-unit cube of the test data in `unit`, in a wind tunnel at 20 m/s.
fn cube_project(name: &str, unit: &str, ground: &str, quality: &str) -> (Registry, Session) {
    let reg = foamcraft_api::registry();
    let base = std::env::temp_dir().join(format!("fc-solve-rt-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&base);
    let mut s = Session::new(base.join("data"));
    reg.execute(
        &mut s,
        "project.new",
        json!({"path": base.join("p"), "name": name}),
    )
    .unwrap();
    let model =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/data/geometry/cube_binary.stl");
    reg.execute(
        &mut s,
        "geometry.import",
        json!({"path": model, "unit": unit}),
    )
    .unwrap();
    reg.execute(
        &mut s,
        "scenario.set_wind_tunnel",
        json!({"speed_m_s": 20.0, "direction_deg": 0.0, "ground": ground,
               "medium": {"kind": "air", "temperature_c": 20.0}, "quality": quality}),
    )
    .unwrap();
    (reg, s)
}

fn solve(reg: &Registry, s: &mut Session, params: Value) -> Value {
    let out = reg.execute(s, "run.start", params);
    let result = out.as_ref().map(|o| o["result"].clone());
    clean(reg, s);
    result.unwrap_or_else(|e| panic!("run failed: {e}"))
}

#[test]
#[ignore = "needs Docker and the runtime image"]
fn runtime_draft_cube_ends_with_a_result_object() {
    let (reg, mut s) = cube_project("draft", "mm", "fixed", "draft");
    let mesh = reg
        .execute(&mut s, "mesh.generate", json!({"max_cells": 150000}))
        .unwrap();
    assert!(mesh["cells"].as_u64().unwrap() > 50_000);
    let r = solve(&reg, &mut s, json!({"wait": true, "iterations": 600}));
    eprintln!(
        "reason {} decided {} iterations {} cd {} cl {} fluct {}% y+ {} timings {}",
        r["reason"],
        r["decided_at_iteration"],
        r["iterations"],
        r["coefficients"]["cd"],
        r["coefficients"]["cl"],
        r["fluctuation_percent"],
        r["y_plus"],
        r["timings_s"]
    );
    assert!(
        ["stable", "oscillating", "limit_reached"].contains(&r["reason"].as_str().unwrap()),
        "{}",
        r["reason"]
    );
    let cd = r["coefficients"]["cd"].as_f64().unwrap();
    assert!((0.5..2.0).contains(&cd), "cd {cd}");
    assert!(r["forces"]["drag_n"].as_f64().unwrap() > 0.0);
    assert!(!r["y_plus"].as_array().unwrap().is_empty());
    assert!(r["history"]["cd"].as_array().unwrap().len() >= 100);
    let stored = reg.execute(&mut s, "run.result", json!({})).unwrap();
    assert_eq!(stored["job_id"], r["job_id"]);
}

/// Reference: drag coefficient of a sharp-edged cube facing the flow, C_D = 1.05
/// (S. F. Hoerner, Fluid-Dynamic Drag, 1965, chapter 3; independent of the Reynolds number above
/// about 10⁴ because the flow separates at the edges). Free stream, no ground.
const CUBE_CD: f64 = 1.05;
/// Accepted deviation at standard quality (fixed in Task_10; never widen it).
const CUBE_TOLERANCE: f64 = 0.15;

#[test]
#[ignore = "slow CFD reference case; needs Docker and the runtime image"]
fn validation_free_cube_drag_matches_the_literature() {
    // 0.2 m cube at 20 m/s in air: Re = 2.7e5.
    let (reg, mut s) = cube_project("validation", "cm", "none", "standard");
    let r = solve(&reg, &mut s, json!({"wait": true}));
    let cd = r["coefficients"]["cd"].as_f64().unwrap();
    eprintln!(
        "cube cd {cd} (reference {CUBE_CD}) reason {} decided {} cells {} cores {} fluct {}% y+ {} timings {}",
        r["reason"],
        r["decided_at_iteration"],
        r["cells"],
        r["cores"],
        r["fluctuation_percent"],
        r["y_plus"],
        r["timings_s"]
    );
    let deviation = (cd - CUBE_CD).abs() / CUBE_CD;
    assert!(
        deviation <= CUBE_TOLERANCE,
        "cd {cd} deviates {:.1} % from {CUBE_CD}",
        deviation * 100.0
    );
}
