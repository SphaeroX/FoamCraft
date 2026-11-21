// SPDX-License-Identifier: GPL-3.0-or-later
//! Mesh readers for STL (ASCII and binary), OBJ and 3MF.

use std::path::PathBuf;

use foamcraft_geometry::{Format, read_mesh};

fn data(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/data/geometry")
        .join(name)
}

#[test]
fn every_format_reads_the_cube_with_twelve_triangles() {
    for (file, format) in [
        ("cube_ascii.stl", Format::Stl),
        ("cube_binary.stl", Format::Stl),
        ("cube.obj", Format::Obj),
        ("cube.3mf", Format::ThreeMf),
    ] {
        let mesh = read_mesh(&data(file)).unwrap_or_else(|e| panic!("{file}: {e}"));
        assert_eq!(mesh.format, format, "{file}");
        assert_eq!(mesh.triangles.len(), 12, "{file}");
        assert_eq!(mesh.positions.len(), 8, "{file}: vertices must be welded");
        let (min, max) = mesh.bounds();
        assert_eq!(min, [0.0, 0.0, 0.0], "{file}");
        assert_eq!(max, [20.0, 20.0, 20.0], "{file}");
    }
}

#[test]
fn three_mf_reports_its_unit() {
    let mesh = read_mesh(&data("cube.3mf")).unwrap();
    assert_eq!(mesh.declared_unit.as_deref(), Some("mm"));
}

#[test]
fn broken_and_unknown_files_have_error_codes() {
    assert_eq!(
        read_mesh(&data("broken.stl")).unwrap_err().code,
        "geometry.read_failed"
    );
    assert_eq!(
        read_mesh(&data("README.md")).unwrap_err().code,
        "geometry.unsupported_format"
    );
    assert_eq!(
        read_mesh(&data("missing.stl")).unwrap_err().code,
        "geometry.read_failed"
    );
}

#[test]
fn binary_stl_whose_header_starts_with_solid_is_still_binary() {
    let src = std::fs::read(data("cube_binary.stl")).unwrap();
    let mut bytes = b"solid but actually binary".to_vec();
    bytes.extend_from_slice(&src[25..]);
    let path = std::env::temp_dir().join(format!("fc-solid-bin-{}.stl", std::process::id()));
    std::fs::write(&path, bytes).unwrap();
    assert_eq!(read_mesh(&path).unwrap().triangles.len(), 12);
}
