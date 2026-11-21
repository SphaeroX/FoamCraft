// SPDX-License-Identifier: GPL-3.0-or-later
//! Command registry: schemas, validation, undo and redo, events.

use std::sync::{Arc, Mutex};

use foamcraft_core::command::{Session, core_registry};
use serde_json::{Value, json};

fn temp(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("fc-cmd-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

fn session_with_project(name: &str) -> (Session, foamcraft_core::command::Registry) {
    let reg = core_registry();
    let base = temp(name);
    let mut s = Session::new(base.join("data"));
    let path = base.join("project");
    reg.execute(&mut s, "project.new", json!({"path": path, "name": "Test"}))
        .unwrap();
    (s, reg)
}

#[test]
fn every_command_has_schemas_summary_and_example() {
    let reg = core_registry();
    let names: Vec<_> = reg.specs().map(|s| s.name).collect();
    for needed in [
        "project.new",
        "project.open",
        "project.info",
        "project.close",
        "project.rename",
        "project.set_display_units",
        "project.recent",
        "history.undo",
        "history.redo",
        "history.state",
    ] {
        assert!(names.contains(&needed), "{needed} missing");
    }
    for spec in reg.specs() {
        assert!(!spec.summary.is_empty(), "{}", spec.name);
        assert!(spec.params.is_object(), "{} params schema", spec.name);
        assert!(spec.result.is_object(), "{} result schema", spec.name);
        let (domain, verb) = spec.name.split_once('.').unwrap();
        assert!(!domain.is_empty() && !verb.is_empty());
        assert!(
            spec.name
                .chars()
                .all(|c| c.is_ascii_lowercase() || c == '.' || c == '_')
        );
    }
}

#[test]
fn unknown_command_and_invalid_params_have_codes() {
    let (mut s, reg) = session_with_project("invalid");
    assert_eq!(
        reg.execute(&mut s, "nope.nothing", json!({}))
            .unwrap_err()
            .code,
        "command.unknown"
    );
    let err = reg
        .execute(&mut s, "project.rename", json!({"title": 3}))
        .unwrap_err();
    assert_eq!(err.code, "command.invalid_params");
    let err = reg
        .execute(
            &mut s,
            "project.set_display_units",
            json!({"units": "cubits"}),
        )
        .unwrap_err();
    assert_eq!(err.code, "command.invalid_params");
}

#[test]
fn changing_commands_need_an_open_project() {
    let reg = core_registry();
    let mut s = Session::new(temp("none").join("data"));
    let err = reg
        .execute(&mut s, "project.rename", json!({"name": "X"}))
        .unwrap_err();
    assert_eq!(err.code, "project.none_open");
}

fn info(reg: &foamcraft_core::command::Registry, s: &mut Session) -> Value {
    reg.execute(s, "project.info", json!({})).unwrap()
}

#[test]
fn undo_and_redo_restore_state_exactly_for_every_undoable_command() {
    let reg = core_registry();
    let undoable: Vec<_> = reg.specs().filter(|s| s.undoable).map(|s| s.name).collect();
    assert!(!undoable.is_empty());
    for name in undoable {
        let (mut s, reg) = session_with_project(&format!("undo-{name}"));
        let before = info(&reg, &mut s);
        let example = reg.describe(name).unwrap().example.clone();
        reg.execute(&mut s, name, example).unwrap();
        let after = info(&reg, &mut s);
        assert_ne!(before, after, "{name} example must change the project");
        reg.execute(&mut s, "history.undo", json!({})).unwrap();
        assert_eq!(info(&reg, &mut s), before, "{name} undo");
        reg.execute(&mut s, "history.redo", json!({})).unwrap();
        assert_eq!(info(&reg, &mut s), after, "{name} redo");
        // The saved manifest matches the session state too.
        let path = after["path"].as_str().unwrap().to_string();
        reg.execute(&mut s, "project.open", json!({"path": path}))
            .unwrap();
        assert_eq!(info(&reg, &mut s), after, "{name} saved");
    }
}

#[test]
fn undo_without_history_has_a_code_and_new_change_clears_redo() {
    let (mut s, reg) = session_with_project("history");
    assert_eq!(
        reg.execute(&mut s, "history.undo", json!({}))
            .unwrap_err()
            .code,
        "history.nothing_to_undo"
    );
    reg.execute(&mut s, "project.rename", json!({"name": "A"}))
        .unwrap();
    reg.execute(&mut s, "history.undo", json!({})).unwrap();
    reg.execute(&mut s, "project.rename", json!({"name": "B"}))
        .unwrap();
    let state = reg.execute(&mut s, "history.state", json!({})).unwrap();
    assert_eq!(
        state,
        json!({"can_undo": true, "can_redo": false, "undo": "project.rename", "redo": null})
    );
}

#[test]
fn changes_emit_events() {
    let (mut s, reg) = session_with_project("events");
    let seen = Arc::new(Mutex::new(Vec::new()));
    let sink = seen.clone();
    s.events()
        .subscribe(move |e| sink.lock().unwrap().push(e.name.clone()));
    reg.execute(&mut s, "project.rename", json!({"name": "Z"}))
        .unwrap();
    let names = seen.lock().unwrap().clone();
    assert!(names.contains(&"project.changed".to_string()), "{names:?}");
    assert!(names.contains(&"history.changed".to_string()), "{names:?}");
}

#[test]
fn recent_projects_are_remembered_newest_first() {
    let reg = core_registry();
    let base = temp("recent");
    let mut s = Session::new(base.join("data"));
    for name in ["One", "Two"] {
        let path = base.join(name);
        reg.execute(&mut s, "project.new", json!({"path": path, "name": name}))
            .unwrap();
    }
    let mut s2 = Session::new(base.join("data"));
    let recent = reg.execute(&mut s2, "project.recent", json!({})).unwrap();
    let names: Vec<_> = recent
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r["name"].clone())
        .collect();
    assert_eq!(names, [json!("Two"), json!("One")]);
}

#[test]
fn the_same_project_spelled_differently_is_listed_once() {
    use foamcraft_core::command::recent::{RecentProject, load, remember};
    let dir = std::env::temp_dir().join(format!("fc-recent-spelling-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    remember(
        &dir,
        RecentProject {
            path: r"C:\Work\Projects\Demo".into(),
            name: "Demo".into(),
        },
    );
    remember(
        &dir,
        RecentProject {
            path: "C:/Work/Projects/Demo".into(),
            name: "Demo".into(),
        },
    );
    remember(
        &dir,
        RecentProject {
            path: "c:/work/projects/demo/".into(),
            name: "Demo".into(),
        },
    );
    let list = load(&dir);
    assert_eq!(list.len(), 1, "{list:?}");
}

#[test]
fn a_project_without_a_path_is_created_in_the_default_folder() {
    let reg = core_registry();
    let base = temp("default-folder");
    let mut s = Session::new(base.join("data"));
    s.set_projects_dir(base.join("projects"));
    let loc = reg
        .execute(&mut s, "project.default_location", json!({"name": "Wing"}))
        .unwrap();
    assert_eq!(loc["folder"], json!(base.join("projects")));
    assert_eq!(loc["path"], json!(base.join("projects").join("Wing")));
    let a = reg
        .execute(&mut s, "project.new", json!({"name": "Wing"}))
        .unwrap();
    assert_eq!(a["path"], json!(base.join("projects").join("Wing")));
    // The name is taken now: the next project gets a free folder, nothing is overwritten.
    let loc = reg
        .execute(&mut s, "project.default_location", json!({"name": "Wing"}))
        .unwrap();
    assert_eq!(loc["path"], json!(base.join("projects").join("Wing 2")));
    let b = reg
        .execute(&mut s, "project.new", json!({"name": "Wing"}))
        .unwrap();
    assert_eq!(b["path"], json!(base.join("projects").join("Wing 2")));
    assert!(base.join("projects/Wing/foamcraft.json").is_file());
    assert!(base.join("projects/Wing 2/foamcraft.json").is_file());
}

#[test]
fn default_folder_names_avoid_characters_a_file_system_rejects() {
    let reg = core_registry();
    let base = temp("folder-names");
    let mut s = Session::new(base.join("data"));
    s.set_projects_dir(base.join("projects"));
    let info = reg
        .execute(&mut s, "project.new", json!({"name": "Fan: 20 m/s? "}))
        .unwrap();
    // The project keeps its name; only the folder is cleaned.
    assert_eq!(info["name"], "Fan: 20 m/s? ");
    assert_eq!(
        info["path"],
        json!(base.join("projects").join("Fan_ 20 m_s_"))
    );
    // A name without any usable character still gives a folder.
    let info = reg
        .execute(&mut s, "project.new", json!({"name": " .. "}))
        .unwrap();
    assert_eq!(info["path"], json!(base.join("projects").join("Project")));
}

#[test]
fn the_default_location_without_a_name_is_the_projects_folder() {
    let reg = core_registry();
    let base = temp("default-location");
    let mut s = Session::new(base.join("data"));
    s.set_projects_dir(base.join("projects"));
    let loc = reg
        .execute(&mut s, "project.default_location", json!({}))
        .unwrap();
    assert_eq!(loc["folder"], json!(base.join("projects")));
    assert!(loc["path"].is_null());
}

#[test]
fn changes_from_background_work_are_applied_before_the_next_command() {
    let (mut s, reg) = session_with_project("updates");
    // Background work keeps a handle and hands in a change while no command runs.
    let updates = s.updates();
    let worker = std::thread::spawn(move || {
        updates.push(|session| {
            if let Ok(project) = session.project() {
                project.manifest.name = "Renamed in the background".into();
            }
        });
    });
    worker.join().unwrap();
    assert_eq!(
        s.project_ref().unwrap().manifest.name,
        "Test",
        "not yet applied"
    );
    assert_eq!(info(&reg, &mut s)["name"], "Renamed in the background");
    // Applied once, and it is no undo step of its own.
    assert_eq!(
        reg.execute(&mut s, "history.undo", json!({}))
            .unwrap_err()
            .code,
        "history.nothing_to_undo"
    );
    s.apply_updates();
    assert_eq!(info(&reg, &mut s)["name"], "Renamed in the background");
}
