// SPDX-License-Identifier: GPL-3.0-or-later
//! Distance to the nearest wall: a uniform cell grid over the triangles of a surface. A query
//! searches rings of cells around the point and stops when no unvisited cell can hold a closer
//! triangle; beyond `far` the distance counts as `far`. Used by the ride (Task_32), surface flow
//! lines (Task_33) and heavy particles (Task_34).

use std::collections::HashMap;

use glam::DVec3;

pub struct WallDistance {
    tris: Vec<[DVec3; 3]>,
    cell: f64,
    far: f64,
    cells: HashMap<[i64; 3], Vec<u32>>,
}

/// Closest point on a triangle (Ericson, "Real-Time Collision Detection", 5.1.5).
pub fn closest_on_triangle(p: DVec3, [a, b, c]: [DVec3; 3]) -> DVec3 {
    let (ab, ac, ap) = (b - a, c - a, p - a);
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

impl WallDistance {
    /// Grid with cells of size `cell` (metres); distances above `far` are reported as `far`.
    pub fn new(points: &[[f32; 3]], triangles: &[[u32; 3]], cell: f64, far: f64) -> Self {
        let cell = cell.max(1e-9);
        let tris: Vec<[DVec3; 3]> = triangles
            .iter()
            .map(|t| t.map(|i| DVec3::from(points[i as usize].map(f64::from))))
            .collect();
        let mut cells: HashMap<[i64; 3], Vec<u32>> = HashMap::new();
        for (n, t) in tris.iter().enumerate() {
            let lo = t[0].min(t[1]).min(t[2]);
            let hi = t[0].max(t[1]).max(t[2]);
            let a = (lo / cell).floor().as_i64vec3();
            let b = (hi / cell).floor().as_i64vec3();
            for x in a.x..=b.x {
                for y in a.y..=b.y {
                    for z in a.z..=b.z {
                        cells.entry([x, y, z]).or_default().push(n as u32);
                    }
                }
            }
        }
        Self {
            tris,
            cell,
            far,
            cells,
        }
    }

    /// The triangles of the surface.
    pub fn triangles(&self) -> &[[DVec3; 3]] {
        &self.tris
    }

    /// Triangles in the cells around the segment `a`→`b` (its bounding box), each once.
    pub fn candidates(&self, a: DVec3, b: DVec3) -> Vec<u32> {
        let lo = (a.min(b) / self.cell).floor().as_i64vec3();
        let hi = (a.max(b) / self.cell).floor().as_i64vec3();
        let mut out: Vec<u32> = Vec::new();
        for x in lo.x..=hi.x {
            for y in lo.y..=hi.y {
                for z in lo.z..=hi.z {
                    if let Some(list) = self.cells.get(&[x, y, z]) {
                        out.extend(list);
                    }
                }
            }
        }
        out.sort_unstable();
        out.dedup();
        out
    }

    /// Distance from `p` to the nearest triangle, at most `far`.
    pub fn distance(&self, p: [f64; 3]) -> f64 {
        let p = DVec3::from(p);
        let c = (p / self.cell).floor().as_i64vec3();
        let rings = (self.far / self.cell).ceil() as i64 + 1;
        let mut best = self.far;
        for r in 0..=rings {
            // Every triangle in a cell of ring r is at least (r - 1) cells away.
            if (r - 1) as f64 * self.cell > best {
                break;
            }
            for x in -r..=r {
                for y in -r..=r {
                    for z in -r..=r {
                        if x.abs().max(y.abs()).max(z.abs()) != r {
                            continue;
                        }
                        let Some(list) = self.cells.get(&[c.x + x, c.y + y, c.z + z]) else {
                            continue;
                        };
                        for &n in list {
                            let q = closest_on_triangle(p, self.tris[n as usize]);
                            best = best.min((q - p).length());
                        }
                    }
                }
            }
        }
        best
    }
}
