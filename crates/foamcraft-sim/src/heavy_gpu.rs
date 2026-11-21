// SPDX-License-Identifier: GPL-3.0-or-later
//! CPU reference of the GPU heavy particles (`particles_heavy.wgsl`, Task_34): the same steps in
//! f32 with the distance field for impacts, so the GPU can be checked against it. The exact
//! variant with triangles (`heavy::track`) is the one for the impact map and the ride.

use crate::particles::{VelocityLevel, hash, random};

/// Properties as the shader gets them.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HeavyProps {
    pub tau_p: f32,
    pub diameter: f32,
    pub nu: f32,
    pub stick: bool,
    pub gravity: [f32; 3],
    pub restitution: f32,
    pub friction: f32,
    pub stick_time: f32,
    pub mark: f32,
    pub settle: [f32; 3],
}

type V3 = [f32; 3];

fn add(a: V3, b: V3) -> V3 {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}
fn sub(a: V3, b: V3) -> V3 {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}
fn mul(a: V3, s: f32) -> V3 {
    [a[0] * s, a[1] * s, a[2] * s]
}
fn dot(a: V3, b: V3) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
fn len(a: V3) -> f32 {
    dot(a, a).sqrt()
}

/// Cell of the finest level containing `p`: level, base corner and fractions.
fn cell(levels: &[VelocityLevel], p: V3) -> Option<(usize, [u32; 3], V3)> {
    for (n, l) in levels.iter().enumerate().rev() {
        let mut base = [0u32; 3];
        let mut frac = [0.0f32; 3];
        let mut inside = true;
        for a in 0..3 {
            let step = (l.max[a] - l.min[a]) / (l.dims[a] + 1) as f32;
            let c = (p[a] - l.min[a]) / step - 1.0;
            if !(0.0..=(l.dims[a] - 1) as f32).contains(&c) {
                inside = false;
                break;
            }
            let b = (c.floor() as u32).min(l.dims[a].saturating_sub(2));
            base[a] = b;
            frac[a] = c - b as f32;
        }
        if inside {
            return Some((n, base, frac));
        }
    }
    None
}

fn corners(l: &VelocityLevel, base: [u32; 3], frac: V3) -> impl Iterator<Item = (usize, f32)> + '_ {
    (0..8u32).map(move |corner| {
        let o = [corner & 1, (corner >> 1) & 1, (corner >> 2) & 1];
        let q = [base[0] + o[0], base[1] + o[1], base[2] + o[2]];
        let id = ((q[2] * l.dims[1] + q[1]) * l.dims[0] + q[0]) as usize;
        let w: f32 = (0..3)
            .map(|a| if o[a] == 1 { frac[a] } else { 1.0 - frac[a] })
            .product();
        (id, w)
    })
}

/// Velocity with masked points as resting wall; `None` outside every grid.
pub fn sample_wall(levels: &[VelocityLevel], p: V3) -> Option<V3> {
    let (n, base, frac) = cell(levels, p)?;
    let l = &levels[n];
    let mut u = [0.0f32; 3];
    for (id, w) in corners(l, base, frac) {
        let d = l.data[id];
        for k in 0..3 {
            u[k] += w * d[k] * d[3];
        }
    }
    Some(u)
}

/// Distance field (all levels one after the other) at `p`; large outside.
pub fn distance_at(levels: &[VelocityLevel], dist: &[f32], p: V3) -> f32 {
    let Some((n, base, frac)) = cell(levels, p) else {
        return 1e30;
    };
    let offset: usize = levels[..n].iter().map(|l| l.data.len()).sum();
    let l = &levels[n];
    corners(l, base, frac)
        .map(|(id, w)| w * dist.get(offset + id).copied().unwrap_or(1e30))
        .sum()
}

fn normal_at(levels: &[VelocityLevel], dist: &[f32], p: V3, e: f32) -> V3 {
    let d = |q: V3| distance_at(levels, dist, q);
    let g = [
        d(add(p, [e, 0.0, 0.0])) - d(sub(p, [e, 0.0, 0.0])),
        d(add(p, [0.0, e, 0.0])) - d(sub(p, [0.0, e, 0.0])),
        d(add(p, [0.0, 0.0, e])) - d(sub(p, [0.0, 0.0, e])),
    ];
    let n = len(g);
    if n < 1e-30 {
        [0.0, 0.0, 1.0]
    } else {
        mul(g, 1.0 / n)
    }
}

fn drag_factor(re: f32) -> f32 {
    if re <= 1000.0 {
        1.0 + 0.15 * re.max(0.0).powf(0.687)
    } else {
        0.44 * re / 24.0
    }
}

/// The GPU heavy particles on the CPU.
pub struct HeavyReference<'a> {
    levels: &'a [VelocityLevel],
    dist: &'a [f32],
    seeds: Vec<V3>,
    seed: u32,
    max_age: f32,
    frame: u32,
    props: HeavyProps,
    /// Position and age per particle.
    pub state: Vec<[f32; 4]>,
    /// Velocity and stuck time (w < 0 flying, >= 0 stuck).
    pub motion: Vec<[f32; 4]>,
}

impl<'a> HeavyReference<'a> {
    pub fn new(
        levels: &'a [VelocityLevel],
        dist: &'a [f32],
        seeds: Vec<V3>,
        count: u32,
        seed: u32,
        max_age: f32,
        props: HeavyProps,
    ) -> Self {
        let mut r = Self {
            levels,
            dist,
            seeds,
            seed,
            max_age,
            frame: 0,
            props,
            state: vec![[0.0; 4]; count as usize],
            motion: vec![[0.0; 4]; count as usize],
        };
        for i in 0..count {
            let s = r.respawn(i, 0);
            let age = random(i, 0, seed, 7) * max_age;
            r.state[i as usize] = [s[0], s[1], s[2], age];
            let v = r.start_velocity(s);
            r.motion[i as usize] = [v[0], v[1], v[2], -1.0];
        }
        r
    }

    fn respawn(&self, i: u32, frame: u32) -> V3 {
        let n = self.seeds.len().max(1) as u32;
        let pick = hash(i ^ hash(frame ^ hash(self.seed))) % n;
        self.seeds.get(pick as usize).copied().unwrap_or([0.0; 3])
    }

    fn tau_e(&self, rel: f32) -> f32 {
        let p = &self.props;
        p.tau_p / drag_factor(rel * p.diameter / p.nu.max(1e-30))
    }

    fn start_velocity(&self, p: V3) -> V3 {
        let u = sample_wall(self.levels, p).unwrap_or([0.0; 3]);
        add(u, self.props.settle)
    }

    fn exact(&self, x: V3, v: V3, u: V3, tau: f32, dt: f32) -> (V3, V3) {
        let v_inf = add(u, mul(self.props.gravity, tau));
        if tau <= 1e-30 {
            return (add(x, mul(v_inf, dt)), v_inf);
        }
        let e = (-dt / tau).exp();
        let dv = sub(v, v_inf);
        (
            add(add(x, mul(v_inf, dt)), mul(dv, tau * (1.0 - e))),
            add(v_inf, mul(dv, e)),
        )
    }

    pub fn step(&mut self, dt: f32) {
        self.frame = self.frame.wrapping_add(1);
        for i in 0..self.state.len() {
            let mut s = self.state[i];
            let mut m = self.motion[i];
            if m[3] >= 0.0 {
                m[3] += dt;
                if m[3] > self.props.stick_time {
                    let p = self.respawn(i as u32, self.frame);
                    s = [p[0], p[1], p[2], 0.0];
                    let v = self.start_velocity(p);
                    m = [v[0], v[1], v[2], -1.0];
                }
                self.state[i] = s;
                self.motion[i] = m;
                continue;
            }
            let x = [s[0], s[1], s[2]];
            let v = [m[0], m[1], m[2]];
            let age = s[3] + dt;
            let Some(u0) = sample_wall(self.levels, x).filter(|_| age <= self.max_age) else {
                let p = self.respawn(i as u32, self.frame);
                let v = self.start_velocity(p);
                self.state[i] = [p[0], p[1], p[2], 0.0];
                self.motion[i] = [v[0], v[1], v[2], -1.0];
                continue;
            };
            let (x1, _) = self.exact(x, v, u0, self.tau_e(len(sub(u0, v))), dt);
            let um = sample_wall(self.levels, mul(add(x, x1), 0.5)).unwrap_or(u0);
            let (mut nx, mut nv) = self.exact(x, v, um, self.tau_e(len(sub(um, v))), dt);
            let d0 = distance_at(self.levels, self.dist, x);
            let d1 = distance_at(self.levels, self.dist, nx);
            let mut stuck = false;
            if d0 > 0.0 && d1 <= 0.0 {
                let f = d0 / (d0 - d1).max(1e-30);
                let hit = add(x, mul(sub(nx, x), f));
                let vi = add(v, mul(sub(nv, v), f));
                let n = normal_at(self.levels, self.dist, hit, 0.25 * self.props.mark);
                let bounces = (-m[3] - 1.0) as u32;
                if self.props.stick || bounces >= 5 {
                    nx = hit;
                    nv = [0.0; 3];
                    stuck = true;
                } else {
                    let vn = mul(n, dot(vi, n));
                    nv = add(
                        mul(vn, -self.props.restitution),
                        mul(sub(vi, vn), 1.0 - self.props.friction),
                    );
                    nx = add(hit, mul(n, 0.05 * self.props.mark));
                    m[3] -= 1.0;
                }
            }
            self.state[i] = [nx[0], nx[1], nx[2], age];
            self.motion[i] = if stuck {
                [0.0; 4]
            } else {
                [nv[0], nv[1], nv[2], m[3]]
            };
        }
    }
}
