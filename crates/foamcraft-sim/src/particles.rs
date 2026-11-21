// SPDX-License-Identifier: GPL-3.0-or-later
//! Particles in the sampled velocity: the CPU reference of the GPU advection (same sampling,
//! integrator, random numbers and respawning), seed points of start regions and streamlines.

use foamcraft_core::error::FoamError;

use crate::results::Grid;

/// One velocity grid in a GPU-friendly form: `(ux, uy, uz, valid)` per point.
#[derive(Debug, Clone, PartialEq)]
pub struct VelocityLevel {
    pub min: [f32; 3],
    pub max: [f32; 3],
    pub dims: [u32; 3],
    pub data: Vec<[f32; 4]>,
}

impl VelocityLevel {
    pub fn from_grid(g: &Grid) -> Result<Self, FoamError> {
        let col = |n: &str| {
            g.names
                .iter()
                .position(|x| x == n)
                .ok_or_else(|| FoamError::new("results.unknown_field", n))
        };
        let (x, y, z) = (col("Ux")?, col("Uy")?, col("Uz")?);
        let data = (0..g.spec.len())
            .map(|i| {
                if g.mask[i] {
                    [0.0; 4]
                } else {
                    [g.values[x][i], g.values[y][i], g.values[z][i], 1.0]
                }
            })
            .collect();
        Ok(Self {
            min: g.spec.min.map(|v| v as f32),
            max: g.spec.max.map(|v| v as f32),
            dims: g.spec.dims,
            data,
        })
    }

    fn index(&self, i: u32, j: u32, k: u32) -> usize {
        ((k * self.dims[1] + j) * self.dims[0] + i) as usize
    }
}

/// Result of a velocity lookup.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Sample {
    Velocity([f32; 3]),
    /// Inside a solid (a corner of the cell is masked).
    Blocked,
    /// Outside every grid.
    Outside,
}

/// Velocity at `p` from the finest level that contains it; a masked corner blocks.
pub fn sample(levels: &[VelocityLevel], p: [f32; 3]) -> Sample {
    for l in levels.iter().rev() {
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
        if !inside {
            continue;
        }
        let mut u = [0.0f32; 3];
        for corner in 0..8u32 {
            let o = [corner & 1, corner >> 1 & 1, corner >> 2 & 1];
            let d = l.data[l.index(base[0] + o[0], base[1] + o[1], base[2] + o[2])];
            if d[3] == 0.0 {
                return Sample::Blocked;
            }
            let w: f32 = (0..3)
                .map(|a| if o[a] == 1 { frac[a] } else { 1.0 - frac[a] })
                .product();
            for c in 0..3 {
                u[c] += w * d[c];
            }
        }
        return Sample::Velocity(u);
    }
    Sample::Outside
}

/// PCG hash (M. O'Neill's PCG output function), the same in the GPU shader.
pub fn hash(v: u32) -> u32 {
    let state = v.wrapping_mul(747_796_405).wrapping_add(2_891_336_453);
    let word = ((state >> ((state >> 28) + 4)) ^ state).wrapping_mul(277_803_737);
    (word >> 22) ^ word
}

/// A random number in 0..1 for particle `i` at `frame` (stream `k`).
pub fn random(i: u32, frame: u32, seed: u32, k: u32) -> f32 {
    let h = hash(i ^ hash(frame ^ hash(seed.wrapping_add(k))));
    (h >> 8) as f32 / (1u32 << 24) as f32
}

/// A start region: the parallelogram origin + s u + t v (a line when `v` is zero).
#[derive(Debug, Clone, PartialEq)]
pub struct SeedRegion {
    pub origin: [f32; 3],
    pub u: [f32; 3],
    pub v: [f32; 3],
}

/// `n` start points in a region, the same for the same seed.
pub fn seed_points(r: &SeedRegion, n: u32, seed: u32) -> Vec<[f32; 3]> {
    (0..n)
        .map(|i| {
            let (s, t) = (random(i, 0, seed, 101), random(i, 0, seed, 202));
            [0, 1, 2].map(|k| r.origin[k] + s * r.u[k] + t * r.v[k])
        })
        .collect()
}

/// Particles advected with the midpoint rule; a particle that leaves the grids, is blocked or is
/// older than `max_age` starts again at a seed point.
pub struct Particles<'a> {
    levels: &'a [VelocityLevel],
    seeds: Vec<[f32; 3]>,
    seed: u32,
    max_age: f32,
    frame: u32,
    pub positions: Vec<[f32; 3]>,
    pub ages: Vec<f32>,
    pub respawned: u64,
}

impl<'a> Particles<'a> {
    pub fn new(
        levels: &'a [VelocityLevel],
        seeds: Vec<[f32; 3]>,
        count: u32,
        seed: u32,
        max_age: f32,
    ) -> Self {
        let mut p = Self {
            levels,
            seeds,
            seed,
            max_age,
            frame: 0,
            positions: vec![[0.0; 3]; count as usize],
            ages: vec![0.0; count as usize],
            respawned: 0,
        };
        for i in 0..count {
            p.respawn(i);
            // Spread the ages so particles do not start again all at once.
            p.ages[i as usize] = random(i, 0, seed, 7) * max_age;
        }
        p
    }

    fn respawn(&mut self, i: u32) {
        let n = self.seeds.len().max(1) as u32;
        let pick = hash(i ^ hash(self.frame ^ hash(self.seed))) % n;
        self.positions[i as usize] = self.seeds.get(pick as usize).copied().unwrap_or([0.0; 3]);
        self.ages[i as usize] = 0.0;
    }

    pub fn step(&mut self, dt: f32) {
        self.frame = self.frame.wrapping_add(1);
        for i in 0..self.positions.len() as u32 {
            let p = self.positions[i as usize];
            let next = match sample(self.levels, p) {
                Sample::Velocity(u1) => {
                    let mid = [0, 1, 2].map(|k| p[k] + 0.5 * dt * u1[k]);
                    match sample(self.levels, mid) {
                        Sample::Velocity(u2) => Some([0, 1, 2].map(|k| p[k] + dt * u2[k])),
                        _ => None,
                    }
                }
                _ => None,
            };
            let age = self.ages[i as usize] + dt;
            match next {
                Some(q)
                    if age <= self.max_age
                        && matches!(sample(self.levels, q), Sample::Velocity(_)) =>
                {
                    self.positions[i as usize] = q;
                    self.ages[i as usize] = age;
                }
                _ => {
                    self.respawn(i);
                    self.respawned += 1;
                }
            }
        }
    }
}

/// A streamline from `start` (midpoint rule, step `dt`) until it leaves the grids, reaches a
/// solid or after `max_steps` points.
pub fn streamline(
    levels: &[VelocityLevel],
    start: [f32; 3],
    dt: f32,
    max_steps: u32,
) -> Vec<[f32; 3]> {
    let mut line = vec![start];
    let mut p = start;
    for _ in 0..max_steps {
        let Sample::Velocity(u1) = sample(levels, p) else {
            break;
        };
        let mid = [0, 1, 2].map(|k| p[k] + 0.5 * dt * u1[k]);
        let Sample::Velocity(u2) = sample(levels, mid) else {
            break;
        };
        let q = [0, 1, 2].map(|k| p[k] + dt * u2[k]);
        if !matches!(sample(levels, q), Sample::Velocity(_)) {
            break;
        }
        line.push(q);
        p = q;
    }
    line
}

/// The positions on the visible side of a clip plane (`(p - origin) · normal <= 0`), as the GPU
/// draws them; all in the same frame.
pub fn visible_positions(
    positions: &[[f32; 3]],
    origin: [f32; 3],
    normal: [f32; 3],
) -> Vec<[f32; 3]> {
    positions
        .iter()
        .copied()
        .filter(|p| (0..3).map(|k| (p[k] - origin[k]) * normal[k]).sum::<f32>() <= 0.0)
        .collect()
}

/// Start points of a rake: `nozzles` nozzles evenly on `origin + s·u` (s in 0..1), each point in a
/// small disc of `radius` across `u`, so every nozzle gives one thin smoke line.
pub fn rake_points(
    origin: [f32; 3],
    u: [f32; 3],
    radius: f32,
    nozzles: u32,
    n: u32,
    seed: u32,
) -> Vec<[f32; 3]> {
    let len = (u[0] * u[0] + u[1] * u[1] + u[2] * u[2]).sqrt();
    let axis = if len > 1e-12 {
        u.map(|c| c / len)
    } else {
        [1.0, 0.0, 0.0]
    };
    // Two directions across the rake.
    let helper = if axis[0].abs() < 0.9 {
        [1.0, 0.0, 0.0]
    } else {
        [0.0, 1.0, 0.0]
    };
    let cross = |a: [f32; 3], b: [f32; 3]| {
        [
            a[1] * b[2] - a[2] * b[1],
            a[2] * b[0] - a[0] * b[2],
            a[0] * b[1] - a[1] * b[0],
        ]
    };
    let norm = |a: [f32; 3]| {
        let l = (a[0] * a[0] + a[1] * a[1] + a[2] * a[2]).sqrt().max(1e-30);
        a.map(|c| c / l)
    };
    let e1 = norm(cross(axis, helper));
    let e2 = cross(axis, e1);
    let nozzles = nozzles.max(1);
    (0..n)
        .map(|i| {
            let k = i % nozzles;
            let s = if nozzles == 1 {
                0.5
            } else {
                k as f32 / (nozzles - 1) as f32
            };
            // Uniform in the disc: radius from the square root.
            let r = radius * random(i, 0, seed, 303).sqrt();
            let a = random(i, 0, seed, 404) * std::f32::consts::TAU;
            let (c, sn) = (a.cos() * r, a.sin() * r);
            [0, 1, 2].map(|j| origin[j] + s * u[j] + c * e1[j] + sn * e2[j])
        })
        .collect()
}
