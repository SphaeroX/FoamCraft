// SPDX-License-Identifier: GPL-3.0-or-later
//! Flow on the surface (Task_33): mesh neighbours, the tangential field, lines over edges and
//! spheres, even spacing, the attached map and the tufts.

use foamcraft_sim::surface_flow::{
    SurfaceMesh, TangentField, attached, even_lines, flutter_amplitude, flutter_angle,
    reverse_share, trace, tuft_feet, tuft_rest,
};
use glam::DVec3;

/// A flat plate in the XY plane, n × n squares of size 1/n over 0..w × 0..1 (points repeated per
/// triangle, like OpenFOAM output, to check the welding).
fn plate(w: f64, n: u32) -> (Vec<[f32; 3]>, Vec<[u32; 3]>) {
    let mut pts = Vec::new();
    let mut tris = Vec::new();
    let nx = (f64::from(n) * w) as u32;
    let p = |i: u32, j: u32| {
        [
            (f64::from(i) / f64::from(n)) as f32,
            (f64::from(j) / f64::from(n)) as f32,
            0.0,
        ]
    };
    for j in 0..n {
        for i in 0..nx {
            for t in [
                [(i, j), (i + 1, j), (i + 1, j + 1)],
                [(i, j), (i + 1, j + 1), (i, j + 1)],
            ] {
                let base = pts.len() as u32;
                for (a, b) in t {
                    pts.push(p(a, b));
                }
                tris.push([base, base + 1, base + 2]);
            }
        }
    }
    (pts, tris)
}

/// Unit cube 0..1, each face split into n × n squares (outward normals).
fn cube(n: u32) -> (Vec<[f32; 3]>, Vec<[u32; 3]>) {
    let mut pts: Vec<[f32; 3]> = Vec::new();
    let mut tris = Vec::new();
    // Each face: origin, u, v with u × v outward.
    let faces = [
        ([0.0, 0.0, 0.0], [0.0, 1.0, 0.0], [1.0, 0.0, 0.0]),
        ([0.0, 0.0, 1.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]),
        ([0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 0.0, 1.0]),
        ([0.0, 1.0, 0.0], [0.0, 0.0, 1.0], [1.0, 0.0, 0.0]),
        ([0.0, 0.0, 0.0], [0.0, 0.0, 1.0], [0.0, 1.0, 0.0]),
        ([1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]),
    ];
    for (o, u, v) in faces {
        let (o, u, v) = (DVec3::from(o), DVec3::from(u), DVec3::from(v));
        let at = |i: u32, j: u32| {
            let q = o + u * (f64::from(i) / f64::from(n)) + v * (f64::from(j) / f64::from(n));
            [q.x as f32, q.y as f32, q.z as f32]
        };
        for j in 0..n {
            for i in 0..n {
                let base = pts.len() as u32;
                pts.extend([at(i, j), at(i + 1, j), at(i + 1, j + 1), at(i, j + 1)]);
                tris.push([base, base + 1, base + 2]);
                tris.push([base, base + 2, base + 3]);
            }
        }
    }
    (pts, tris)
}

fn sphere(n: u32) -> (Vec<[f32; 3]>, Vec<[u32; 3]>) {
    let mut pts = Vec::new();
    for i in 0..=n {
        let th = std::f64::consts::PI * f64::from(i) / f64::from(n);
        for j in 0..2 * n {
            let ph = std::f64::consts::PI * f64::from(j) / f64::from(n);
            pts.push([
                th.cos() as f32,
                (th.sin() * ph.cos()) as f32,
                (th.sin() * ph.sin()) as f32,
            ]);
        }
    }
    let m = 2 * n;
    let mut tris = Vec::new();
    for i in 0..n {
        for j in 0..m {
            let a = i * m + j;
            let b = i * m + (j + 1) % m;
            let c = (i + 1) * m + j;
            let d = (i + 1) * m + (j + 1) % m;
            tris.push([a, c, d]);
            tris.push([a, d, b]);
        }
    }
    (pts, tris)
}

fn field(mesh: &SurfaceMesh, f: impl Fn(DVec3) -> DVec3) -> Vec<DVec3> {
    (0..mesh.tris.len()).map(|t| f(mesh.centroid(t))).collect()
}

#[test]
fn repeated_points_are_welded_and_neighbours_found() {
    let (pts, tris) = plate(1.0, 4);
    let mesh = SurfaceMesh::new(&pts, &tris, 1e-6);
    assert_eq!(mesh.points.len(), 25);
    assert_eq!(mesh.tris.len(), 32);
    // Inner triangles have three neighbours, the boundary is open.
    let open: usize = mesh
        .neighbors
        .iter()
        .map(|n| n.iter().filter(|x| x.is_none()).count())
        .sum();
    assert_eq!(open, 16);
}

#[test]
fn lines_on_a_plate_are_straight_parallel_and_as_long_as_the_plate() {
    let (pts, tris) = plate(2.0, 10);
    let mesh = SurfaceMesh::new(&pts, &tris, 1e-6);
    let f = TangentField::new(&mesh, &field(&mesh, |_| DVec3::X));
    let lines = even_lines(&mesh, &f, 0.1, 3, 10.0);
    assert!(lines.len() >= 5, "{}", lines.len());
    for l in &lines {
        let y0 = l.points[0].pos.y;
        assert!(l.points.iter().all(|p| (p.pos.y - y0).abs() < 1e-9));
        assert!((l.length() - 2.0).abs() < 0.05, "{}", l.length());
    }
}

#[test]
fn a_line_runs_over_a_cube_edge_without_gap_or_kink() {
    let (pts, tris) = cube(6);
    let mesh = SurfaceMesh::new(&pts, &tris, 1e-6);
    // Flow along +X on the top, down the side x = 1 (the field follows the faces).
    let f = TangentField::new(
        &mesh,
        &field(&mesh, |c| if c.x > 0.999 { -DVec3::Z } else { DVec3::X }),
    );
    let t = (0..mesh.tris.len())
        .find(|t| (mesh.centroid(*t) - DVec3::new(0.5, 0.45, 1.0)).length() < 0.15)
        .unwrap();
    let start = mesh.centroid(t);
    let ds = 0.3 * mesh.mean_edge;
    let line = trace(&mesh, &f, t, start, 1.0, ds, 3.0, &mut |_| false);
    // It comes down the side.
    assert!(
        line.iter().any(|p| p.pos.x > 0.999 && p.pos.z < 0.5),
        "{:?}",
        line.last()
    );
    let mut prev = start;
    for p in &line {
        assert!((p.pos - prev).length() <= 1.5 * ds + 1e-9);
        prev = p.pos;
    }
    // Unfolded about the edge the line stays straight: y does not change.
    assert!(line.iter().all(|p| (p.pos.y - start.y).abs() < 1e-6));
}

#[test]
fn lines_on_a_sphere_stay_on_it_and_follow_the_meridians() {
    let n = 24;
    let (pts, tris) = sphere(n);
    let mesh = SurfaceMesh::new(&pts, &tris, 1e-6);
    // Tangential potential flow from pole (x = -1) to pole (x = +1).
    let f = TangentField::new(
        &mesh,
        &field(&mesh, |c| {
            let r = c.normalize();
            DVec3::X - r * r.x
        }),
    );
    let sagitta = 1.0 - (std::f64::consts::PI / f64::from(n)).cos();
    let t = (0..mesh.tris.len())
        .find(|t| {
            let c = mesh.centroid(*t);
            c.x.abs() < 0.1 && c.y > 0.9
        })
        .unwrap();
    let start = mesh.centroid(t);
    let lon = |p: DVec3| p.z.atan2(p.y);
    let ds = 0.3 * mesh.mean_edge;
    for sign in [1.0, -1.0] {
        let line = trace(&mesh, &f, t, start, sign, ds, 4.0, &mut |_| false);
        assert!(line.len() > 10);
        for p in &line {
            let r = p.pos.length();
            assert!(r <= 1.0 + 1e-9 && r >= 1.0 - sagitta - 1e-9, "{r}");
            if p.pos.x.abs() < 0.95 {
                let d = (lon(p.pos) - lon(start)).abs().to_degrees();
                assert!(d < 2.0, "{d}");
            }
        }
    }
}

#[test]
fn a_line_ends_at_a_zero_of_the_field() {
    let (pts, tris) = plate(2.0, 10);
    let mesh = SurfaceMesh::new(&pts, &tris, 1e-6);
    // Flow towards x = 1 from both sides: a line of zeros there.
    let f = TangentField::new(&mesh, &field(&mesh, |c| DVec3::X * (1.0 - c.x)));
    let t = (0..mesh.tris.len())
        .find(|t| (mesh.centroid(*t) - DVec3::new(0.3, 0.5, 0.0)).length() < 0.1)
        .unwrap();
    let line = trace(&mesh, &f, t, mesh.centroid(t), 1.0, 0.03, 10.0, &mut |_| {
        false
    });
    let last = line.last().unwrap().pos;
    assert!(last.x < 1.05 && last.x > 0.8, "{last}");
}

fn min_distance_between_lines(lines: &[foamcraft_sim::surface_flow::SurfaceLine]) -> f64 {
    let mut best = f64::MAX;
    for (i, a) in lines.iter().enumerate() {
        for b in &lines[i + 1..] {
            for p in &a.points {
                for q in &b.points {
                    best = best.min((p.pos - q.pos).length());
                }
            }
        }
    }
    best
}

#[test]
fn lines_are_evenly_spaced_on_plate_and_sphere() {
    let d_sep = 0.12;
    for (pts, tris) in [plate(2.0, 12), sphere(16)] {
        let mesh = SurfaceMesh::new(&pts, &tris, 1e-6);
        let f = TangentField::new(
            &mesh,
            &field(&mesh, |c| {
                let r = c.normalize_or_zero();
                let v = DVec3::new(1.0, 0.3, 0.0);
                if c.z.abs() < 1e-9 && c.length() > 0.0 && mesh_is_plate(&c) {
                    v
                } else {
                    v - r * r.dot(v)
                }
            }),
        );
        let lines = even_lines(&mesh, &f, d_sep, 5, 10.0);
        assert!(min_distance_between_lines(&lines) >= 0.5 * d_sep - 1e-9);
        // No empty area larger than 3 d_sep: every triangle centroid is within 1.5 d_sep of a
        // line.
        for t in 0..mesh.tris.len() {
            let c = mesh.centroid(t);
            let near = lines
                .iter()
                .flat_map(|l| l.points.iter())
                .any(|p| (p.pos - c).length() <= 1.5 * d_sep);
            assert!(near, "empty around {c}");
        }
        // The same seed gives the same lines.
        let again = even_lines(&mesh, &f, d_sep, 5, 10.0);
        assert_eq!(lines, again);
    }
}

fn mesh_is_plate(c: &DVec3) -> bool {
    c.z.abs() < 1e-12
}

#[test]
fn the_attached_map_and_its_reverse_share() {
    let (pts, tris) = plate(1.0, 4);
    let mesh = SurfaceMesh::new(&pts, &tris, 1e-6);
    let along = attached(&mesh, &field(&mesh, |_| DVec3::X), DVec3::X);
    assert!(along.iter().all(|a| (a - 1.0).abs() < 1e-12));
    let back = attached(&mesh, &field(&mesh, |_| -DVec3::X), DVec3::X);
    assert!(back.iter().all(|a| (a + 1.0).abs() < 1e-12));
    assert!((reverse_share(&mesh, &back) - 1.0).abs() < 1e-12);
    // Half the plate in reverse flow.
    let half = attached(
        &mesh,
        &field(&mesh, |c| if c.x < 0.5 { DVec3::X } else { -DVec3::X }),
        DVec3::X,
    );
    assert!((reverse_share(&mesh, &half) - 0.5).abs() < 1e-12);
}

#[test]
fn tufts_follow_the_field_hang_without_flow_and_keep_their_spacing() {
    let (pts, tris) = plate(2.0, 12);
    let mesh = SurfaceMesh::new(&pts, &tris, 1e-6);
    let f = TangentField::new(&mesh, &field(&mesh, |_| DVec3::new(1.0, 0.0, 0.0)));
    let feet = tuft_feet(&mesh, 0.2, 2, 4000);
    assert!(feet.len() > 10);
    for (i, a) in feet.iter().enumerate() {
        for b in &feet[i + 1..] {
            assert!((a.1 - b.1).length() >= 0.2 - 1e-12);
        }
    }
    let (t, foot) = feet[0];
    let down = -DVec3::Y;
    let rest = tuft_rest(&mesh, &f, t, foot, 0.12, 1.0, down);
    assert_eq!(rest.hang, 0.0);
    let dir = (rest.points[4] - rest.points[0]).normalize();
    assert!(dir.x > 0.98, "{dir}");
    // Lifted 8° off the wall.
    let lift = dir.z.asin().to_degrees();
    assert!((lift - 8.0).abs() < 0.5, "{lift}");
    // No flow: the tuft hangs along gravity in the plane.
    let still = TangentField::new(&mesh, &field(&mesh, |_| DVec3::new(1e-9, 0.0, 0.0)));
    let hang = tuft_rest(&mesh, &still, t, foot, 0.12, 1.0, down);
    assert!(hang.hang > 0.99);
    let dir = (hang.points[4] - hang.points[0]).normalize();
    assert!(dir.y < -0.98, "{dir}");
}

#[test]
fn flutter_is_zero_for_attached_calm_flow_bounded_and_repeatable() {
    assert_eq!(flutter_amplitude(Some(0.0), 1.0), 0.0);
    assert!(flutter_amplitude(Some(10.0), 1.0) <= 55.0 + 1e-12);
    assert!(flutter_amplitude(Some(0.0), -0.5) >= 20.0);
    assert!(flutter_amplitude(None, -0.5) >= 20.0);
    assert_eq!(flutter_amplitude(None, 0.8), 0.0);
    let a = flutter_angle(1.234, 40.0, 5.0, [0.3, 1.1]);
    assert_eq!(a, flutter_angle(1.234, 40.0, 5.0, [0.3, 1.1]));
    for k in 0..200 {
        let x = flutter_angle(f64::from(k) * 0.013, 40.0, 5.0, [0.3, 1.1]);
        assert!(x.abs() <= 40f64.to_radians() + 1e-12);
    }
}
