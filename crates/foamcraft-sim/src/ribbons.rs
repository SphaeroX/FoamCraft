// SPDX-License-Identifier: GPL-3.0-or-later
//! Ribbons and tubes along streamlines (Task_35): a frame that is carried along the line without
//! twisting and turned about the tangent by half the vorticity along it (the way a fluid element
//! spins), so a ribbon stays flat in swirl-free flow and screws up in a vortex. Tubes get thinner
//! where the flow is faster; time marks show the speed.

use glam::{DQuat, DVec3};

/// One station of a ribbon or tube.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Frame {
    pub pos: DVec3,
    /// Unit tangent.
    pub t: DVec3,
    /// Unit normal across the line (the ribbon's face normal).
    pub n: DVec3,
    /// Flow time from the start of the line, s.
    pub tau: f64,
    pub speed: f64,
    /// Vorticity along the tangent (s⁻¹): positive is a right-handed spin about the flow.
    pub swirl: f64,
    /// Size of the vorticity (s⁻¹).
    pub vorticity: f64,
}

fn perpendicular(v: DVec3, to: DVec3) -> DVec3 {
    (v - to * v.dot(to)).normalize_or_zero()
}

/// Vorticity (curl of the velocity) at `p` from central differences with step `h`; zero where a
/// neighbour has no flow.
pub fn vorticity(u_at: &dyn Fn(DVec3) -> Option<DVec3>, p: DVec3, h: f64) -> DVec3 {
    let d = |axis: DVec3| -> Option<DVec3> {
        Some((u_at(p + axis * h)? - u_at(p - axis * h)?) / (2.0 * h))
    };
    let (Some(dx), Some(dy), Some(dz)) = (d(DVec3::X), d(DVec3::Y), d(DVec3::Z)) else {
        return DVec3::ZERO;
    };
    DVec3::new(dy.z - dz.y, dz.x - dx.z, dx.y - dy.x)
}

/// The frames along a line (`points` with flow times `taus`, speeds and vorticity per point).
pub fn frames(points: &[DVec3], taus: &[f64], speeds: &[f64], omega: &[DVec3]) -> Vec<Frame> {
    let n = points.len();
    if n < 2 {
        return Vec::new();
    }
    let tangent = |i: usize| {
        let (a, b) = (i.saturating_sub(1), (i + 1).min(n - 1));
        (points[b] - points[a]).normalize_or(DVec3::X)
    };
    let mut out = Vec::with_capacity(n);
    let t0 = tangent(0);
    let mut normal = perpendicular(DVec3::Z, t0);
    if normal == DVec3::ZERO || t0.z.abs() > 0.99 {
        normal = perpendicular(DVec3::X, t0);
    }
    for i in 0..n {
        let t = tangent(i);
        // Carry the normal along without twisting.
        let carried = perpendicular(normal, t);
        normal = if carried == DVec3::ZERO {
            perpendicular(DVec3::X, t)
        } else {
            carried
        };
        let swirl = omega[i].dot(t);
        if i > 0 {
            // Half the vorticity along the line turns the fluid element.
            let dphi = 0.5 * swirl * (taus[i] - taus[i - 1]);
            normal = (DQuat::from_axis_angle(t, dphi) * normal).normalize();
            normal = perpendicular(normal, t);
        }
        out.push(Frame {
            pos: points[i],
            t,
            n: normal,
            tau: taus[i],
            speed: speeds[i],
            swirl,
            vorticity: omega[i].length(),
        });
    }
    out
}

/// Ribbon of width `w`: two points per frame at `pos ± w/2 · (T × N)`, normal `N`.
pub fn ribbon(frames: &[Frame], w: f64) -> Vec<[DVec3; 2]> {
    frames
        .iter()
        .map(|f| {
            let side = f.t.cross(f.n) * (0.5 * w);
            [f.pos - side, f.pos + side]
        })
        .collect()
}

/// Tube radius: a tube that always carries the same flow gets thinner where it is faster
/// (`r0 · sqrt(U / max(|u|, 0.2 U))`, between 0.5 and 2 times `r0`).
pub fn tube_radius(speed: f64, u_ref: f64, r0: f64) -> f64 {
    let r = r0 * (u_ref / speed.max(0.2 * u_ref).max(1e-30)).sqrt();
    r.clamp(0.5 * r0, 2.0 * r0)
}

/// A ring of `k` points per frame around the line, with outward normals.
pub fn tube(frames: &[Frame], u_ref: f64, r0: f64, k: usize) -> Vec<Vec<(DVec3, DVec3)>> {
    frames
        .iter()
        .map(|f| {
            let r = tube_radius(f.speed, u_ref, r0);
            let b = f.t.cross(f.n);
            (0..k)
                .map(|j| {
                    let a = std::f64::consts::TAU * j as f64 / k as f64;
                    let dir = f.n * a.cos() + b * a.sin();
                    (f.pos + dir * r, dir)
                })
                .collect()
        })
        .collect()
}

/// Arc lengths of the time marks every `dtau` of flow time (linear between frames).
pub fn marks(frames: &[Frame], dtau: f64) -> Vec<f64> {
    let mut out = Vec::new();
    if frames.len() < 2 || dtau <= 0.0 {
        return out;
    }
    let mut s = 0.0;
    let mut next = frames[0].tau + dtau;
    for w in frames.windows(2) {
        let ds = (w[1].pos - w[0].pos).length();
        while next <= w[1].tau && w[1].tau > w[0].tau {
            let f = (next - w[0].tau) / (w[1].tau - w[0].tau);
            out.push(s + f * ds);
            next += dtau;
        }
        s += ds;
    }
    out
}
