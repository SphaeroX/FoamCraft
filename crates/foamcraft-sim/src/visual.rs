// SPDX-License-Identifier: GPL-3.0-or-later
//! Flow visualisation on the sampled grids: slices through the volume, isosurfaces (marching
//! tetrahedra) and derived fields (speed, Q criterion). Grids are given coarse to fine; a finer
//! grid takes over inside its box.

use crate::results::Grid;

/// Triangles with one value per vertex.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct FieldMesh {
    pub positions: Vec<[f32; 3]>,
    pub triangles: Vec<[u32; 3]>,
    pub values: Vec<f32>,
}

/// Value of the finest grid that has one at `p`.
pub fn sample_levels(levels: &[Grid], field: &str, p: [f64; 3]) -> Option<f64> {
    levels.iter().rev().find_map(|g| g.sample(field, p))
}

fn sub(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}
fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}
fn unit(a: [f64; 3]) -> [f64; 3] {
    let l = dot(a, a).sqrt();
    if l > 0.0 {
        a.map(|x| x / l)
    } else {
        [0.0, 0.0, 1.0]
    }
}

/// A plane through `origin` with `normal`, sampled with `n` × `n` points across the box of the
/// coarsest grid; quads with four values become two triangles.
pub fn slice(
    levels: &[Grid],
    field: &str,
    origin: [f64; 3],
    normal: [f64; 3],
    n: u32,
) -> FieldMesh {
    let Some(coarse) = levels.first() else {
        return FieldMesh::default();
    };
    let (lo, hi) = (coarse.spec.min, coarse.spec.max);
    let nrm = unit(normal);
    let helper = if nrm[0].abs() < 0.9 {
        [1.0, 0.0, 0.0]
    } else {
        [0.0, 1.0, 0.0]
    };
    let u = unit(cross(nrm, helper));
    let v = cross(nrm, u);
    let (mut u0, mut u1, mut v0, mut v1) = (f64::MAX, f64::MIN, f64::MAX, f64::MIN);
    for c in 0..8 {
        let corner = [0, 1, 2].map(|k| if c >> k & 1 == 1 { hi[k] } else { lo[k] });
        let d = sub(corner, origin);
        (u0, u1) = (u0.min(dot(d, u)), u1.max(dot(d, u)));
        (v0, v1) = (v0.min(dot(d, v)), v1.max(dot(d, v)));
    }
    let n = n.max(2);
    let mut index = vec![u32::MAX; (n * n) as usize];
    let mut out = FieldMesh::default();
    for j in 0..n {
        for i in 0..n {
            let a = u0 + (u1 - u0) * f64::from(i) / f64::from(n - 1);
            let b = v0 + (v1 - v0) * f64::from(j) / f64::from(n - 1);
            let p = [0, 1, 2].map(|k| origin[k] + a * u[k] + b * v[k]);
            if (0..3).any(|k| p[k] < lo[k] || p[k] > hi[k]) {
                continue;
            }
            if let Some(value) = sample_levels(levels, field, p) {
                index[(j * n + i) as usize] = out.positions.len() as u32;
                out.positions.push(p.map(|x| x as f32));
                out.values.push(value as f32);
            }
        }
    }
    for j in 0..n - 1 {
        for i in 0..n - 1 {
            let q = [(i, j), (i + 1, j), (i + 1, j + 1), (i, j + 1)]
                .map(|(x, y)| index[(y * n + x) as usize]);
            if q.contains(&u32::MAX) {
                continue;
            }
            out.triangles.push([q[0], q[1], q[2]]);
            out.triangles.push([q[0], q[2], q[3]]);
        }
    }
    out
}

/// Adds the speed `U_mag` and the Q criterion `Q` = (|Ω|² − |S|²) / 2 (central differences,
/// one-sided at edges and next to masked points) to a grid with `Ux`, `Uy`, `Uz`.
pub fn add_derived(grid: &mut Grid) {
    let pos = |name: &str| grid.names.iter().position(|n| n == name);
    let (Some(ux), Some(uy), Some(uz)) = (pos("Ux"), pos("Uy"), pos("Uz")) else {
        return;
    };
    let spec = grid.spec.clone();
    let step = spec.step();
    let d = spec.dims;
    let n = spec.len();
    let comp = [ux, uy, uz];
    let value =
        |c: usize, i: u32, j: u32, k: u32| f64::from(grid.values[comp[c]][spec.index(i, j, k)]);
    let live = |i: i64, j: i64, k: i64| {
        i >= 0
            && j >= 0
            && k >= 0
            && i < i64::from(d[0])
            && j < i64::from(d[1])
            && k < i64::from(d[2])
            && !grid.mask[spec.index(i as u32, j as u32, k as u32)]
    };
    let mut speed = vec![0.0f32; n];
    let mut q = vec![0.0f32; n];
    for k in 0..d[2] {
        for j in 0..d[1] {
            for i in 0..d[0] {
                let id = spec.index(i, j, k);
                if grid.mask[id] {
                    continue;
                }
                speed[id] = (0..3)
                    .map(|c| value(c, i, j, k).powi(2))
                    .sum::<f64>()
                    .sqrt() as f32;
                // g[c][a] = d u_c / d x_a
                let mut g = [[0.0; 3]; 3];
                for a in 0..3 {
                    let mut lo = [i64::from(i), i64::from(j), i64::from(k)];
                    let mut hi = lo;
                    lo[a] -= 1;
                    hi[a] += 1;
                    let (l_ok, h_ok) = (live(lo[0], lo[1], lo[2]), live(hi[0], hi[1], hi[2]));
                    let here = [i64::from(i), i64::from(j), i64::from(k)];
                    let (from, to, span) = match (l_ok, h_ok) {
                        (true, true) => (lo, hi, 2.0),
                        (true, false) => (lo, here, 1.0),
                        (false, true) => (here, hi, 1.0),
                        (false, false) => continue,
                    };
                    for (c, row) in g.iter_mut().enumerate() {
                        let f = value(c, from[0] as u32, from[1] as u32, from[2] as u32);
                        let t = value(c, to[0] as u32, to[1] as u32, to[2] as u32);
                        row[a] = (t - f) / (span * step[a]);
                    }
                }
                // |Ω|² − |S|² with S and Ω the symmetric and antisymmetric parts of g.
                let sum: f64 = (0..9)
                    .map(|n| {
                        let (a, b) = (n / 3, n % 3);
                        let s = 0.5 * (g[a][b] + g[b][a]);
                        let w = 0.5 * (g[a][b] - g[b][a]);
                        w * w - s * s
                    })
                    .sum();
                q[id] = (0.5 * sum) as f32;
            }
        }
    }
    grid.names.push("U_mag".into());
    grid.values.push(speed);
    grid.names.push("Q".into());
    grid.values.push(q);
}

/// Tetrahedra of a cube (Kuhn split along the diagonal 0–7; corner bits are x, y, z).
const TETS: [[usize; 4]; 6] = [
    [0, 1, 3, 7],
    [0, 1, 5, 7],
    [0, 2, 3, 7],
    [0, 2, 6, 7],
    [0, 4, 5, 7],
    [0, 4, 6, 7],
];

/// The surface where `field` equals `iso` (marching tetrahedra). Cells of a grid whose centre
/// lies inside a finer grid's box are left to the finer grid.
pub fn isosurface(levels: &[Grid], field: &str, iso: f64) -> FieldMesh {
    let mut out = FieldMesh::default();
    for (l, grid) in levels.iter().enumerate() {
        let Some(f) = grid.names.iter().position(|n| n == field) else {
            continue;
        };
        let spec = &grid.spec;
        let d = spec.dims;
        let finer = &levels[l + 1..];
        for k in 0..d[2].saturating_sub(1) {
            for j in 0..d[1].saturating_sub(1) {
                for i in 0..d[0].saturating_sub(1) {
                    let corners: Vec<(usize, [f64; 3])> = (0..8u32)
                        .map(|c| {
                            let (a, b, e) = (i + (c & 1), j + (c >> 1 & 1), k + (c >> 2 & 1));
                            (spec.index(a, b, e), spec.point(a, b, e))
                        })
                        .collect();
                    if corners.iter().any(|(id, _)| grid.mask[*id]) {
                        continue;
                    }
                    let centre = [0, 1, 2].map(|a| (corners[0].1[a] + corners[7].1[a]) / 2.0);
                    if finer.iter().any(|g| {
                        (0..3).all(|a| centre[a] > g.spec.min[a] && centre[a] < g.spec.max[a])
                    }) {
                        continue;
                    }
                    let vals: Vec<f64> = corners
                        .iter()
                        .map(|(id, _)| f64::from(grid.values[f][*id]))
                        .collect();
                    if vals.iter().all(|v| *v < iso) || vals.iter().all(|v| *v >= iso) {
                        continue;
                    }
                    for tet in TETS {
                        march(&mut out, tet.map(|c| (corners[c].1, vals[c])), iso);
                    }
                }
            }
        }
    }
    out
}

fn march(out: &mut FieldMesh, tet: [([f64; 3], f64); 4], iso: f64) {
    let inside: Vec<usize> = (0..4).filter(|v| tet[*v].1 < iso).collect();
    let outside: Vec<usize> = (0..4).filter(|v| tet[*v].1 >= iso).collect();
    let cut = |a: usize, b: usize| {
        let (pa, va) = tet[a];
        let (pb, vb) = tet[b];
        let t = ((iso - va) / (vb - va)).clamp(0.0, 1.0);
        [0, 1, 2].map(|k| (pa[k] + t * (pb[k] - pa[k])) as f32)
    };
    let mut push = |ps: &[[f32; 3]]| {
        let base = out.positions.len() as u32;
        out.positions.extend_from_slice(ps);
        out.values.extend(std::iter::repeat_n(iso as f32, ps.len()));
        out.triangles.push([base, base + 1, base + 2]);
        if ps.len() == 4 {
            out.triangles.push([base, base + 2, base + 3]);
        }
    };
    match (inside.len(), outside.len()) {
        (1, 3) => push(&[
            cut(inside[0], outside[0]),
            cut(inside[0], outside[1]),
            cut(inside[0], outside[2]),
        ]),
        (3, 1) => push(&[
            cut(outside[0], inside[0]),
            cut(outside[0], inside[1]),
            cut(outside[0], inside[2]),
        ]),
        (2, 2) => push(&[
            cut(inside[0], outside[0]),
            cut(inside[0], outside[1]),
            cut(inside[1], outside[1]),
            cut(inside[1], outside[0]),
        ]),
        _ => {}
    }
}
