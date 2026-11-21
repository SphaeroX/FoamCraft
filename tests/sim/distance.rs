// SPDX-License-Identifier: GPL-3.0-or-later
//! Distance to the nearest wall (Task_32): a cell grid over the triangles of the result surface.

use foamcraft_sim::distance::WallDistance;

/// Unit cube 0..1 as twelve triangles.
fn cube() -> (Vec<[f32; 3]>, Vec<[u32; 3]>) {
    let p = |i: u32| [(i & 1) as f32, ((i >> 1) & 1) as f32, ((i >> 2) & 1) as f32];
    let points = (0..8).map(p).collect();
    let quads = [
        [0, 1, 3, 2],
        [4, 6, 7, 5],
        [0, 4, 5, 1],
        [2, 3, 7, 6],
        [0, 2, 6, 4],
        [1, 5, 7, 3],
    ];
    let mut tris = Vec::new();
    for q in quads {
        tris.push([q[0], q[1], q[2]]);
        tris.push([q[0], q[2], q[3]]);
    }
    (points, tris)
}

#[test]
fn faces_edges_and_corners_give_the_exact_distance() {
    let (pts, tris) = cube();
    let w = WallDistance::new(&pts, &tris, 0.1, 0.8);
    let close = |p: [f64; 3], d: f64| {
        let got = w.distance(p);
        assert!((got - d).abs() < 1e-9, "{p:?}: {got} vs {d}");
    };
    close([0.5, 0.5, 1.3], 0.3);
    close([1.2, 0.5, 1.2], 0.08f64.sqrt());
    close([1.1, 1.1, 1.1], 0.03f64.sqrt());
    close([0.5, 0.5, -0.05], 0.05);
    // Beyond the search limit the distance counts as "far".
    close([5.0, 5.0, 5.0], 0.8);
}

#[test]
fn a_faceted_sphere_is_within_the_sagitta() {
    // Latitude-longitude sphere of radius 1 with n segments.
    let n = 48u32;
    let mut pts = Vec::new();
    for i in 0..=n {
        let th = std::f64::consts::PI * f64::from(i) / f64::from(n);
        for j in 0..n {
            let ph = 2.0 * std::f64::consts::PI * f64::from(j) / f64::from(n);
            pts.push([
                (th.sin() * ph.cos()) as f32,
                (th.sin() * ph.sin()) as f32,
                th.cos() as f32,
            ]);
        }
    }
    let mut tris = Vec::new();
    for i in 0..n {
        for j in 0..n {
            let a = i * n + j;
            let b = i * n + (j + 1) % n;
            let c = (i + 1) * n + j;
            let d = (i + 1) * n + (j + 1) % n;
            tris.push([a, c, d]);
            tris.push([a, d, b]);
        }
    }
    let w = WallDistance::new(&pts, &tris, 0.05, 1.0);
    // The facets lie inside the true sphere by at most the sagitta 1 - cos(pi / n).
    let sagitta = 1.0 - (std::f64::consts::PI / f64::from(n)).cos();
    for p in [[1.5f64, 0.0, 0.0], [0.0, 1.3, 0.4], [-0.4, -0.4, 1.1]] {
        let r = (p[0] * p[0] + p[1] * p[1] + p[2] * p[2]).sqrt();
        let got = w.distance(p);
        assert!(
            got >= r - 1.0 - 1e-6 && got <= r - 1.0 + sagitta + 1e-6,
            "{p:?}: {got} vs {}",
            r - 1.0
        );
    }
}
