// SPDX-License-Identifier: GPL-3.0-or-later
//! Internal flow in the real runtime (needs Docker and the runtime image):
//!     cargo test --workspace -- --ignored runtime_
//!     cargo test --workspace -- --ignored validation_

use std::path::PathBuf;

use foamcraft_core::command::{Registry, Session};
use serde_json::{Value, json};

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

/// A project with one of the internal test bodies, all openings capped; the opening with the
/// smallest x is the inlet, the others outlets.
fn project(name: &str, model: &str, unit: &str) -> (Registry, Session, Vec<String>) {
    let reg = foamcraft_api::registry();
    let base = std::env::temp_dir().join(format!("fc-internal-rt-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&base);
    let mut s = Session::new(base.join("data"));
    run(
        &reg,
        &mut s,
        "project.new",
        json!({"path": base.join("p"), "name": name}),
    );
    let file = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/data/geometry/internal")
        .join(format!("{model}.stl"));
    run(
        &reg,
        &mut s,
        "geometry.import",
        json!({"path": file, "unit": unit}),
    );
    let report = run(
        &reg,
        &mut s,
        "geometry.openings",
        json!({"model": "model-1"}),
    );
    // The opening with the smallest x is the inlet (flow along +x).
    let mut openings = report["openings"].as_array().unwrap().clone();
    openings.sort_by(|a, b| {
        a["centroid_m"][0]
            .as_f64()
            .unwrap()
            .total_cmp(&b["centroid_m"][0].as_f64().unwrap())
    });
    let mut keys = Vec::new();
    for o in &openings {
        let cap = run(
            &reg,
            &mut s,
            "geometry.cap_opening",
            json!({"model": "model-1", "point": o["centroid_m"]}),
        );
        keys.push(cap["key"].as_str().unwrap().to_string());
    }
    run(
        &reg,
        &mut s,
        "scenario.set_boundary_role",
        json!({"keys": [keys[0]], "role": "inlet"}),
    );
    run(
        &reg,
        &mut s,
        "scenario.set_boundary_role",
        json!({"keys": keys[1..], "role": "outlet"}),
    );
    (reg, s, keys)
}

fn set_flow(
    reg: &Registry,
    s: &mut Session,
    keys: &[String],
    inlet: Value,
    quality: &str,
) -> Value {
    let outlets: Vec<Value> = keys[1..]
        .iter()
        .map(|k| json!({"key": k, "pressure_pa": 0.0}))
        .collect();
    run(
        reg,
        s,
        "scenario.set_internal_flow",
        json!({
            "medium": {"kind": "water", "temperature_c": 20.0},
            "quality": quality,
            "inlets": [{"key": keys[0], "condition": inlet}],
            "outlets": outlets
        }),
    )
}

fn solve(reg: &Registry, s: &mut Session, max_cells: u64, iterations: u32) -> Value {
    let mesh = reg.execute(s, "mesh.generate", json!({"max_cells": max_cells}));
    let result = mesh.and_then(|_| {
        reg.execute(
            s,
            "run.start",
            json!({"wait": true, "iterations": iterations}),
        )
    });
    clean(s);
    let out = result.unwrap_or_else(|e| panic!("run failed: {e}"));
    out["result"].clone()
}

#[test]
#[ignore = "needs Docker and the runtime image"]
fn runtime_straight_pipe_gives_flow_and_pressure_loss() {
    let (reg, mut s, keys) = project("pipe", "pipe_open", "mm");
    // 0.5 m/s through 20 mm: Re ≈ 10 000 (turbulent).
    set_flow(
        &reg,
        &mut s,
        &keys,
        json!({"type": "velocity", "m_s": 0.5}),
        "draft",
    );
    let r = solve(&reg, &mut s, 150_000, 600);
    eprintln!(
        "pipe: reason {} decided {} dp {} Pa static {} Pa balance {} openings {}",
        r["reason"],
        r["decided_at_iteration"],
        r["pressure_loss_pa"],
        r["static_pressure_loss_pa"],
        r["mass_balance_error"],
        r["openings"]
    );
    assert_eq!(r["kind"], "internal_flow");
    let q = r["openings"][0]["flow_m3_s"].as_f64().unwrap();
    let want = 0.5 * std::f64::consts::PI * 1e-4;
    assert!((q - want).abs() < 0.02 * want, "inflow {q}");
    assert!(r["pressure_loss_pa"].as_f64().unwrap() > 0.0);
    assert!(r["mass_balance_error"].as_f64().unwrap() < 0.01);
    assert_eq!(r["regime"], "turbulent");
}

#[test]
#[ignore = "needs Docker and the runtime image"]
fn runtime_tee_splits_the_flow_with_a_closed_mass_balance() {
    let (reg, mut s, keys) = project("tee", "tee_open", "mm");
    assert_eq!(keys.len(), 3);
    set_flow(
        &reg,
        &mut s,
        &keys,
        json!({"type": "volume_flow", "m3_s": 5e-5}),
        "draft",
    );
    let r = solve(&reg, &mut s, 150_000, 600);
    eprintln!(
        "tee: reason {} dp {} Pa balance {} openings {}",
        r["reason"], r["pressure_loss_pa"], r["mass_balance_error"], r["openings"]
    );
    assert!(
        r["mass_balance_error"].as_f64().unwrap() < 0.01,
        "{}",
        r["mass_balance_error"]
    );
    let shares: f64 = r["openings"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|o| o["share"].as_f64())
        .sum();
    assert!((shares - 1.0).abs() < 1e-9);
}

/// Area-averaged pressure (Pa) on planes x = const of the latest result in a job's volume.
fn plane_pressures(volume: &str, xs: &[f64], density: f64) -> Vec<f64> {
    use foamcraft_runtime::{DockerRunner, Mount, Runner, StartOptions};
    let r = DockerRunner::local();
    let c = r
        .start(&StartOptions {
            mount: Some(Mount::Volume(volume.into())),
            ..Default::default()
        })
        .unwrap();
    let mut dict = String::from(
        "FoamFile { version 2.0; format ascii; class dictionary; object planes; }\nfunctions\n{\n",
    );
    for (i, x) in xs.iter().enumerate() {
        dict.push_str(&format!(
            "plane{i} {{ type surfaceFieldValue; libs (fieldFunctionObjects); log true; \
             writeFields false; regionType sampledSurface; name plane{i}; \
             sampledSurfaceDict {{ type cuttingPlane; \
             planeType pointAndNormal; pointAndNormalDict {{ point ({x} 0 0); normal (1 0 0); }} \
             interpolate false; }} operation areaAverage; fields (p); }}\n"
        ));
    }
    dict.push_str("}\n");
    let script = format!(
        "cd case && cat > system/planes <<'EOF'\n{dict}EOF\nsimpleFoam -postProcess -dict system/planes -latestTime"
    );
    let out = r.exec_openfoam(&c, &script, &mut |_| {}).unwrap();
    let _ = r.stop(&c);
    let mut log = foamcraft_sim::internal::FieldValueLog::new(xs.len());
    for line in out.stdout.lines() {
        log.feed(line);
    }
    let sample = log
        .samples
        .last()
        .unwrap_or_else(|| panic!("no plane values:\n{}\n{}", out.stdout, out.stderr));
    (0..xs.len())
        .map(|i| sample[&format!("plane{i}")] * density)
        .collect()
}

/// Runs the pipe and returns (result, pressure drop between the planes in Pa).
fn pipe_run(
    name: &str,
    model: &str,
    inlet: Value,
    planes: [f64; 2],
    iterations: u32,
) -> (Value, f64) {
    let (reg, mut s, keys) = project(name, model, "mm");
    set_flow(&reg, &mut s, &keys, inlet, "draft");
    let mesh = reg.execute(&mut s, "mesh.generate", json!({}));
    let out = mesh.and_then(|_| {
        reg.execute(
            &mut s,
            "run.start",
            json!({"wait": true, "iterations": iterations}),
        )
    });
    let result = match out {
        Ok(o) => o["result"].clone(),
        Err(e) => {
            clean(&mut s);
            panic!("run failed: {e}");
        }
    };
    // The solve job works in the volume of the mesh job.
    let project = s.project().unwrap();
    let mesh_job = project.manifest.mesh.clone().unwrap().job_id;
    let volume = foamcraft_core::job::JobStore::new(project.root.join("runs"))
        .load(&mesh_job)
        .unwrap()
        .volume
        .unwrap();
    let density = foamcraft_sim::medium_properties(&foamcraft_core::project::MediumSpec::Water {
        temperature_c: 20.0,
    })
    .density;
    let p = plane_pressures(&volume, &planes, density);
    clean(&mut s);
    (result, p[0] - p[1])
}

/// Hagen-Poiseuille: dp = 128 mu Q L / (pi D^4) for fully developed laminar pipe flow
/// (G. Hagen 1839, J. Poiseuille 1840; e.g. F. M. White, Fluid Mechanics, eq. 6.12).
/// Evaluated between 120 mm and 270 mm of a 300 mm pipe (D = 10 mm, Re ≈ 100, entrance length
/// ≈ 0.06 Re D = 60 mm). Tolerance 5 % (fixed in Task_13; never widen it).
#[test]
#[ignore = "slow CFD reference case; needs Docker and the runtime image"]
fn validation_laminar_pipe_matches_hagen_poiseuille() {
    let (r, dp) = pipe_run(
        "laminar",
        "pipe_d10_l300",
        json!({"type": "velocity", "m_s": 0.01}),
        [0.12, 0.27],
        2000,
    );
    let inlet = &r["openings"][0];
    let q = inlet["flow_m3_s"].as_f64().unwrap();
    let area = q / inlet["velocity_m_s"].as_f64().unwrap();
    let d = (4.0 * area / std::f64::consts::PI).sqrt();
    let medium = foamcraft_sim::medium_properties(&foamcraft_core::project::MediumSpec::Water {
        temperature_c: 20.0,
    });
    let mu = medium.density * medium.kinematic_viscosity;
    let want = 128.0 * mu * q * 0.15 / (std::f64::consts::PI * d.powi(4));
    eprintln!(
        "laminar: dp {dp} Pa, Hagen-Poiseuille {want} Pa, regime {} reason {} cells {}",
        r["regime"], r["reason"], r["cells"]
    );
    assert_eq!(r["regime"], "laminar");
    let deviation = (dp - want).abs() / want;
    assert!(
        deviation <= 0.05,
        "dp {dp} vs {want}: {:.1} %",
        deviation * 100.0
    );
}

/// Blasius: f = 0.3164 Re^-0.25 for smooth pipes, 4000 < Re < 1e5 (H. Blasius 1913; e.g. White,
/// Fluid Mechanics, eq. 6.48); dp = f L / D rho U² / 2. D = 50 mm, U = 1 m/s water (Re ≈ 50 000),
/// evaluated between 1000 mm and 1850 mm of a 2000 mm pipe. Tolerance 15 % (fixed in Task_13;
/// never widen it).
#[test]
#[ignore = "slow CFD reference case; needs Docker and the runtime image"]
fn validation_turbulent_pipe_matches_blasius() {
    let (r, dp) = pipe_run(
        "turbulent",
        "pipe_d50_l2000",
        json!({"type": "velocity", "m_s": 1.0}),
        [1.0, 1.85],
        2000,
    );
    let inlet = &r["openings"][0];
    let q = inlet["flow_m3_s"].as_f64().unwrap();
    let area = q / inlet["velocity_m_s"].as_f64().unwrap();
    let d = (4.0 * area / std::f64::consts::PI).sqrt();
    let u = q / area;
    let medium = foamcraft_sim::medium_properties(&foamcraft_core::project::MediumSpec::Water {
        temperature_c: 20.0,
    });
    let re = u * d / medium.kinematic_viscosity;
    let f = 0.3164 * re.powf(-0.25);
    let want = f * 0.85 / d * 0.5 * medium.density * u * u;
    eprintln!(
        "turbulent: dp {dp} Pa, Blasius {want} Pa (Re {re:.0}), reason {} cells {}",
        r["reason"], r["cells"]
    );
    assert_eq!(r["regime"], "turbulent");
    let deviation = (dp - want).abs() / want;
    assert!(
        deviation <= 0.15,
        "dp {dp} vs {want}: {:.1} %",
        deviation * 100.0
    );
}
