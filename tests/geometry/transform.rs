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

// --- Task_25: rotation about a pivot and pivot points ---

use foamcraft_geometry::{Format, Mesh, MeshBuilder, pivots, rotation_to_deg};
use glam::{DMat3, DVec3};

fn world_point(p: &Placement, local: [f32; 3]) -> DVec3 {
    let mut b = MeshBuilder::with_capacity(1);
    b.triangle(local, local, local);
    let m = p.apply(&b.finish(Format::Stl, None));
    DVec3::from(m.positions[0].map(f64::from))
}

#[test]
fn rotation_about_a_pivot_keeps_the_pivot_in_place() {
    let start = Placement {
        rotation_deg: [10.0, 20.0, 30.0],
        translation: [0.3, -0.2, 0.1],
        ..Default::default()
    };
    let pivot = DVec3::new(0.5, 0.5, 0.2);
    // The local point that sits at the pivot now.
    let local = start.rotation().transpose() * (pivot - DVec3::from(start.translation));
    let local = [local.x as f32, local.y as f32, local.z as f32];
    for axis in [DVec3::X, DVec3::Y, DVec3::Z] {
        let turned = start.rotated_about(axis, 37f64.to_radians(), pivot);
        assert!(
            (world_point(&turned, local) - pivot).length() < 1e-6,
            "{axis}"
        );
        // Another point turns by 37° around the axis through the pivot.
        let q = world_point(&start, [1.0, 0.0, 0.0]) - pivot;
        let q2 = world_point(&turned, [1.0, 0.0, 0.0]) - pivot;
        let expected = DMat3::from_axis_angle(axis, 37f64.to_radians()) * q;
        assert!((q2 - expected).length() < 1e-5, "{axis}");
    }
}

#[test]
fn four_quarter_turns_give_the_start() {
    let start = Placement {
        rotation_deg: [15.0, -40.0, 70.0],
        translation: [1.0, 2.0, 3.0],
        ..Default::default()
    };
    let pivot = DVec3::new(-0.4, 0.7, 0.0);
    for axis in [DVec3::X, DVec3::Y, DVec3::Z] {
        let mut p = start.clone();
        for _ in 0..4 {
            p = p.rotated_about(axis, std::f64::consts::FRAC_PI_2, pivot);
        }
        let a = world_point(&start, [0.3, 0.1, -0.2]);
        let b = world_point(&p, [0.3, 0.1, -0.2]);
        assert!((a - b).length() < 1e-5, "{axis}: {a} vs {b}");
        for k in 0..3 {
            assert!((p.translation[k] - start.translation[k]).abs() < 1e-6);
        }
    }
}

#[test]
fn euler_angles_round_trip_including_gimbal_lock() {
    for deg in [
        [10.0, 20.0, 30.0],
        [-170.0, 45.0, 100.0],
        [30.0, 90.0, 0.0],
        [0.0, -90.0, 0.0],
    ] {
        let p = Placement {
            rotation_deg: deg,
            ..Default::default()
        };
        let back = Placement {
            rotation_deg: rotation_to_deg(p.rotation()),
            ..Default::default()
        };
        let diff = p.rotation() - back.rotation();
        let err = diff.x_axis.length() + diff.y_axis.length() + diff.z_axis.length();
        assert!(err < 1e-9, "{deg:?} -> {:?}", back.rotation_deg);
    }
}

#[test]
fn pivots_of_an_uneven_body_differ() {
    // Corner tetrahedron: box centre (0.5, 0.5, 0.5), centroid (0.25, 0.25, 0.25).
    let o = [0.0, 0.0, 0.0];
    let (x, y, z) = ([1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]);
    let mut b = MeshBuilder::with_capacity(4);
    b.triangle(o, y, x);
    b.triangle(o, x, z);
    b.triangle(o, z, y);
    b.triangle(x, y, z);
    let mesh: Mesh = b.finish(Format::Stl, None);
    let p = pivots(&mesh, [7.0, 8.0, 9.0]);
    for k in 0..3 {
        assert!((p.center[k] - 0.5).abs() < 1e-6);
        assert!((p.centroid[k] - 0.25).abs() < 1e-6, "{:?}", p.centroid);
    }
    assert_eq!(p.origin, [7.0, 8.0, 9.0]);
}
