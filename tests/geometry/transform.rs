// SPDX-License-Identifier: GPL-3.0-or-later
//! Unit scaling, rotation, translation, placing on the ground, fixing normals.

use std::path::PathBuf;

use foamcraft_geometry::{Placement, diagnose, fix_normals, ground_offset, read_mesh, unit_scale};

fn data(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/data/geometry")
        .join(name)
}

#[test]
fn units_scale_to_metres() {
    assert_eq!(unit_scale("mm"), Some(0.001));
    assert_eq!(unit_scale("cm"), Some(0.01));
    assert_eq!(unit_scale("m"), Some(1.0));
    assert_eq!(unit_scale("in"), Some(0.0254));
    assert_eq!(unit_scale("furlong"), None);
}

#[test]
fn placement_scales_rotates_and_moves() {
    let mesh = read_mesh(&data("cube_binary.stl")).unwrap();
    let p = Placement {
        unit: "mm".into(),
        scale: 1.0,
        rotation_deg: [0.0, 0.0, 0.0],
        translation: [1.0, 0.0, 0.0],
    };
    let world = p.apply(&mesh);
    let (min, max) = world.bounds();
    assert!((min[0] - 1.0).abs() < 1e-6 && (max[0] - 1.02).abs() < 1e-6);
    assert!((max[2] - 0.02).abs() < 1e-6);
}

#[test]
fn after_rotation_and_place_on_ground_the_bottom_is_at_zero() {
    let mesh = read_mesh(&data("cube_binary.stl")).unwrap();
    let mut p = Placement {
        unit: "mm".into(),
        scale: 2.0,
        rotation_deg: [30.0, 45.0, 10.0],
        translation: [0.0, 0.0, 5.0],
    };
    p.translation[2] += ground_offset(&p.apply(&mesh));
    let (min, _) = p.apply(&mesh).bounds();
    assert!(min[2].abs() < 1e-6, "bottom at {}", min[2]);
}

#[test]
fn fixing_normals_makes_orientation_consistent_and_outward() {
    let mut mesh = read_mesh(&data("cube_flipped_face.stl")).unwrap();
    fix_normals(&mut mesh);
    let d = diagnose(&mesh);
    assert!(!d.inconsistent_normals);
    assert!(
        d.volume > 0.0,
        "normals must point outwards, volume {}",
        d.volume
    );
    // A fully inverted closed mesh is turned outward too.
    let mut inverted = read_mesh(&data("cube_binary.stl")).unwrap();
    for t in &mut inverted.triangles {
        t.swap(1, 2);
    }
    fix_normals(&mut inverted);
    assert!(diagnose(&inverted).volume > 0.0);
}
