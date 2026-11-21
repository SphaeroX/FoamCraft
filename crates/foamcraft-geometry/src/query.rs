// SPDX-License-Identifier: GPL-3.0-or-later
//! Ray and point queries on meshes; used for selection without a window.

use glam::DVec3;

use crate::mesh::Mesh;

#[derive(Debug, Clone, PartialEq)]
pub struct Hit {
    pub triangle: usize,
    pub distance: f64,
    pub point: [f64; 3],
}

fn corners(mesh: &Mesh, t: [u32; 3]) -> [DVec3; 3] {
    t.map(|i| {
        let p = mesh.positions[i as usize];
        DVec3::new(f64::from(p[0]), f64::from(p[1]), f64::from(p[2]))
    })
}

/// Nearest triangle hit by the ray (both sides count). `dir` need not be normalised.
pub fn raycast(mesh: &Mesh, origin: [f64; 3], dir: [f64; 3]) -> Option<Hit> {
    let o = DVec3::from(origin);
    let d = DVec3::from(dir).normalize_or_zero();
    if d == DVec3::ZERO {
        return None;
    }
    let mut best: Option<(usize, f64)> = None;
    for (i, t) in mesh.triangles.iter().enumerate() {
        let [a, b, c] = corners(mesh, *t);
        // Möller-Trumbore intersection.
        let e1 = b - a;
        let e2 = c - a;
        let p = d.cross(e2);
        let det = e1.dot(p);
        if det.abs() < 1e-18 {
            continue;
        }
        let inv = 1.0 / det;
        let s = o - a;
        let u = s.dot(p) * inv;
        if !(0.0..=1.0).contains(&u) {
            continue;
        }
        let q = s.cross(e1);
        let v = d.dot(q) * inv;
        if v < 0.0 || u + v > 1.0 {
            continue;
        }
        let dist = e2.dot(q) * inv;
        if dist > 1e-12 && best.is_none_or(|(_, b)| dist < b) {
            best = Some((i, dist));
        }
    }
    best.map(|(triangle, distance)| Hit {
        triangle,
        distance,
        point: (o + d * distance).into(),
    })
}

/// Closest point on triangle `abc` to `p` (Ericson, Real-Time Collision Detection, 5.1.5).
fn closest_point(p: DVec3, a: DVec3, b: DVec3, c: DVec3) -> DVec3 {
    let ab = b - a;
    let ac = c - a;
    let ap = p - a;
    let (d1, d2) = (ab.dot(ap), ac.dot(ap));
    if d1 <= 0.0 && d2 <= 0.0 {
        return a;
    }
    let bp = p - b;
    let (d3, d4) = (ab.dot(bp), ac.dot(bp));
    if d3 >= 0.0 && d4 <= d3 {
        return b;
    }
    let vc = d1 * d4 - d3 * d2;
    if vc <= 0.0 && d1 >= 0.0 && d3 <= 0.0 {
        return a + ab * (d1 / (d1 - d3));
    }
    let cp = p - c;
    let (d5, d6) = (ab.dot(cp), ac.dot(cp));
    if d6 >= 0.0 && d5 <= d6 {
        return c;
    }
    let vb = d5 * d2 - d1 * d6;
    if vb <= 0.0 && d2 >= 0.0 && d6 <= 0.0 {
        return a + ac * (d2 / (d2 - d6));
    }
    let va = d3 * d6 - d5 * d4;
    if va <= 0.0 && (d4 - d3) >= 0.0 && (d5 - d6) >= 0.0 {
        return b + (c - b) * ((d4 - d3) / ((d4 - d3) + (d5 - d6)));
    }
    let denom = 1.0 / (va + vb + vc);
    a + ab * (vb * denom) + ac * (vc * denom)
}

/// Triangle closest to a point.
pub fn nearest_triangle(mesh: &Mesh, point: [f64; 3]) -> Option<usize> {
    let p = DVec3::from(point);
    mesh.triangles
        .iter()
        .enumerate()
        .map(|(i, t)| {
            let [a, b, c] = corners(mesh, *t);
            (i, closest_point(p, a, b, c).distance_squared(p))
        })
        .min_by(|x, y| x.1.total_cmp(&y.1))
        .map(|(i, _)| i)
}
