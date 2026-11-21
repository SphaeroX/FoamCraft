// SPDX-License-Identifier: GPL-3.0-or-later
//! Flow on the surface (Task_33): lines that follow the near-wall flow over the model like an oil
//! film in a wind tunnel, evenly spaced, and wool tufts. Pure logic on a triangle surface: the
//! direction field per triangle is `-wall shear stress` (the near-wall flow) or, for old runs, the
//! tangential velocity just off the wall.

use std::collections::HashMap;

use glam::DVec3;

use crate::particles::random;

/// Angle between triangle normals above which an edge counts as sharp (no smoothing across it).
pub const SHARP_EDGE_DEG: f64 = 40.0;

/// A welded triangle surface with neighbours across edges.
pub struct SurfaceMesh {
    pub points: Vec<DVec3>,
    pub tris: Vec<[u32; 3]>,
    pub normals: Vec<DVec3>,
    pub areas: Vec<f64>,
    /// Neighbour across edge k (from corner k to corner k + 1).
    pub neighbors: Vec<[Option<u32>; 3]>,
    pub mean_edge: f64,
    /// Triangles around each point.
    pub around: Vec<Vec<u32>>,
    /// Index of each triangle in the input (degenerate ones are left out).
    pub source: Vec<u32>,
}

impl SurfaceMesh {
    /// Points closer than `weld` are merged (OpenFOAM output may repeat points).
    pub fn new(points: &[[f32; 3]], tris: &[[u32; 3]], weld: f64) -> Self {
        let weld = weld.max(1e-12);
        let mut map: HashMap<[i64; 3], u32> = HashMap::new();
        let mut index = Vec::with_capacity(points.len());
        let mut merged: Vec<DVec3> = Vec::new();
        for p in points {
            let v = DVec3::from(p.map(f64::from));
            let key = (v / weld).round().as_i64vec3().to_array();
            let id = *map.entry(key).or_insert_with(|| {
                merged.push(v);
                (merged.len() - 1) as u32
            });
            index.push(id);
        }
        let (tris, source): (Vec<[u32; 3]>, Vec<u32>) = tris
            .iter()
            .enumerate()
            .map(|(n, t)| (t.map(|i| index[i as usize]), n as u32))
            .filter(|(t, _)| t[0] != t[1] && t[1] != t[2] && t[0] != t[2])
            .unzip();
        let mut normals = Vec::with_capacity(tris.len());
        let mut areas = Vec::with_capacity(tris.len());
        let mut edges: HashMap<(u32, u32), Vec<(u32, usize)>> = HashMap::new();
        let mut around = vec![Vec::new(); merged.len()];
        let mut edge_sum = 0.0;
        for (n, t) in tris.iter().enumerate() {
            let [a, b, c] = t.map(|i| merged[i as usize]);
            let cross = (b - a).cross(c - a);
            normals.push(cross.normalize_or_zero());
            areas.push(0.5 * cross.length());
            edge_sum += (b - a).length() + (c - b).length() + (a - c).length();
            for k in 0..3 {
                let (i, j) = (t[k], t[(k + 1) % 3]);
                edges
                    .entry((i.min(j), i.max(j)))
                    .or_default()
                    .push((n as u32, k));
                around[t[k] as usize].push(n as u32);
            }
        }
        let mut neighbors = vec![[None; 3]; tris.len()];
        for list in edges.values() {
            if list.len() == 2 {
                let (a, ka) = list[0];
                let (b, kb) = list[1];
                neighbors[a as usize][ka] = Some(b);
                neighbors[b as usize][kb] = Some(a);
            }
        }
        let mean_edge = if tris.is_empty() {
            0.0
        } else {
            edge_sum / (3 * tris.len()) as f64
        };
        Self {
            points: merged,
            tris,
            normals,
            areas,
            neighbors,
            mean_edge,
            around,
            source,
        }
    }

    pub fn corners(&self, t: usize) -> [DVec3; 3] {
        self.tris[t].map(|i| self.points[i as usize])
    }

    pub fn centroid(&self, t: usize) -> DVec3 {
        let [a, b, c] = self.corners(t);
        (a + b + c) / 3.0
    }

    /// Barycentric coordinates of `p` (projected into the triangle's plane).
    pub fn bary(&self, t: usize, p: DVec3) -> [f64; 3] {
        let [a, b, c] = self.corners(t);
        let (v0, v1, v2) = (b - a, c - a, p - a);
        let (d00, d01, d11) = (v0.dot(v0), v0.dot(v1), v1.dot(v1));
        let (d20, d21) = (v2.dot(v0), v2.dot(v1));
        let den = d00 * d11 - d01 * d01;
        if den.abs() < 1e-300 {
            return [1.0 / 3.0; 3];
        }
        let v = (d11 * d20 - d01 * d21) / den;
        let w = (d00 * d21 - d01 * d20) / den;
        [1.0 - v - w, v, w]
    }

    pub fn total_area(&self) -> f64 {
        self.areas.iter().sum()
    }

    /// Bounding box size (largest side).
    pub fn size(&self) -> f64 {
        let (mut lo, mut hi) = (DVec3::splat(f64::MAX), DVec3::splat(f64::MIN));
        for p in &self.points {
            lo = lo.min(*p);
            hi = hi.max(*p);
        }
        if lo.x > hi.x {
            0.0
        } else {
            (hi - lo).max_element()
        }
    }
}

fn in_plane(v: DVec3, n: DVec3) -> DVec3 {
    v - n * v.dot(n)
}

/// A tangential field, smooth across gentle edges: one vector per triangle corner, the
/// area-weighted mean of the triangles around the point whose normal is within 40° of the
/// corner's triangle.
pub struct TangentField {
    corner: Vec<[DVec3; 3]>,
    /// Largest magnitude per triangle.
    pub max: f64,
    per_tri: Vec<DVec3>,
}

impl TangentField {
    pub fn new(mesh: &SurfaceMesh, per_tri: &[DVec3]) -> Self {
        let cos_sharp = SHARP_EDGE_DEG.to_radians().cos();
        let flat: Vec<DVec3> = per_tri
            .iter()
            .zip(&mesh.normals)
            .map(|(v, n)| in_plane(*v, *n))
            .collect();
        let mut corner = Vec::with_capacity(mesh.tris.len());
        for (t, tri) in mesh.tris.iter().enumerate() {
            let n = mesh.normals[t];
            corner.push(tri.map(|p| {
                let mut sum = DVec3::ZERO;
                for &o in &mesh.around[p as usize] {
                    let o = o as usize;
                    if mesh.normals[o].dot(n) >= cos_sharp {
                        sum += flat[o] * mesh.areas[o];
                    }
                }
                let w: f64 = mesh.around[p as usize]
                    .iter()
                    .filter(|o| mesh.normals[**o as usize].dot(n) >= cos_sharp)
                    .map(|o| mesh.areas[*o as usize])
                    .sum();
                if w > 0.0 { sum / w } else { flat[t] }
            }));
        }
        let max = flat.iter().map(|v| v.length()).fold(0.0, f64::max);
        Self {
            corner,
            max,
            per_tri: flat,
        }
    }

    /// The field at `p` in triangle `t`, in the triangle's plane.
    pub fn at(&self, mesh: &SurfaceMesh, t: usize, p: DVec3) -> DVec3 {
        let b = mesh.bary(t, p).map(|x| x.clamp(0.0, 1.0));
        let s = b[0] + b[1] + b[2];
        let c = &self.corner[t];
        let v = (c[0] * b[0] + c[1] * b[1] + c[2] * b[2]) / s.max(1e-12);
        in_plane(v, mesh.normals[t])
    }

    /// The (projected) vector of triangle `t` itself.
    pub fn triangle(&self, t: usize) -> DVec3 {
        self.per_tri[t]
    }
}

/// Why a step stopped.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stop {
    /// Reached the boundary of the surface.
    Boundary,
    /// The field vanishes (a critical point).
    Critical,
}

/// Moves `len` along the surface from `p` in triangle `t` in direction `dir` (in the plane),
/// crossing edges by folding the direction about the shared edge. Answers the new triangle,
/// point and direction, or where it stopped.
pub fn advance(
    mesh: &SurfaceMesh,
    mut t: usize,
    mut p: DVec3,
    dir: DVec3,
    len: f64,
) -> Result<(usize, DVec3, DVec3), (usize, DVec3, Stop)> {
    let mut dir = in_plane(dir, mesh.normals[t]).normalize_or_zero();
    if dir == DVec3::ZERO {
        return Err((t, p, Stop::Critical));
    }
    let mut remaining = len;
    let mut entered: Option<usize> = None;
    for _ in 0..8 {
        let corners = mesh.corners(t);
        let n = mesh.normals[t];
        // Smallest distance to an edge along `dir`.
        let mut exit: Option<(f64, usize)> = None;
        for k in 0..3 {
            if entered == Some(k) {
                continue;
            }
            let (a, b, c) = (corners[k], corners[(k + 1) % 3], corners[(k + 2) % 3]);
            let mut m = n.cross(b - a).normalize_or_zero();
            if (c - a).dot(m) < 0.0 {
                m = -m;
            }
            let rate = dir.dot(m);
            if rate >= -1e-15 {
                continue;
            }
            let dist = (p - a).dot(m).max(0.0);
            let s = dist / -rate;
            if exit.is_none_or(|(e, _)| s < e) {
                exit = Some((s, k));
            }
        }
        let Some((s, k)) = exit else {
            p += dir * remaining;
            return Ok((t, p, dir));
        };
        if s >= remaining {
            p += dir * remaining;
            return Ok((t, p, dir));
        }
        p += dir * s;
        remaining -= s;
        let Some(nb) = mesh.neighbors[t][k] else {
            return Err((t, p, Stop::Boundary));
        };
        let nb = nb as usize;
        // Fold the direction about the shared edge into the neighbour's plane.
        let (a, b) = (corners[k], corners[(k + 1) % 3]);
        let e = (b - a).normalize_or_zero();
        let (n1, n2) = (n, mesh.normals[nb]);
        let angle = n1.cross(n2).dot(e).atan2(n1.dot(n2));
        let rot = glam::DQuat::from_axis_angle(e, angle);
        dir = in_plane(rot * dir, n2).normalize_or_zero();
        if dir == DVec3::ZERO {
            return Err((nb, p, Stop::Critical));
        }
        // The edge we came through, seen from the neighbour.
        let tri = mesh.tris[t];
        let (i, j) = (tri[k], tri[(k + 1) % 3]);
        let ntri = mesh.tris[nb];
        entered = (0..3).find(|q| {
            let (x, y) = (ntri[*q], ntri[(*q + 1) % 3]);
            (x == i && y == j) || (x == j && y == i)
        });
        t = nb;
    }
    Err((t, p, Stop::Boundary))
}

/// One point of a surface line.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LinePoint {
    pub pos: DVec3,
    pub tri: usize,
    /// Field magnitude there.
    pub strength: f64,
}

/// Traces from (`t`, `p`) along `sign · field` with steps `ds` (midpoint rule) until the boundary,
/// a critical point, `max_len`, or until `stop` says so for a new point.
#[allow(clippy::too_many_arguments)]
pub fn trace(
    mesh: &SurfaceMesh,
    field: &TangentField,
    t: usize,
    p: DVec3,
    sign: f64,
    ds: f64,
    max_len: f64,
    stop: &mut dyn FnMut(DVec3) -> bool,
) -> Vec<LinePoint> {
    let mut out = Vec::new();
    let (mut t, mut p) = (t, p);
    let critical = 1e-4 * field.max;
    let mut len = 0.0;
    while len < max_len {
        let f1 = field.at(mesh, t, p) * sign;
        if f1.length() <= critical {
            break;
        }
        // Midpoint: half a step, the direction there.
        let mid = match advance(mesh, t, p, f1, 0.5 * ds) {
            Ok((mt, mp, _)) => field.at(mesh, mt, mp) * sign,
            Err(_) => f1,
        };
        let dir = if mid.length() > critical { mid } else { f1 };
        match advance(mesh, t, p, dir, ds) {
            Ok((nt, np, _)) => {
                len += (np - p).length().max(ds * 0.999);
                t = nt;
                p = np;
                if stop(p) {
                    break;
                }
                out.push(LinePoint {
                    pos: p,
                    tri: t,
                    strength: field.at(mesh, t, p).length(),
                });
            }
            Err((nt, np, _)) => {
                if (np - p).length() > 1e-12 && !stop(np) {
                    out.push(LinePoint {
                        pos: np,
                        tri: nt,
                        strength: field.at(mesh, nt, np).length(),
                    });
                }
                break;
            }
        }
    }
    out
}

/// A finished surface line with arc length.
#[derive(Debug, Clone, PartialEq)]
pub struct SurfaceLine {
    pub points: Vec<LinePoint>,
    pub s: Vec<f64>,
    /// Phase of the moving light, 0..1 (from the seed).
    pub phase: f64,
}

impl SurfaceLine {
    pub fn length(&self) -> f64 {
        self.s.last().copied().unwrap_or(0.0)
    }

    /// Running time along the line: arc length over a speed proportional to the square root of
    /// the strength (relative to `reference`), so fast regions run faster but not by orders of
    /// magnitude.
    pub fn runtimes(&self, reference: f64) -> Vec<f64> {
        let speed = |w: f64| (w / reference.max(1e-30)).max(0.0).sqrt().max(0.05);
        let mut out = Vec::with_capacity(self.points.len());
        let mut acc = 0.0;
        for i in 0..self.points.len() {
            if i > 0 {
                let ds = self.s[i] - self.s[i - 1];
                let v = 0.5 * (speed(self.points[i].strength) + speed(self.points[i - 1].strength));
                acc += ds / v;
            }
            out.push(acc);
        }
        out
    }
}

/// Points of lines kept in cells of size `cell`, to find the nearest point of another line.
struct Occupancy {
    cell: f64,
    map: HashMap<[i64; 3], Vec<(u32, DVec3)>>,
}

impl Occupancy {
    fn new(cell: f64) -> Self {
        Self {
            cell: cell.max(1e-12),
            map: HashMap::new(),
        }
    }

    fn key(&self, p: DVec3) -> [i64; 3] {
        (p / self.cell).floor().as_i64vec3().to_array()
    }

    /// True when a point of a line other than `own` lies within `d` of `p` (d <= cell).
    fn near(&self, p: DVec3, d: f64, own: Option<u32>) -> bool {
        let k = self.key(p);
        for x in -1..=1 {
            for y in -1..=1 {
                for z in -1..=1 {
                    if let Some(list) = self.map.get(&[k[0] + x, k[1] + y, k[2] + z])
                        && list
                            .iter()
                            .any(|(id, q)| Some(*id) != own && (*q - p).length() < d)
                    {
                        return true;
                    }
                }
            }
        }
        false
    }

    fn add(&mut self, id: u32, p: DVec3) {
        let k = self.key(p);
        self.map.entry(k).or_default().push((id, p));
    }
}

/// A random point on the surface, area weighted: triangle and point.
fn sample_point(mesh: &SurfaceMesh, cum: &[f64], i: u32, seed: u32) -> (usize, DVec3) {
    let total = *cum.last().unwrap_or(&0.0);
    let r = f64::from(random(i, 0, seed, 31)) * total;
    let t = cum.partition_point(|c| *c < r).min(mesh.tris.len() - 1);
    let (mut u, mut v) = (
        f64::from(random(i, 0, seed, 32)),
        f64::from(random(i, 0, seed, 33)),
    );
    if u + v > 1.0 {
        (u, v) = (1.0 - u, 1.0 - v);
    }
    let [a, b, c] = mesh.corners(t);
    (t, a + (b - a) * u + (c - a) * v)
}

fn cumulative(mesh: &SurfaceMesh) -> Vec<f64> {
    let mut acc = 0.0;
    mesh.areas
        .iter()
        .map(|a| {
            acc += a;
            acc
        })
        .collect()
}

/// Evenly spaced lines over the surface: seeds in a fixed random order (area weighted) are
/// skipped near an existing line; a line ends half a spacing before another one; lines shorter
/// than four spacings are dropped. `max_len` limits each direction.
pub fn even_lines(
    mesh: &SurfaceMesh,
    field: &TangentField,
    d_sep: f64,
    seed: u32,
    max_len: f64,
) -> Vec<SurfaceLine> {
    if mesh.tris.is_empty() || d_sep <= 0.0 {
        return vec![];
    }
    let cum = cumulative(mesh);
    let ds = (0.3 * mesh.mean_edge).min(0.25 * d_sep).max(1e-9);
    let mut occ = Occupancy::new(d_sep);
    let mut lines = Vec::new();
    let seeds = ((4.0 * mesh.total_area() / (d_sep * d_sep)).ceil() as u32).clamp(64, 400_000);
    for i in 0..seeds {
        let (t, p) = sample_point(mesh, &cum, i, seed);
        if occ.near(p, d_sep, None) {
            continue;
        }
        let id = lines.len() as u32;
        let mut stop = |q: DVec3| occ.near(q, 0.5 * d_sep, None);
        let fwd = trace(mesh, field, t, p, 1.0, ds, max_len, &mut stop);
        let back = trace(mesh, field, t, p, -1.0, ds, max_len, &mut stop);
        let strength = field.at(mesh, t, p).length();
        let mut points: Vec<LinePoint> = back.into_iter().rev().collect();
        points.push(LinePoint {
            pos: p,
            tri: t,
            strength,
        });
        points.extend(fwd);
        let mut s = vec![0.0; points.len()];
        for k in 1..points.len() {
            s[k] = s[k - 1] + (points[k].pos - points[k - 1].pos).length();
        }
        if s.last().copied().unwrap_or(0.0) < 4.0 * d_sep {
            continue;
        }
        for q in &points {
            occ.add(id, q.pos);
        }
        lines.push(SurfaceLine {
            points,
            s,
            phase: f64::from(random(id, 0, seed, 34)),
        });
    }
    lines
}

/// Per triangle: cosine between the near-wall flow and the oncoming flow, both in the triangle's
/// plane: +1 attached along the flow, -1 reverse flow (0 where either vanishes).
pub fn attached(mesh: &SurfaceMesh, per_tri: &[DVec3], oncoming: DVec3) -> Vec<f64> {
    per_tri
        .iter()
        .zip(&mesh.normals)
        .map(|(v, n)| {
            let a = in_plane(*v, *n).normalize_or_zero();
            let b = in_plane(oncoming, *n).normalize_or_zero();
            a.dot(b)
        })
        .collect()
}

/// Share of the surface area in reverse flow (attached < 0).
pub fn reverse_share(mesh: &SurfaceMesh, attached: &[f64]) -> f64 {
    let total = mesh.total_area();
    if total <= 0.0 {
        return 0.0;
    }
    attached
        .iter()
        .zip(&mesh.areas)
        .filter(|(a, _)| **a < 0.0)
        .map(|(_, w)| w)
        .sum::<f64>()
        / total
}

// --- Tufts -----------------------------------------------------------------------------------

/// Foot points with at least `spacing` between them, at most `max` (fixed random order).
pub fn tuft_feet(mesh: &SurfaceMesh, spacing: f64, seed: u32, max: usize) -> Vec<(usize, DVec3)> {
    if mesh.tris.is_empty() || spacing <= 0.0 {
        return vec![];
    }
    let cum = cumulative(mesh);
    let mut occ = Occupancy::new(spacing);
    let mut out = Vec::new();
    let tries = ((6.0 * mesh.total_area() / (spacing * spacing)).ceil() as u32).clamp(64, 400_000);
    for i in 0..tries {
        if out.len() >= max {
            break;
        }
        let (t, p) = sample_point(mesh, &cum, i, seed.wrapping_add(7));
        if occ.near(p, spacing, None) {
            continue;
        }
        occ.add(out.len() as u32, p);
        out.push((t, p));
    }
    out
}

/// Lift of a tuft off the wall, degrees.
pub const TUFT_LIFT_DEG: f64 = 8.0;
/// Below this share of the 90th percentile strength the tuft hangs.
pub const HANG_SHARE: f64 = 0.02;

/// A tuft at rest: five points from the foot (four links), each link along the field at its
/// start; hanging (along gravity in the plane) where the flow has no strength.
#[derive(Debug, Clone, PartialEq)]
pub struct Tuft {
    pub foot: DVec3,
    pub normal: DVec3,
    pub points: [DVec3; 5],
    /// Sideways direction (in the plane, across the tuft) of the flutter.
    pub lateral: DVec3,
    pub length: f64,
    /// 0 = full flow, 1 = hanging.
    pub hang: f64,
}

fn smoothstep(e0: f64, e1: f64, x: f64) -> f64 {
    let t = ((x - e0) / (e1 - e0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// The rest shape of a tuft of length `len` at (`t`, `foot`); `reference` is the 90th percentile
/// field strength on the model, `gravity` the down direction.
pub fn tuft_rest(
    mesh: &SurfaceMesh,
    field: &TangentField,
    t: usize,
    foot: DVec3,
    len: f64,
    reference: f64,
    gravity: DVec3,
) -> Tuft {
    let n = mesh.normals[t];
    let lift = TUFT_LIFT_DEG.to_radians();
    let f = field.at(mesh, t, foot);
    let strength = f.length();
    // 0 at no strength, 1 from 2 % of the reference on.
    let flow = smoothstep(0.0, HANG_SHARE * reference.max(1e-30), strength);
    let hang = 1.0 - flow;
    let down = in_plane(gravity, n).normalize_or_zero();
    let mut points = [foot; 5];
    let seg = len / 4.0;
    if hang < 1e-6 {
        // Each link follows the field at its start, along the surface, lifted off the wall.
        let (mut tt, mut p) = (t, foot);
        for (k, slot) in points.iter_mut().enumerate().skip(1) {
            let d = field.at(mesh, tt, p);
            match advance(mesh, tt, p, d, seg) {
                Ok((nt, np, _)) => {
                    tt = nt;
                    p = np;
                }
                Err((nt, np, _)) => {
                    tt = nt;
                    p = np;
                }
            }
            *slot = p + mesh.normals[tt] * (k as f64 * seg * lift.sin());
        }
    } else {
        let along = f.normalize_or_zero();
        let mut dir = (along * flow + down * hang).normalize_or_zero();
        let mut length = len;
        if down == DVec3::ZERO || dir == DVec3::ZERO {
            // On a level face there is no "down" in the plane: keep the direction, shorter.
            dir = if along == DVec3::ZERO {
                in_plane(DVec3::X, n).normalize_or(in_plane(DVec3::Y, n).normalize())
            } else {
                along
            };
            length = len * (1.0 - 0.5 * hang);
        }
        let dir = (dir * lift.cos() + n * lift.sin()).normalize();
        for (k, q) in points.iter_mut().enumerate() {
            *q = foot + dir * (k as f64 * length / 4.0);
        }
    }
    let axis = (points[4] - points[0]).normalize_or(in_plane(DVec3::X, n).normalize_or(DVec3::Y));
    let lateral = n.cross(axis).normalize_or_zero();
    Tuft {
        foot,
        normal: n,
        points,
        lateral,
        length: len,
        hang,
    }
}

/// Flutter amplitude in degrees: up to 55° with the turbulence intensity, at least 20° in reverse
/// flow; without a turbulence value only the part from reverse flow.
pub fn flutter_amplitude(tu: Option<f64>, attached: f64) -> f64 {
    let from_tu = tu.map_or(0.0, |tu| 55.0 * smoothstep(0.10, 0.50, tu));
    if attached < 0.0 {
        from_tu.max(20.0)
    } else {
        from_tu
    }
}

/// Flutter frequency, Hz: speed over tuft length, between 2 and 12 Hz.
pub fn flutter_frequency(speed: f64, len: f64) -> f64 {
    (speed / len.max(1e-12)).clamp(2.0, 12.0)
}

/// Flutter angle (radians) at time `t` for amplitude `amp_deg`, frequency `f` and two phases:
/// the same in the shader.
pub fn flutter_angle(t: f64, amp_deg: f64, f: f64, phase: [f64; 2]) -> f64 {
    let w = std::f64::consts::TAU * f * t;
    amp_deg.to_radians() * (0.6 * (w + phase[0]).sin() + 0.4 * (1.7 * w + phase[1]).sin())
}
