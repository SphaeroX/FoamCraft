// SPDX-License-Identifier: GPL-3.0-or-later
//! scenario.* commands.

use std::path::PathBuf;

use foamcraft_core::command::{Registry, Session, core_registry};
use serde_json::{Value, json};

fn setup(name: &str, with_model: bool) -> (Registry, Session, PathBuf) {
    let mut reg = core_registry();
    foamcraft_geometry::commands::register(&mut reg);
    foamcraft_sim::commands::register(&mut reg);
    let base = std::env::temp_dir().join(format!("fc-scn-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&base);
    let mut s = Session::new(base.join("data"));
    let project = base.join("p");
    reg.execute(&mut s, "project.new", json!({"path": project, "name": "S"}))
        .unwrap();
    if with_model {
        let cube = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/data/geometry/cube_binary.stl");
        reg.execute(&mut s, "geometry.import", json!({"path": cube}))
            .unwrap();
    }
    (reg, s, project)
}

fn tunnel(speed: f64) -> Value {
    json!({
        "speed_m_s": speed,
        "direction_deg": 0.0,
        "ground": "fixed",
        "medium": {"kind": "air", "temperature_c": 20.0},
        "quality": "standard"
    })
}

#[test]
fn set_wind_tunnel_returns_reference_values_and_budget() {
    let (reg, mut s, _) = setup("set", true);
    let out = reg
        .execute(&mut s, "scenario.set_wind_tunnel", tunnel(10.0))
        .unwrap();
    let area = out["frontal_area_m2"].as_f64().unwrap();
    assert!((area - 4e-4).abs() < 4e-6, "{area}");
    assert!((out["reference_length_m"].as_f64().unwrap() - 0.02).abs() < 1e-9);
    let re = out["reynolds"].as_f64().unwrap();
    assert!((re - 10.0 * 0.02 / 1.516e-5).abs() < 0.01 * re, "{re}");
    assert_eq!(out["flow"]["code"], "scenario.flow_laminar");
    assert!(!out["flow"]["de"]["text"].as_str().unwrap().is_empty());
    assert!(out["budget"]["cells"].as_u64().unwrap() >= 100_000);
    // Model coordinates are stored as f32, so compare to 1e-6 m.
    assert!((out["tunnel"]["length"].as_f64().unwrap() - 0.42).abs() < 1e-6);
    assert_eq!(out["warnings"], json!([]));
}

#[test]
fn scenario_is_saved_undoable_and_reopened() {
    let (reg, mut s, project) = setup("save", true);
    reg.execute(&mut s, "scenario.set_wind_tunnel", tunnel(10.0))
        .unwrap();
    let mut s2 = Session::new(project.join("..").join("data2"));
    reg.execute(&mut s2, "project.open", json!({"path": project}))
        .unwrap();
    let info = reg.execute(&mut s2, "scenario.info", json!({})).unwrap();
    assert_eq!(info["scenario"]["speed_m_s"], 10.0);
    assert_eq!(info["scenario"]["kind"], "wind_tunnel");
    reg.execute(&mut s, "history.undo", json!({})).unwrap();
    assert_eq!(
        reg.execute(&mut s, "scenario.info", json!({}))
            .unwrap_err()
            .code,
        "scenario.none"
    );
}

#[test]
fn fast_flow_and_tiny_models_are_warned_about() {
    let (reg, mut s, _) = setup("warn", true);
    let out = reg
        .execute(&mut s, "scenario.set_wind_tunnel", tunnel(150.0))
        .unwrap();
    let codes: Vec<_> = out["warnings"]
        .as_array()
        .unwrap()
        .iter()
        .map(|w| w["code"].clone())
        .collect();
    assert!(codes.contains(&json!("scenario.mach_high")), "{codes:?}");
    reg.execute(
        &mut s,
        "geometry.set_unit",
        json!({"id": "model-1", "unit": "um"}),
    )
    .unwrap();
    let out = reg.execute(&mut s, "scenario.info", json!({})).unwrap();
    let codes: Vec<_> = out["warnings"]
        .as_array()
        .unwrap()
        .iter()
        .map(|w| w["code"].clone())
        .collect();
    assert!(codes.contains(&json!("scenario.unit_suspect")), "{codes:?}");
}

#[test]
fn invalid_inputs_and_missing_models_have_codes() {
    let (reg, mut s, _) = setup("empty", false);
    assert_eq!(
        reg.execute(&mut s, "scenario.set_wind_tunnel", tunnel(10.0))
            .unwrap_err()
            .code,
        "scenario.no_geometry"
    );
    let (reg, mut s, _) = setup("invalid", true);
    for bad in [tunnel(0.0), tunnel(-3.0)] {
        assert_eq!(
            reg.execute(&mut s, "scenario.set_wind_tunnel", bad)
                .unwrap_err()
                .code,
            "scenario.invalid_input"
        );
    }
    let mut custom = tunnel(10.0);
    custom["medium"] = json!({"kind": "custom", "density": 0.0, "kinematic_viscosity": 1e-5});
    assert_eq!(
        reg.execute(&mut s, "scenario.set_wind_tunnel", custom)
            .unwrap_err()
            .code,
        "scenario.invalid_input"
    );
}
