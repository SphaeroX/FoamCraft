// SPDX-License-Identifier: GPL-3.0-or-later
//! Slices, isosurfaces and the Q criterion on sampled grids, checked with analytic fields.

use foamcraft_sim::results::{Grid, GridSpec, grid_from_rows};
use foamcraft_sim::visual::{add_derived, isosurface, slice};

/// Grid over the unit cube with fields from `f(p) -> [scalar, ux, uy, uz]`.
fn grid(n: u32, f: impl Fn([f64; 3]) -> [f64; 4]) -> Grid {
    let spec = GridSpec {
        min: [0.0; 3],
        max: [1.0; 3],
        dims: [n, n, n],
    };
    let mut rows = Vec::new();
    for k in 0..n {
        for j in 0..n {
            for i in 0..n {
                let p = spec.point(i, j, k);
                let v = f(p);
                rows.push(vec![p[0], p[1], p[2], v[0], v[1], v[2], v[3]]);
            }
        }
    }
    grid_from_rows(&spec, &["s", "Ux", "Uy", "Uz"], &rows)
}

#[test]
fn a_slice_shows_the_field_on_the_plane() {
    let g = grid(20, |p| [p[0], 0.0, 0.0, 0.0]);
    let m = slice(&[g], "s", [0.5, 0.5, 0.5], [0.0, 0.0, 1.0], 40);
    assert!(m.triangles.len() > 1000, "{}", m.triangles.len());
    for (p, v) in m.positions.iter().zip(&m.values) {
        assert!((f64::from(p[2]) - 0.5).abs() < 1e-6);
        // Linear field: trilinear sampling is exact.
        assert!((f64::from(*v) - f64::from(p[0])).abs() < 1e-5, "{p:?} {v}");
    }
}

#[test]
fn a_slice_skips_masked_regions() {
    let spec = GridSpec {
        min: [0.0; 3],
        max: [1.0; 3],
        dims: [20, 20, 20],
    };
    let mut rows = Vec::new();
    for k in 0..20 {
        for j in 0..20 {
            for i in 0..20 {
                let p = spec.point(i, j, k);
                // A solid block in the middle.
                if (0..3).all(|a| p[a] > 0.4 && p[a] < 0.6) {
                    continue;
                }
                rows.push(vec![p[0], p[1], p[2], 1.0, 0.0, 0.0, 0.0]);
            }
        }
    }
    let g = grid_from_rows(&spec, &["s", "Ux", "Uy", "Uz"], &rows);
    let m = slice(&[g], "s", [0.5, 0.5, 0.5], [0.0, 0.0, 1.0], 50);
    assert!(
        m.positions
            .iter()
            .all(|p| !(p[0] > 0.45 && p[0] < 0.55 && p[1] > 0.45 && p[1] < 0.55))
    );
}

#[test]
fn the_isosurface_of_a_sphere_function_has_the_right_radius() {
    let c = [0.5, 0.5, 0.5];
    let g = grid(32, |p| {
        let d = ((p[0] - c[0]).powi(2) + (p[1] - c[1]).powi(2) + (p[2] - c[2]).powi(2)).sqrt();
        [d, 0.0, 0.0, 0.0]
    });
    let h = g.spec.step()[0];
    let m = isosurface(&[g], "s", 0.3);
    assert!(m.triangles.len() > 500);
    for p in &m.positions {
        let d = ((f64::from(p[0]) - c[0]).powi(2)
            + (f64::from(p[1]) - c[1]).powi(2)
            + (f64::from(p[2]) - c[2]).powi(2))
        .sqrt();
        assert!((d - 0.3).abs() < h, "radius {d}");
    }
}

#[test]
fn q_criterion_is_positive_in_rotation_and_zero_in_shear() {
    let omega = 2.0;
    // Rigid rotation about z through the centre: Q = omega².
    let mut rot = grid(16, |p| {
        [0.0, -omega * (p[1] - 0.5), omega * (p[0] - 0.5), 0.0]
    });
    add_derived(&mut rot);
    let q = rot.sample("Q", [0.5, 0.5, 0.5]).unwrap();
    assert!((q - omega * omega).abs() < 1e-6, "Q {q}");
    let speed = rot.sample("U_mag", [0.75, 0.5, 0.5]).unwrap();
    assert!((speed - 0.5).abs() < 0.02, "speed {speed}");
    // Simple shear u = gamma y: vorticity and strain cancel, Q = 0.
    let mut shear = grid(16, |p| [0.0, 3.0 * p[1], 0.0, 0.0]);
    add_derived(&mut shear);
    assert!(shear.sample("Q", [0.5, 0.5, 0.5]).unwrap().abs() < 1e-6);
}

#[test]
fn finer_levels_take_over_inside_their_box() {
    // Coarse level everywhere 1, fine level in the middle 2.
    let coarse = grid(10, |_| [1.0, 0.0, 0.0, 0.0]);
    let fine_spec = GridSpec {
        min: [0.3; 3],
        max: [0.7; 3],
        dims: [10, 10, 10],
    };
    let mut rows = Vec::new();
    for k in 0..10 {
        for j in 0..10 {
            for i in 0..10 {
                let p = fine_spec.point(i, j, k);
                rows.push(vec![p[0], p[1], p[2], 2.0, 0.0, 0.0, 0.0]);
            }
        }
    }
    let fine = grid_from_rows(&fine_spec, &["s", "Ux", "Uy", "Uz"], &rows);
    let m = slice(&[coarse, fine], "s", [0.5, 0.5, 0.5], [1.0, 0.0, 0.0], 60);
    let centre = m
        .positions
        .iter()
        .zip(&m.values)
        .find(|(p, _)| (p[1] - 0.5).abs() < 0.03 && (p[2] - 0.5).abs() < 0.03)
        .unwrap();
    assert_eq!(*centre.1, 2.0);
}
