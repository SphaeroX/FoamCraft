// SPDX-License-Identifier: GPL-3.0-or-later
//! Splitting models into faces (crease angle) and parts (connected bodies), plus ray and point
//! queries used for picking without a window.

use std::path::PathBuf;

use foamcraft_geometry::{
    Format, Mesh, MeshBuilder, nearest_triangle, raycast, read_mesh, segment,
};

fn data(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/data/geometry")
        .join(name)
}

/// Closed cylinder along Z: `n` side quads, fan caps.
fn cylinder(n: usize, r: f32, h: f32) -> Mesh {
    let mut b = MeshBuilder::default();
    let p = |k: usize, z: f32| {
        let a = 2.0 * std::f32::consts::PI * (k % n) as f32 / n as f32;
        [r * a.cos(), r * a.sin(), z]
    };
    for k in 0..n {
        let (a0, a1, b0, b1) = (p(k, 0.0), p(k + 1, 0.0), p(k, h), p(k + 1, h));
        b.triangle(a0, a1, b1);
        b.triangle(a0, b1, b0);
        b.triangle([0.0, 0.0, 0.0], a1, a0);
        b.triangle([0.0, 0.0, h], b0, b1);
    }
    b.finish(Format::Stl, None)
}

/// UV sphere with deterministic radial noise of up to `noise` (relative).
fn noisy_sphere(n: usize, noise: f32) -> Mesh {
    let mut seed = 12345u32;
    let mut rnd = move || {
        seed = seed.wrapping_mul(1_103_515_245).wrapping_add(12345);
        (seed >> 8) as f32 / (1u32 << 24) as f32 * 2.0 - 1.0
    };
    let mut radius = vec![vec![1.0f32; n]; n + 1];
    for row in radius.iter_mut().take(n).skip(1) {
        for r in row.iter_mut() {
            *r = 1.0 + noise * rnd();
        }
    }
    let point = |i: usize, j: usize| {
        if i == 0 || i == n {
            return [0.0, 0.0, if i == 0 { 1.0 } else { -1.0 }];
        }
        let t = std::f32::consts::PI * i as f32 / n as f32;
        let f = 2.0 * std::f32::consts::PI * (j % n) as f32 / n as f32;
        let r = radius[i][j % n];
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

fn two_cubes() -> Mesh {
    let cube = read_mesh(&data("cube_binary.stl")).unwrap();
    let mut b = MeshBuilder::default();
    for offset in [0.0f32, 50.0] {
        for t in &cube.triangles {
            let [p, q, r] = t.map(|i| {
                let v = cube.positions[i as usize];
                [v[0] + offset, v[1], v[2]]
            });
            b.triangle(p, q, r);
        }
    }
    b.finish(Format::Stl, None)
}

#[test]
fn cube_has_six_faces_and_one_part() {
    let s = segment(&read_mesh(&data("cube_binary.stl")).unwrap(), 30.0);
    assert_eq!(s.faces.len(), 6);
    assert_eq!(s.parts, 1);
    assert!(s.faces.iter().all(|f| f.triangles == 2));
}

#[test]
fn cylinder_has_a_mantle_and_two_caps() {
    let s = segment(&cylinder(48, 1.0, 2.0), 30.0);
    let mut sizes: Vec<_> = s.faces.iter().map(|f| f.triangles).collect();
    sizes.sort_unstable();
    assert_eq!(sizes, [48, 48, 96]);
}

#[test]
fn noisy_sphere_is_one_face() {
    let s = segment(&noisy_sphere(40, 0.005), 30.0);
    assert_eq!(
        s.faces.len(),
        1,
        "{:?}",
        s.faces.iter().map(|f| f.triangles).collect::<Vec<_>>()
    );
}

#[test]
fn two_bodies_are_two_parts() {
    let s = segment(&two_cubes(), 30.0);
    assert_eq!(s.parts, 2);
    assert_eq!(s.faces.len(), 12);
    let parts: std::collections::BTreeSet<_> = s.faces.iter().map(|f| f.part).collect();
    assert_eq!(parts.len(), 2);
}

#[test]
fn face_numbers_are_deterministic() {
    let mesh = read_mesh(&data("cube_ascii.stl")).unwrap();
    assert_eq!(segment(&mesh, 30.0), segment(&mesh, 30.0));
    let s = segment(&mesh, 30.0);
    // Face numbers follow the lowest triangle index of each face.
    assert_eq!(s.face_of_triangle[0], 1);
    let firsts: Vec<_> = s.faces.iter().map(|f| f.first_triangle).collect();
    let mut sorted = firsts.clone();
    sorted.sort_unstable();
    assert_eq!(firsts, sorted);
}

#[test]
fn a_ray_and_a_point_find_the_same_triangle() {
    let mesh = read_mesh(&data("cube_binary.stl")).unwrap();
    // From far in front of the -Y side, straight at the middle of that side.
    let hit = raycast(&mesh, [10.0, -100.0, 10.0], [0.0, 1.0, 0.0]).expect("hit");
    assert!((hit.distance - 100.0).abs() < 1e-4);
    assert!((hit.point[1] - 0.0).abs() < 1e-4);
    let near = nearest_triangle(&mesh, [10.0, -0.5, 10.0]).unwrap();
    let s = segment(&mesh, 30.0);
    assert_eq!(s.face_of_triangle[hit.triangle], s.face_of_triangle[near]);
    assert!(raycast(&mesh, [10.0, -100.0, 10.0], [0.0, -1.0, 0.0]).is_none());
}
