// SPDX-License-Identifier: GPL-3.0-or-later
//! surface.* and group.* commands: listing faces, selecting (also by a point in space), naming,
//! grouping.

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
    let base = std::env::temp_dir().join(format!("fc-surf-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&base);
    let mut s = Session::new(base.join("data"));
    let project = base.join("project");
    reg.execute(&mut s, "project.new", json!({"path": project, "name": "S"}))
        .unwrap();
    reg.execute(
        &mut s,
        "geometry.import",
        json!({"path": data("cube_binary.stl")}),
    )
    .unwrap();
    (reg, s, project)
}

fn run(reg: &Registry, s: &mut Session, name: &str, params: Value) -> Value {
    reg.execute(s, name, params)
        .unwrap_or_else(|e| panic!("{name}: {e}"))
}

#[test]
fn surfaces_of_the_cube_are_listed_with_stable_keys() {
    let (reg, mut s, _) = setup("list");
    let list = run(&reg, &mut s, "surface.list", json!({}));
    let keys: Vec<_> = list
        .as_array()
        .unwrap()
        .iter()
        .map(|f| f["key"].as_str().unwrap().to_string())
        .collect();
    assert_eq!(
        keys,
        (1..=6)
            .map(|i| format!("model-1:face-{i}"))
            .collect::<Vec<_>>()
    );
    assert_eq!(list[0]["part"], "model-1:part-1");
    assert!(list[0]["area_m2"].as_f64().unwrap() > 0.0);
}

#[test]
fn a_point_in_space_selects_the_nearest_face() {
    let (reg, mut s, _) = setup("point");
    // Just in front of the -Y side of the 20 mm cube, in metres.
    let out = run(
        &reg,
        &mut s,
        "surface.select_at",
        json!({"point": [0.01, -0.001, 0.01]}),
    );
    let key = out["key"].as_str().unwrap().to_string();
    let list = run(&reg, &mut s, "surface.list", json!({}));
    let face = list
        .as_array()
        .unwrap()
        .iter()
        .find(|f| f["key"] == json!(key))
        .unwrap();
    assert!(
        (face["normal"][1].as_f64().unwrap() + 1.0).abs() < 1e-9,
        "{face}"
    );
    assert_eq!(
        run(&reg, &mut s, "surface.selection", json!({})),
        json!([key])
    );
}

#[test]
fn selection_modes_and_adjacent_faces() {
    let (reg, mut s, _) = setup("modes");
    run(
        &reg,
        &mut s,
        "surface.select",
        json!({"keys": ["model-1:face-1"]}),
    );
    run(
        &reg,
        &mut s,
        "surface.select",
        json!({"keys": ["model-1:face-2"], "mode": "add"}),
    );
    run(
        &reg,
        &mut s,
        "surface.select",
        json!({"keys": ["model-1:face-1"], "mode": "toggle"}),
    );
    assert_eq!(
        run(&reg, &mut s, "surface.selection", json!({})),
        json!(["model-1:face-2"])
    );
    let out = run(&reg, &mut s, "surface.select_adjacent", json!({}));
    assert_eq!(
        out.as_array().unwrap().len(),
        5,
        "a cube face has four neighbours: {out}"
    );
    run(&reg, &mut s, "surface.clear_selection", json!({}));
    assert_eq!(run(&reg, &mut s, "surface.selection", json!({})), json!([]));
    let err = reg
        .execute(
            &mut s,
            "surface.select",
            json!({"keys": ["model-1:face-99"]}),
        )
        .unwrap_err();
    assert_eq!(err.code, "surface.not_found");
}

#[test]
fn names_and_groups_survive_saving_and_opening() {
    let (reg, mut s, project) = setup("persist");
    run(
        &reg,
        &mut s,
        "surface.rename",
        json!({"key": "model-1:face-3", "name": "Front"}),
    );
    run(
        &reg,
        &mut s,
        "surface.select",
        json!({"keys": ["model-1:face-1", "model-1:face-2"]}),
    );
    run(&reg, &mut s, "group.create", json!({"name": "Inlet"}));
    run(
        &reg,
        &mut s,
        "group.add",
        json!({"name": "Inlet", "keys": ["model-1:face-4"]}),
    );
    run(
        &reg,
        &mut s,
        "group.remove",
        json!({"name": "Inlet", "keys": ["model-1:face-2"]}),
    );

    let mut s2 = Session::new(project.join("..").join("data2"));
    run(&reg, &mut s2, "project.open", json!({"path": project}));
    let groups = run(&reg, &mut s2, "group.list", json!({}));
    assert_eq!(
        groups,
        json!([{"name": "Inlet", "keys": ["model-1:face-1", "model-1:face-4"]}])
    );
    let list = run(&reg, &mut s2, "surface.list", json!({}));
    assert_eq!(list[2]["name"], "Front");
    assert_eq!(list[0]["groups"], json!(["Inlet"]));
    let keys_before: Vec<_> = list
        .as_array()
        .unwrap()
        .iter()
        .map(|f| f["key"].clone())
        .collect();
    let again = run(&reg, &mut s2, "surface.list", json!({}));
    let keys_after: Vec<_> = again
        .as_array()
        .unwrap()
        .iter()
        .map(|f| f["key"].clone())
        .collect();
    assert_eq!(keys_before, keys_after);
}

#[test]
fn group_errors_have_codes() {
    let (reg, mut s, _) = setup("errors");
    let err = reg
        .execute(&mut s, "group.create", json!({"name": "Empty"}))
        .unwrap_err();
    assert_eq!(err.code, "group.empty");
    run(
        &reg,
        &mut s,
        "group.create",
        json!({"name": "A", "keys": ["model-1:face-1"]}),
    );
    let err = reg
        .execute(
            &mut s,
            "group.create",
            json!({"name": "A", "keys": ["model-1:face-2"]}),
        )
        .unwrap_err();
    assert_eq!(err.code, "group.exists");
    let err = reg
        .execute(&mut s, "group.delete", json!({"name": "B"}))
        .unwrap_err();
    assert_eq!(err.code, "group.not_found");
}

#[test]
fn every_undoable_surface_and_group_command_undoes_exactly() {
    let (reg, _, _) = setup("undo-specs");
    let names: Vec<_> = reg
        .specs()
        .filter(|s| {
            s.undoable
                && (s.name.starts_with("surface.")
                    || s.name.starts_with("group.")
                    || s.name == "geometry.set_crease_angle")
        })
        .map(|s| s.name)
        .collect();
    assert!(names.len() >= 6, "{names:?}");
    for name in names {
        let (reg, mut s, _) = setup(&format!("undo-{name}"));
        // A group to add to, remove from and delete.
        run(
            &reg,
            &mut s,
            "group.create",
            json!({"name": "Inlet", "keys": ["model-1:face-1", "model-1:face-2"]}),
        );
        let snapshot = |reg: &Registry, s: &mut Session| {
            (
                run(reg, s, "group.list", json!({})),
                run(reg, s, "surface.list", json!({})),
                run(reg, s, "geometry.list", json!({})),
            )
        };
        let before = snapshot(&reg, &mut s);
        let example = reg.describe(name).unwrap().example.clone();
        run(&reg, &mut s, name, example);
        let after = snapshot(&reg, &mut s);
        assert_ne!(before, after, "{name} example must change the project");
        run(&reg, &mut s, "history.undo", json!({}));
        assert_eq!(snapshot(&reg, &mut s), before, "{name} undo");
        run(&reg, &mut s, "history.redo", json!({}));
        assert_eq!(snapshot(&reg, &mut s), after, "{name} redo");
    }
}
