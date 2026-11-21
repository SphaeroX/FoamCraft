// SPDX-License-Identifier: GPL-3.0-or-later
//! Reader for OpenFOAM data: field files (ASCII and binary, uniform and non-uniform), the "foam"
//! surface format of the surfaces function object, and raw sample sets.
//! Data: tests/data/fields (cavity case, OpenFOAM v2606).

use std::path::PathBuf;

use foamcraft_sim::foam_io::{
    FieldData, read_faces, read_internal_field, read_points, read_raw_set, read_scalar_list,
    read_vector_list,
};

fn data(name: &str) -> Vec<u8> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/data/fields")
        .join(name);
    std::fs::read(path).unwrap()
}

fn close(a: f64, b: f64) -> bool {
    (a - b).abs() <= 1e-5 * (1.0 + a.abs().max(b.abs()))
}

#[test]
fn ascii_and_binary_scalar_fields_agree() {
    let FieldData::Scalars(ascii) = read_internal_field(&data("ascii/p")).unwrap() else {
        panic!("p is a scalar field")
    };
    let FieldData::Scalars(binary) = read_internal_field(&data("binary/p")).unwrap() else {
        panic!("p is a scalar field")
    };
    assert_eq!(ascii.len(), 1600);
    assert_eq!(binary.len(), 1600);
    assert!(ascii.iter().zip(&binary).all(|(a, b)| close(*a, *b)));
}

#[test]
fn ascii_and_binary_vector_fields_agree() {
    let FieldData::Vectors(ascii) = read_internal_field(&data("ascii/U")).unwrap() else {
        panic!("U is a vector field")
    };
    let FieldData::Vectors(binary) = read_internal_field(&data("binary/U")).unwrap() else {
        panic!("U is a vector field")
    };
    assert_eq!(ascii.len(), 1600);
    assert!((ascii[0][0] + 0.000296437).abs() < 1e-12);
    assert!(
        ascii
            .iter()
            .zip(&binary)
            .all(|(a, b)| (0..3).all(|k| close(a[k], b[k])))
    );
}

#[test]
fn a_uniform_internal_field_is_one_value() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../runtime/cases/cavity/0/U");
    let field = read_internal_field(&std::fs::read(path).unwrap()).unwrap();
    assert_eq!(field, FieldData::UniformVector([0.0, 0.0, 0.0]));
}

#[test]
fn the_foam_surface_format_is_read() {
    let points = read_points(&data("surface/points")).unwrap();
    let faces = read_faces(&data("surface/faces")).unwrap();
    let p = read_scalar_list(&data("surface/scalarField/p")).unwrap();
    let u = read_vector_list(&data("surface/vectorField/U")).unwrap();
    assert_eq!(faces.len(), 80);
    assert!(
        faces
            .iter()
            .all(|f| f.len() == 3 && f.iter().all(|i| (*i as usize) < points.len()))
    );
    assert_eq!(p.len(), 80);
    assert!((p[0] + 12.5777).abs() < 1e-9);
    // "80{(1 0 0)}": the same value 80 times.
    assert_eq!(u.len(), 80);
    assert!(u.iter().all(|v| *v == [1.0, 0.0, 0.0]));
}

#[test]
fn raw_sample_sets_are_rows_of_numbers() {
    let rows = read_raw_set(&data("grid_level0_p_U.xy")).unwrap();
    assert_eq!(rows.len(), 16);
    assert!(rows.iter().all(|r| r.len() == 7));
    assert_eq!(rows[0][..3], [0.026, 0.026, 0.005]);
    assert!((rows[15][3] - 1.10151).abs() < 1e-12);
}
