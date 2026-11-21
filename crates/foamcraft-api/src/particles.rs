// SPDX-License-Identifier: GPL-3.0-or-later
//! Particles and streamlines of the view: presets, start regions (area, inlets, near the
//! surface), the setup the app animates on the GPU, and `view.set_particles`.

use foamcraft_core::command::{Options, Registry, Session};
use foamcraft_core::error::FoamError;
use foamcraft_core::project::{BoundaryRole, ParticleSpec, Project, SeedRegionSpec};
use foamcraft_render::particles::{FlowLevel, ParticleColor, ParticleSettings};
use foamcraft_render::scene::Mesh;
use foamcraft_sim::particles::{SeedRegion, VelocityLevel, seed_points, streamline};
use foamcraft_sim::results::{Grid, colormap};
use glam::{DMat3, DVec3};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::json;

/// Seed points uploaded to the GPU per start region.
pub const SEEDS: u32 = 4096;
/// Seconds a particle needs to cross the finest grid at speed 1.
pub const CROSSING_SECONDS: f64 = 4.0;
/// Frames per second of the animation.
pub const FPS: f64 = 60.0;
/// Brightness of one light trail by default: many overlapping trails add up (Task_35).
pub const DEFAULT_TRAIL_EXPOSURE: f64 = 0.4;

/// Everything the app needs to animate the particles.
pub struct ParticleSetup {
    pub levels: Vec<FlowLevel>,
    pub seeds: Vec<[f32; 3]>,
    pub settings: ParticleSettings,
    /// Flow time per animation frame, s.
    pub dt: f32,
    pub paused: bool,
    /// Clip plane (origin, normal; world) when the view cuts the particles open.
    pub clip: Option<([f32; 3], [f32; 3])>,
    /// Heavy particles: their properties and the distance field of the grids (Task_34).
    pub heavy: Option<(
        foamcraft_render::particles::HeavySettings,
        std::sync::Arc<Vec<f32>>,
    )>,
    /// Light trails (Task_35): ring length, writing interval and look.
    pub trails: Option<foamcraft_render::trails::TrailSettings>,
    /// The memory limit made the trails shorter than asked ("trails shortened").
    pub trails_shortened: bool,
    /// Long exposure in the view: half-life of the light, screen seconds.
    pub long_exposure: Option<f32>,
}

/// Flow time of a comet tail: 6 % of the time the fastest flow needs to cross the finest grid.
fn comet_tail(grids: &[Grid]) -> f64 {
    let Some(fine) = finest(grids) else {
        return 0.0;
    };
    let length = (0..3)
        .map(|k| fine.spec.max[k] - fine.spec.min[k])
        .fold(0.0, f64::max);
    let (_, umax) = max_of(grids, "U_mag");
    0.06 * CROSSING_SECONDS * (length / umax.max(1e-9) / CROSSING_SECONDS)
}

/// Light trails from the particle spec: `trail_length` (flow seconds, default three comet tails,
/// at most ten) spread over K points within the memory limit.
fn trail_settings(
    spec: &ParticleSpec,
    color: ParticleColor,
    range: (f64, f64),
    tail: f64,
    dt: f32,
) -> (foamcraft_render::trails::TrailSettings, bool) {
    use foamcraft_render::trails::{DEFAULT_K, points_per_trail};
    let k = points_per_trail(spec.count, DEFAULT_K);
    let length = spec
        .trail_length
        .unwrap_or(3.0 * tail)
        .clamp(tail, 10.0 * tail);
    let every = (length / (f64::from(dt).max(1e-12) * f64::from(k - 1)))
        .round()
        .clamp(1.0, 10_000.0) as u32;
    (
        foamcraft_render::trails::TrailSettings {
            k,
            every,
            width_px: spec.width.unwrap_or(2.5) as f32,
            exposure: spec.exposure.unwrap_or(DEFAULT_TRAIL_EXPOSURE) as f32,
            glow: spec.glow.unwrap_or(true),
            by_speed: color == ParticleColor::Speed,
            range: [range.0 as f32, range.1 as f32],
            color: [1.0, 0.86, 0.62],
        },
        k < DEFAULT_K,
    )
}

fn finest(grids: &[Grid]) -> Option<&Grid> {
    grids.last()
}

fn max_of(grids: &[Grid], field: &str) -> (f64, f64) {
    let mut lo = f64::MAX;
    let mut hi = f64::MIN;
    for g in grids {
        if let Some(f) = g.names.iter().position(|n| n == field) {
            for (v, m) in g.values[f].iter().zip(&g.mask) {
                if !m {
                    lo = lo.min(f64::from(*v));
                    hi = hi.max(f64::from(*v));
                }
            }
        }
    }
    if lo > hi { (0.0, 1.0) } else { (lo, hi) }
}

/// Start points (flow frame) of the region.
pub(crate) fn seeds(
    project: &Project,
    region: &SeedRegionSpec,
    grids: &[Grid],
    yaw: f64,
    seed: u32,
) -> Vec<[f32; 3]> {
    let to_flow = DMat3::from_rotation_z(-yaw.to_radians());
    let f = |v: [f64; 3]| {
        let w = to_flow * DVec3::from(v);
        [w.x as f32, w.y as f32, w.z as f32]
    };
    match region.kind.as_str() {
        "inlets" => {
            let views = foamcraft_geometry::caps::cap_views(project);
            let inlets: Vec<_> = views
                .iter()
                .filter(|v| v.role == Some(BoundaryRole::Inlet))
                .collect();
            let per = SEEDS / inlets.len().max(1) as u32;
            let mut out = Vec::new();
            for (n, v) in inlets.iter().enumerate() {
                let normal = DVec3::from(v.inward);
                let (u, w) = (
                    normal.any_orthonormal_vector(),
                    normal.cross(normal.any_orthonormal_vector()),
                );
                let side = v.size * 0.6;
                let origin =
                    DVec3::from(v.centroid) + normal * v.size * 0.05 - (u + w) * side / 2.0;
                let r = SeedRegion {
                    origin: f(origin.into()),
                    u: f((u * side).into()),
                    v: f((w * side).into()),
                };
                out.extend(seed_points(&r, per, seed.wrapping_add(n as u32)));
            }
            out
        }
        "rake" => {
            // Nozzles along u, each with a small scatter disc of radius |v|.
            let radius = DVec3::from(region.v).length() as f32;
            foamcraft_sim::particles::rake_points(
                f(region.origin),
                f(region.u),
                radius,
                crate::presets::RAKE_NOZZLES,
                SEEDS,
                seed,
            )
        }
        "surface" => {
            let Some(job) = project.manifest.view.result_job.clone() else {
                return vec![];
            };
            let Ok(s) = crate::results::load_surface(&project.root, &job) else {
                return vec![];
            };
            let offset = finest(grids).map_or(1e-3, |g| g.spec.step()[0] * 0.75);
            let n = s.triangles.len().max(1) as u32;
            (0..SEEDS)
                .map(|i| {
                    let t = s.triangles[(foamcraft_sim::particles::hash(i ^ seed) % n) as usize];
                    let [a, b, c] = t.map(|k| DVec3::from(s.points[k as usize].map(f64::from)));
                    // Boundary faces point out of the fluid: step against the normal.
                    let normal = (b - a).cross(c - a).normalize_or_zero();
                    let p = (a + b + c) / 3.0 - normal * offset;
                    f(p.into())
                })
                .collect()
        }
        _ => seed_points(
            &SeedRegion {
                origin: f(region.origin),
                u: f(region.u),
                v: f(region.v),
            },
            SEEDS,
            seed,
        ),
    }
}

/// The particle setup of the view (none for streamlines or without particles).
pub fn setup(project: &Project) -> Option<ParticleSetup> {
    let spec = project.manifest.view.particles.clone()?;
    if spec.style == "streamlines" {
        return None;
    }
    let (grids, yaw) = crate::view::load_grids(project).ok()?;
    let fine = finest(&grids)?;
    let length = (0..3)
        .map(|k| fine.spec.max[k] - fine.spec.min[k])
        .fold(0.0, f64::max);
    let (_, umax) = max_of(&grids, "U_mag");
    let time_scale = length / umax.max(1e-9) / CROSSING_SECONDS;
    let levels: Vec<FlowLevel> = grids
        .iter()
        .filter_map(|g| {
            let v = VelocityLevel::from_grid(g).ok()?;
            let pressure = g
                .names
                .iter()
                .position(|n| n == "p")
                .map(|i| g.values[i].clone())
                .unwrap_or_default();
            Some(FlowLevel {
                min: v.min,
                max: v.max,
                dims: v.dims,
                data: v.data,
                pressure,
            })
        })
        .collect();
    let (color, range) = match (spec.style.as_str(), spec.color_by.as_str()) {
        ("smoke", _) => (ParticleColor::Constant, (0.0, 1.0)),
        (_, "age") => (ParticleColor::Age, (0.0, 1.0)),
        (_, "pressure") => (ParticleColor::Pressure, max_of(&grids, "p")),
        (_, "slip") => (ParticleColor::Slip, (0.0, 0.25 * umax)),
        _ => (ParticleColor::Speed, (0.0, umax)),
    };
    let smoke = spec.style == "smoke";
    let dt = (time_scale * spec.speed / FPS) as f32;
    let heavy = spec.inertia.as_ref().map(|i| {
        let job = crate::view::job_of(project).unwrap_or_default();
        let m = crate::heavy::medium_of(project, &job);
        let p = crate::heavy::particle_of(i);
        let g = foamcraft_sim::heavy::gravity_accel(&p, &m);
        let h = fine.spec.step().into_iter().fold(f64::MAX, f64::min);
        let (restitution, friction) = match p.impact {
            foamcraft_sim::heavy::Impact::Bounce {
                restitution,
                friction,
            } => (restitution, friction),
            foamcraft_sim::heavy::Impact::Stick => (0.0, 0.0),
        };
        (
            foamcraft_render::particles::HeavySettings {
                tau_p: foamcraft_sim::heavy::tau_p(&p, &m) as f32,
                diameter: p.diameter as f32,
                nu: m.kinematic_viscosity as f32,
                stick: matches!(p.impact, foamcraft_sim::heavy::Impact::Stick),
                gravity: [g.x as f32, g.y as f32, g.z as f32],
                restitution: restitution as f32,
                friction: friction as f32,
                // 1.5 s on screen at 60 frames per second.
                stick_time: dt * 90.0,
                mark: (0.6 * h) as f32,
                settle: {
                    let v = foamcraft_sim::heavy::terminal_velocity(&p, &m);
                    [v.x as f32, v.y as f32, v.z as f32]
                },
            },
            crate::heavy::distance_field(project, &grids, yaw),
        )
    });
    let tail = comet_tail(&grids);
    let (trails, trails_shortened) = if spec.style == "trails" {
        let (t, short) = trail_settings(&spec, color, range, tail, dt);
        (Some(t), short)
    } else {
        (None, false)
    };
    Some(ParticleSetup {
        trails,
        trails_shortened,
        long_exposure: spec
            .long_exposure
            .filter(|_| spec.style == "trails")
            .map(|h| h as f32),
        seeds: seeds(project, &spec.region, &grids, yaw, spec.seed),
        levels,
        settings: ParticleSettings {
            count: spec.count,
            seed: spec.seed,
            max_age: (CROSSING_SECONDS * 1.5 * time_scale) as f32,
            trail: (if smoke { 0.12 } else { 0.06 } * CROSSING_SECONDS * time_scale) as f32,
            color,
            range: [range.0 as f32, range.1 as f32],
            constant: [0.92, 0.92, 0.95],
            opacity: if smoke { 0.5 } else { 1.0 },
            yaw_deg: yaw as f32,
        },
        dt,
        paused: spec.paused,
        clip: project
            .manifest
            .view
            .clip
            .as_ref()
            .filter(|_| project.manifest.view.clips("particles"))
            .map(|c| (c.origin.map(|x| x as f32), c.normal.map(|x| x as f32))),
        heavy,
    })
}

/// Trails for a long exposure: those of the view, or for other styles the shortest ring (the
/// exposure draws only the newest piece of each step).
pub fn exposure_trails(setup: &ParticleSetup) -> foamcraft_render::trails::TrailSettings {
    setup
        .trails
        .clone()
        .unwrap_or(foamcraft_render::trails::TrailSettings {
            k: 2,
            every: 1,
            width_px: 2.0,
            exposure: 1.0,
            glow: true,
            by_speed: setup.settings.color == ParticleColor::Speed,
            range: setup.settings.range,
            color: [1.0, 0.86, 0.62],
        })
}

/// Uploads the particles of a setup (massless or heavy).
pub fn create_gpu(
    renderer: &foamcraft_render::Renderer,
    setup: &ParticleSetup,
) -> foamcraft_render::particles::GpuParticles {
    let mut p = match &setup.heavy {
        Some((h, dist)) => {
            renderer.create_heavy_particles(&setup.levels, &setup.seeds, &setup.settings, h, dist)
        }
        None => match &setup.trails {
            Some(t) => {
                let mut p = renderer.create_trail_particles(
                    &setup.levels,
                    &setup.seeds,
                    &setup.settings,
                    t,
                );
                renderer.set_trail_fade(&mut p, setup.long_exposure);
                // One life ahead: the trails start spread over their way, not as one bright
                // sheet at the seed area.
                let warm = (setup.settings.max_age / setup.dt.max(1e-12))
                    .ceil()
                    .min(3000.0) as u32;
                renderer.step_particles(&mut p, setup.dt, warm);
                p
            }
            None => renderer.create_particles(&setup.levels, &setup.seeds, &setup.settings),
        },
    };
    renderer.set_particle_clip(&mut p, setup.clip);
    p
}

/// Ribbons and tubes of the view (Task_35): streamlines from the start region (at most 200),
/// as triangles with time marks that run downstream and a colour by speed or by swirl.
pub fn ribbon_geometry(project: &Project) -> Option<foamcraft_render::flow::FlowGeometry> {
    use foamcraft_render::flow::{FlowGeometry, FlowVertex};
    use foamcraft_sim::ribbons::{frames, ribbon, tube, vorticity};
    let spec = project.manifest.view.particles.clone()?;
    if !["ribbons", "tubes"].contains(&spec.style.as_str()) {
        return None;
    }
    let (grids, yaw) = crate::view::load_grids(project).ok()?;
    let levels: Vec<VelocityLevel> = grids
        .iter()
        .filter_map(|g| VelocityLevel::from_grid(g).ok())
        .collect();
    let fine = finest(&grids)?;
    let h = fine.spec.step().into_iter().fold(f64::MAX, f64::min);
    let length = (0..3)
        .map(|k| fine.spec.max[k] - fine.spec.min[k])
        .fold(0.0, f64::max);
    let (_, umax) = max_of(&grids, "U_mag");
    let model = crate::ride::load_flow(project).ok();
    let (l, u_ref) = model
        .as_ref()
        .map_or((length / 4.0, umax), |rf| (rf.flow.length, rf.flow.u_ref));
    let u_at = |p: DVec3| match foamcraft_sim::particles::sample(
        &levels,
        [p.x as f32, p.y as f32, p.z as f32],
    ) {
        foamcraft_sim::particles::Sample::Velocity(u) => Some(DVec3::new(
            f64::from(u[0]),
            f64::from(u[1]),
            f64::from(u[2]),
        )),
        _ => None,
    };
    // Flow seconds per screen second, as the particle animation.
    let time_scale = length / umax.max(1e-9) / CROSSING_SECONDS;
    let dt = (length / umax.max(1e-9) / 400.0) as f32;
    let starts = seeds(project, &spec.region, &grids, yaw, spec.seed);
    let count = (spec.count as usize).clamp(1, 200);
    // Swirl colour: hide weak vorticity (between the 50th and 90th percentile of |ω|).
    let mut mags: Vec<f64> = (0..fine.spec.len())
        .step_by(7)
        .filter(|i| !fine.mask[*i])
        .map(|i| {
            let d = fine.spec.dims;
            let k = (i / (d[0] * d[1]) as usize) as u32;
            let j = ((i / d[0] as usize) % d[1] as usize) as u32;
            let ii = (i % d[0] as usize) as u32;
            vorticity(&u_at, DVec3::from(fine.spec.point(ii, j, k)), h).length()
        })
        .collect();
    mags.sort_by(f64::total_cmp);
    let pct = |q: f64| {
        mags.get(((mags.len() as f64 * q) as usize).min(mags.len().saturating_sub(1)))
            .copied()
            .unwrap_or(0.0)
    };
    let (w50, w90) = (pct(0.5), pct(0.9).max(pct(0.5) + 1e-12));
    let to_world = DMat3::from_rotation_z(yaw.to_radians());
    let f3 = |v: DVec3| {
        let w = to_world * v;
        [w.x as f32, w.y as f32, w.z as f32]
    };
    let width = spec.width.unwrap_or(0.012).clamp(0.002, 0.05) * l;
    let mark = (0.05 * l / u_ref.max(1e-9) / time_scale) as f32;
    let paused = spec.paused;
    let mut geo = FlowGeometry::default();
    for s in starts.iter().take(count) {
        let line = streamline(&levels, *s, dt, 2000);
        if line.len() < 3 {
            continue;
        }
        let pts: Vec<DVec3> = line.iter().map(|p| DVec3::from(p.map(f64::from))).collect();
        let taus: Vec<f64> = (0..pts.len()).map(|i| i as f64 * f64::from(dt)).collect();
        let speeds: Vec<f64> = pts
            .iter()
            .map(|p| u_at(*p).map_or(0.0, |u| u.length()))
            .collect();
        let omega: Vec<DVec3> = pts.iter().map(|p| vorticity(&u_at, *p, h)).collect();
        let fr = frames(&pts, &taus, &speeds, &omega);
        let color = |f: &foamcraft_sim::ribbons::Frame| -> [f32; 3] {
            if spec.color_by == "swirl" {
                let x = ((f.vorticity - w50) / (w90 - w50)).clamp(0.0, 1.0);
                let weight = x * x * (3.0 - 2.0 * x);
                let s = (f.swirl / (f.vorticity + 1e-12)) * weight;
                // Blue for left-handed, orange for right-handed: told apart with common colour
                // blindness.
                let (neutral, right, left) =
                    ([0.78, 0.78, 0.8], [0.96, 0.58, 0.12], [0.2, 0.5, 0.95]);
                let side = if s >= 0.0 { right } else { left };
                let a = s.abs() as f32;
                [0, 1, 2].map(|k| neutral[k] + (side[k] - neutral[k]) * a)
            } else {
                foamcraft_sim::results::colormap(f.speed / umax.max(1e-9))
            }
        };
        let p = |f: &foamcraft_sim::ribbons::Frame| {
            [
                (f.tau / time_scale) as f32,
                0.0,
                if paused { 0.0 } else { mark },
                0.0,
            ]
        };
        let base = geo.ribbons.len() as u32;
        if spec.style == "ribbons" {
            for (f, [a, b]) in fr.iter().zip(ribbon(&fr, width)) {
                for q in [a, b] {
                    geo.ribbons.push(FlowVertex {
                        pos: f3(q),
                        color: color(f),
                        lateral: f3(f.n),
                        p: p(f),
                        q: 0.0,
                    });
                }
            }
            for i in 0..fr.len() as u32 - 1 {
                let (a, b, c, d) = (
                    base + 2 * i,
                    base + 2 * i + 1,
                    base + 2 * i + 2,
                    base + 2 * i + 3,
                );
                geo.ribbon_indices.extend([a, b, d, a, d, c]);
            }
        } else {
            let k = 8u32;
            let rings = tube(&fr, u_ref, 0.5 * width, k as usize);
            for (f, ring) in fr.iter().zip(&rings) {
                for (j, (q, n)) in ring.iter().enumerate() {
                    // A dark line along N shows the twist.
                    let c = color(f);
                    let c = if j == 0 { c.map(|x| x * 0.35) } else { c };
                    geo.ribbons.push(FlowVertex {
                        pos: f3(*q),
                        color: c,
                        lateral: f3(*n),
                        p: p(f),
                        q: 0.0,
                    });
                }
            }
            for i in 0..fr.len() as u32 - 1 {
                for j in 0..k {
                    let a = base + i * k + j;
                    let b = base + i * k + (j + 1) % k;
                    let c = a + k;
                    let d = b + k;
                    geo.ribbon_indices.extend([a, b, d, a, d, c]);
                }
            }
        }
    }
    (!geo.ribbons.is_empty()).then_some(geo)
}

/// Streamlines of the view as one line mesh coloured by speed (world coordinates).
pub fn streamline_meshes(project: &Project) -> Vec<Mesh> {
    let Some(spec) = project.manifest.view.particles.clone() else {
        return vec![];
    };
    if spec.style != "streamlines" {
        return vec![];
    }
    let Ok((grids, yaw)) = crate::view::load_grids(project) else {
        return vec![];
    };
    let levels: Vec<VelocityLevel> = grids
        .iter()
        .filter_map(|g| VelocityLevel::from_grid(g).ok())
        .collect();
    let Some(fine) = finest(&grids) else {
        return vec![];
    };
    let length = (0..3)
        .map(|k| fine.spec.max[k] - fine.spec.min[k])
        .fold(0.0, f64::max);
    let (_, umax) = max_of(&grids, "U_mag");
    let dt = (length / umax.max(1e-9) / 400.0) as f32;
    let starts = seeds(project, &spec.region, &grids, yaw, spec.seed);
    let count = spec.count.clamp(1, 400) as usize;
    let to_world = DMat3::from_rotation_z(yaw.to_radians());
    let mut lines = Vec::new();
    let mut colors = Vec::new();
    for s in starts.iter().take(count) {
        let line = streamline(&levels, *s, dt, 2000);
        if line.len() < 2 {
            continue;
        }
        for p in &line {
            let speed = match foamcraft_sim::particles::sample(&levels, *p) {
                foamcraft_sim::particles::Sample::Velocity(u) => {
                    (u[0] * u[0] + u[1] * u[1] + u[2] * u[2]).sqrt()
                }
                _ => 0.0,
            };
            colors.push(colormap(f64::from(speed) / umax.max(1e-9)));
        }
        lines.push(
            line.iter()
                .map(|p| {
                    let w =
                        to_world * DVec3::new(f64::from(p[0]), f64::from(p[1]), f64::from(p[2]));
                    [w.x as f32, w.y as f32, w.z as f32]
                })
                .collect::<Vec<_>>(),
        );
    }
    if lines.is_empty() {
        return vec![];
    }
    let mut mesh = foamcraft_render::scene::polylines(&lines, [1.0; 3], false);
    mesh.vertex_colors = Some(colors);
    vec![mesh]
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ParticleParams {
    /// `wind_tunnel_smoke` or `internal_flow`: start from these settings.
    #[serde(default)]
    pub preset: Option<String>,
    #[serde(default)]
    pub style: Option<String>,
    #[serde(default)]
    pub count: Option<u32>,
    #[serde(default)]
    pub seed: Option<u32>,
    #[serde(default)]
    pub color_by: Option<String>,
    #[serde(default)]
    pub speed: Option<f64>,
    #[serde(default)]
    pub paused: Option<bool>,
    #[serde(default)]
    pub region: Option<SeedRegionSpec>,
    /// Heavy particles (material or own values); null switches back to massless particles.
    #[serde(default, deserialize_with = "present")]
    #[schemars(with = "Option<crate::heavy::InertiaParams>")]
    pub inertia: Option<Option<crate::heavy::InertiaParams>>,
    /// Light trails: length of a trail in flow seconds (limited to 1 to 10 comet tails).
    #[serde(default)]
    pub trail_length: Option<f64>,
    /// Light trails: length in comet tails (1 to 10); sets `trail_length`.
    #[serde(default)]
    pub trail_factor: Option<f64>,
    /// Light trails: width at the head, 1.5 to 4 pixels; ribbons and tubes: width as a share of
    /// the model size (0.002 to 0.05).
    #[serde(default)]
    pub width: Option<f64>,
    /// Light trails: soft glow around bright trails.
    #[serde(default)]
    pub glow: Option<bool>,
    /// Light trails: brightness, 0.1 to 10.
    #[serde(default)]
    pub exposure: Option<f64>,
    /// Light trails in the view: long exposure with this half-life, 0.5 to 8 s; null is off.
    #[serde(default, deserialize_with = "present")]
    #[schemars(with = "Option<f64>")]
    pub long_exposure: Option<Option<f64>>,
}

/// A field that may be given as null: absent → None, null → Some(None).
fn present<'de, D, T>(d: D) -> Result<Option<Option<T>>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: serde::Deserialize<'de>,
{
    Option::<T>::deserialize(d).map(Some)
}

/// Upstream area of the finest grid (wind tunnel smoke), in the world.
fn upstream_area(project: &Project) -> Result<SeedRegionSpec, FoamError> {
    let (grids, yaw) = crate::view::load_grids(project)?;
    let g = finest(&grids).ok_or_else(|| FoamError::new("results.none", ""))?;
    let (lo, hi) = (g.spec.min, g.spec.max);
    let to_world = DMat3::from_rotation_z(yaw.to_radians());
    let w = |v: [f64; 3]| -> [f64; 3] { (to_world * DVec3::from(v)).into() };
    let x = lo[0] + 0.03 * (hi[0] - lo[0]);
    Ok(SeedRegionSpec {
        kind: "area".into(),
        origin: w([
            x,
            lo[1] + 0.1 * (hi[1] - lo[1]),
            lo[2] + 0.02 * (hi[2] - lo[2]),
        ]),
        u: w([0.0, 0.8 * (hi[1] - lo[1]), 0.0]),
        v: w([0.0, 0.0, 0.8 * (hi[2] - lo[2])]),
    })
}

fn set_particles(
    s: &mut Session,
    p: ParticleParams,
) -> Result<foamcraft_core::project::ViewSettings, FoamError> {
    let project = s.project()?;
    let (grids, _) = crate::view::load_grids(project)?;
    let mut spec = match p.preset.as_deref() {
        Some("wind_tunnel_smoke") => ParticleSpec {
            style: "smoke".into(),
            count: 60_000,
            seed: 1,
            color_by: "speed".into(),
            speed: 1.0,
            paused: false,
            region: upstream_area(project)?,
            inertia: None,
            trail_length: None,
            width: None,
            glow: None,
            exposure: None,
            long_exposure: None,
        },
        Some("internal_flow") => ParticleSpec {
            style: "comets".into(),
            count: 50_000,
            seed: 1,
            color_by: "speed".into(),
            speed: 1.0,
            paused: false,
            region: SeedRegionSpec {
                kind: "inlets".into(),
                origin: [0.0; 3],
                u: [0.0; 3],
                v: [0.0; 3],
            },
            inertia: None,
            trail_length: None,
            width: None,
            glow: None,
            exposure: None,
            long_exposure: None,
        },
        Some(other) => return Err(FoamError::new("view.unknown_preset", other.to_string())),
        None => match project.manifest.view.particles.clone() {
            Some(current) => current,
            None => ParticleSpec {
                style: "comets".into(),
                count: 50_000,
                seed: 1,
                color_by: "speed".into(),
                speed: 1.0,
                paused: false,
                region: upstream_area(project)?,
                inertia: None,
                trail_length: None,
                width: None,
                glow: None,
                exposure: None,
                long_exposure: None,
            },
        },
    };
    if let Some(v) = p.style {
        if ![
            "comets",
            "smoke",
            "streamlines",
            "trails",
            "ribbons",
            "tubes",
        ]
        .contains(&v.as_str())
        {
            return Err(FoamError::new(
                "view.invalid_particles",
                format!("style {v}"),
            ));
        }
        // Pixels for trails, a share of the model for ribbons: a width does not carry over.
        if v != spec.style {
            spec.width = None;
        }
        spec.style = v;
    }
    if let Some(v) = p.color_by {
        if !["speed", "pressure", "age", "slip", "swirl"].contains(&v.as_str()) {
            return Err(FoamError::new(
                "view.invalid_particles",
                format!("colour {v}"),
            ));
        }
        spec.color_by = v;
    }
    if let Some(v) = p.count {
        spec.count = v.clamp(100, 2_000_000);
    }
    if let Some(v) = p.seed {
        spec.seed = v;
    }
    if let Some(v) = p.speed {
        if !(v.is_finite() && v > 0.0) {
            return Err(FoamError::new(
                "view.invalid_particles",
                format!("speed {v}"),
            ));
        }
        spec.speed = v;
    }
    if let Some(v) = p.paused {
        spec.paused = v;
    }
    if let Some(r) = p.region {
        if !["area", "inlets", "surface", "rake"].contains(&r.kind.as_str()) {
            return Err(FoamError::new(
                "view.invalid_particles",
                format!("region {}", r.kind),
            ));
        }
        spec.region = r;
    }
    match p.inertia {
        Some(Some(i)) => {
            let job = crate::view::job_of(project)?;
            let medium = crate::heavy::medium_of(project, &job);
            spec.inertia = Some(crate::heavy::resolve(&i, &medium)?);
            // Heavy particles are comets: the tail along their own velocity.
            if spec.style == "streamlines" {
                spec.style = "comets".into();
            }
        }
        Some(None) => {
            spec.inertia = None;
            if spec.color_by == "slip" {
                spec.color_by = "speed".into();
            }
        }
        None => {}
    }
    let bad = |what: &str, v: f64| FoamError::new("view.invalid_particles", format!("{what} {v}"));
    if let Some(f) = p.trail_factor {
        if !(1.0..=10.0).contains(&f) {
            return Err(bad("trail_factor", f));
        }
        spec.trail_length = Some(f * comet_tail(&grids));
    }
    if let Some(v) = p.trail_length {
        if !(v.is_finite() && v > 0.0) {
            return Err(bad("trail_length", v));
        }
        spec.trail_length = Some(v);
    }
    if let Some(v) = p.width {
        let ribbon = ["ribbons", "tubes"].contains(&spec.style.as_str());
        let ok = if ribbon {
            (0.002..=0.05).contains(&v)
        } else {
            (1.5..=4.0).contains(&v)
        };
        if !ok {
            return Err(bad("width", v));
        }
        spec.width = Some(v);
    }
    if let Some(v) = p.glow {
        spec.glow = Some(v);
    }
    if let Some(v) = p.exposure {
        if !(0.1..=10.0).contains(&v) {
            return Err(bad("exposure", v));
        }
        spec.exposure = Some(v);
    }
    match p.long_exposure {
        Some(Some(v)) => {
            if !(0.5..=8.0).contains(&v) {
                return Err(bad("long_exposure", v));
            }
            spec.long_exposure = Some(v);
        }
        Some(None) => spec.long_exposure = None,
        None => {}
    }
    if spec.color_by == "slip" && spec.inertia.is_none() {
        return Err(FoamError::new(
            "view.invalid_particles",
            "colour slip needs heavy particles",
        ));
    }
    project.manifest.view.particles = Some(spec);
    project.save()?;
    let view = project.manifest.view.clone();
    s.events().emit("project.changed", serde_json::Value::Null);
    Ok(view)
}

pub fn register(r: &mut Registry) {
    r.add(
        "view.set_particles",
        "Show animated particles or streamlines: a preset (wind_tunnel_smoke, internal_flow) and/or \
         style, count, seed, colouring, speed, pause and start region.",
        Options {
            undoable: false,
            errors: &[
                "project.none_open",
                "results.none",
                "view.unknown_preset",
                "view.invalid_particles",
                "particles.too_light",
                "particles.invalid_material",
            ],
            example: json!({"preset": "wind_tunnel_smoke"}),
        },
        set_particles,
    );
    r.add(
        "view.particles_off",
        "Hide the particles and streamlines.",
        Options {
            undoable: false,
            errors: &["project.none_open"],
            example: json!({}),
        },
        |s, _: crate::results::JobParams| {
            let project = s.project()?;
            project.manifest.view.particles = None;
            project.save()?;
            let view = project.manifest.view.clone();
            s.events().emit("project.changed", serde_json::Value::Null);
            Ok(view)
        },
    );
}
