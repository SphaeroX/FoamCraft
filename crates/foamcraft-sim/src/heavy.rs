// SPDX-License-Identifier: GPL-3.0-or-later
//! Heavy particles (Task_34): spheres with mass carried by the flow through drag (Schiller and
//! Naumann), with gravity and buoyancy, one-way coupled. The integrator is the exact solution for
//! a locally constant flow, stable for any step and the massless motion in the limit. Impacts are
//! found exactly on the triangles of the result surface; particles stick or bounce.
//!
//! Left out (fine while the particle is much denser than the medium): lift, added mass, history
//! force, deformation of large drops.

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use glam::DVec3;

use crate::distance::{WallDistance, closest_on_triangle};

pub const G: f64 = 9.81;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Medium {
    /// kg/m³
    pub density: f64,
    /// m²/s
    pub kinematic_viscosity: f64,
}

/// What happens at the wall.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Impact {
    Stick,
    /// The normal speed is reversed and scaled by `restitution`, the tangential one by
    /// `1 - friction`.
    Bounce {
        restitution: f64,
        friction: f64,
    },
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Particle {
    /// m
    pub diameter: f64,
    /// kg/m³
    pub density: f64,
    pub gravity: bool,
    pub impact: Impact,
}

/// Most bounces before a particle sticks.
pub const MAX_BOUNCES: usize = 5;

/// Response time in creeping flow, s.
pub fn tau_p(p: &Particle, m: &Medium) -> f64 {
    p.density * p.diameter * p.diameter / (18.0 * m.density * m.kinematic_viscosity).max(1e-300)
}

/// Drag correction over Stokes drag at a particle Reynolds number (Schiller and Naumann; Newton
/// regime above 1000).
pub fn drag_factor(re: f64) -> f64 {
    if re <= 1000.0 {
        1.0 + 0.15 * re.max(0.0).powf(0.687)
    } else {
        0.44 * re / 24.0
    }
}

/// Gravity minus buoyancy (along -Z).
pub fn gravity_accel(p: &Particle, m: &Medium) -> DVec3 {
    if p.gravity {
        DVec3::new(0.0, 0.0, -G * (1.0 - m.density / p.density))
    } else {
        DVec3::ZERO
    }
}

/// Effective response time at a relative speed.
pub fn tau_e(p: &Particle, m: &Medium, relative: f64) -> f64 {
    let re = relative * p.diameter / m.kinematic_viscosity.max(1e-300);
    tau_p(p, m) / drag_factor(re)
}

/// Velocity of steady settling in still medium (gravity minus buoyancy against the corrected
/// drag): solved by damped fixed-point iteration of `v = tau_e(|v|) g'`.
pub fn terminal_velocity(p: &Particle, m: &Medium) -> DVec3 {
    let g = gravity_accel(p, m);
    let gl = g.length();
    if gl == 0.0 {
        return DVec3::ZERO;
    }
    let mut v = tau_p(p, m) * gl;
    for _ in 0..200 {
        let next = tau_e(p, m, v) * gl;
        if (next - v).abs() <= 1e-12 * v.max(1e-30) {
            break;
        }
        v = 0.5 * (v + next);
    }
    g / gl * v
}

/// Stokes number `tau_p U / L`.
pub fn stokes(p: &Particle, m: &Medium, speed: f64, length: f64) -> f64 {
    tau_p(p, m) * speed / length.max(1e-300)
}

/// The exact step for a constant flow `u` and response time `tau`.
fn exact(x: DVec3, v: DVec3, u: DVec3, g: DVec3, tau: f64, dt: f64) -> (DVec3, DVec3) {
    let v_inf = u + g * tau;
    if tau <= 1e-300 {
        return (x + v_inf * dt, v_inf);
    }
    let e = (-dt / tau).exp();
    let x1 = x + v_inf * dt + (v - v_inf) * tau * (1.0 - e);
    let v1 = v_inf + (v - v_inf) * e;
    (x1, v1)
}

/// One step: a provisional step with the flow at the start, then the step with the flow in the
/// middle between the start and the provisional point. A point without flow (`None`) uses the
/// flow at the start.
pub fn step(
    x: DVec3,
    v: DVec3,
    u_at: &dyn Fn(DVec3) -> Option<DVec3>,
    dt: f64,
    p: &Particle,
    m: &Medium,
) -> (DVec3, DVec3) {
    let g = gravity_accel(p, m);
    let u0 = u_at(x).unwrap_or(DVec3::ZERO);
    let (x1, _) = exact(x, v, u0, g, tau_e(p, m, (u0 - v).length()), dt);
    let um = u_at((x + x1) * 0.5).unwrap_or(u0);
    exact(x, v, um, g, tau_e(p, m, (um - v).length()), dt)
}

/// Where a segment meets the surface.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Hit {
    pub tri: usize,
    pub point: DVec3,
    /// Unit normal facing the side the segment comes from.
    pub normal: DVec3,
    /// Position along the segment, 0..1.
    pub t: f64,
    /// Speed into the wall at the impact, m/s (filled in by `track`).
    pub normal_speed: f64,
    /// Angle between the path and the surface, degrees (90 = head on).
    pub angle_deg: f64,
}

/// Möller–Trumbore: parameter along `a`→`b` and the hit point, if the segment crosses the
/// triangle (edges included).
fn intersect(a: DVec3, b: DVec3, [p, q, r]: [DVec3; 3]) -> Option<f64> {
    let d = b - a;
    let (e1, e2) = (q - p, r - p);
    let h = d.cross(e2);
    let det = e1.dot(h);
    if det.abs() < 1e-300 {
        return None;
    }
    let f = 1.0 / det;
    let s = a - p;
    let u = f * s.dot(h);
    let eps = 1e-12;
    if !(-eps..=1.0 + eps).contains(&u) {
        return None;
    }
    let qv = s.cross(e1);
    let v = f * d.dot(qv);
    if v < -eps || u + v > 1.0 + eps {
        return None;
    }
    let t = f * e2.dot(qv);
    (-eps..=1.0 + eps).contains(&t).then_some(t.clamp(0.0, 1.0))
}

/// The first triangle the segment `a`→`b` meets (cells of the triangle grid along the segment).
pub fn segment_hit(wall: &WallDistance, a: DVec3, b: DVec3) -> Option<Hit> {
    let mut best: Option<Hit> = None;
    for n in wall.candidates(a, b) {
        let tri = wall.triangles()[n as usize];
        if let Some(t) = intersect(a, b, tri)
            && best.is_none_or(|h| t < h.t)
        {
            let [p, q, r] = tri;
            let mut normal = (q - p).cross(r - p).normalize_or_zero();
            if normal.dot(b - a) > 0.0 {
                normal = -normal;
            }
            best = Some(Hit {
                tri: n as usize,
                point: a + (b - a) * t,
                normal,
                t,
                normal_speed: 0.0,
                angle_deg: 0.0,
            });
        }
    }
    best
}

/// A bounce: velocity before and after.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Bounce {
    pub point: DVec3,
    pub tri: usize,
    pub before: DVec3,
    pub after: DVec3,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Fate {
    Stuck(Hit),
    Left,
    Timeout,
}

/// The path of one heavy particle.
#[derive(Debug, Clone, PartialEq)]
pub struct Track {
    pub points: Vec<DVec3>,
    pub times: Vec<f64>,
    pub velocities: Vec<DVec3>,
    /// Flow velocity at each point (zero where there is none).
    pub flow: Vec<DVec3>,
    pub bounces: Vec<Bounce>,
    pub fate: Fate,
}

fn impact_numbers(mut hit: Hit, v: DVec3) -> Hit {
    let speed = v.length();
    let vn = -v.dot(hit.normal);
    hit.normal_speed = vn.max(0.0);
    hit.angle_deg = if speed > 0.0 {
        (vn / speed).clamp(-1.0, 1.0).asin().to_degrees()
    } else {
        0.0
    };
    hit
}

/// Follows a particle from `x0` with velocity `v0` until it sticks, leaves the flow (`u_at`
/// answers `None`) or `max_time` passes. Steps move at most half of `h`.
#[allow(clippy::too_many_arguments)]
pub fn track(
    x0: DVec3,
    v0: DVec3,
    u_at: &dyn Fn(DVec3) -> Option<DVec3>,
    wall: &WallDistance,
    p: &Particle,
    m: &Medium,
    h: f64,
    max_time: f64,
) -> Track {
    let mut out = Track {
        points: vec![x0],
        times: vec![0.0],
        velocities: vec![v0],
        flow: vec![u_at(x0).unwrap_or(DVec3::ZERO)],
        bounces: Vec::new(),
        fate: Fate::Timeout,
    };
    let (mut x, mut v, mut t) = (x0, v0, 0.0);
    let mut steps = 0usize;
    while t < max_time && steps < 200_000 {
        steps += 1;
        let Some(u) = u_at(x) else {
            out.fate = Fate::Left;
            return out;
        };
        let speed = v.length().max(u.length()).max(1e-12);
        let dt = (0.5 * h / speed).min(max_time - t).max(1e-15);
        let (x1, v1) = step(x, v, u_at, dt, p, m);
        if let Some(hit) = segment_hit(wall, x, x1) {
            let vi = v.lerp(v1, hit.t);
            let ti = t + dt * hit.t;
            let hit = impact_numbers(hit, vi);
            out.points.push(hit.point);
            out.times.push(ti);
            out.velocities.push(vi);
            out.flow.push(u_at(hit.point).unwrap_or(DVec3::ZERO));
            match p.impact {
                Impact::Bounce {
                    restitution,
                    friction,
                } if out.bounces.len() < MAX_BOUNCES => {
                    let vn = hit.normal * vi.dot(hit.normal);
                    let after = -vn * restitution + (vi - vn) * (1.0 - friction);
                    out.bounces.push(Bounce {
                        point: hit.point,
                        tri: hit.tri,
                        before: vi,
                        after,
                    });
                    // Just off the wall so the next step starts on the fluid side.
                    x = hit.point + hit.normal * (1e-6 * h);
                    v = after;
                    t = ti;
                    continue;
                }
                _ => {
                    out.fate = Fate::Stuck(hit);
                    return out;
                }
            }
        }
        x = x1;
        v = v1;
        t += dt;
        out.points.push(x);
        out.times.push(t);
        out.velocities.push(v);
        out.flow.push(u_at(x).unwrap_or(DVec3::ZERO));
    }
    out
}

/// Hits of `n` particles on a cylinder (or sphere) of radius 1 in potential flow (U = 1) at the
/// inertia parameter `k = tau_p U / R`, with Stokes drag only and no gravity. Particles start ten
/// radii upstream with the flow speed, spread evenly over the frontal area.
pub fn critical_hits(sphere: bool, k: f64, n: u32) -> u32 {
    let field = |x: DVec3| -> Option<DVec3> {
        if sphere {
            let r = x.length();
            let r3 = r * r * r;
            Some(DVec3::X * (1.0 + 0.5 / r3) - x * (1.5 * x.x / (r3 * r * r)))
        } else {
            let r2 = x.x * x.x + x.y * x.y;
            let r4 = r2 * r2;
            Some(DVec3::new(
                1.0 - (x.x * x.x - x.y * x.y) / r4,
                -2.0 * x.x * x.y / r4,
                0.0,
            ))
        }
    };
    let mut hits = 0;
    for i in 0..n {
        // Evenly over the frontal area: a line for the cylinder, rings of equal area for the
        // sphere.
        let s = (f64::from(i) + 0.5) / f64::from(n);
        let y = if sphere { s.sqrt() } else { 2.0 * s - 1.0 };
        let mut x = DVec3::new(-10.0, y * 0.999, 0.0);
        let mut v = field(x).unwrap_or(DVec3::X);
        let mut t = 0.0;
        while t < 40.0 && x.x < 2.0 {
            let dt = 2e-3;
            let u0 = field(x).unwrap_or(DVec3::X);
            let (x1, _) = exact(x, v, u0, DVec3::ZERO, k, dt);
            let um = field((x + x1) * 0.5).unwrap_or(u0);
            let (x2, v2) = exact(x, v, um, DVec3::ZERO, k, dt);
            let r = if sphere {
                x2.length()
            } else {
                (x2.x * x2.x + x2.y * x2.y).sqrt()
            };
            if r <= 1.0 {
                hits += 1;
                break;
            }
            x = x2;
            v = v2;
            t += dt;
        }
    }
    hits
}

/// Counts of an impact map.
#[derive(Debug, Clone, PartialEq)]
pub struct ImpactCounts {
    /// Hits per triangle.
    pub hits: Vec<u32>,
    /// Sum of the speeds into the wall per triangle, m/s.
    pub speed_sum: Vec<f64>,
    pub released: u64,
    /// Particles that hit.
    pub total: u64,
}

/// Hits, speed sums and number of hits of one block of particles.
type BlockCounts = (Vec<u32>, Vec<f64>, u64);

/// Follows all particles from `starts` (velocity from `v0`) on `threads` cores, each core a fixed
/// block, so the counts do not depend on the number of cores. `None` when cancelled.
#[allow(clippy::too_many_arguments)]
pub fn impact_counts(
    starts: &[DVec3],
    v0: &(dyn Fn(DVec3) -> DVec3 + Sync),
    u_at: &(dyn Fn(DVec3) -> Option<DVec3> + Sync),
    wall: &WallDistance,
    p: &Particle,
    m: &Medium,
    h: f64,
    max_time: f64,
    threads: usize,
    cancel: &AtomicBool,
    progress: &AtomicU64,
) -> Option<ImpactCounts> {
    let tris = wall.triangles().len();
    let block = 1024usize;
    let blocks: Vec<&[DVec3]> = starts.chunks(block).collect();
    let next = std::sync::atomic::AtomicUsize::new(0);
    let results: Vec<std::sync::Mutex<Option<BlockCounts>>> =
        blocks.iter().map(|_| std::sync::Mutex::new(None)).collect();
    std::thread::scope(|scope| {
        for _ in 0..threads.max(1) {
            scope.spawn(|| {
                loop {
                    let b = next.fetch_add(1, Ordering::SeqCst);
                    if b >= blocks.len() || cancel.load(Ordering::SeqCst) {
                        break;
                    }
                    let mut hits = vec![0u32; tris];
                    let mut speed = vec![0.0; tris];
                    let mut total = 0;
                    for s in blocks[b] {
                        if cancel.load(Ordering::SeqCst) {
                            break;
                        }
                        let t = track(*s, v0(*s), u_at, wall, p, m, h, max_time);
                        if let Fate::Stuck(hit) = t.fate {
                            hits[hit.tri] += 1;
                            speed[hit.tri] += hit.normal_speed;
                            total += 1;
                        }
                        progress.fetch_add(1, Ordering::SeqCst);
                    }
                    if let Ok(mut r) = results[b].lock() {
                        *r = Some((hits, speed, total));
                    }
                }
            });
        }
    });
    if cancel.load(Ordering::SeqCst) {
        return None;
    }
    // Sum the blocks in their order: the same result for any number of cores.
    let mut out = ImpactCounts {
        hits: vec![0; tris],
        speed_sum: vec![0.0; tris],
        released: starts.len() as u64,
        total: 0,
    };
    for r in results {
        let (hits, speed, total) = r.into_inner().ok().flatten()?;
        for i in 0..tris {
            out.hits[i] += hits[i];
            out.speed_sum[i] += speed[i];
        }
        out.total += total;
    }
    Some(out)
}

/// Spreads values per triangle to its corners and back (area weighted): smooth, with the same sum.
pub fn smooth(triangles: &[[u32; 3]], areas: &[f64], points: usize, values: &[f64]) -> Vec<f64> {
    // Each triangle gives a third of its value to each corner.
    let mut at = vec![0.0; points];
    let mut weight = vec![0.0; points];
    for (t, tri) in triangles.iter().enumerate() {
        for &c in tri {
            at[c as usize] += values[t] / 3.0;
            weight[c as usize] += areas[t];
        }
    }
    // Back: each corner splits its share over its triangles by area.
    let mut out = vec![0.0; triangles.len()];
    for (t, tri) in triangles.iter().enumerate() {
        for &c in tri {
            let w = weight[c as usize];
            if w > 0.0 {
                out[t] += at[c as usize] * areas[t] / w;
            }
        }
    }
    out
}

/// The closest point on the surface to `p` (for checks).
pub fn closest_point(wall: &WallDistance, p: DVec3) -> Option<DVec3> {
    wall.triangles()
        .iter()
        .map(|t| closest_on_triangle(p, *t))
        .min_by(|a, b| (*a - p).length().total_cmp(&(*b - p).length()))
}
