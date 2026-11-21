// SPDX-License-Identifier: GPL-3.0-or-later
//! Reference quantities of a body in a flow: projected frontal area and length along the flow.

use foamcraft_geometry::Mesh;
use glam::DVec3;

/// Cells along the longer side of the projection; the area error is about one cell row along the
/// outline, well below 1 % for any convex shape.
const RASTER: usize = 2048;

/// Unit flow direction for a yaw angle in degrees (0 = towards +X, 90 = towards +Y), horizontal.
pub fn flow_direction(yaw_deg: f64) -> [f64; 3] {
    let (s, c) = yaw_deg.to_radians().sin_cos();
    [c, s, 0.0]
}

fn points<'a>(meshes: &'a [&'a Mesh]) -> impl Iterator<Item = DVec3> + 'a {
    meshes
        .iter()
        .flat_map(|m| m.positions.iter())
        .map(|p| DVec3::new(f64::from(p[0]), f64::from(p[1]), f64::from(p[2])))
}

/// Extent of the meshes along `direction` (metres when the meshes are in world coordinates).
pub fn length_along(meshes: &[&Mesh], direction: [f64; 3]) -> f64 {
    let d = DVec3::from(direction).normalize_or_zero();
    let (lo, hi) = points(meshes)
        .map(|p| p.dot(d))
        .fold((f64::MAX, f64::MIN), |(lo, hi), x| (lo.min(x), hi.max(x)));
    if hi < lo { 0.0 } else { hi - lo }
}

/// Area of the shadow the meshes cast on a plane normal to `direction` (union of all triangles,
/// rasterised).
pub fn frontal_area(meshes: &[&Mesh], direction: [f64; 3]) -> f64 {
    let d = DVec3::from(direction).normalize_or_zero();
    if d == DVec3::ZERO {
        return 0.0;
    }
    let helper = if d.z.abs() < 0.9 { DVec3::Z } else { DVec3::X };
    let u = d.cross(helper).normalize();
    let v = d.cross(u);
    let project = |p: DVec3| (p.dot(u), p.dot(v));
    let (mut min, mut max) = ((f64::MAX, f64::MAX), (f64::MIN, f64::MIN));
    for p in points(meshes) {
        let (x, y) = project(p);
        min = (min.0.min(x), min.1.min(y));
        max = (max.0.max(x), max.1.max(y));
    }
    let span = (max.0 - min.0).max(max.1 - min.1);
    // Also catches NaN (no points).
    if span.is_nan() || span <= 0.0 {
        return 0.0;
    }
    let cell = span / RASTER as f64;
    let nx = ((max.0 - min.0) / cell).ceil() as usize + 1;
    let ny = ((max.1 - min.1) / cell).ceil() as usize + 1;
    let mut covered = vec![false; nx * ny];
    for mesh in meshes {
        for t in &mesh.triangles {
            let [a, b, c] = t.map(|i| {
                let p = mesh.positions[i as usize];
                let (x, y) = project(DVec3::new(
                    f64::from(p[0]),
                    f64::from(p[1]),
                    f64::from(p[2]),
                ));
                ((x - min.0) / cell, (y - min.1) / cell)
            });
            let area2 = (b.0 - a.0) * (c.1 - a.1) - (b.1 - a.1) * (c.0 - a.0);
            if area2.abs() < 1e-12 {
                continue;
            }
            let x0 = a.0.min(b.0).min(c.0).floor().max(0.0) as usize;
            let x1 = (a.0.max(b.0).max(c.0).ceil() as usize).min(nx - 1);
            let y0 = a.1.min(b.1).min(c.1).floor().max(0.0) as usize;
            let y1 = (a.1.max(b.1).max(c.1).ceil() as usize).min(ny - 1);
            let edge = |p: (f64, f64), q: (f64, f64), x: f64, y: f64| {
                ((q.0 - p.0) * (y - p.1) - (q.1 - p.1) * (x - p.0)) * area2.signum()
            };
            for iy in y0..=y1 {
                for ix in x0..=x1 {
                    // Cell centre inside the triangle (edges inclusive, so shared edges leave no gaps).
                    let (x, y) = (ix as f64 + 0.5, iy as f64 + 0.5);
                    if edge(a, b, x, y) >= 0.0 && edge(b, c, x, y) >= 0.0 && edge(c, a, x, y) >= 0.0
                    {
                        covered[iy * nx + ix] = true;
                    }
                }
            }
        }
    }
    covered.iter().filter(|c| **c).count() as f64 * cell * cell
}
