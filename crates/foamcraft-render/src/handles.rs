// SPDX-License-Identifier: GPL-3.0-or-later
//! Handles for moving and turning things in the 3D view: the drag maths (pure functions on
//! camera rays) and the meshes of arrows and rings. Reused by later tasks (wind direction,
//! slices, flow sources).

use glam::Vec3;

use crate::scene::{Mesh, Vertex};

/// Below this |sin| between ray and axis the axis points at the camera: keep the last value.
const PARALLEL_LIMIT: f32 = 0.02;
/// Below this |cos| between ray and ring axis the ring is seen edge-on: keep the last value.
const EDGE_ON_LIMIT: f32 = 0.05;

/// Angle in radians wrapped to -π..π.
pub fn wrap_pi(a: f32) -> f32 {
    use std::f32::consts::{PI, TAU};
    (a + PI).rem_euclid(TAU) - PI
}

/// `value` rounded to a multiple of `step` (unchanged for a non-positive step).
pub fn snap(value: f64, step: f64) -> f64 {
    if step <= 0.0 {
        return value;
    }
    let n = (value / step).round();
    // Rounded to twelve decimals so 0.1 stays 0.1 and not 0.10000000000000002.
    ((n * step) * 1e12).round() / 1e12
}

/// Round move step for a model of this size (metres): 1, 2 or 5 times a power of ten, close to
/// a twentieth of the size.
pub fn move_step(size: f64) -> f64 {
    if size <= 0.0 || !size.is_finite() {
        return 0.001;
    }
    let raw = size / 20.0;
    let power = 10f64.powf(raw.log10().floor());
    let m = raw / power;
    let k = if m < 1.5 {
        1.0
    } else if m < 3.5 {
        2.0
    } else if m < 7.5 {
        5.0
    } else {
        10.0
    };
    let step = k * power;
    // Clean decimal value (e.g. 0.05, not 0.05000000000000001).
    let digits = (-step.log10().floor()).max(0.0) as i32 + 1;
    let scale = 10f64.powi(digits);
    (step * scale).round() / scale
}

/// Parameter t of the point on the line `origin + t·axis` closest to the ray, or `None` when
/// the ray runs (almost) along the axis.
pub fn axis_param(origin: Vec3, axis: Vec3, ray: (Vec3, Vec3)) -> Option<f32> {
    let (r, d) = ray;
    let a = axis.normalize_or_zero();
    let d = d.normalize_or_zero();
    let w = r - origin;
    let ad = a.dot(d);
    let denom = 1.0 - ad * ad;
    if denom < PARALLEL_LIMIT * PARALLEL_LIMIT {
        return None;
    }
    Some((a.dot(w) - d.dot(w) * ad) / denom)
}

/// Angle in radians of the point where the ray meets the plane through `pivot` normal to
/// `axis`, measured in a fixed basis of that plane; `None` when the plane is seen edge-on.
pub fn ring_angle(pivot: Vec3, axis: Vec3, ray: (Vec3, Vec3)) -> Option<f32> {
    let (r, d) = ray;
    let n = axis.normalize_or_zero();
    let d = d.normalize_or_zero();
    let dn = d.dot(n);
    if dn.abs() < EDGE_ON_LIMIT {
        return None;
    }
    let s = (pivot - r).dot(n) / dn;
    let hit = r + d * s - pivot;
    let (u, v) = plane_basis(n);
    let (x, y) = (hit.dot(u), hit.dot(v));
    if x * x + y * y < 1e-12 {
        return None;
    }
    Some(y.atan2(x))
}

/// Two unit vectors spanning the plane normal to `n`, with u × v = n.
pub fn plane_basis(n: Vec3) -> (Vec3, Vec3) {
    let helper = if n.x.abs() < 0.9 { Vec3::X } else { Vec3::Y };
    let u = (helper - n * helper.dot(n)).normalize();
    (u, n.cross(u))
}

/// A drag along an axis: distance moved since the start, in the units of the world.
#[derive(Debug, Clone, Copy)]
pub struct AxisDrag {
    origin: Vec3,
    axis: Vec3,
    start: Option<f32>,
    last: f32,
}

impl AxisDrag {
    pub fn new(origin: Vec3, axis: Vec3, ray: (Vec3, Vec3)) -> Self {
        Self {
            origin,
            axis: axis.normalize_or_zero(),
            start: axis_param(origin, axis, ray),
            last: 0.0,
        }
    }

    /// Distance along the axis since the start; keeps the last value while the axis points at
    /// the camera.
    pub fn update(&mut self, ray: (Vec3, Vec3)) -> f32 {
        if let Some(t) = axis_param(self.origin, self.axis, ray) {
            match self.start {
                Some(s) => self.last = t - s,
                // Started while looking along the axis: count from here.
                None => self.start = Some(t - self.last),
            }
        }
        self.last
    }
}

/// A drag around a ring: angle turned since the start in radians, counting past ±180°.
#[derive(Debug, Clone, Copy)]
pub struct RingDrag {
    pivot: Vec3,
    axis: Vec3,
    previous: Option<f32>,
    total: f32,
}

impl RingDrag {
    pub fn new(pivot: Vec3, axis: Vec3, ray: (Vec3, Vec3)) -> Self {
        Self {
            pivot,
            axis: axis.normalize_or_zero(),
            previous: ring_angle(pivot, axis, ray),
            total: 0.0,
        }
    }

    /// Angle turned since the start; keeps the last value while the ring is seen edge-on.
    pub fn update(&mut self, ray: (Vec3, Vec3)) -> f32 {
        if let Some(a) = ring_angle(self.pivot, self.axis, ray) {
            if let Some(p) = self.previous {
                self.total += wrap_pi(a - p);
            }
            self.previous = Some(a);
        }
        self.total
    }
}

/// Keys of the handles, in the order of [`handle_meshes`]: pick id `first_pick + i` is
/// `HANDLE_KEYS[i]`.
pub const HANDLE_KEYS: [&str; 6] = [
    "handle:move-x",
    "handle:move-y",
    "handle:move-z",
    "handle:rotate-x",
    "handle:rotate-y",
    "handle:rotate-z",
];

/// Colours of the X, Y and Z handles (same hues as the world axes, a little brighter).
const HANDLE_COLORS: [[f32; 3]; 3] = [[0.95, 0.30, 0.26], [0.35, 0.82, 0.38], [0.32, 0.55, 1.0]];

/// Arrows (length 0.9 × `size`) and rings (radius 0.7 × `size`) at `pivot`, one mesh per handle
/// with pick id `first_pick + i` for `HANDLE_KEYS[i]`. `size` is the radius of the model's
/// bounding sphere, so the handles scale with the model and not with the zoom.
pub fn handle_meshes(pivot: Vec3, size: f32, first_pick: u32) -> Vec<Mesh> {
    let axes = [Vec3::X, Vec3::Y, Vec3::Z];
    let mut out = Vec::with_capacity(6);
    for (i, axis) in axes.iter().enumerate() {
        out.push(arrow(
            pivot,
            *axis,
            0.9 * size,
            1.0,
            first_pick + i as u32,
            HANDLE_COLORS[i],
        ));
    }
    for (i, axis) in axes.iter().enumerate() {
        let c = HANDLE_COLORS[i].map(|v| (v * 0.85 + 0.1).min(1.0));
        out.push(ring(pivot, *axis, 0.7 * size, first_pick + 3 + i as u32, c));
    }
    out
}

fn push_vertex(v: &mut Vec<Vertex>, p: Vec3, n: Vec3, pick: u32) -> u32 {
    v.push(Vertex {
        position: p.into(),
        normal: n.normalize_or_zero().into(),
        pick,
    });
    (v.len() - 1) as u32
}

fn solid(vertices: Vec<Vertex>, indices: Vec<u32>, color: [f32; 3]) -> Mesh {
    Mesh {
        vertices,
        indices,
        color,
        lines: false,
        opacity: 1.0,
        vertex_colors: None,
    }
}

/// Shaft and cone from `pivot` along `axis`, `length` long; `thickness` scales the radii.
fn arrow(pivot: Vec3, axis: Vec3, length: f32, thickness: f32, pick: u32, color: [f32; 3]) -> Mesh {
    const SEG: usize = 20;
    let (u, w) = plane_basis(axis);
    let shaft_r = length * 0.018 * thickness;
    let head_r = length * 0.06 * thickness;
    let head_len = length * 0.2;
    let shaft_end = length - head_len;
    let mut v = Vec::new();
    let mut idx = Vec::new();
    let around = |k: usize| {
        let a = k as f32 / SEG as f32 * std::f32::consts::TAU;
        u * a.cos() + w * a.sin()
    };
    // Shaft side: rings at the start and at the head.
    for k in 0..SEG {
        let (d0, d1) = (around(k), around(k + 1));
        let a = push_vertex(&mut v, pivot + d0 * shaft_r, d0, pick);
        let b = push_vertex(&mut v, pivot + d1 * shaft_r, d1, pick);
        let c = push_vertex(&mut v, pivot + axis * shaft_end + d1 * shaft_r, d1, pick);
        let d = push_vertex(&mut v, pivot + axis * shaft_end + d0 * shaft_r, d0, pick);
        idx.extend_from_slice(&[a, b, c, a, c, d]);
    }
    // Cone: base disc and slanted side up to the tip.
    let base = pivot + axis * shaft_end;
    let tip = pivot + axis * length;
    for k in 0..SEG {
        let (d0, d1) = (around(k), around(k + 1));
        let n0 = d0 * head_len + axis * head_r;
        let n1 = d1 * head_len + axis * head_r;
        let a = push_vertex(&mut v, base + d0 * head_r, n0, pick);
        let b = push_vertex(&mut v, base + d1 * head_r, n1, pick);
        let t = push_vertex(&mut v, tip, (n0 + n1) / 2.0, pick);
        idx.extend_from_slice(&[a, b, t]);
        let c = push_vertex(&mut v, base, -axis, pick);
        let e = push_vertex(&mut v, base + d1 * head_r, -axis, pick);
        let f = push_vertex(&mut v, base + d0 * head_r, -axis, pick);
        idx.extend_from_slice(&[c, e, f]);
    }
    solid(v, idx, color)
}

/// Torus of `radius` around `axis` through `pivot`.
fn ring(pivot: Vec3, axis: Vec3, radius: f32, pick: u32, color: [f32; 3]) -> Mesh {
    const MAJOR: usize = 64;
    const MINOR: usize = 10;
    let tube = radius * 0.025;
    let (u, w) = plane_basis(axis);
    let mut v = Vec::with_capacity((MAJOR + 1) * (MINOR + 1));
    for i in 0..=MAJOR {
        let a = i as f32 / MAJOR as f32 * std::f32::consts::TAU;
        let radial = u * a.cos() + w * a.sin();
        let centre = pivot + radial * radius;
        for j in 0..=MINOR {
            let b = j as f32 / MINOR as f32 * std::f32::consts::TAU;
            let n = radial * b.cos() + axis * b.sin();
            push_vertex(&mut v, centre + n * tube, n, pick);
        }
    }
    let mut idx = Vec::with_capacity(MAJOR * MINOR * 6);
    let row = (MINOR + 1) as u32;
    for i in 0..MAJOR as u32 {
        for j in 0..MINOR as u32 {
            let a = i * row + j;
            let b = (i + 1) * row + j;
            idx.extend_from_slice(&[a, b, b + 1, a, b + 1, a + 1]);
        }
    }
    solid(v, idx, color)
}

/// Key of the wind arrow handle.
pub const WIND_KEY: &str = "handle:wind";
/// Orange of the wind arrow (as the arrow at the tunnel inlet).
const WIND_COLOR: [f32; 3] = [0.95, 0.55, 0.15];

/// Unit vector of the flow for a wind direction in degrees (0 = towards +X, 90 = towards +Y).
pub fn flow_vector(direction_deg: f32) -> Vec3 {
    let a = direction_deg.to_radians();
    Vec3::new(a.cos(), a.sin(), 0.0)
}

/// Large wind arrow on the upstream side of a model with bounding sphere (`centre`, `radius`):
/// it ends 1.2 radii before the centre, points downstream and is 1.2 radii long.
pub fn wind_arrow_mesh(centre: Vec3, radius: f32, direction_deg: f32, pick: u32) -> Mesh {
    let flow = flow_vector(direction_deg);
    let length = 1.2 * radius;
    let tail = centre - flow * (1.2 * radius + length);
    arrow(tail, flow, length, 3.0, pick, WIND_COLOR)
}

/// Wind direction in degrees (-180..180) set by dragging the wind arrow: the mouse ray meets the
/// horizontal plane through `centre`, and the wind blows from that point towards the centre.
/// `None` when the plane is seen edge-on or the point is the centre.
pub fn wind_direction_deg(centre: Vec3, ray: (Vec3, Vec3)) -> Option<f32> {
    let (r, d) = ray;
    let d = d.normalize_or_zero();
    if d.z.abs() < EDGE_ON_LIMIT {
        return None;
    }
    let s = (centre.z - r.z) / d.z;
    let hit = r + d * s;
    let (dx, dy) = (centre.x - hit.x, centre.y - hit.y);
    if dx * dx + dy * dy < 1e-12 {
        return None;
    }
    Some(dy.atan2(dx).to_degrees())
}

/// How far a plane through `origin` with `normal` can move along the normal and still cut the
/// box `min`..`max`: [lowest, highest] offset (the corners' extreme projections).
pub fn slice_range(origin: [f32; 3], normal: [f32; 3], min: [f32; 3], max: [f32; 3]) -> [f32; 2] {
    let n = Vec3::from(normal).normalize_or_zero();
    let base = n.dot(Vec3::from(origin));
    let mut lo = f32::MAX;
    let mut hi = f32::MIN;
    for c in 0..8 {
        let p = Vec3::new(
            if c & 1 == 0 { min[0] } else { max[0] },
            if c & 2 == 0 { min[1] } else { max[1] },
            if c & 4 == 0 { min[2] } else { max[2] },
        );
        let d = n.dot(p) - base;
        lo = lo.min(d);
        hi = hi.max(d);
    }
    [lo, hi]
}

/// Colour of the sources of air.
const JET_COLOR: [f32; 3] = [0.30, 0.78, 0.84];

/// Marker of a source of air: a thin disc of the source diameter and an arrow three diameters
/// long in the blowing direction, one mesh with one pick id.
pub fn jet_marker(center: Vec3, direction: Vec3, diameter: f32, pick: u32) -> Mesh {
    let d = direction.normalize_or_zero();
    // A flat cylinder of the source radius.
    let mut disc = solid(Vec::new(), Vec::new(), JET_COLOR);
    const SEG: usize = 32;
    let (u, w) = plane_basis(d);
    let r = diameter / 2.0;
    let t = diameter * 0.08;
    for k in 0..SEG {
        let a0 = k as f32 / SEG as f32 * std::f32::consts::TAU;
        let a1 = (k + 1) as f32 / SEG as f32 * std::f32::consts::TAU;
        let (p0, p1) = (u * a0.cos() + w * a0.sin(), u * a1.cos() + w * a1.sin());
        let back = center - d * t;
        let i0 = push_vertex(&mut disc.vertices, back + p0 * r, p0, pick);
        let i1 = push_vertex(&mut disc.vertices, back + p1 * r, p1, pick);
        let i2 = push_vertex(&mut disc.vertices, center + p1 * r, p1, pick);
        let i3 = push_vertex(&mut disc.vertices, center + p0 * r, p0, pick);
        disc.indices.extend_from_slice(&[i0, i1, i2, i0, i2, i3]);
        for (face, n) in [(center, d), (back, -d)] {
            let c = push_vertex(&mut disc.vertices, face, n, pick);
            let a = push_vertex(&mut disc.vertices, face + p0 * r, n, pick);
            let b = push_vertex(&mut disc.vertices, face + p1 * r, n, pick);
            if n == d {
                disc.indices.extend_from_slice(&[c, a, b]);
            } else {
                disc.indices.extend_from_slice(&[c, b, a]);
            }
        }
    }
    let shaft = arrow(center, d, diameter * 3.0, 2.0, pick, JET_COLOR);
    let base = disc.vertices.len() as u32;
    disc.vertices.extend(shaft.vertices);
    disc.indices.extend(shaft.indices.iter().map(|i| i + base));
    disc
}
