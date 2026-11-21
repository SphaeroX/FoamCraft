// SPDX-License-Identifier: GPL-3.0-or-later
use std::process::{Command, Output};

fn foamcraft(args: &[&str], data: &std::path::Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_foamcraft"))
        .args(args)
        .env("FOAMCRAFT_DATA_DIR", data)
        .output()
        .expect("run foamcraft")
}

fn temp(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("fc-cli-cmd-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

fn stdout(o: &Output) -> String {
    String::from_utf8_lossy(&o.stdout).into_owned()
}

#[test]
fn project_new_creates_a_project_that_info_reads() {
    let base = temp("new");
    let dir = base.join("Wing");
    let out = foamcraft(
        &["project", "new", dir.to_str().unwrap(), "--name", "Wing"],
        &base,
    );
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(dir.join("foamcraft.json").is_file());
    let info = foamcraft(&["project", "info", dir.to_str().unwrap()], &base);
    let json: serde_json::Value = serde_json::from_str(&stdout(&info)).expect("json");
    assert_eq!(json["name"], "Wing");
    assert_eq!(
        json["schema_version"],
        foamcraft_core::project::SCHEMA_VERSION
    );
}

#[test]
fn command_list_names_every_command() {
    let out = foamcraft(&["command", "list"], &temp("list"));
    let text = stdout(&out);
    for name in [
        "project.new",
        "project.rename",
        "history.undo",
        "run.start",
        "run.status",
    ] {
        assert!(text.contains(name), "{name} missing:\n{text}");
    }
}

#[test]
fn command_describe_prints_the_json_schema() {
    let out = foamcraft(
        &["command", "describe", "project.rename"],
        &temp("describe"),
    );
    let json: serde_json::Value = serde_json::from_str(&stdout(&out)).expect("json");
    assert_eq!(json["name"], "project.rename");
    assert_eq!(json["params"]["properties"]["name"]["type"], "string");
    assert_eq!(json["undoable"], true);
}

#[test]
fn command_run_changes_the_project_on_disk() {
    let base = temp("run");
    let dir = base.join("P");
    foamcraft(&["project", "new", dir.to_str().unwrap()], &base);
    let out = foamcraft(
        &[
            "command",
            "run",
            "project.rename",
            r#"{"name":"Renamed"}"#,
            "--project",
            dir.to_str().unwrap(),
        ],
        &base,
    );
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let manifest = std::fs::read_to_string(dir.join("foamcraft.json")).unwrap();
    assert!(manifest.contains("Renamed"));
}

#[test]
fn invalid_parameters_are_a_schema_error() {
    let base = temp("invalid");
    let dir = base.join("P");
    foamcraft(&["project", "new", dir.to_str().unwrap()], &base);
    let out = foamcraft(
        &[
            "command",
            "run",
            "project.rename",
            r#"{"nom":1}"#,
            "--project",
            dir.to_str().unwrap(),
        ],
        &base,
    );
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("command.invalid_params"));
}

#[test]
fn a_face_is_selected_and_grouped_without_a_window() {
    let base = temp("select");
    let dir = base.join("P");
    foamcraft(&["project", "new", dir.to_str().unwrap()], &base);
    let cube = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/data/geometry/cube_binary.stl");
    let batch = serde_json::json!([
        {"name": "geometry.import", "params": {"path": cube}},
        {"name": "surface.select_at", "params": {"point": [0.01, -0.001, 0.01]}},
        {"name": "group.create", "params": {"name": "Inlet"}}
    ]);
    let out = foamcraft(
        &[
            "command",
            "batch",
            &batch.to_string(),
            "--project",
            dir.to_str().unwrap(),
        ],
        &base,
    );
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let results: serde_json::Value = serde_json::from_str(&stdout(&out)).expect("json");
    let key = results[1]["key"].as_str().unwrap().to_string();
    let groups = foamcraft(
        &[
            "command",
            "run",
            "group.list",
            "--project",
            dir.to_str().unwrap(),
        ],
        &base,
    );
    let groups: serde_json::Value = serde_json::from_str(&stdout(&groups)).unwrap();
    assert_eq!(
        groups,
        serde_json::json!([{"name": "Inlet", "keys": [key]}])
    );
}
