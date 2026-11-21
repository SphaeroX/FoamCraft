// SPDX-License-Identifier: GPL-3.0-or-later
//! Project folder with a versioned manifest.

use foamcraft_core::project::{MANIFEST_FILE, Project, SCHEMA_VERSION};
use foamcraft_core::units::DisplayUnits;

fn temp(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("fc-project-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

#[test]
fn new_project_creates_folder_layout_and_manifest() {
    let dir = temp("new");
    let project = Project::create(&dir, "Wing").unwrap();
    assert!(dir.join(MANIFEST_FILE).is_file());
    for sub in ["geometry", "runs", "cache"] {
        assert!(dir.join(sub).is_dir(), "{sub} missing");
    }
    assert_eq!(project.manifest.name, "Wing");
    assert_eq!(project.manifest.schema_version, SCHEMA_VERSION);
}

#[test]
fn saved_project_opens_identically() {
    let dir = temp("roundtrip");
    let mut project = Project::create(&dir, "Car").unwrap();
    project.manifest.display_units = DisplayUnits::Imperial;
    project.manifest.runs.push("job-1".into());
    project.save().unwrap();
    let opened = Project::open(&dir).unwrap();
    assert_eq!(opened.manifest, project.manifest);
}

#[test]
fn creating_in_a_folder_with_a_project_fails() {
    let dir = temp("twice");
    Project::create(&dir, "A").unwrap();
    assert_eq!(
        Project::create(&dir, "B").unwrap_err().code,
        "project.already_exists"
    );
}

#[test]
fn opening_a_folder_without_manifest_fails() {
    let dir = temp("empty");
    std::fs::create_dir_all(&dir).unwrap();
    assert_eq!(Project::open(&dir).unwrap_err().code, "project.not_found");
}

#[test]
fn higher_schema_version_is_rejected() {
    let dir = temp("future");
    Project::create(&dir, "Future").unwrap();
    let path = dir.join(MANIFEST_FILE);
    let mut json: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    json["schema_version"] = serde_json::json!(SCHEMA_VERSION + 1);
    std::fs::write(&path, json.to_string()).unwrap();
    assert_eq!(
        Project::open(&dir).unwrap_err().code,
        "project.schema_too_new"
    );
}

#[test]
fn broken_manifest_is_reported() {
    let dir = temp("broken");
    Project::create(&dir, "X").unwrap();
    std::fs::write(dir.join(MANIFEST_FILE), "{ not json").unwrap();
    assert_eq!(
        Project::open(&dir).unwrap_err().code,
        "project.invalid_manifest"
    );
}

#[test]
fn version_one_manifest_is_migrated_and_keeps_its_data() {
    let dir = temp("v1");
    std::fs::create_dir_all(&dir).unwrap();
    let v1 = r#"{"schema_version":1,"name":"Old","created":5,"display_units":"imperial","runs":["job-1"]}"#;
    std::fs::write(dir.join(MANIFEST_FILE), v1).unwrap();
    let project = Project::open(&dir).unwrap();
    assert_eq!(project.manifest.schema_version, SCHEMA_VERSION);
    assert_eq!(project.manifest.name, "Old");
    assert_eq!(project.manifest.runs, ["job-1"]);
    assert!(project.manifest.geometry.is_empty());
}

#[test]
fn geometry_entries_round_trip() {
    use foamcraft_core::project::GeometryEntry;
    let dir = temp("geo");
    let mut project = Project::create(&dir, "G").unwrap();
    project.manifest.geometry.push(GeometryEntry {
        id: "model-1".into(),
        name: "cube.stl".into(),
        file: "geometry/model-1.stl".into(),
        source: None,
        unit: "mm".into(),
        scale: 1.0,
        rotation_deg: [0.0, 90.0, 0.0],
        translation: [0.0, 0.0, 0.01],
        fix_normals: true,
        crease_angle_deg: 30.0,
    });
    project.save().unwrap();
    assert_eq!(Project::open(&dir).unwrap().manifest, project.manifest);
}
