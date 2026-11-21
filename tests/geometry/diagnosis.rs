// SPDX-License-Identifier: GPL-3.0-or-later
//! Diagnosis of triangle meshes.

use std::path::PathBuf;

use foamcraft_core::error::{Language, message};
use foamcraft_geometry::{Severity, diagnose, read_mesh, suggest_unit};

fn data(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/data/geometry")
        .join(name)
}

fn codes(file: &str) -> Vec<String> {
    let mesh = read_mesh(&data(file)).unwrap();
    diagnose(&mesh)
        .findings
        .into_iter()
        .map(|f| f.code)
        .collect()
}

#[test]
fn closed_cube_is_watertight_with_one_body() {
    let mesh = read_mesh(&data("cube_binary.stl")).unwrap();
    let d = diagnose(&mesh);
    assert!(d.watertight);
    assert_eq!(d.boundary_loops, 0);
    assert_eq!(d.bodies, 1);
    assert_eq!(d.size, [20.0, 20.0, 20.0]);
    assert!(
        d.findings.iter().all(|f| f.severity == Severity::Info),
        "{:?}",
        d.findings
    );
}

#[test]
fn open_cube_has_one_boundary_loop() {
    let mesh = read_mesh(&data("cube_open.stl")).unwrap();
    let d = diagnose(&mesh);
    assert!(!d.watertight);
    assert_eq!(d.boundary_loops, 1);
    assert!(codes("cube_open.stl").contains(&"geometry.open".to_string()));
}

#[test]
fn flipped_face_is_detected() {
    let d = diagnose(&read_mesh(&data("cube_flipped_face.stl")).unwrap());
    assert!(d.inconsistent_normals);
    assert!(codes("cube_flipped_face.stl").contains(&"geometry.flipped_normals".to_string()));
}

#[test]
fn duplicate_and_degenerate_triangles_are_counted() {
    let mut mesh = read_mesh(&data("cube_ascii.stl")).unwrap();
    let first = mesh.triangles[0];
    mesh.triangles.push(first);
    mesh.triangles.push([0, 0, 1]);
    let d = diagnose(&mesh);
    assert_eq!(d.duplicate_triangles, 1);
    assert_eq!(d.degenerate_triangles, 1);
}

#[test]
fn every_finding_has_english_and_german_text() {
    for file in ["cube_open.stl", "cube_flipped_face.stl", "cube_binary.stl"] {
        for code in codes(file) {
            assert!(message(&code, Language::En).is_some(), "{code} en");
            assert!(message(&code, Language::De).is_some(), "{code} de");
        }
    }
}

#[test]
fn unit_suggestion_follows_model_size() {
    assert_eq!(suggest_unit(20.0), "mm");
    assert_eq!(suggest_unit(250.0), "mm");
    assert_eq!(suggest_unit(1.8), "m");
    assert_eq!(suggest_unit(0.3), "m");
}
