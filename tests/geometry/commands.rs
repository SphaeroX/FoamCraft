// SPDX-License-Identifier: GPL-3.0-or-later
//! geometry.* commands on a project.

use std::path::PathBuf;

use foamcraft_core::command::{Registry, Session, core_registry};
use serde_json::{Value, json};

fn data(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/data/geometry")
        .join(name)
}

fn setup(name: &str) -> (Registry, Session, PathBuf) {
    let mut reg = core_registry();
    foamcraft_geometry::commands::register(&mut reg);
    let base = std::env::temp_dir().join(format!("fc-geo-cmd-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&base);
    let mut s = Session::new(base.join("data"));
    let project = base.join("project");
    reg.execute(&mut s, "project.new", json!({"path": project, "name": "G"}))
        .unwrap();
    (reg, s, project)
}

fn import(reg: &Registry, s: &mut Session, file: &str) -> Value {
    reg.execute(s, "geometry.import", json!({"path": data(file)}))
        .unwrap()
}

#[test]
fn import_copies_the_file_and_returns_a_diagnosis_with_texts() {
    for file in ["cube_ascii.stl", "cube_binary.stl", "cube.obj", "cube.3mf"] {
        let (reg, mut s, project) = setup(&format!("import-{file}"));
        let out = import(&reg, &mut s, file);
        assert_eq!(out["id"], "model-1", "{file}");
        assert_eq!(out["diagnosis"]["triangles"], 12, "{file}");
        assert_eq!(out["diagnosis"]["watertight"], true, "{file}");
        assert_eq!(out["suggested_unit"], "mm", "{file}");
        assert_eq!(out["unit"], "mm", "{file}");
        let msg = &out["messages"][0];
        assert_eq!(msg["code"], "geometry.ok");
        assert!(msg["en"]["text"].as_str().unwrap().contains("closed"));
        assert!(msg["de"]["text"].as_str().unwrap().contains("geschlossen"));
        let copied = project.join(out["file"].as_str().unwrap());
        assert!(copied.is_file(), "{file}: copy missing");
        let size = &out["size_m"];
        assert!(
            (size[0].as_f64().unwrap() - 0.02).abs() < 1e-9,
            "{file}: {size}"
        );
    }
}

#[test]
fn import_of_an_open_model_reports_it() {
    let (reg, mut s, _) = setup("open");
    let out = import(&reg, &mut s, "cube_open.stl");
    let codes: Vec<_> = out["messages"]
        .as_array()
        .unwrap()
        .iter()
        .map(|m| m["code"].clone())
        .collect();
    assert!(codes.contains(&json!("geometry.open")), "{codes:?}");
    assert_eq!(out["diagnosis"]["boundary_loops"], 1);
}

#[test]
fn unknown_file_type_fails_with_a_code() {
    let (reg, mut s, _) = setup("bad");
    let err = reg
        .execute(
            &mut s,
            "geometry.import",
            json!({"path": data("README.md")}),
        )
        .unwrap_err();
    assert_eq!(err.code, "geometry.unsupported_format");
}

#[test]
fn unit_rotation_and_position_are_saved_in_the_project() {
    let (reg, mut s, project) = setup("settings");
    import(&reg, &mut s, "cube_binary.stl");
    reg.execute(
        &mut s,
        "geometry.set_unit",
        json!({"id": "model-1", "unit": "cm"}),
    )
    .unwrap();
    reg.execute(
        &mut s,
        "geometry.set_transform",
        json!({"id": "model-1", "rotation_deg": [0, 45, 0], "translation": [1, 2, 3]}),
    )
    .unwrap();
    let info = reg
        .execute(&mut s, "geometry.place_on_ground", json!({"id": "model-1"}))
        .unwrap();
    assert!(
        info["bounds_m"]["min"][2].as_f64().unwrap().abs() < 1e-6,
        "{info}"
    );
    let manifest: Value =
        serde_json::from_str(&std::fs::read_to_string(project.join("foamcraft.json")).unwrap())
            .unwrap();
    let entry = &manifest["geometry"][0];
    assert_eq!(entry["unit"], "cm");
    assert_eq!(entry["rotation_deg"], json!([0.0, 45.0, 0.0]));
    assert_eq!(entry["translation"][0], 1.0);
    let err = reg
        .execute(
            &mut s,
            "geometry.set_unit",
            json!({"id": "model-1", "unit": "yard"}),
        )
        .unwrap_err();
    assert_eq!(err.code, "geometry.invalid_unit");
}

#[test]
fn fix_normals_removes_the_finding() {
    let (reg, mut s, _) = setup("fix");
    import(&reg, &mut s, "cube_flipped_face.stl");
    let info = reg
        .execute(&mut s, "geometry.fix_normals", json!({"id": "model-1"}))
        .unwrap();
    assert_eq!(info["diagnosis"]["inconsistent_normals"], false, "{info}");
}

#[test]
fn every_undoable_geometry_command_undoes_and_redoes_exactly() {
    let (reg, _, _) = setup("specs");
    let names: Vec<_> = reg
        .specs()
        .filter(|s| s.undoable && s.name.starts_with("geometry."))
        .map(|s| s.name)
        .collect();
    assert!(names.len() >= 5, "{names:?}");
    for name in names {
        let (reg, mut s, _) = setup(&format!("undo-{name}"));
        let mut example = reg.describe(name).unwrap().example.clone();
        if name == "geometry.import" {
            example["path"] = json!(data("cube_flipped_face.stl"));
        } else {
            import(&reg, &mut s, "cube_flipped_face.stl");
            // Lift the model so that placing it on the ground changes something.
            reg.execute(
                &mut s,
                "geometry.set_transform",
                json!({"id": "model-1", "translation": [0.0, 0.0, 0.5]}),
            )
            .unwrap();
        }
        let before = reg.execute(&mut s, "project.info", json!({})).unwrap();
        let list_before = reg.execute(&mut s, "geometry.list", json!({})).unwrap();
        reg.execute(&mut s, name, example)
            .unwrap_or_else(|e| panic!("{name}: {e}"));
        let list_after = reg.execute(&mut s, "geometry.list", json!({})).unwrap();
        assert_ne!(list_before, list_after, "{name} must change the project");
        reg.execute(&mut s, "history.undo", json!({})).unwrap();
        assert_eq!(
            reg.execute(&mut s, "geometry.list", json!({})).unwrap(),
            list_before,
            "{name} undo"
        );
        assert_eq!(
            reg.execute(&mut s, "project.info", json!({})).unwrap(),
            before
        );
        reg.execute(&mut s, "history.redo", json!({})).unwrap();
        assert_eq!(
            reg.execute(&mut s, "geometry.list", json!({})).unwrap(),
            list_after,
            "{name} redo"
        );
    }
}
