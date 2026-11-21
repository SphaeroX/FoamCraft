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

/// Everything the app needs to animate the particles.
pub struct ParticleSetup {
    pub levels: Vec<FlowLevel>,
    pub seeds: Vec<[f32; 3]>,
    pub settings: ParticleSettings,
    /// Flow time per animation frame, s.
    pub dt: f32,
    pub paused: bool,
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
fn seeds(
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
        _ => (ParticleColor::Speed, (0.0, umax)),
    };
    let smoke = spec.style == "smoke";
    Some(ParticleSetup {
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
        dt: (time_scale * spec.speed / FPS) as f32,
        paused: spec.paused,
    })
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
    crate::view::load_grids(project)?;
    let mut spec = match p.preset.as_deref() {
        Some("wind_tunnel_smoke") => ParticleSpec {
            style: "smoke".into(),
            count: 60_000,
            seed: 1,
            color_by: "speed".into(),
            speed: 1.0,
            paused: false,
            region: upstream_area(project)?,
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
            },
        },
    };
    if let Some(v) = p.style {
        if !["comets", "smoke", "streamlines"].contains(&v.as_str()) {
            return Err(FoamError::new(
                "view.invalid_particles",
                format!("style {v}"),
            ));
        }
        spec.style = v;
    }
    if let Some(v) = p.color_by {
        if !["speed", "pressure", "age"].contains(&v.as_str()) {
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
        if !["area", "inlets", "surface"].contains(&r.kind.as_str()) {
            return Err(FoamError::new(
                "view.invalid_particles",
                format!("region {}", r.kind),
            ));
        }
        spec.region = r;
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
