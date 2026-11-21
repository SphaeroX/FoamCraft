// SPDX-License-Identifier: GPL-3.0-or-later
//! Projected frontal area and reference length against analytic values (within 1 %).

use foamcraft_geometry::{Format, Mesh, MeshBuilder, Placement};
use foamcraft_sim::{flow_direction, frontal_area, length_along};

/// Closed UV sphere of radius `r`.
fn sphere(r: f32, n: usize) -> Mesh {
    let point = |i: usize, j: usize| {
        if i == 0 || i == n {
            return [0.0, 0.0, if i == 0 { r } else { -r }];
        }
        let t = std::f32::consts::PI * i as f32 / n as f32;
        let f = 2.0 * std::f32::consts::PI * (j % n) as f32 / n as f32;
        [r * t.sin() * f.cos(), r * t.sin() * f.sin(), r * t.cos()]
    };
    let mut b = MeshBuilder::default();
    for i in 0..n {
        for j in 0..n {
            let (a, bb, c, d) = (
                point(i, j),
                point(i + 1, j),
                point(i + 1, j + 1),
                point(i, j + 1),
            );
            if i != 0 {
                b.triangle(a, bb, d);
            }
            if i != n - 1 {
                b.triangle(bb, c, d);
            }
        }
    }
    b.finish(Format::Stl, None)
}

fn cube(a: f32) -> Mesh {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/data/geometry/cube_binary.stl");
    let mesh = foamcraft_geometry::read_mesh(&path).unwrap();
    // The file cube is 20 units; scale to `a` metres.
    Placement {
        unit: "m".into(),
        scale: f64::from(a) / 20.0,
        ..Default::default()
    }
    .apply(&mesh)
}

fn close(value: f64, expected: f64) {
    assert!(
        (value - expected).abs() <= 0.01 * expected,
        "{value} vs {expected}"
    );
}

#[test]
fn sphere_shows_a_disc() {
    let s = sphere(0.5, 200);
    for yaw in [0.0, 37.0, 90.0] {
        close(
            frontal_area(&[&s], flow_direction(yaw)),
            std::f64::consts::PI * 0.25,
        );
    }
    close(length_along(&[&s], flow_direction(0.0)), 1.0);
}

#[test]
fn cube_face_on_and_rotated() {
    let c = cube(2.0);
    close(frontal_area(&[&c], flow_direction(0.0)), 4.0);
    // Flow at 45 degrees sees two faces: a^2 * (cos 45 + sin 45).
    close(
        frontal_area(&[&c], flow_direction(45.0)),
        4.0 * std::f64::consts::SQRT_2,
    );
    close(
        length_along(&[&c], flow_direction(45.0)),
        2.0 * std::f64::consts::SQRT_2,
    );
}

#[test]
fn rotated_cube_matches_the_sum_of_projected_faces() {
    let file = cube(1.0);
    let placement = Placement {
        unit: "m".into(),
        scale: 1.0,
        rotation_deg: [20.0, 30.0, 10.0],
        translation: [0.0; 3],
    };
    let rotated = placement.apply(&file);
    let r = placement.rotation();
    let d = glam::DVec3::from(flow_direction(0.0));
    // Projected area of a unit box: sum over its three face normals of |n . d|.
    let expected: f64 = [glam::DVec3::X, glam::DVec3::Y, glam::DVec3::Z]
        .iter()
        .map(|n| (r * *n).dot(d).abs())
        .sum();
    close(frontal_area(&[&rotated], flow_direction(0.0)), expected);
}

#[test]
fn flow_direction_is_a_unit_vector_in_the_ground_plane() {
    let d = flow_direction(90.0);
    assert!((d[0]).abs() < 1e-12 && (d[1] - 1.0).abs() < 1e-12 && d[2] == 0.0);
}
