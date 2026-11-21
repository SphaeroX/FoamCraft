// SPDX-License-Identifier: GPL-3.0-or-later
//! geometry.openings, geometry.cap_opening, cap.remove, cap.list,
//! geometry.fluid_check and scenario.set_boundary_role.

use std::f64::consts::PI;
use std::path::PathBuf;

use foamcraft_core::command::{Registry, Session, core_registry};
use serde_json::{Value, json};

fn setup(name: &str, model: &str) -> (Registry, Session, PathBuf) {
    let mut reg = core_registry();
    foamcraft_geometry::commands::register(&mut reg);
    let base = std::env::temp_dir().join(format!("fc-caps-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&base);
    let mut s = Session::new(base.join("data"));
    let project = base.join("project");
    reg.execute(&mut s, "project.new", json!({"path": project, "name": "C"}))
        .unwrap();
    let file = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/data/geometry/internal")
        .join(format!("{model}.stl"));
    reg.execute(
        &mut s,
        "geometry.import",
        json!({"path": file, "unit": "mm"}),
    )
    .unwrap();
    (reg, s, project)
}

fn run(reg: &Registry, s: &mut Session, name: &str, params: Value) -> Value {
    reg.execute(s, name, params)
        .unwrap_or_else(|e| panic!("{name}: {e}"))
}

#[test]
fn an_open_pipe_is_capped_marked_and_sealed() {
    let (reg, mut s, project) = setup("pipe", "pipe_open");
    let report = run(
        &reg,
        &mut s,
        "geometry.openings",
        json!({"model": "model-1"}),
    );
    assert_eq!(report["kind"], "open_shell");
    assert_eq!(report["openings"].as_array().unwrap().len(), 2);
    let area = report["openings"][0]["area_m2"].as_f64().unwrap();
    assert!((area - PI * 1e-4).abs() < 0.01 * PI * 1e-4, "{area}");

    // Not sealed before capping.
    let check = run(&reg, &mut s, "geometry.fluid_check", json!({}));
    assert_eq!(check["sealed"], false);
    assert_eq!(check["warnings"][0]["code"], "geometry.not_sealed");

    let a = run(
        &reg,
        &mut s,
        "geometry.cap_opening",
        json!({"model": "model-1", "point": [0.0, 0.0, 0.009]}),
    );
    assert_eq!(a["key"], "model-1:cap-1");
    assert!(a["centroid_m"][0].as_f64().unwrap().abs() < 1e-6);
    let b = run(
        &reg,
        &mut s,
        "geometry.cap_opening",
        json!({"model": "model-1", "point": [0.05, 0.009, 0.0]}),
    );
    assert_eq!(b["key"], "model-1:cap-2");
    let again = reg
        .execute(
            &mut s,
            "geometry.cap_opening",
            json!({"model": "model-1", "point": [0.05, 0.0, 0.0]}),
        )
        .unwrap_err();
    assert_eq!(again.code, "geometry.already_capped");

    let report = run(
        &reg,
        &mut s,
        "geometry.openings",
        json!({"model": "model-1"}),
    );
    let capped: Vec<_> = report["openings"]
        .as_array()
        .unwrap()
        .iter()
        .map(|o| o["cap"].clone())
        .collect();
    assert!(capped.contains(&json!("model-1:cap-1")) && capped.contains(&json!("model-1:cap-2")));

    let check = run(&reg, &mut s, "geometry.fluid_check", json!({}));
    assert_eq!(check["sealed"], true);
    let p = check["fluid_point_m"].as_array().unwrap();
    let x = p[0].as_f64().unwrap();
    assert!(x > 0.0 && x < 0.05, "{p:?}");

    run(
        &reg,
        &mut s,
        "scenario.set_boundary_role",
        json!({"keys": ["model-1:cap-1"], "role": "inlet"}),
    );
    run(
        &reg,
        &mut s,
        "scenario.set_boundary_role",
        json!({"keys": ["model-1:cap-2"], "role": "outlet"}),
    );
    let bad = reg
        .execute(
            &mut s,
            "scenario.set_boundary_role",
            json!({"keys": ["model-1:cap-9"], "role": "inlet"}),
        )
        .unwrap_err();
    assert_eq!(bad.code, "surface.not_found");
    // Caps can be selected like faces.
    run(
        &reg,
        &mut s,
        "surface.select",
        json!({"keys": ["model-1:cap-1"]}),
    );

    // Keys and roles survive saving and opening.
    run(&reg, &mut s, "project.close", json!({}));
    run(&reg, &mut s, "project.open", json!({"path": project}));
    let caps = run(&reg, &mut s, "cap.list", json!({}));
    assert_eq!(caps[0]["key"], "model-1:cap-1");
    assert_eq!(caps[0]["role"], "inlet");
    assert_eq!(caps[1]["key"], "model-1:cap-2");
    assert_eq!(caps[1]["role"], "outlet");
    let area = caps[1]["area_m2"].as_f64().unwrap();
    assert!((area - PI * 1e-4).abs() < 0.01 * PI * 1e-4);
}

#[test]
fn caps_follow_the_model_and_undo_removes_them() {
    let (reg, mut s, _) = setup("move", "pipe_open");
    run(
        &reg,
        &mut s,
        "geometry.cap_opening",
        json!({"model": "model-1", "point": [0.0, 0.0, 0.0]}),
    );
    run(
        &reg,
        &mut s,
        "geometry.set_transform",
        json!({"id": "model-1", "translation": [1.0, 0.0, 0.0]}),
    );
    let caps = run(&reg, &mut s, "cap.list", json!({}));
    assert!((caps[0]["centroid_m"][0].as_f64().unwrap() - 1.0).abs() < 1e-6);
    run(&reg, &mut s, "history.undo", json!({}));
    run(&reg, &mut s, "history.undo", json!({}));
    assert_eq!(run(&reg, &mut s, "cap.list", json!({})), json!([]));
    run(&reg, &mut s, "history.redo", json!({}));
    assert_eq!(
        run(&reg, &mut s, "cap.list", json!({}))
            .as_array()
            .unwrap()
            .len(),
        1
    );
    run(&reg, &mut s, "cap.remove", json!({"key": "model-1:cap-1"}));
    assert_eq!(run(&reg, &mut s, "cap.list", json!({})), json!([]));
}

#[test]
fn a_curved_opening_warns() {
    let (reg, mut s, _) = setup("saddle", "pipe_saddle");
    let cap = run(
        &reg,
        &mut s,
        "geometry.cap_opening",
        json!({"model": "model-1", "point": [0.055, 0.0, 0.009]}),
    );
    assert_eq!(cap["warnings"][0]["code"], "geometry.opening_not_flat");
    assert!(
        !cap["warnings"][0]["de"]["text"]
            .as_str()
            .unwrap()
            .is_empty()
    );
}

#[test]
fn a_thick_tube_is_capped_by_clicking_its_inner_wall() {
    let (reg, mut s, _) = setup("tube", "tube_thick");
    let report = run(
        &reg,
        &mut s,
        "geometry.openings",
        json!({"model": "model-1"}),
    );
    assert_eq!(report["kind"], "thick_walled");
    // The inner wall: the cylindrical face with the area 2 pi 7 mm 40 mm.
    let faces = run(&reg, &mut s, "surface.list", json!({}));
    let inner = faces
        .as_array()
        .unwrap()
        .iter()
        .find(|f| (f["area_m2"].as_f64().unwrap() - 2.0 * PI * 0.007 * 0.04).abs() < 2e-5)
        .expect("inner wall")["key"]
        .as_str()
        .unwrap()
        .to_string();
    for x in [0.001, 0.039] {
        let cap = run(
            &reg,
            &mut s,
            "geometry.cap_opening",
            json!({"model": "model-1", "point": [x, 0.0, 0.007], "face": inner}),
        );
        let area = cap["area_m2"].as_f64().unwrap();
        assert!((area - PI * 49e-6).abs() < 0.01 * PI * 49e-6, "{area}");
    }
    let check = run(&reg, &mut s, "geometry.fluid_check", json!({}));
    assert_eq!(check["sealed"], true);
    let p: Vec<f64> = check["fluid_point_m"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_f64().unwrap())
        .collect();
    assert!((p[1] * p[1] + p[2] * p[2]).sqrt() < 0.007, "{p:?}");
}

#[test]
fn every_test_body_can_be_sealed_by_command() {
    for model in [
        "pipe_open",
        "tee_open",
        "housing_bores",
        "annulus_open",
        "pipe_saddle",
        "tube_thick",
    ] {
        let (reg, mut s, _) = setup(model, model);
        let report = run(
            &reg,
            &mut s,
            "geometry.openings",
            json!({"model": "model-1"}),
        );
        for o in report["openings"].as_array().unwrap() {
            run(
                &reg,
                &mut s,
                "geometry.cap_opening",
                json!({"model": "model-1", "point": o["centroid_m"]}),
            );
        }
        let check = run(&reg, &mut s, "geometry.fluid_check", json!({}));
        assert_eq!(check["sealed"], true, "{model}");
    }
}

#[test]
fn boundary_roles_are_listed() {
    let (reg, mut s, _) = setup("roles", "pipe_open");
    run(
        &reg,
        &mut s,
        "geometry.cap_opening",
        json!({"model": "model-1", "point": [0.0, 0.0, 0.0]}),
    );
    assert_eq!(
        run(&reg, &mut s, "scenario.boundary_roles", json!({})),
        json!({})
    );
    run(
        &reg,
        &mut s,
        "scenario.set_boundary_role",
        json!({"keys": ["model-1:cap-1"], "role": "inlet"}),
    );
    assert_eq!(
        run(&reg, &mut s, "scenario.boundary_roles", json!({})),
        json!({"model-1:cap-1": "inlet"})
    );
}
