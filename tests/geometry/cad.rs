// SPDX-License-Identifier: GPL-3.0-or-later
//! CAD import: Gmsh meshes (.msh 4.1) keep the CAD faces; STEP and IGES go through a converter.
//! The .msh files in tests/data/cad were produced by Gmsh 4.14 from the STEP and IGES files next to
//! them (see the README there).

use std::path::{Path, PathBuf};
use std::sync::Arc;

use foamcraft_core::command::{Session, core_registry};
use foamcraft_core::error::FoamError;
use foamcraft_geometry::{CadConverter, Format, diagnose, iges_unit, read_mesh, segment};
use serde_json::json;

fn data(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/data/cad")
        .join(name)
}

#[test]
fn step_cube_mesh_has_six_cad_faces_in_metres() {
    let mesh = read_mesh(&data("cube_step.msh")).unwrap();
    assert_eq!(mesh.format, Format::Msh);
    let s = segment(&mesh, 30.0);
    assert_eq!(s.faces.len(), 6);
    assert_eq!(s.parts, 1);
    let (min, max) = mesh.bounds();
    assert!(min.iter().all(|v| v.abs() < 1e-9));
    assert!(max.iter().all(|v| (v - 0.02).abs() < 1e-9), "{max:?}");
    let d = diagnose(&mesh);
    assert!(d.watertight, "{:?}", d.findings);
}

#[test]
fn pipe_with_flange_keeps_its_six_cad_faces_even_with_a_small_crease_angle() {
    let mesh = read_mesh(&data("pipe_flange_step.msh")).unwrap();
    // CAD faces win over crease detection: the curved faces stay single faces.
    let s = segment(&mesh, 5.0);
    assert_eq!(s.faces.len(), 6);
    assert_eq!(s.parts, 1);
    assert!(diagnose(&mesh).watertight);
}

#[test]
fn iges_cube_mesh_loads() {
    let mesh = read_mesh(&data("cube_iges.msh")).unwrap();
    assert_eq!(segment(&mesh, 30.0).faces.len(), 6);
}

#[test]
fn broken_mesh_file_has_an_error_code() {
    let path = std::env::temp_dir().join(format!("fc-broken-{}.msh", std::process::id()));
    std::fs::write(&path, "$MeshFormat\n4.1 0 8\n$EndMeshFormat\n$Nodes\n1 2\n").unwrap();
    assert_eq!(read_mesh(&path).unwrap_err().code, "geometry.read_failed");
}

#[test]
fn step_and_iges_need_a_converter() {
    assert_eq!(
        read_mesh(&data("cube.step")).unwrap_err().code,
        "geometry.cad_unavailable"
    );
    let mut reg = core_registry();
    foamcraft_geometry::commands::register(&mut reg);
    let base = std::env::temp_dir().join(format!("fc-cad-none-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&base);
    let mut s = Session::new(base.join("data"));
    reg.execute(
        &mut s,
        "project.new",
        json!({"path": base.join("p"), "name": "C"}),
    )
    .unwrap();
    let err = reg
        .execute(
            &mut s,
            "geometry.import",
            json!({"path": data("cube.step")}),
        )
        .unwrap_err();
    assert_eq!(err.code, "geometry.cad_unavailable");
}

/// Stands in for the container: copies the prepared Gmsh output.
struct PreparedConverter;

impl CadConverter for PreparedConverter {
    fn convert(&self, input: &Path, output: &Path) -> Result<String, FoamError> {
        let stem = input.file_stem().unwrap().to_string_lossy().into_owned();
        let ext = input.extension().unwrap().to_string_lossy().into_owned();
        // Inputs are copies named model-N.<ext>; map back to the test file by extension.
        let (source, unit) = match (stem.as_str(), ext.as_str()) {
            (_, "iges" | "igs") => (data("cube_iges.msh"), "mm"),
            _ if std::fs::read_to_string(input)
                .unwrap()
                .contains("CYLINDRICAL_SURFACE") =>
            {
                (data("pipe_flange_step.msh"), "m")
            }
            _ => (data("cube_step.msh"), "m"),
        };
        std::fs::copy(source, output)
            .map(|_| unit.to_string())
            .map_err(|e| FoamError::new("geometry.read_failed", e.to_string()))
    }
}

#[test]
fn geometry_import_of_step_and_iges_gives_clickable_cad_faces() {
    let mut reg = core_registry();
    foamcraft_geometry::commands::register_with_cad(&mut reg, Some(Arc::new(PreparedConverter)));
    let base = std::env::temp_dir().join(format!("fc-cad-import-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&base);
    let mut s = Session::new(base.join("data"));
    let project = base.join("p");
    reg.execute(&mut s, "project.new", json!({"path": project, "name": "C"}))
        .unwrap();
    let out = reg
        .execute(
            &mut s,
            "geometry.import",
            json!({"path": data("pipe_flange.step")}),
        )
        .unwrap();
    assert_eq!(out["unit"], "m");
    assert_eq!(out["format"], "step");
    assert!(
        project.join(out["file"].as_str().unwrap()).is_file(),
        "mesh copy"
    );
    assert!(
        project.join(out["source"].as_str().unwrap()).is_file(),
        "original CAD file"
    );
    assert!(
        (out["size_m"][2].as_f64().unwrap() - 0.05).abs() < 1e-6,
        "{}",
        out["size_m"]
    );
    let iges = reg
        .execute(
            &mut s,
            "geometry.import",
            json!({"path": data("cube.iges")}),
        )
        .unwrap();
    assert_eq!(iges["unit"], "mm");
    assert!(
        (iges["size_m"][0].as_f64().unwrap() - 0.02).abs() < 1e-6,
        "{}",
        iges["size_m"]
    );
    let faces = reg.execute(&mut s, "surface.list", json!({})).unwrap();
    let keys: Vec<_> = faces
        .as_array()
        .unwrap()
        .iter()
        .map(|f| f["key"].as_str().unwrap().to_string())
        .collect();
    assert_eq!(keys.iter().filter(|k| k.starts_with("model-1:")).count(), 6);
    assert_eq!(keys.iter().filter(|k| k.starts_with("model-2:")).count(), 6);
    // Clicking a CAD face works like any other face.
    let picked = reg
        .execute(
            &mut s,
            "surface.select_at",
            json!({"point": [0.0, 0.0, 0.06]}),
        )
        .unwrap();
    assert!(picked["key"].as_str().unwrap().starts_with("model-1:face-"));
}

#[test]
fn iges_unit_comes_from_the_global_section() {
    assert_eq!(
        iges_unit(&std::fs::read(data("cube.iges")).unwrap()),
        Some("mm")
    );
    assert_eq!(iges_unit(b"not an iges file"), None);
}
