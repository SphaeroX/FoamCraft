// SPDX-License-Identifier: GPL-3.0-or-later
//! Heavy particles (Task_34): materials (`particles.materials`), the inertia settings of the view
//! particles, the distance field for the GPU, and the impact map (`results.impact`,
//! `results.impact_status`, `results.impact_cancel`): many particles followed exactly on the CPU
//! until they hit the model, counted per triangle and kept next to the result cache.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, OnceLock};

use foamcraft_core::command::{EventBus, Options, Registry, Session};
use foamcraft_core::error::FoamError;
use foamcraft_core::project::{InertiaSpec, MediumSpec, Project, Scenario};
use foamcraft_sim::heavy::{Impact, Medium, Particle, impact_counts, smooth, tau_p};
use foamcraft_sim::results::{Grid, Surface};
use foamcraft_sim::ride::Probe;
use glam::{DMat3, DVec3};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

/// A material preset.
#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct Material {
    pub id: &'static str,
    pub diameter_m: f64,
    pub density_kg_m3: f64,
    /// `stick` or `bounce`.
    pub on_impact: &'static str,
    pub restitution: f64,
}

pub const MATERIALS: [Material; 5] = [
    Material {
        id: "fine_dust",
        diameter_m: 10e-6,
        density_kg_m3: 1500.0,
        on_impact: "stick",
        restitution: 0.3,
    },
    Material {
        id: "spray",
        diameter_m: 50e-6,
        density_kg_m3: 1000.0,
        on_impact: "stick",
        restitution: 0.3,
    },
    Material {
        id: "rain",
        diameter_m: 1e-3,
        density_kg_m3: 1000.0,
        on_impact: "stick",
        restitution: 0.3,
    },
    Material {
        id: "sand",
        diameter_m: 0.3e-3,
        density_kg_m3: 2650.0,
        on_impact: "bounce",
        restitution: 0.5,
    },
    Material {
        id: "snow",
        diameter_m: 3e-3,
        density_kg_m3: 100.0,
        on_impact: "stick",
        restitution: 0.3,
    },
];

/// Wall friction of a bounce.
pub const WALL_FRICTION: f64 = 0.2;

/// Density and viscosity of the medium of a run (from its result file, as the run saw it).
pub fn medium_of(project: &Project, job: &str) -> Medium {
    let r = crate::solve::load_result(&project.root.join(crate::solve::RESULTS_DIR), Some(job))
        .unwrap_or(Value::Null);
    let spec: Option<MediumSpec> = serde_json::from_value(r["scenario"]["medium"].clone())
        .ok()
        .or_else(|| match &project.manifest.scenario {
            Some(Scenario::WindTunnel(w)) => Some(w.medium.clone()),
            Some(Scenario::InternalFlow(f)) => Some(f.medium.clone()),
            None => None,
        });
    let props = spec.map(|m| foamcraft_sim::medium_properties(&m));
    Medium {
        density: r["reference"]["density_kg_m3"]
            .as_f64()
            .or(props.as_ref().map(|p| p.density))
            .unwrap_or(1.2),
        kinematic_viscosity: props.map_or(1.5e-5, |p| p.kinematic_viscosity),
    }
}

pub fn particle_of(spec: &InertiaSpec) -> Particle {
    Particle {
        diameter: spec.diameter_m,
        density: spec.density_kg_m3,
        gravity: spec.gravity,
        impact: if spec.on_impact == "bounce" {
            Impact::Bounce {
                restitution: spec.restitution,
                friction: spec.wall_friction,
            }
        } else {
            Impact::Stick
        },
    }
}

/// Inertia settings as `view.set_particles` and `results.impact` take them.
#[derive(Debug, Clone, Default, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct InertiaParams {
    /// A material preset; own values override it.
    #[serde(default)]
    pub material: Option<String>,
    #[serde(default)]
    pub diameter_m: Option<f64>,
    #[serde(default)]
    pub density_kg_m3: Option<f64>,
    /// Default on.
    #[serde(default)]
    pub gravity: Option<bool>,
    /// `stick` or `bounce`.
    #[serde(default)]
    pub on_impact: Option<String>,
    #[serde(default)]
    pub restitution: Option<f64>,
    #[serde(default)]
    pub wall_friction: Option<f64>,
}

/// The inertia settings from parameters, checked against the medium.
pub fn resolve(p: &InertiaParams, medium: &Medium) -> Result<InertiaSpec, FoamError> {
    let preset = match &p.material {
        Some(id) => Some(
            MATERIALS
                .iter()
                .find(|m| m.id == id)
                .ok_or_else(|| FoamError::new("particles.invalid_material", id.clone()))?,
        ),
        None => None,
    };
    let diameter = p
        .diameter_m
        .or(preset.map(|m| m.diameter_m))
        .unwrap_or(1e-4);
    let density = p
        .density_kg_m3
        .or(preset.map(|m| m.density_kg_m3))
        .unwrap_or(1000.0);
    if !(diameter.is_finite() && diameter > 0.0 && diameter < 0.1) {
        return Err(FoamError::new(
            "particles.invalid_material",
            format!("diameter {diameter} m"),
        ));
    }
    if !density.is_finite() || density <= medium.density {
        return Err(FoamError::new(
            "particles.too_light",
            format!("{density} kg/m³ vs {} kg/m³", medium.density),
        ));
    }
    let on_impact = p
        .on_impact
        .clone()
        .or(preset.map(|m| m.on_impact.to_string()))
        .unwrap_or_else(|| "stick".into());
    if !["stick", "bounce"].contains(&on_impact.as_str()) {
        return Err(FoamError::new(
            "particles.invalid_material",
            format!("on_impact {on_impact}"),
        ));
    }
    let restitution = p
        .restitution
        .or(preset.map(|m| m.restitution))
        .unwrap_or(0.3)
        .clamp(0.0, 1.0);
    // Own values that differ from the preset are no longer that material.
    let material = preset
        .filter(|m| {
            (m.diameter_m - diameter).abs() <= 1e-12 * m.diameter_m
                && (m.density_kg_m3 - density).abs() <= 1e-9 * m.density_kg_m3
        })
        .map(|m| m.id.to_string());
    Ok(InertiaSpec {
        material,
        diameter_m: diameter,
        density_kg_m3: density,
        gravity: p.gravity.unwrap_or(true),
        on_impact,
        restitution,
        wall_friction: p.wall_friction.unwrap_or(WALL_FRICTION).clamp(0.0, 1.0),
    })
}

/// Stokes number and its sentence code for a particle in the shown run.
pub fn stokes_of(project: &Project, spec: &InertiaSpec) -> Option<(f64, &'static str)> {
    let rf = crate::ride::load_flow(project).ok()?;
    let job = crate::view::job_of(project).ok()?;
    let m = medium_of(project, &job);
    let st = foamcraft_sim::heavy::stokes(&particle_of(spec), &m, rf.flow.u_ref, rf.flow.length);
    let code = if st < 0.05 {
        "particles.stokes.low"
    } else if st <= 2.0 {
        "particles.stokes.middle"
    } else {
        "particles.stokes.high"
    };
    Some((st, code))
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct MaterialInfo {
    #[serde(flatten)]
    pub material: Material,
    /// Stokes number in the shown run (none without a result).
    pub stokes: Option<f64>,
    /// Catalog code of the sentence about the Stokes number.
    pub sentence: Option<String>,
    /// The particle is less than five times denser than the medium: a rough approximation.
    pub rough: bool,
}

fn materials(project: Option<&Project>, custom: Option<&InertiaParams>) -> Vec<MaterialInfo> {
    let medium = project
        .and_then(|p| crate::view::job_of(p).ok().map(|j| medium_of(p, &j)))
        .unwrap_or(Medium {
            density: 1.2,
            kinematic_viscosity: 1.5e-5,
        });
    let mut list: Vec<Material> = MATERIALS.to_vec();
    // Own values: a further entry `custom` with its Stokes number.
    if let Some(c) = custom {
        list.push(Material {
            id: "custom",
            diameter_m: c.diameter_m.unwrap_or(1e-4),
            density_kg_m3: c.density_kg_m3.unwrap_or(1000.0),
            on_impact: "stick",
            restitution: 0.3,
        });
    }
    list.iter()
        .map(|m| {
            let spec = InertiaSpec {
                material: Some(m.id.into()),
                diameter_m: m.diameter_m,
                density_kg_m3: m.density_kg_m3,
                gravity: true,
                on_impact: m.on_impact.into(),
                restitution: m.restitution,
                wall_friction: WALL_FRICTION,
            };
            let st = project.and_then(|p| stokes_of(p, &spec));
            MaterialInfo {
                material: m.clone(),
                stokes: st.map(|s| s.0),
                sentence: st.map(|s| s.1.to_string()),
                rough: m.density_kg_m3 < 5.0 * medium.density,
            }
        })
        .collect()
}

// --- Distance field for the GPU ---------------------------------------------------------------

type DistanceCache = Mutex<HashMap<(PathBuf, String), Arc<Vec<f32>>>>;

fn distance_cache() -> &'static DistanceCache {
    static C: OnceLock<DistanceCache> = OnceLock::new();
    C.get_or_init(Default::default)
}

/// Distance to the model surface at every point of every grid (coarse to fine, flow frame),
/// negative inside solids. Only the finest grid gets real distances near the wall; elsewhere the
/// sign from the mask is enough (the finest grid holds the model).
pub fn distance_field(project: &Project, grids: &[Grid], yaw_deg: f64) -> Arc<Vec<f32>> {
    let job = crate::view::job_of(project).unwrap_or_default();
    let key = (project.root.clone(), job.clone());
    if let Some(d) = distance_cache()
        .lock()
        .ok()
        .and_then(|m| m.get(&key).cloned())
    {
        return d;
    }
    let to_flow = DMat3::from_rotation_z(-yaw_deg.to_radians());
    let wall = crate::results::load_surface(&project.root, &job)
        .ok()
        .map(|s| {
            let pts: Vec<[f32; 3]> = s
                .points
                .iter()
                .map(|p| {
                    let v = to_flow * DVec3::from(p.map(f64::from));
                    [v.x as f32, v.y as f32, v.z as f32]
                })
                .collect();
            let h = grids
                .last()
                .map_or(1e-3, |g| g.spec.step().into_iter().fold(f64::MAX, f64::min));
            (
                foamcraft_sim::distance::WallDistance::new(&pts, &s.triangles, 2.0 * h, 3.0 * h),
                h,
            )
        });
    let mut out = Vec::new();
    let last = grids.len().saturating_sub(1);
    for (n, g) in grids.iter().enumerate() {
        let step = g.spec.step().into_iter().fold(f64::MAX, f64::min);
        let len = g.spec.len();
        let mut level = vec![0.0f32; len];
        let d = g.spec.dims;
        let fill = |range: std::ops::Range<usize>, out: &mut [f32]| {
            for (slot, i) in out.iter_mut().zip(range) {
                let sign = if g.mask[i] { -1.0 } else { 1.0 };
                let k = (i / (d[0] * d[1]) as usize) as u32;
                let j = ((i / d[0] as usize) % d[1] as usize) as u32;
                let ii = (i % d[0] as usize) as u32;
                let magnitude = match (&wall, n == last) {
                    (Some((w, _)), true) => w.distance(g.spec.point(ii, j, k)),
                    _ => step,
                };
                *slot = (sign * magnitude.max(1e-9)) as f32;
            }
        };
        let threads = std::thread::available_parallelism().map_or(4, |n| n.get());
        let chunk = len.div_ceil(threads).max(1);
        std::thread::scope(|scope| {
            for (c, part) in level.chunks_mut(chunk).enumerate() {
                let start = c * chunk;
                let end = start + part.len();
                scope.spawn(move || fill(start..end, part));
            }
        });
        out.extend(level);
    }
    let out = Arc::new(out);
    if let Ok(mut m) = distance_cache().lock() {
        m.insert(key, out.clone());
    }
    out
}

// --- Impact map --------------------------------------------------------------------------------

#[derive(Debug, Default, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ImpactParams {
    /// Material or own values; default the inertia of the view particles, else rain.
    #[serde(default)]
    pub inertia: Option<InertiaParams>,
    /// Particles to release, default 200 000.
    #[serde(default)]
    pub count: Option<u32>,
    #[serde(default)]
    pub seed: Option<u32>,
    /// Block until the map is ready.
    #[serde(default)]
    pub wait: bool,
}

/// The stored impact map of a run.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ImpactMap {
    pub key: String,
    pub inertia: InertiaSpec,
    pub count: u32,
    pub seed: u32,
    /// Impact rate per triangle (wind tunnel: β, 1 = as much as in the free stream; internal:
    /// hits per area over the mean).
    pub impact: Vec<f32>,
    /// Mean speed into the wall at impact per triangle, m/s.
    pub impact_speed: Vec<f32>,
    pub hits: u64,
    pub released: u64,
    /// Share that hits, %: of the particles flying at the frontal area (wind tunnel) or of the
    /// particles that entered (internal flow).
    pub share_percent: f64,
    /// Statistical uncertainty of the share, % of the share (1 / sqrt(hits)).
    pub uncertainty_percent: f64,
    pub compute_s: f64,
}

#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct ImpactStatus {
    pub impact_id: String,
    /// `running`, `done`, `failed` or `cancelled`.
    pub state: String,
    pub done: u64,
    pub total: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub map: Option<ImpactMap>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

struct Job {
    cancel: AtomicBool,
    progress: AtomicU64,
    status: Mutex<ImpactStatus>,
}

fn jobs() -> &'static Mutex<HashMap<String, Arc<Job>>> {
    static J: OnceLock<Mutex<HashMap<String, Arc<Job>>>> = OnceLock::new();
    J.get_or_init(Default::default)
}

fn map_path(root: &Path, job: &str, key: &str) -> PathBuf {
    crate::results::cache_dir(root, job).join(format!("impact-{key}.json"))
}

fn latest_path(root: &Path, job: &str) -> PathBuf {
    crate::results::cache_dir(root, job).join("impact-latest.json")
}

/// The impact map shown with the run (the latest computed), if any.
pub fn latest_map(root: &Path, job: &str) -> Option<ImpactMap> {
    let key = std::fs::read_to_string(latest_path(root, job)).ok()?;
    let text = std::fs::read_to_string(map_path(root, job, key.trim())).ok()?;
    serde_json::from_str(&text).ok()
}

fn key_of(spec: &InertiaSpec, count: u32, seed: u32) -> String {
    let text = serde_json::to_string(&(spec, count, seed)).unwrap_or_default();
    // FNV-1a: a short, stable name.
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in text.bytes() {
        h ^= u64::from(b);
        h = h.wrapping_mul(0x0100_0000_01b3);
    }
    format!("{h:016x}")
}

/// Everything the background computation needs, taken from the project at the start.
struct Inputs {
    root: PathBuf,
    job: String,
    surface: Surface,
    grids: Vec<Grid>,
    yaw: f64,
    internal: bool,
    inlets: Vec<DVec3>,
    spec: InertiaSpec,
    medium: Medium,
    count: u32,
    seed: u32,
    key: String,
}

fn release_points(
    inp: &Inputs,
    flow: &foamcraft_sim::ride::Flow,
    p: &Particle,
) -> (Vec<DVec3>, f64, f64) {
    let n = inp.count.max(1);
    if inp.internal {
        // On the inlets, weighted by the local speed (rejection with a fixed sequence).
        let speeds: Vec<f64> = inp
            .inlets
            .iter()
            .map(|q| match flow.velocity(*q) {
                Probe::Fluid(u) => u.length(),
                _ => 0.0,
            })
            .collect();
        let max = speeds.iter().copied().fold(1e-12, f64::max);
        let mut out = Vec::with_capacity(n as usize);
        let mut i = 0u32;
        while out.len() < n as usize && i < 50 * n && !inp.inlets.is_empty() {
            let k = (foamcraft_sim::particles::hash(i ^ inp.seed) as usize) % inp.inlets.len();
            if f64::from(foamcraft_sim::particles::random(i, 0, inp.seed, 51)) * max <= speeds[k] {
                out.push(inp.inlets[k]);
            }
            i += 1;
        }
        return (out, 0.0, 0.0);
    }
    // Wind tunnel: a rectangle one model length upstream, the model's cross-section plus a margin
    // of 0.3 L, extended upwards by the settling on the way.
    let (lo, hi, l) = (flow.model_min, flow.model_max, flow.length);
    let m = Medium {
        density: inp.medium.density,
        kinematic_viscosity: inp.medium.kinematic_viscosity,
    };
    let settle = foamcraft_sim::heavy::terminal_velocity(p, &m).z.abs();
    let x = lo.x - l;
    let drop = settle / flow.u_ref.max(1e-9) * (lo.x - x + (hi.x - lo.x));
    let (y0, y1) = (lo.y - 0.3 * l, hi.y + 0.3 * l);
    let (z0, z1) = (lo.z - 0.3 * l, hi.z + 0.3 * l + drop);
    let cols = ((n as f64).sqrt() * ((y1 - y0) / (z1 - z0)).sqrt())
        .ceil()
        .max(1.0) as u32;
    let rows = n.div_ceil(cols);
    let mut out = Vec::with_capacity(n as usize);
    for r in 0..rows {
        for c in 0..cols {
            let i = r * cols + c;
            if i >= n {
                break;
            }
            let a = (f64::from(c)
                + f64::from(foamcraft_sim::particles::random(i, 0, inp.seed, 52)))
                / f64::from(cols);
            let b = (f64::from(r)
                + f64::from(foamcraft_sim::particles::random(i, 0, inp.seed, 53)))
                / f64::from(rows);
            out.push(DVec3::new(x, y0 + a * (y1 - y0), z0 + b * (z1 - z0)));
        }
    }
    (out, (y1 - y0) * (z1 - z0), 0.0)
}

fn compute(inp: &Inputs, job: &Job) -> Result<ImpactMap, FoamError> {
    let start = std::time::Instant::now();
    let to_flow = DMat3::from_rotation_z(-inp.yaw.to_radians());
    let pts: Vec<[f32; 3]> = inp
        .surface
        .points
        .iter()
        .map(|p| {
            let v = to_flow * DVec3::from(p.map(f64::from));
            [v.x as f32, v.y as f32, v.z as f32]
        })
        .collect();
    let surface = Surface {
        points: pts,
        triangles: inp.surface.triangles.clone(),
        fields: vec![],
    };
    let flow = foamcraft_sim::ride::Flow::new(inp.grids.clone(), Some(&surface), None, 1.0)?;
    let h = flow.h;
    let wall = foamcraft_sim::distance::WallDistance::new(
        &surface.points,
        &surface.triangles,
        2.0 * h,
        8.0 * h,
    );
    let p = particle_of(&inp.spec);
    let (starts, release_area, _) = release_points(inp, &flow, &p);
    let total = starts.len() as u64;
    if let Ok(mut s) = job.status.lock() {
        s.total = total;
    }
    let u_at = |x: DVec3| match flow.velocity(x) {
        Probe::Fluid(u) => Some(u),
        Probe::Blocked => Some(DVec3::ZERO),
        Probe::Outside => None,
    };
    let m = inp.medium;
    let settle = foamcraft_sim::heavy::terminal_velocity(&p, &m);
    let v0 = |x: DVec3| u_at(x).unwrap_or(DVec3::X * flow.u_ref) + settle;
    let max_time = 20.0 * flow.length / flow.u_ref.max(1e-9);
    let threads = std::thread::available_parallelism().map_or(4, |n| n.get());
    let counts = impact_counts(
        &starts,
        &v0,
        &u_at,
        &wall,
        &p,
        &m,
        h,
        max_time,
        threads,
        &job.cancel,
        &job.progress,
    )
    .ok_or_else(|| FoamError::new("impact.cancelled", ""))?;
    // Per triangle: areas, smoothed counts, rates.
    let areas: Vec<f64> = surface
        .triangles
        .iter()
        .map(|t| {
            let [a, b, c] = t.map(|i| DVec3::from(surface.points[i as usize].map(f64::from)));
            0.5 * (b - a).cross(c - a).length()
        })
        .collect();
    let raw: Vec<f64> = counts.hits.iter().map(|h| f64::from(*h)).collect();
    let smoothed = smooth(&surface.triangles, &areas, surface.points.len(), &raw);
    let total_area: f64 = areas.iter().sum();
    let impact: Vec<f32> = if inp.internal {
        let mean = counts.total as f64 / total_area.max(1e-300);
        smoothed
            .iter()
            .zip(&areas)
            .map(|(h, a)| (h / a.max(1e-300) / mean.max(1e-300)) as f32)
            .collect()
    } else {
        let flux = total as f64 / release_area.max(1e-300);
        smoothed
            .iter()
            .zip(&areas)
            .map(|(h, a)| (h / a.max(1e-300) / flux) as f32)
            .collect()
    };
    let impact_speed: Vec<f32> = counts
        .speed_sum
        .iter()
        .zip(&counts.hits)
        .map(|(s, h)| {
            if *h > 0 {
                (s / f64::from(*h)) as f32
            } else {
                0.0
            }
        })
        .collect();
    let share = if inp.internal {
        counts.total as f64 / total.max(1) as f64
    } else {
        // Of the particles flying at the frontal area.
        let mesh = foamcraft_geometry::Mesh {
            positions: surface.points.clone(),
            triangles: surface.triangles.clone(),
            format: foamcraft_geometry::Format::Stl,
            declared_unit: None,
            face_tags: None,
            part_tags: None,
        };
        // Seen along the flight of a free particle (with the flow, sinking): rain also falls on
        // the top, so the area it flies at is more than the frontal area across the flow.
        let flight = (DVec3::X * flow.u_ref + settle).normalize_or(DVec3::X);
        let frontal = foamcraft_sim::frontal_area(&[&mesh], flight.into());
        let aimed = total as f64 * frontal / (release_area * flight.x.abs().max(1e-6)).max(1e-300);
        counts.total as f64 / aimed.max(1e-300)
    };
    Ok(ImpactMap {
        key: inp.key.clone(),
        inertia: inp.spec.clone(),
        count: inp.count,
        seed: inp.seed,
        impact,
        impact_speed,
        hits: counts.total,
        released: total,
        share_percent: 100.0 * share,
        uncertainty_percent: if counts.total > 0 {
            100.0 / (counts.total as f64).sqrt()
        } else {
            100.0
        },
        compute_s: start.elapsed().as_secs_f64(),
    })
}

fn status(id: &str) -> Result<ImpactStatus, FoamError> {
    let j = jobs()
        .lock()
        .ok()
        .and_then(|m| m.get(id).cloned())
        .ok_or_else(|| FoamError::new("impact.not_found", id))?;
    let mut s = j
        .status
        .lock()
        .map_or_else(|p| p.into_inner().clone(), |s| s.clone());
    s.done = j.progress.load(Ordering::SeqCst);
    Ok(s)
}

fn start(project: &Project, events: &EventBus, p: ImpactParams) -> Result<Value, FoamError> {
    let job_id = crate::view::job_of(project)?;
    let medium = medium_of(project, &job_id);
    let inertia = match &p.inertia {
        Some(i) => resolve(i, &medium)?,
        None => match project
            .manifest
            .view
            .particles
            .as_ref()
            .and_then(|s| s.inertia.clone())
        {
            Some(i) => i,
            None => resolve(
                &InertiaParams {
                    material: Some("rain".into()),
                    ..Default::default()
                },
                &medium,
            )?,
        },
    };
    let count = p.count.unwrap_or(200_000).clamp(100, 2_000_000);
    let seed = p.seed.unwrap_or(1);
    let key = key_of(&inertia, count, seed);
    static NEXT: AtomicU32 = AtomicU32::new(1);
    let id = format!(
        "impact-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::SeqCst)
    );
    let job = Arc::new(Job {
        cancel: AtomicBool::new(false),
        progress: AtomicU64::new(0),
        status: Mutex::new(ImpactStatus {
            impact_id: id.clone(),
            state: "running".into(),
            done: 0,
            total: u64::from(count),
            map: None,
            error: None,
        }),
    });
    if let Ok(mut m) = jobs().lock() {
        m.insert(id.clone(), job.clone());
    }
    // Same inputs: the map comes from the cache.
    if let Ok(text) = std::fs::read_to_string(map_path(&project.root, &job_id, &key))
        && let Ok(map) = serde_json::from_str::<ImpactMap>(&text)
    {
        let _ = std::fs::write(latest_path(&project.root, &job_id), &key);
        if let Ok(mut s) = job.status.lock() {
            s.state = "done".into();
            s.done = map.released;
            s.total = map.released;
            s.map = Some(map);
        }
        job.progress.store(u64::from(count), Ordering::SeqCst);
        events.emit("project.changed", Value::Null);
        return Ok(json!({"impact_id": id, "cached": true, "status": status(&id)?}));
    }
    let (grids, yaw) = crate::view::load_grids(project)?;
    let surface = crate::results::load_surface(&project.root, &job_id)?;
    let internal = matches!(project.manifest.scenario, Some(Scenario::InternalFlow(_)));
    let inlets = if internal {
        let region = foamcraft_core::project::SeedRegionSpec {
            kind: "inlets".into(),
            origin: [0.0; 3],
            u: [0.0; 3],
            v: [0.0; 3],
        };
        crate::particles::seeds(project, &region, &grids, yaw, seed)
            .into_iter()
            .map(|q| DVec3::from(q.map(f64::from)))
            .collect()
    } else {
        Vec::new()
    };
    let inputs = Inputs {
        root: project.root.clone(),
        job: job_id,
        surface,
        grids,
        yaw,
        internal,
        inlets,
        spec: inertia,
        medium,
        count,
        seed,
        key,
    };
    let events = events.clone();
    let run_id = id.clone();
    let work = move || {
        let result = compute(&inputs, &job);
        let mut s = job.status.lock().unwrap_or_else(|e| e.into_inner());
        match result {
            Ok(map) => {
                let path = map_path(&inputs.root, &inputs.job, &inputs.key);
                let written = serde_json::to_string(&map)
                    .map_err(|e| e.to_string())
                    .and_then(|t| std::fs::write(&path, t).map_err(|e| e.to_string()))
                    .and_then(|()| {
                        std::fs::write(latest_path(&inputs.root, &inputs.job), &inputs.key)
                            .map_err(|e| e.to_string())
                    });
                match written {
                    Ok(()) => {
                        s.state = "done".into();
                        s.map = Some(map);
                    }
                    Err(e) => {
                        s.state = "failed".into();
                        s.error = Some(e);
                    }
                }
            }
            Err(e) if e.code == "impact.cancelled" => s.state = "cancelled".into(),
            Err(e) => {
                s.state = "failed".into();
                s.error = Some(e.code.clone());
            }
        }
        let state = s.state.clone();
        drop(s);
        events.emit(
            "impact.finished",
            json!({"impact_id": run_id, "state": state}),
        );
        if state == "done" {
            events.emit("project.changed", Value::Null);
        }
    };
    if p.wait {
        work();
        return Ok(json!({"impact_id": id, "cached": false, "status": status(&id)?}));
    }
    std::thread::spawn(work);
    Ok(json!({"impact_id": id, "cached": false}))
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct IdParams {
    pub impact_id: String,
}

#[derive(Debug, Default, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct MaterialsParams {
    /// Own values (diameter_m, density_kg_m3): listed as `custom` with their Stokes number.
    #[serde(default)]
    pub custom: Option<InertiaParams>,
}

pub fn register(r: &mut Registry) {
    r.add(
        "particles.materials",
        "Material presets of heavy particles (fine dust, spray, rain, sand, snow) with diameter, \
         density, impact behaviour and the Stokes number in the shown run with its sentence code.",
        Options {
            undoable: false,
            errors: &[],
            example: json!({}),
        },
        |s: &mut Session, p: MaterialsParams| {
            Ok(materials(s.project().ok().map(|p| &*p), p.custom.as_ref()))
        },
    );
    r.add(
        "results.impact",
        "Compute the impact map of heavy particles on the model in the background: impact rate \
         and speed per face, the share that hits and its uncertainty. Answers at once with \
         impact_id unless wait is true; the same inputs come from the cache.",
        Options {
            undoable: false,
            errors: &[
                "project.none_open",
                "results.none",
                "particles.too_light",
                "particles.invalid_material",
            ],
            example: json!({"inertia": {"material": "rain"}, "count": 200000}),
        },
        |s: &mut Session, p: ImpactParams| {
            let events = s.events().clone();
            start(s.project()?, &events, p)
        },
    );
    r.add(
        "results.impact_status",
        "Progress of an impact map: state, particles done and the map with its figures when done.",
        Options {
            undoable: false,
            errors: &["impact.not_found"],
            example: json!({"impact_id": "impact-1-1"}),
        },
        |_, p: IdParams| status(&p.impact_id),
    );
    r.add(
        "results.impact_cancel",
        "Cancel an impact map; nothing is written.",
        Options {
            undoable: false,
            errors: &["impact.not_found"],
            example: json!({"impact_id": "impact-1-1"}),
        },
        |_, p: IdParams| {
            let j = jobs()
                .lock()
                .ok()
                .and_then(|m| m.get(&p.impact_id).cloned())
                .ok_or_else(|| FoamError::new("impact.not_found", p.impact_id.clone()))?;
            j.cancel.store(true, Ordering::SeqCst);
            Ok(json!({"impact_id": p.impact_id}))
        },
    );
}

/// `tau_p` of the view particles' inertia (for the time scales of the animation).
pub fn tau_of(spec: &InertiaSpec, m: &Medium) -> f64 {
    tau_p(&particle_of(spec), m)
}
