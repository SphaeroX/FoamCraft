// SPDX-License-Identifier: GPL-3.0-or-later
//! STEP and IGES conversion in the real runtime (needs Docker and the runtime image):
//!     cargo test --workspace -- --ignored runtime_

use std::path::PathBuf;
use std::sync::Arc;

use foamcraft_api::DockerCad;
use foamcraft_geometry::{CadConverter, read_mesh, segment};
use foamcraft_runtime::DockerRunner;

fn data(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/data/cad")
        .join(name)
}

fn convert(file: &str) -> Result<foamcraft_geometry::Mesh, foamcraft_core::error::FoamError> {
    let cad = DockerCad {
        runner: Arc::new(DockerRunner::local()),
    };
    let out = std::env::temp_dir().join(format!("fc-cad-{}-{file}.msh", std::process::id()));
    cad.convert(&data(file), &out)?;
    read_mesh(&out)
}

#[test]
#[ignore]
fn runtime_step_cube_has_six_faces_in_metres() {
    let mesh = convert("cube.step").unwrap();
    assert_eq!(segment(&mesh, 30.0).faces.len(), 6);
    let (_, max) = mesh.bounds();
    assert!(max.iter().all(|v| (v - 0.02).abs() < 1e-6), "{max:?}");
}

#[test]
#[ignore]
fn runtime_step_pipe_with_flange_has_six_faces_and_one_part() {
    let s = segment(&convert("pipe_flange.step").unwrap(), 30.0);
    assert_eq!((s.faces.len(), s.parts), (6, 1));
}

#[test]
#[ignore]
fn runtime_iges_cube_loads() {
    assert_eq!(segment(&convert("cube.iges").unwrap(), 30.0).faces.len(), 6);
}

#[test]
#[ignore]
fn runtime_broken_step_file_has_an_error_code() {
    let broken = std::env::temp_dir().join(format!("fc-broken-{}.step", std::process::id()));
    std::fs::write(&broken, "ISO-10303-21;\nHEADER;\nthis is not a model\n").unwrap();
    let cad = DockerCad {
        runner: Arc::new(DockerRunner::local()),
    };
    let out = broken.with_extension("msh");
    let err = cad
        .convert(&broken, &out)
        .and_then(|_| read_mesh(&out).map(|_| ()))
        .unwrap_err();
    assert!(
        ["geometry.read_failed", "geometry.empty"].contains(&err.code.as_str()),
        "{err:?}"
    );
}
