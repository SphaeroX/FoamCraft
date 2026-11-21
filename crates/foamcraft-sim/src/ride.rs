// SPDX-License-Identifier: GPL-3.0-or-later
//! The ride (Task_32): the path of one massless particle through a steady result, its readings,
//! the screen timing, the camera track, the events along the way and the suggested paths. Pure
//! logic in the flow frame (flow along +X); the caller turns results into the world.

use foamcraft_core::error::FoamError;
use glam::DVec3;

use crate::distance::WallDistance;
use crate::particles::random;
use crate::results::{Grid, Surface};

/// Below this share of the reference speed the particle counts as standing still.
const STILL: f64 = 0.02;

/// How one direction of the integration ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum End {
    /// Left every grid.
    Outside,
    /// Started or ended in a solid.
    Blocked,
    /// Longer than the length limit.
    Length,
    /// More steps than the step limit.
    Steps,
    /// Below 2 % of the reference speed for 200 steps: a stagnation point.
    Stalled,
}

/// A velocity lookup.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Probe {
    Fluid(DVec3),
    Blocked,
    Outside,
}

/// The flow a ride runs through: the result grids (coarse to fine, flow frame), the walls and the
/// reference values.
pub struct Flow {
    pub grids: Vec<Grid>,
    cols: Vec<Cols>,
    pub wall: Option<WallDistance>,
    /// Reference speed (free stream or mean inlet speed), m/s.
    pub u_ref: f64,
    pub rho: f64,
    /// Reference pressure for `Cp`, Pa.
    pub p_ref: f64,
    /// Smallest step of the finest grid, m.
    pub h: f64,
    /// Largest size of the model, m.
    pub length: f64,
    /// Bounding box of the model (flow frame).
    pub model_min: DVec3,
    pub model_max: DVec3,
    /// Length limit per direction, m (30 L).
    pub max_length: f64,
    /// Step limit per direction.
    pub max_steps: usize,
    /// Main flow direction for "reverse" (wind tunnel: +X); `None` for internal flow.
    pub main: Option<DVec3>,
    /// The Q value above which a vortex counts (90th percentile of the positive values).
    pub q_threshold: Option<f64>,
}

struct Cols {
    u: [usize; 3],
    p: Option<usize>,
    q: Option<usize>,
}

/// Grid cell of a point: base corner and fractions, `None` outside.
fn locate(g: &Grid, p: DVec3) -> Option<([u32; 3], [f64; 3])> {
    let s = g.spec.step();
    let d = g.spec.dims;
    let mut base = [0u32; 3];
    let mut frac = [0.0; 3];
    for a in 0..3 {
        let c = (p[a] - g.spec.min[a]) / s[a] - 1.0;
        if !(0.0..=f64::from(d[a].saturating_sub(1))).contains(&c) || d[a] < 2 {
            return None;
        }
        let b = (c.floor() as u32).min(d[a] - 2);
        base[a] = b;
        frac[a] = c - f64::from(b);
    }
    Some((base, frac))
}

fn corners(g: &Grid, base: [u32; 3], frac: [f64; 3]) -> impl Iterator<Item = (usize, f64)> + '_ {
    (0..8u32).map(move |corner| {
        let o = [corner & 1, (corner >> 1) & 1, (corner >> 2) & 1];
        let id = g.spec.index(base[0] + o[0], base[1] + o[1], base[2] + o[2]);
        let w: f64 = (0..3)
            .map(|a| if o[a] == 1 { frac[a] } else { 1.0 - frac[a] })
            .product();
        (id, w)
    })
}

fn smoothstep(e0: f64, e1: f64, x: f64) -> f64 {
    let t = ((x - e0) / (e1 - e0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

impl Flow {
    /// A flow from result grids (coarse to fine) and the model surface, both in the flow frame.
    /// Without `u_ref` the median speed of the coarsest grid is used.
    pub fn new(
        grids: Vec<Grid>,
        surface: Option<&Surface>,
        u_ref: Option<f64>,
        rho: f64,
    ) -> Result<Self, FoamError> {
        let col = |g: &Grid, n: &str| g.names.iter().position(|x| x == n);
        let mut cols = Vec::new();
        for g in &grids {
            let (Some(x), Some(y), Some(z)) = (col(g, "Ux"), col(g, "Uy"), col(g, "Uz")) else {
                return Err(FoamError::new("results.unknown_field", "U"));
            };
            cols.push(Cols {
                u: [x, y, z],
                p: col(g, "p"),
                q: col(g, "Q"),
            });
        }
        let (Some(fine), Some(coarse)) = (grids.last(), grids.first()) else {
            return Err(FoamError::new("ride.no_path", "no result grids"));
        };
        let h = fine.spec.step().into_iter().fold(f64::MAX, f64::min);
        let u_ref = u_ref.filter(|u| *u > 0.0).unwrap_or_else(|| {
            let c = &cols[0];
            let mut speeds: Vec<f64> = (0..coarse.spec.len())
                .filter(|i| !coarse.mask[*i])
                .map(|i| DVec3::from(c.u.map(|k| f64::from(coarse.values[k][i]))).length())
                .collect();
            speeds.sort_by(f64::total_cmp);
            speeds
                .get(speeds.len() / 2)
                .copied()
                .unwrap_or(1.0)
                .max(1e-6)
        });
        let (mut lo, mut hi) = (DVec3::splat(f64::MAX), DVec3::splat(f64::MIN));
        if let Some(s) = surface {
            for p in &s.points {
                let v = DVec3::from(p.map(f64::from));
                lo = lo.min(v);
                hi = hi.max(v);
            }
        }
        if lo.x > hi.x {
            lo = DVec3::from(fine.spec.min);
            hi = DVec3::from(fine.spec.max);
        }
        let length = (hi - lo).max_element().max(h);
        let wall = surface
            .filter(|s| !s.triangles.is_empty())
            .map(|s| WallDistance::new(&s.points, &s.triangles, 2.0 * h, 8.0 * h));
        let q_threshold = cols.last().and_then(|c| c.q).and_then(|q| {
            let mut pos: Vec<f64> = (0..fine.spec.len())
                .filter(|i| !fine.mask[*i] && fine.values[q][*i] > 0.0)
                .map(|i| f64::from(fine.values[q][i]))
                .collect();
            pos.sort_by(f64::total_cmp);
            pos.get(pos.len() * 9 / 10).copied()
        });
        Ok(Self {
            grids,
            cols,
            wall,
            u_ref,
            rho,
            p_ref: 0.0,
            h,
            length,
            model_min: lo,
            model_max: hi,
            max_length: 30.0 * length,
            max_steps: 20_000,
            main: Some(DVec3::X),
            q_threshold,
        })
    }

    /// The finest grid holding `p`.
    fn cell(&self, p: DVec3) -> Option<(usize, [u32; 3], [f64; 3])> {
        self.grids
            .iter()
            .enumerate()
            .rev()
            .find_map(|(n, g)| locate(g, p).map(|(b, f)| (n, b, f)))
    }

    /// Velocity at `p`. Masked grid points lie in a solid and count as resting wall (velocity
    /// zero); a point closer to the solid than to the fluid (masked weight above one half) is
    /// blocked. So the speed falls to zero towards a wall and a particle can stall in front of it.
    pub fn velocity(&self, p: DVec3) -> Probe {
        let Some((n, base, frac)) = self.cell(p) else {
            return Probe::Outside;
        };
        let g = &self.grids[n];
        let c = &self.cols[n];
        let mut u = DVec3::ZERO;
        let mut masked = 0.0;
        for (id, w) in corners(g, base, frac) {
            if g.mask[id] {
                masked += w;
            } else {
                u += w * DVec3::from(c.u.map(|k| f64::from(g.values[k][id])));
            }
        }
        if masked > 0.5 {
            Probe::Blocked
        } else {
            Probe::Fluid(u)
        }
    }

    /// A scalar field at `p` from the fluid corners only (weights renormalised).
    fn scalar(&self, p: DVec3, pick: impl Fn(&Cols) -> Option<usize>) -> Option<f64> {
        let (n, base, frac) = self.cell(p)?;
        let g = &self.grids[n];
        let f = pick(&self.cols[n])?;
        let (mut sum, mut wsum) = (0.0, 0.0);
        for (id, w) in corners(g, base, frac) {
            if !g.mask[id] {
                sum += w * f64::from(g.values[f][id]);
                wsum += w;
            }
        }
        (wsum > 1e-12).then(|| sum / wsum)
    }

    pub fn pressure(&self, p: DVec3) -> Option<f64> {
        self.scalar(p, |c| c.p)
    }

    pub fn q(&self, p: DVec3) -> Option<f64> {
        self.scalar(p, |c| c.q)
    }

    /// Distance to the nearest wall, at most 8 h.
    pub fn wall_distance(&self, p: DVec3) -> f64 {
        self.wall
            .as_ref()
            .map_or(8.0 * self.h, |w| w.distance(p.into()))
    }

    /// Pressure coefficient of a pressure.
    pub fn cp(&self, p: f64) -> f64 {
        (p - self.p_ref) / (0.5 * self.rho * self.u_ref * self.u_ref).max(1e-12)
    }

    /// One direction of the path from `start` (`dir` = 1 forward, -1 backward): points after the
    /// start with flow time and velocity.
    fn integrate(&self, start: DVec3, dir: f64) -> (Vec<(DVec3, f64, DVec3)>, End) {
        let mut out = Vec::new();
        let mut p = start;
        let mut tau = 0.0;
        let mut length = 0.0;
        let mut still = 0;
        let floor = STILL * self.u_ref;
        let max_move = 0.25 * self.h;
        let mut u1 = match self.velocity(p) {
            Probe::Fluid(u) => u,
            Probe::Blocked => return (out, End::Blocked),
            Probe::Outside => return (out, End::Outside),
        };
        loop {
            if out.len() >= self.max_steps {
                return (out, End::Steps);
            }
            let full = dir * max_move / u1.length().max(floor);
            // Take the largest of dt, dt/2, dt/4 ... that stays in the fluid: a path that runs into
            // a wall creeps up to it instead of ending, and slides along it where the flow does.
            let mut dt = full;
            let mut step = None;
            let mut outside = false;
            for _ in 0..10 {
                let mid = p + 0.5 * dt * u1;
                let u2 = match self.velocity(mid) {
                    Probe::Fluid(u) => u,
                    Probe::Outside => {
                        outside = true;
                        break;
                    }
                    Probe::Blocked => {
                        dt *= 0.5;
                        continue;
                    }
                };
                let q = p + dt * u2;
                match self.velocity(q) {
                    Probe::Fluid(u3) => {
                        step = Some((q, u3));
                        break;
                    }
                    Probe::Outside => {
                        outside = true;
                        break;
                    }
                    Probe::Blocked => dt *= 0.5,
                }
            }
            if outside {
                return (out, End::Outside);
            }
            let moved = if let Some((q, u3)) = step {
                let m = (q - p).length();
                p = q;
                u1 = u3;
                m
            } else {
                0.0
            };
            tau += dt;
            length += moved;
            if step.is_some() {
                out.push((p, tau, u1));
            }
            if u1.length() < floor || moved < 0.01 * max_move {
                still += 1;
                if still >= 200 {
                    return (out, End::Stalled);
                }
            } else {
                still = 0;
            }
            if length > self.max_length {
                return (out, End::Length);
            }
        }
    }

    /// The path through `start`, upstream and downstream, resampled to equal arc length
    /// (a quarter grid step) with its readings. `None` when `start` is not in the fluid.
    pub fn trace(&self, start: [f64; 3]) -> Option<Path> {
        let start = DVec3::from(start);
        let Probe::Fluid(u0) = self.velocity(start) else {
            return None;
        };
        let (back, back_end) = self.integrate(start, -1.0);
        let (fwd, forward_end) = self.integrate(start, 1.0);
        let mut raw: Vec<(DVec3, f64, DVec3, DVec3)> = back
            .into_iter()
            .rev()
            .map(|(p, t, u)| (p, t, u, u))
            .collect();
        let start_raw = raw.len();
        raw.push((start, 0.0, u0, u0));
        raw.extend(fwd.into_iter().map(|(p, t, u)| (p, t, u, u)));
        Some(self.resample(raw, start_raw, back_end, forward_end))
    }

    /// The path of a heavy particle (Task_34): points with flow time, own velocity and the flow
    /// velocity there, resampled like a streamline; it starts at the first point.
    pub fn heavy_path(&self, raw: Vec<(DVec3, f64, DVec3, DVec3)>, end: End) -> Option<Path> {
        (raw.len() > 1).then(|| self.resample(raw, 0, End::Outside, end))
    }

    /// Equal arc length (a quarter grid step) and the readings; `raw` holds position, flow time,
    /// particle velocity and flow velocity.
    fn resample(
        &self,
        raw: Vec<(DVec3, f64, DVec3, DVec3)>,
        start_raw: usize,
        back_end: End,
        forward_end: End,
    ) -> Path {
        // Arc length of the raw points.
        let mut arc = vec![0.0; raw.len()];
        for i in 1..raw.len() {
            arc[i] = arc[i - 1] + (raw[i].0 - raw[i - 1].0).length();
        }
        let total = *arc.last().unwrap_or(&0.0);
        let ds = 0.25 * self.h;
        let mut targets: Vec<f64> = (0..)
            .map(|k| f64::from(k) * ds)
            .take_while(|s| *s <= total)
            .collect();
        if total - targets.last().copied().unwrap_or(0.0) > 1e-9 * ds {
            targets.push(total);
        }
        let first_tau = raw[0].1;
        let mut points: Vec<RidePoint> = Vec::with_capacity(targets.len());
        let mut j = 0;
        for s in targets {
            while j + 2 < raw.len() && arc[j + 1] < s {
                j += 1;
            }
            let (a, b) = if raw.len() == 1 { (0, 0) } else { (j, j + 1) };
            let span = arc[b] - arc[a];
            let f = if span > 0.0 {
                ((s - arc[a]) / span).clamp(0.0, 1.0)
            } else {
                0.0
            };
            let pos = raw[a].0.lerp(raw[b].0, f);
            let vel = raw[a].2.lerp(raw[b].2, f);
            let flow = raw[a].3.lerp(raw[b].3, f);
            let tau = raw[a].1 + (raw[b].1 - raw[a].1) * f - first_tau;
            points.push(RidePoint {
                pos: pos.into(),
                s,
                tau,
                vel: vel.into(),
                speed: vel.length(),
                p: 0.0,
                cp: 0.0,
                d: self.wall_distance(pos),
                accel: 0.0,
                q: 0.0,
                slip: (flow - vel).length(),
            });
        }
        let mut last_p = self.p_ref;
        for pt in &mut points {
            let pos = DVec3::from(pt.pos);
            last_p = self.pressure(pos).unwrap_or(last_p);
            pt.p = last_p;
            pt.cp = self.cp(last_p);
            pt.q = self.q(pos).unwrap_or(0.0);
        }
        let n = points.len();
        for i in 0..n {
            let (a, b) = (i.saturating_sub(1), (i + 1).min(n - 1));
            let dt = points[b].tau - points[a].tau;
            if dt.abs() > 1e-30 {
                let du = DVec3::from(points[b].vel) - DVec3::from(points[a].vel);
                points[i].accel = du.length() / dt.abs();
            }
        }
        Path {
            points,
            ds,
            start_s: arc[start_raw],
            back_end,
            forward_end,
        }
    }
}

/// One resampled point of a path (flow frame, SI units).
#[derive(Debug, Clone, PartialEq)]
pub struct RidePoint {
    pub pos: [f64; 3],
    /// Arc length from the start of the path, m.
    pub s: f64,
    /// Flow time from the start of the path, s.
    pub tau: f64,
    pub vel: [f64; 3],
    pub speed: f64,
    /// Pressure, Pa.
    pub p: f64,
    pub cp: f64,
    /// Wall distance, m.
    pub d: f64,
    /// Acceleration |du/dτ|, m/s².
    pub accel: f64,
    pub q: f64,
    /// Heavy particles: speed relative to the flow |u - v|, m/s (0 for a massless particle).
    pub slip: f64,
}

/// A path: points at equal arc length `ds` (the last one may be closer).
#[derive(Debug, Clone, PartialEq)]
pub struct Path {
    pub points: Vec<RidePoint>,
    pub ds: f64,
    /// Arc length of the start point.
    pub start_s: f64,
    pub back_end: End,
    pub forward_end: End,
}

impl Path {
    pub fn length(&self) -> f64 {
        self.points.last().map_or(0.0, |p| p.s)
    }

    /// Index of the segment holding arc length `s` and the fraction in it.
    fn locate(&self, s: f64) -> (usize, f64) {
        let n = self.points.len();
        if n < 2 {
            return (0, 0.0);
        }
        let s = s.clamp(0.0, self.length());
        let i = ((s / self.ds).floor() as usize).min(n - 2);
        let (a, b) = (self.points[i].s, self.points[i + 1].s);
        let f = if b > a { (s - a) / (b - a) } else { 0.0 };
        (i, f.clamp(0.0, 1.0))
    }

    /// The stretch of the ride around the model: from one model length in front of the model
    /// box to 1.5 lengths behind it (the long ways from the inlet and to the outlet are left out).
    /// Unchanged when the path never comes that close.
    pub fn trimmed(mut self, flow: &Flow) -> Path {
        let l = flow.length;
        let gap = |p: &RidePoint| {
            let v = DVec3::from(p.pos);
            (flow.model_min - v)
                .max(v - flow.model_max)
                .max(DVec3::ZERO)
                .length()
        };
        let Some(a) = self.points.iter().position(|p| gap(p) <= l) else {
            return self;
        };
        let b = self
            .points
            .iter()
            .rposition(|p| gap(p) <= 1.5 * l)
            .unwrap_or(a)
            .max(a);
        let (s0, tau0) = (self.points[a].s, self.points[a].tau);
        self.points.truncate(b + 1);
        self.points.drain(..a);
        for p in &mut self.points {
            p.s -= s0;
            p.tau -= tau0;
        }
        let len = self.length();
        self.start_s = (self.start_s - s0).clamp(0.0, len);
        self
    }

    pub fn position_at(&self, s: f64) -> DVec3 {
        if self.points.len() < 2 {
            return self.points.first().map_or(DVec3::ZERO, |p| p.pos.into());
        }
        let (i, f) = self.locate(s);
        DVec3::from(self.points[i].pos).lerp(DVec3::from(self.points[i + 1].pos), f)
    }

    fn lerp_of(&self, s: f64, g: impl Fn(&RidePoint) -> f64) -> f64 {
        if self.points.len() < 2 {
            return self.points.first().map_or(0.0, &g);
        }
        let (i, f) = self.locate(s);
        g(&self.points[i]) * (1.0 - f) + g(&self.points[i + 1]) * f
    }
}

// --- Features and suggestions ----------------------------------------------------------------

/// What a path offers, used to pick suggestions.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct Features {
    pub d_min: f64,
    pub near: f64,
    pub stall: f64,
    pub turn: f64,
    pub reverse: f64,
    pub peak: f64,
    pub suction: f64,
    pub stay: f64,
    /// Path length in model lengths.
    pub length: f64,
    /// The point closest to the model.
    #[serde(skip)]
    pub closest: [f64; 3],
    /// The path comes in from outside and leaves again (or stalls at the model): it does not
    /// circle in a closed region such as a separation bubble.
    pub through: bool,
    /// The path goes on past the model (it does not end standing still at it).
    pub passes: bool,
}

/// An end of a path leaves the flow: out of the grids, or standing still away from any wall (at
/// an opening of an internal flow the grid holds no fluid beyond the opening).
fn leaves(end: End, last: Option<&RidePoint>, h: f64) -> bool {
    end == End::Outside
        || (matches!(end, End::Stalled | End::Blocked) && last.is_some_and(|p| p.d >= 2.0 * h))
}

pub fn features(path: &Path, flow: &Flow) -> Features {
    let pts = &path.points;
    let (h, l, u) = (flow.h, flow.length, flow.u_ref);
    let mut f = Features {
        d_min: f64::MAX,
        near: 0.0,
        stall: 0.0,
        turn: 0.0,
        reverse: 0.0,
        peak: 0.0,
        suction: 0.0,
        stay: 0.0,
        length: path.length() / l,
        closest: [0.0; 3],
        through: leaves(path.back_end, pts.first(), h)
            && (leaves(path.forward_end, pts.last(), h) || path.forward_end == End::Stalled),
        passes: leaves(path.forward_end, pts.last(), h),
    };
    let mut slow = f64::MAX;
    for (i, p) in pts.iter().enumerate() {
        if p.d < f.d_min {
            f.d_min = p.d;
            f.closest = p.pos;
        }
        if p.d < 5.0 * h {
            slow = slow.min(p.speed);
        }
        f.peak = f.peak.max(p.speed / u);
        f.suction = f.suction.min(p.cp);
        if i > 0 && p.d < 3.0 * h {
            f.near += p.s - pts[i - 1].s;
        }
    }
    f.near /= l;
    if slow < f64::MAX {
        f.stall = (1.0 - slow / u).max(0.0);
    }
    let tangents: Vec<DVec3> = pts
        .windows(2)
        .map(|w| DVec3::from(w[1].pos) - DVec3::from(w[0].pos))
        .filter(|t| t.length() > 1e-12)
        .collect();
    f.turn = tangents.windows(2).map(|w| w[0].angle_between(w[1])).sum();
    if let (Some(main), Some(first), Some(last)) = (flow.main, pts.first(), pts.last()) {
        let total = last.tau - first.tau;
        let back: f64 = pts
            .windows(2)
            .filter(|w| (DVec3::from(w[0].vel) + DVec3::from(w[1].vel)).dot(main) < 0.0)
            .map(|w| w[1].tau - w[0].tau)
            .sum();
        if total > 0.0 {
            f.reverse = back / total;
        }
    }
    if let (Some(first), Some(last)) = (pts.first(), pts.last()) {
        let dist = (DVec3::from(last.pos) - DVec3::from(first.pos)).length();
        f.stay = (last.tau - first.tau) / (dist / u).max(1e-12);
    }
    if f.d_min == f64::MAX {
        f.d_min = 8.0 * h;
    }
    f
}

/// Kind of flow, which decides the categories.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    WindTunnel,
    Internal,
}

/// The suggestion categories in order of choice.
pub const CATEGORIES: [&str; 6] = ["stagnation", "hugging", "fast", "wake", "turn", "long"];

/// One suggested ride.
#[derive(Debug, Clone, PartialEq)]
pub struct Suggestion {
    pub id: &'static str,
    /// Start point (flow frame): the ride is the path through it.
    pub start: [f64; 3],
    pub features: Features,
}

/// 384 start points on a stratified 24 × 16 grid in a rectangle half a model length in front of
/// the model, 1.2 times the model's cross-section.
pub fn wind_candidates(flow: &Flow, seed: u32) -> Vec<[f64; 3]> {
    let (lo, hi) = (flow.model_min, flow.model_max);
    let c = (lo + hi) * 0.5;
    let size = (hi - lo) * 1.2;
    let x = lo.x - 0.5 * flow.length;
    let mut out = Vec::with_capacity(384);
    for j in 0..16u32 {
        for i in 0..24u32 {
            let n = j * 24 + i;
            let a = (f64::from(i) + f64::from(random(n, 0, seed, 11))) / 24.0;
            let b = (f64::from(j) + f64::from(random(n, 0, seed, 12))) / 16.0;
            out.push([x, c.y + (a - 0.5) * size.y, c.z + (b - 0.5) * size.z]);
        }
    }
    out
}

/// `n` start points 1.5 grid steps off the model surface, on the fluid side.
pub fn surface_candidates(flow: &Flow, n: u32, seed: u32) -> Vec<[f64; 3]> {
    let Some(wall) = &flow.wall else {
        return vec![];
    };
    let tris = wall.triangles();
    if tris.is_empty() {
        return vec![];
    }
    // Area-weighted choice of triangles.
    let mut cum = Vec::with_capacity(tris.len());
    let mut sum = 0.0;
    for t in tris {
        sum += 0.5 * (t[1] - t[0]).cross(t[2] - t[0]).length();
        cum.push(sum);
    }
    let mut out = Vec::new();
    for i in 0..n {
        let r = f64::from(random(i, 0, seed, 21)) * sum;
        let k = cum.partition_point(|c| *c < r).min(tris.len() - 1);
        let [a, b, c] = tris[k];
        let (mut u, mut v) = (
            f64::from(random(i, 0, seed, 22)),
            f64::from(random(i, 0, seed, 23)),
        );
        if u + v > 1.0 {
            (u, v) = (1.0 - u, 1.0 - v);
        }
        let p = a + (b - a) * u + (c - a) * v;
        let normal = (b - a).cross(c - a).normalize_or_zero();
        // Boundary faces point out of the fluid; try that side first, then the other one.
        for side in [-1.0, 1.0] {
            let q = p + normal * side * 1.5 * flow.h;
            if matches!(flow.velocity(q), Probe::Fluid(_)) {
                out.push(q.into());
                break;
            }
        }
    }
    out
}

/// Path features of all candidates, computed on all cores.
pub fn trace_all(flow: &Flow, starts: &[[f64; 3]]) -> Vec<Option<(Features, f64)>> {
    let threads = std::thread::available_parallelism()
        .map_or(4, |n| n.get())
        .clamp(1, 16);
    let chunk = starts.len().div_ceil(threads).max(1);
    std::thread::scope(|scope| {
        let handles: Vec<_> = starts
            .chunks(chunk)
            .map(|part| {
                scope.spawn(move || {
                    part.iter()
                        .map(|s| flow.trace(*s).map(|p| (features(&p, flow), p.length())))
                        .collect::<Vec<_>>()
                })
            })
            .collect();
        handles
            .into_iter()
            .flat_map(|h| h.join().unwrap_or_default())
            .collect()
    })
}

fn score(id: &str, f: &Features) -> Option<f64> {
    // A ride towards the stagnation point goes on around the model: the one that comes
    // closest to standing still and still passes.
    if id == "stagnation" && !f.passes {
        return None;
    }
    let (v, min) = match id {
        "stagnation" => (f.stall, 0.3),
        "hugging" => (f.near, 0.05),
        "fast" => (f.peak, 1.02),
        "wake" => (f.reverse, 0.05),
        "turn" => (f.turn, 0.5),
        _ => (f.stay, 1.2),
    };
    (v >= min).then_some(v)
}

/// Up to five suggestions from the candidate start points: per category the best path that is
/// close to the model, at least one model length long and not too close to an earlier pick.
pub fn suggest(flow: &Flow, starts: &[[f64; 3]], kind: Kind) -> Vec<Suggestion> {
    let traced = trace_all(flow, starts);
    let eligible: Vec<(usize, &Features)> = traced
        .iter()
        .enumerate()
        .filter_map(|(i, t)| t.as_ref().map(|(f, _)| (i, f)))
        .filter(|(_, f)| f.through && f.d_min < 0.1 * flow.length && f.length > 1.0)
        .collect();
    let mut out: Vec<Suggestion> = Vec::new();
    for id in CATEGORIES {
        if out.len() >= 5 {
            break;
        }
        let wanted = !matches!(
            (id, kind),
            ("wake", Kind::Internal) | ("long", Kind::WindTunnel)
        );
        if !wanted {
            continue;
        }
        let mut best: Option<(f64, usize)> = None;
        for (i, f) in &eligible {
            let Some(v) = score(id, f) else { continue };
            let taken = out.iter().any(|s| {
                s.start == starts[*i]
                    || (DVec3::from(s.features.closest) - DVec3::from(f.closest)).length()
                        < 0.05 * flow.length
            });
            if taken {
                continue;
            }
            if best.is_none_or(|(b, _)| v > b) {
                best = Some((v, *i));
            }
        }
        if let Some((_, i)) = best {
            let f = traced[i]
                .as_ref()
                .map(|t| t.0.clone())
                .unwrap_or_else(|| unreachable!());
            out.push(Suggestion {
                id,
                start: starts[i],
                features: f,
            });
        }
    }
    out
}

// --- Timing ------------------------------------------------------------------------------------

/// How the ride feels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// Calm and explaining: compressed speeds, slower near the wall.
    Calm,
    /// Proportional to the real flow.
    Coaster,
}

/// Screen time per path point.
#[derive(Debug, Clone, PartialEq)]
pub struct Timing {
    /// Screen time of each point, s (strictly increasing).
    pub t: Vec<f64>,
    /// Screen speed at each point, m per screen second.
    pub v: Vec<f64>,
    /// Flow time per screen time at each point (dτ/dt).
    pub factor: Vec<f64>,
}

/// The screen timing of a path; `speed` (0.25 to 3) scales the whole ride.
pub fn timing(path: &Path, flow: &Flow, mode: Mode, speed: f64) -> Timing {
    let speed = speed.clamp(0.25, 3.0);
    let (l, h, u) = (flow.length, flow.h, flow.u_ref);
    let floor = STILL * u;
    let v: Vec<f64> = match mode {
        Mode::Calm => {
            let v_ref = l / 2.5;
            path.points
                .iter()
                .map(|p| {
                    let w = 0.4 + 0.6 * smoothstep(0.0, 4.0 * h, p.d);
                    (v_ref * (p.speed / u).sqrt() * w).max(0.08 * l)
                })
                .collect()
        }
        Mode::Coaster => {
            let target = (2.5 * path.length() / l).clamp(10.0, 45.0);
            // Duration with c = 1, then scale c to reach the target.
            let raw: f64 = path
                .points
                .windows(2)
                .map(|w| {
                    (w[1].s - w[0].s) / (0.5 * (w[0].speed.max(floor) + w[1].speed.max(floor)))
                })
                .sum();
            let c = raw / target;
            path.points.iter().map(|p| c * p.speed.max(floor)).collect()
        }
    };
    let mut t = Vec::with_capacity(v.len());
    let mut acc = 0.0;
    for i in 0..v.len() {
        if i > 0 {
            let ds = path.points[i].s - path.points[i - 1].s;
            acc += ds / (0.5 * (v[i] + v[i - 1])) / speed;
        }
        t.push(acc);
    }
    let factor = path
        .points
        .iter()
        .zip(&v)
        .map(|(p, vs)| vs * speed / p.speed.max(floor))
        .collect();
    Timing { t, v, factor }
}

impl Timing {
    pub fn duration(&self) -> f64 {
        self.t.last().copied().unwrap_or(0.0)
    }

    /// Arc length at screen time `t`.
    pub fn s_at(&self, path: &Path, t: f64) -> f64 {
        let n = self.t.len();
        if n < 2 {
            return 0.0;
        }
        let t = t.clamp(0.0, self.duration());
        let i = self.t.partition_point(|x| *x <= t).clamp(1, n - 1) - 1;
        let span = self.t[i + 1] - self.t[i];
        let f = if span > 0.0 {
            (t - self.t[i]) / span
        } else {
            0.0
        };
        path.points[i].s + (path.points[i + 1].s - path.points[i].s) * f
    }

    /// Screen time at arc length `s`.
    pub fn t_at(&self, path: &Path, s: f64) -> f64 {
        if self.t.len() < 2 {
            return 0.0;
        }
        let (i, f) = path.locate(s);
        self.t[i] + (self.t[i + 1] - self.t[i]) * f
    }

    /// Flow time per screen time at screen time `t`.
    pub fn factor_at(&self, path: &Path, t: f64) -> f64 {
        let s = self.s_at(path, t);
        if self.factor.len() < 2 {
            return self.factor.first().copied().unwrap_or(1.0);
        }
        let (i, f) = path.locate(s);
        self.factor[i] * (1.0 - f) + self.factor[i + 1] * f
    }
}

// --- Camera track ------------------------------------------------------------------------------

/// One camera of the track (flow frame).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CamKey {
    pub t: f64,
    pub eye: DVec3,
    pub forward: DVec3,
    pub up: DVec3,
    pub fov_deg: f64,
    pub near: f64,
    pub far: f64,
    /// Bank into the turn against the twist-free frame, degrees (coaster only).
    pub bank_deg: f64,
}

/// Track sampling, s.
pub const TRACK_STEP: f64 = 1.0 / 120.0;

fn perpendicular(v: DVec3, to: DVec3) -> DVec3 {
    (v - to * v.dot(to)).normalize_or_zero()
}

/// The camera along the ride, sampled every 1/120 s of screen time: eye on the particle, looking
/// ahead along the path (smoothed), level horizon (calm) or a twist-free frame banking into turns
/// (coaster).
pub fn camera_track(path: &Path, timing: &Timing, flow: &Flow, mode: Mode) -> Vec<CamKey> {
    let l = flow.length;
    let (ahead, tc) = match mode {
        Mode::Calm => (0.15 * l, 0.35),
        Mode::Coaster => (0.02 * l, 0.05),
    };
    let duration = timing.duration();
    let steps = (duration / TRACK_STEP).ceil() as usize;
    let max_speed = path.points.iter().map(|p| p.speed).fold(1e-12, f64::max);
    let mut out: Vec<CamKey> = Vec::with_capacity(steps + 1);
    let mut forward = DVec3::X;
    let mut frame_up = DVec3::Z;
    let mut bank = DVec3::ZERO;
    let total = path.length();
    let alpha = 1.0 - (-TRACK_STEP / tc).exp();
    for k in 0..=steps {
        let t = (k as f64 * TRACK_STEP).min(duration);
        let s = timing.s_at(path, t);
        let eye = path.position_at(s);
        // Look at the point ahead; at the end hold the last direction.
        let target = path.position_at((s + ahead).min(total));
        let want = (target - eye).normalize_or_zero();
        let want = if want == DVec3::ZERO {
            // Near the end: the last direction of the path.
            let back = path.position_at((total - ahead).max(0.0));
            let d = (eye - back).normalize_or_zero();
            if d == DVec3::ZERO { forward } else { d }
        } else {
            want
        };
        let prev = forward;
        forward = if k == 0 {
            want
        } else {
            (forward + (want - forward) * alpha).normalize_or_zero()
        };
        if forward == DVec3::ZERO {
            forward = prev;
        }
        // Parallel transport of the previous frame: no twist about the view direction.
        let transported = {
            let u = perpendicular(frame_up, forward);
            if u == DVec3::ZERO {
                perpendicular(DVec3::X, forward)
            } else {
                u
            }
        };
        if k == 0 {
            let level = perpendicular(DVec3::Z, forward);
            frame_up = if level == DVec3::ZERO {
                perpendicular(DVec3::X, forward)
            } else {
                level
            };
        } else {
            frame_up = transported;
        }
        let mut bank_deg = 0.0;
        let up = match mode {
            Mode::Calm => {
                let level = perpendicular(DVec3::Z, forward);
                let w = smoothstep(0.95, 0.99, forward.dot(DVec3::Z).abs());
                let mixed = (level * (1.0 - w) + frame_up * w).normalize_or_zero();
                let up = if mixed == DVec3::ZERO {
                    frame_up
                } else {
                    mixed
                };
                frame_up = up;
                up
            }
            Mode::Coaster => {
                // Lateral acceleration from the turn of the view direction and the screen speed.
                let turn = if k == 0 {
                    DVec3::ZERO
                } else {
                    (forward - prev) / TRACK_STEP
                };
                let screen_v = timing_speed(timing, path, s);
                let lateral = perpendicular(turn, forward) * (turn.length() * screen_v);
                bank += (lateral - bank) * alpha;
                let a0 = l; // one model length per s² gives a bank of 45°
                let phi = (bank.length() / a0).atan().min(35f64.to_radians());
                let dir = perpendicular(bank, forward);
                bank_deg = phi.to_degrees();
                if dir == DVec3::ZERO {
                    frame_up
                } else {
                    (frame_up * phi.cos() + dir * phi.sin()).normalize()
                }
            }
        };
        let fov_deg = match mode {
            Mode::Calm => 75.0,
            Mode::Coaster => 90.0 + 10.0 * path.lerp_of(s, |p| p.speed) / max_speed,
        };
        let d = path.lerp_of(s, |p| p.d);
        out.push(CamKey {
            t,
            eye,
            forward,
            up,
            fov_deg,
            near: (0.2 * d).clamp(0.02 * flow.h, 0.01 * l),
            far: 100.0 * l,
            bank_deg,
        });
        if t >= duration {
            break;
        }
    }
    out
}

fn timing_speed(timing: &Timing, path: &Path, s: f64) -> f64 {
    if timing.v.len() < 2 {
        return timing.v.first().copied().unwrap_or(0.0);
    }
    let (i, f) = path.locate(s);
    timing.v[i] * (1.0 - f) + timing.v[i + 1] * f
}

/// The camera at screen time `t`, interpolated between track samples.
pub fn camera_at(track: &[CamKey], t: f64) -> Option<CamKey> {
    let first = track.first()?;
    let last = track.last()?;
    let t = t.clamp(first.t, last.t);
    let i = ((t / TRACK_STEP).floor() as usize).min(track.len().saturating_sub(2));
    if track.len() < 2 {
        return Some(*first);
    }
    let (a, b) = (&track[i], &track[i + 1]);
    let f = if b.t > a.t {
        ((t - a.t) / (b.t - a.t)).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let forward = a.forward.lerp(b.forward, f).normalize_or(a.forward);
    let up = perpendicular(a.up.lerp(b.up, f), forward);
    Some(CamKey {
        t,
        eye: a.eye.lerp(b.eye, f),
        forward,
        up: if up == DVec3::ZERO { a.up } else { up },
        fov_deg: a.fov_deg + (b.fov_deg - a.fov_deg) * f,
        near: a.near + (b.near - a.near) * f,
        far: a.far,
        bank_deg: a.bank_deg + (b.bank_deg - a.bank_deg) * f,
    })
}

/// The chase camera: behind and above the particle; moved closer while that point is not in the
/// fluid (at most four halvings).
pub fn chase(flow: &Flow, key: &CamKey) -> CamKey {
    let mut a = 0.25 * flow.length;
    let mut eye = key.eye - key.forward * a + key.up * 0.3 * a;
    for _ in 0..4 {
        if matches!(flow.velocity(eye), Probe::Fluid(_)) {
            break;
        }
        a *= 0.5;
        eye = key.eye - key.forward * a + key.up * 0.3 * a;
    }
    let forward = (key.eye - eye).normalize_or(key.forward);
    CamKey {
        eye,
        forward,
        up: perpendicular(key.up, forward),
        fov_deg: 60.0,
        near: (0.02 * a).max(1e-4),
        ..*key
    }
}

// --- Events --------------------------------------------------------------------------------------

/// Something that happens along the ride.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct RideEvent {
    pub id: &'static str,
    /// Arc length and screen time where it starts.
    pub s: f64,
    pub t: f64,
    /// The number for the sentence (speed share, Cp, wall distance ...).
    pub value: f64,
    /// A second number (the angle of an impact, degrees).
    #[serde(default)]
    pub extra: f64,
}

/// Shortest run of a condition that counts, screen seconds.
pub const EVENT_MIN: f64 = 0.3;
/// Same event again only after this many screen seconds.
pub const EVENT_COOLDOWN: f64 = 2.0;

/// The events of a ride in order of arc length.
pub fn events(path: &Path, flow: &Flow, timing: &Timing) -> Vec<RideEvent> {
    let pts = &path.points;
    let n = pts.len();
    if n < 2 {
        return vec![];
    }
    let (u, h, l) = (flow.u_ref, flow.h, flow.length);
    let mut found: Vec<RideEvent> = Vec::new();
    // A run of a condition: start index, end index.
    let runs = |cond: &dyn Fn(usize) -> bool| {
        let mut out = Vec::new();
        let mut start = None;
        for i in 0..=n {
            let c = i < n && cond(i);
            match (c, start) {
                (true, None) => start = Some(i),
                (false, Some(a)) => {
                    out.push((a, i - 1));
                    start = None;
                }
                _ => {}
            }
        }
        out
    };
    let mut extra: Vec<RideEvent> = Vec::new();
    let mut add = |id: &'static str, a: usize, b: usize, value: f64, by_length: Option<f64>| {
        let long_enough = match by_length {
            Some(min) => pts[b].s - pts[a].s >= min,
            None => timing.t[b] - timing.t[a] >= EVENT_MIN,
        };
        if long_enough {
            found.push(RideEvent {
                id,
                s: pts[a].s,
                t: timing.t[a],
                value,
                extra: 0.0,
            });
        }
    };
    for (a, b) in runs(&|i| {
        pts[i].speed < 0.7 * u && i > 0 && pts[i].cp > pts[i - 1].cp && pts[i].d >= 5.0 * h
    }) {
        add("approach", a, b, pts[b].speed / u, None);
    }
    for (a, b) in runs(&|i| pts[i].speed < 0.25 * u && pts[i].d < 5.0 * h) {
        let cp = pts[a..=b].iter().map(|p| p.cp).fold(f64::MIN, f64::max);
        add("stagnation", a, b, cp, None);
    }
    for (a, b) in runs(&|i| pts[i].speed > 1.15 * u) {
        let peak = pts[a..=b].iter().map(|p| p.speed).fold(0.0, f64::max);
        add("acceleration", a, b, peak / u, None);
    }
    if let Some((i, p)) = pts
        .iter()
        .enumerate()
        .min_by(|x, y| x.1.cp.total_cmp(&y.1.cp))
        .filter(|(_, p)| p.cp < -0.3)
    {
        extra.push(RideEvent {
            id: "suction",
            s: p.s,
            t: timing.t[i],
            value: p.cp,
            extra: 0.0,
        });
    }
    for (a, b) in runs(&|i| pts[i].d < 2.0 * h) {
        let d = pts[a..=b].iter().map(|p| p.d).fold(f64::MAX, f64::min);
        add("near_wall", a, b, d, Some(0.1 * l));
    }
    if let Some(main) = flow.main {
        for (a, b) in runs(&|i| DVec3::from(pts[i].vel).dot(main) < 0.0) {
            add("reverse", a, b, 0.0, None);
        }
    }
    if let Some(q) = flow.q_threshold {
        for (a, b) in runs(&|i| pts[i].q > q) {
            add("vortex", a, b, 0.0, None);
        }
    }
    // Calm again after the model: past the last point near it, back at 95 %.
    if let Some(closest) = (0..n).rev().find(|i| pts[*i].d < 5.0 * h)
        && let Some((a, b)) = runs(&|i| i > closest && pts[i].speed >= 0.95 * u)
            .into_iter()
            .find(|(_, b)| *b == n - 1 || timing.t[*b] - timing.t[closest] > EVENT_MIN)
    {
        add("calm", a, b, pts[a].speed / u, None);
    }
    found.extend(extra);
    found.sort_by(|a, b| a.s.total_cmp(&b.s));
    // Cooldown per id.
    let mut out: Vec<RideEvent> = Vec::new();
    for e in found {
        if out
            .iter()
            .rev()
            .any(|o| o.id == e.id && e.t - o.t < EVENT_COOLDOWN)
        {
            continue;
        }
        out.push(e);
    }
    out
}
