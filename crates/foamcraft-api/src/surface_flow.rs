// SPDX-License-Identifier: GPL-3.0-or-later
//! Flow on the model surface (Task_33): `view.set_surface_flow`, `view.surface_flow_off` and
//! `view.surface_flow_info`, and the geometry the app, `view.render` and the export draw: lines
//! that follow the wall shear stress like an oil film (with lights running along them) and wool
//! tufts that lie flat, flutter or hang. Runs without the wall shear stress vector (evaluated
//! before Task_33) use the tangential velocity just off the wall instead ("approximate").

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, OnceLock};

use foamcraft_core::command::{Options, Registry, Session};
use foamcraft_core::error::FoamError;
use foamcraft_core::project::{Project, Scenario, SurfaceFlowSpec, SurfaceLinesSpec, TuftsSpec};
use foamcraft_render::flow::{FlowGeometry, FlowVertex};
use foamcraft_sim::results::{Grid, Surface};
use foamcraft_sim::ride::Probe;
use foamcraft_sim::surface_flow::{
    SurfaceMesh, TangentField, attached, even_lines, flutter_amplitude, flutter_frequency,
    tuft_feet, tuft_rest,
};
use glam::{DMat3, DVec3};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

/// Default spacing of the lines and the tufts, as a share of the model size.
pub const LINE_SPACING: f64 = 1.0 / 60.0;
pub const TUFT_SPACING: f64 = 1.0 / 25.0;
/// Most tufts on a model.
pub const MAX_TUFTS: usize = 4000;
/// Seconds between two lights on a line.
const LIGHT_PERIOD: f64 = 1.5;

/// The near-wall flow of a run on its (welded) wall surface, normals into the fluid.
pub struct FlowData {
    pub mesh: SurfaceMesh,
    /// Direction field per triangle: -wall shear stress, or the tangential velocity off the wall.
    pub dir: Vec<DVec3>,
    pub approximate: bool,
    /// Largest size of the model, m.
    pub size: f64,
}

/// The three components of the wall shear stress per triangle, when the cache has them.
pub fn wss_vectors(s: &Surface) -> Option<Vec<DVec3>> {
    let (x, y, z) = (s.field("wss_x")?, s.field("wss_y")?, s.field("wss_z")?);
    Some(
        (0..x.len())
            .map(|i| DVec3::new(f64::from(x[i]), f64::from(y[i]), f64::from(z[i])))
            .collect(),
    )
}

/// The value of a volume field at a world point, from the finest grid that has it.
fn grid_value(grids: &[Grid], to_flow: DMat3, field: &str, p: DVec3) -> Option<f64> {
    let q = to_flow * p;
    grids.iter().rev().find_map(|g| g.sample(field, q.into()))
}

fn load(project: &Project) -> Result<FlowData, FoamError> {
    let job = crate::view::job_of(project)?;
    let s = crate::results::load_surface(&project.root, &job)?;
    let (grids, yaw) = crate::view::load_grids(project)?;
    let to_flow = DMat3::from_rotation_z(-yaw.to_radians());
    let to_world = to_flow.transpose();
    let flow = foamcraft_sim::ride::Flow::new(grids, None, None, 1.0)?;
    let h = flow.h;
    // Normals into the fluid: OpenFOAM's boundary faces point out of it, an imported model may
    // point either way. The grid decides: on which side of most faces is fluid?
    let fluid = |p: DVec3| matches!(flow.velocity(to_flow * p), Probe::Fluid(_));
    let probe_mesh = SurfaceMesh::new(&s.points, &s.triangles, 1e-12);
    let step = (probe_mesh.tris.len() / 500).max(1);
    let mut votes = 0i64;
    for t in (0..probe_mesh.tris.len()).step_by(step) {
        let (c, n) = (probe_mesh.centroid(t), probe_mesh.normals[t]);
        match (fluid(c + n * 1.5 * h), fluid(c - n * 1.5 * h)) {
            (true, false) => votes += 1,
            (false, true) => votes -= 1,
            _ => {}
        }
    }
    let tris: Vec<[u32; 3]> = if votes < 0 {
        s.triangles.iter().map(|t| [t[0], t[2], t[1]]).collect()
    } else {
        s.triangles.clone()
    };
    let (mut lo, mut hi) = (DVec3::splat(f64::MAX), DVec3::splat(f64::MIN));
    for p in &s.points {
        let v = DVec3::from(p.map(f64::from));
        lo = lo.min(v);
        hi = hi.max(v);
    }
    let size = (hi - lo).max_element().max(1e-9);
    let mesh = SurfaceMesh::new(&s.points, &tris, 1e-6 * size);
    if let Some(w) = wss_vectors(&s) {
        return Ok(FlowData {
            dir: mesh.source.iter().map(|i| -w[*i as usize]).collect(),
            mesh,
            approximate: false,
            size,
        });
    }
    // Old run: the velocity 1.5 grid steps off the wall, without its normal part.
    let dir = (0..mesh.tris.len())
        .map(|t| {
            let n = mesh.normals[t];
            let p = to_flow * (mesh.centroid(t) + n * 1.5 * h);
            match flow.velocity(p) {
                Probe::Fluid(u) => {
                    let u = to_world * u;
                    u - n * u.dot(n)
                }
                _ => DVec3::ZERO,
            }
        })
        .collect();
    Ok(FlowData {
        mesh,
        dir,
        approximate: true,
        size,
    })
}

/// Facts about the surface flow of the view, for the interface.
#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct FlowInfo {
    /// The direction comes from the velocity off the wall (run without wall shear stress).
    pub approximate: bool,
    pub lines: usize,
    pub tufts: usize,
    /// Time to compute lines and tufts, ms.
    pub compute_ms: u64,
}

type CacheKey = (PathBuf, String, String);
type Cache = Mutex<HashMap<CacheKey, Arc<(FlowGeometry, FlowInfo)>>>;

fn cache() -> &'static Cache {
    static C: OnceLock<Cache> = OnceLock::new();
    C.get_or_init(Default::default)
}

fn f3(v: DVec3) -> [f32; 3] {
    [v.x as f32, v.y as f32, v.z as f32]
}

/// 90th percentile of the strength (for scaling speeds and the hanging threshold).
fn p90(values: &[f64]) -> f64 {
    let mut v: Vec<f64> = values.iter().copied().filter(|x| *x > 0.0).collect();
    if v.is_empty() {
        return 1.0;
    }
    v.sort_by(f64::total_cmp);
    v[(v.len() * 9 / 10).min(v.len() - 1)]
}

fn visible(project: &Project, p: DVec3) -> bool {
    match project
        .manifest
        .view
        .clip
        .as_ref()
        .filter(|_| project.manifest.view.clips("model"))
    {
        Some(c) => (p - DVec3::from(c.origin)).dot(DVec3::from(c.normal)) <= 0.0,
        None => true,
    }
}

fn lines_geometry(
    project: &Project,
    data: &FlowData,
    field: &TangentField,
    spec: &SurfaceLinesSpec,
    out: &mut Vec<FlowVertex>,
) -> usize {
    let size = data.size;
    let d_sep = spec.density.clamp(1.0 / 120.0, 1.0 / 30.0) * size;
    let lines = even_lines(&data.mesh, field, d_sep, spec.seed, 3.0 * size);
    let strengths: Vec<f64> = data.dir.iter().map(|v| v.length()).collect();
    let reference = p90(&strengths);
    let max = strengths.iter().copied().fold(1e-30, f64::max);
    let lift = 0.002 * size;
    // At the reference strength a light moves 15 % of the model size per second.
    let pace = 0.15 * size;
    for line in &lines {
        let times = line.runtimes(reference);
        let at = |i: usize| {
            let p = &line.points[i];
            p.pos + data.mesh.normals[p.tri] * lift
        };
        for k in 1..line.points.len() {
            let (a, b) = (at(k - 1), at(k));
            if !visible(project, a) || !visible(project, b) {
                continue;
            }
            for (i, p) in [(k - 1, a), (k, b)] {
                let color = if spec.color_by == "wall_shear_stress" {
                    foamcraft_sim::results::colormap(line.points[i].strength / max)
                } else {
                    [0.88, 0.9, 0.93]
                };
                out.push(FlowVertex {
                    pos: f3(p),
                    color,
                    lateral: [0.0; 3],
                    p: [
                        (times[i] * size / pace) as f32,
                        line.phase as f32,
                        if spec.animate {
                            LIGHT_PERIOD as f32
                        } else {
                            0.0
                        },
                        0.0,
                    ],
                    q: 0.0,
                });
            }
        }
    }
    lines.len()
}

fn tufts_geometry(
    project: &Project,
    data: &FlowData,
    field: &TangentField,
    spec: &TuftsSpec,
    geo: &mut FlowGeometry,
) -> usize {
    let size = data.size;
    let spacing = spec.spacing.clamp(1.0 / 50.0, 1.0 / 12.0) * size;
    let len = 0.6 * spacing;
    let strengths: Vec<f64> = data.dir.iter().map(|v| v.length()).collect();
    let reference = p90(&strengths);
    let wind = !matches!(project.manifest.scenario, Some(Scenario::InternalFlow(_)));
    let grids = crate::view::load_grids(project).ok();
    let oncoming = grids.as_ref().map_or(DVec3::X, |(_, yaw)| {
        DMat3::from_rotation_z(yaw.to_radians()) * DVec3::X
    });
    let attached_map = if wind {
        attached(&data.mesh, &data.dir, oncoming)
    } else {
        vec![1.0; data.mesh.tris.len()]
    };
    let u_ref = grids
        .as_ref()
        .map(|(g, _)| {
            foamcraft_sim::ride::Flow::new(g.clone(), None, None, 1.0).map_or(1.0, |f| f.u_ref)
        })
        .unwrap_or(1.0);
    let feet = tuft_feet(&data.mesh, spacing, spec.seed, MAX_TUFTS);
    let mut shown = 0;
    for (n, (t, foot)) in feet.iter().enumerate() {
        if !visible(project, *foot) {
            continue;
        }
        let rest = tuft_rest(&data.mesh, field, *t, *foot, len, reference, -DVec3::Z);
        // Turbulence and speed 2 grid steps off the wall.
        let (tu, speed) = grids.as_ref().map_or((None, u_ref), |(g, yaw)| {
            let to_flow = DMat3::from_rotation_z(-yaw.to_radians());
            let h = g
                .last()
                .map_or(1e-3, |l| l.spec.step().into_iter().fold(f64::MAX, f64::min));
            let p = *foot + rest.normal * 2.0 * h;
            let speed = ["Ux", "Uy", "Uz"].map(|c| grid_value(g, to_flow, c, p).unwrap_or(0.0));
            let speed = DVec3::from(speed).length();
            let tu = grid_value(g, to_flow, "k", p)
                .map(|k| (2.0 * k.max(0.0) / 3.0).sqrt() / speed.max(0.05 * u_ref));
            (tu, speed)
        });
        let amp = if spec.animate {
            flutter_amplitude(tu, attached_map[*t])
        } else {
            0.0
        };
        let freq = flutter_frequency(speed, len);
        let phase = [
            f64::from(foamcraft_sim::particles::random(n as u32, 0, spec.seed, 41))
                * std::f64::consts::TAU,
            f64::from(foamcraft_sim::particles::random(n as u32, 0, spec.seed, 42))
                * std::f64::consts::TAU,
        ];
        let color = if spec.color_by == "state" && amp >= 20.0 {
            [1.0, 0.55, 0.25]
        } else if rest.hang > 0.5 {
            [0.75, 0.75, 0.72]
        } else {
            [0.97, 0.93, 0.8]
        };
        let half = 0.04 * len;
        let base = geo.tufts.len() as u32;
        for (k, p) in rest.points.iter().enumerate() {
            for side in [-1.0, 1.0] {
                geo.tufts.push(FlowVertex {
                    pos: f3(*p + rest.lateral * (side * half)),
                    color,
                    lateral: f3(rest.lateral),
                    p: [
                        (k as f64 * len / 4.0) as f32,
                        amp.to_radians() as f32,
                        freq as f32,
                        phase[0] as f32,
                    ],
                    q: phase[1] as f32,
                });
            }
        }
        for k in 0..4u32 {
            let (a, b, c, d) = (
                base + 2 * k,
                base + 2 * k + 1,
                base + 2 * k + 2,
                base + 2 * k + 3,
            );
            geo.tuft_indices.extend([a, b, d, a, d, c]);
        }
        shown += 1;
    }
    shown
}

/// The geometry of the view's surface flow (cached per run, settings and clip plane).
pub fn geometry(project: &Project) -> Option<Arc<(FlowGeometry, FlowInfo)>> {
    let spec = project.manifest.view.surface_flow.clone()?;
    if spec.lines.is_none() && spec.tufts.is_none() {
        return None;
    }
    let job = crate::view::job_of(project).ok()?;
    let key = (
        project.root.clone(),
        job,
        serde_json::to_string(&(
            &spec.lines,
            &spec.tufts,
            &project.manifest.view.clip,
            &project.manifest.view.clip_targets,
        ))
        .unwrap_or_default(),
    );
    if let Some(g) = cache().lock().ok().and_then(|m| m.get(&key).cloned()) {
        return Some(g);
    }
    let start = std::time::Instant::now();
    let data = load(project).ok()?;
    let field = TangentField::new(&data.mesh, &data.dir);
    let mut geo = FlowGeometry::default();
    let lines = spec.lines.as_ref().map_or(0, |l| {
        lines_geometry(project, &data, &field, l, &mut geo.lines)
    });
    let tufts = spec
        .tufts
        .as_ref()
        .map_or(0, |t| tufts_geometry(project, &data, &field, t, &mut geo));
    let info = FlowInfo {
        approximate: data.approximate,
        lines,
        tufts,
        compute_ms: start.elapsed().as_millis() as u64,
    };
    let out = Arc::new((geo, info));
    if let Ok(mut m) = cache().lock() {
        m.insert(key, out.clone());
    }
    Some(out)
}

/// Animation time of the surface flow at `seconds` of real time.
pub fn time_at(project: &Project, seconds: f64) -> f32 {
    animation_speed(project).map_or(0.0, |speed| (seconds * speed) as f32)
}

/// Speed of the time-driven animations (surface flow, ribbon marks); none when nothing moves.
pub fn animation_speed(project: &Project) -> Option<f64> {
    let view = &project.manifest.view;
    let flow = view
        .surface_flow
        .as_ref()
        .filter(|s| !s.paused)
        .map(|s| s.speed);
    let ribbons = view
        .particles
        .as_ref()
        .filter(|p| ["ribbons", "tubes"].contains(&p.style.as_str()) && !p.paused)
        .map(|p| p.speed);
    flow.or(ribbons)
}

/// The `attached` map of a surface with the wall shear stress vector: +1 attached along the
/// oncoming flow (`yaw_deg`), -1 reverse flow.
pub fn attached_field(s: &Surface, yaw_deg: f64) -> Option<Vec<f32>> {
    let w = wss_vectors(s)?;
    let oncoming = DMat3::from_rotation_z(yaw_deg.to_radians()) * DVec3::X;
    Some(
        s.triangles
            .iter()
            .zip(&w)
            .map(|(t, v)| {
                let [a, b, c] = t.map(|i| DVec3::from(s.points[i as usize].map(f64::from)));
                let n = (b - a).cross(c - a).normalize_or_zero();
                let d = (-*v - n * (-*v).dot(n)).normalize_or_zero();
                let o = (oncoming - n * oncoming.dot(n)).normalize_or_zero();
                d.dot(o) as f32
            })
            .collect(),
    )
}

#[derive(Debug, Default, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct LinesParams {
    #[serde(default)]
    pub on: Option<bool>,
    /// Spacing as a share of the model size (1/120 to 1/30).
    #[serde(default)]
    pub density: Option<f64>,
    /// `plain` or `wall_shear_stress`.
    #[serde(default)]
    pub color_by: Option<String>,
    #[serde(default)]
    pub animate: Option<bool>,
    #[serde(default)]
    pub seed: Option<u32>,
}

#[derive(Debug, Default, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TuftsParams {
    #[serde(default)]
    pub on: Option<bool>,
    /// Spacing as a share of the model size (1/50 to 1/12).
    #[serde(default)]
    pub spacing: Option<f64>,
    /// `plain` or `state`.
    #[serde(default)]
    pub color_by: Option<String>,
    #[serde(default)]
    pub animate: Option<bool>,
    #[serde(default)]
    pub seed: Option<u32>,
}

#[derive(Debug, Default, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SetParams {
    #[serde(default)]
    pub lines: Option<LinesParams>,
    #[serde(default)]
    pub tufts: Option<TuftsParams>,
    #[serde(default)]
    pub speed: Option<f64>,
    #[serde(default)]
    pub paused: Option<bool>,
}

fn invalid(what: &str) -> FoamError {
    FoamError::new("view.invalid_surface_flow", what)
}

/// Applies the changes of `p` to the current settings.
pub fn merged(old: Option<SurfaceFlowSpec>, p: SetParams) -> Result<SurfaceFlowSpec, FoamError> {
    let old = old.unwrap_or(SurfaceFlowSpec {
        lines: None,
        tufts: None,
        speed: 1.0,
        paused: false,
    });
    let lines = match p.lines {
        None => old.lines,
        Some(l) if l.on == Some(false) => None,
        Some(l) => {
            let base = old.lines.unwrap_or(SurfaceLinesSpec {
                density: LINE_SPACING,
                color_by: "plain".into(),
                animate: true,
                seed: 1,
            });
            let color_by = l.color_by.unwrap_or(base.color_by);
            if !["plain", "wall_shear_stress"].contains(&color_by.as_str()) {
                return Err(invalid(&format!("color_by {color_by}")));
            }
            let density = l.density.unwrap_or(base.density);
            if !density.is_finite() || density <= 0.0 {
                return Err(invalid("density"));
            }
            Some(SurfaceLinesSpec {
                density: density.clamp(1.0 / 120.0, 1.0 / 30.0),
                color_by,
                animate: l.animate.unwrap_or(base.animate),
                seed: l.seed.unwrap_or(base.seed),
            })
        }
    };
    let tufts = match p.tufts {
        None => old.tufts,
        Some(t) if t.on == Some(false) => None,
        Some(t) => {
            let base = old.tufts.unwrap_or(TuftsSpec {
                spacing: TUFT_SPACING,
                color_by: "plain".into(),
                animate: true,
                seed: 1,
            });
            let color_by = t.color_by.unwrap_or(base.color_by);
            if !["plain", "state"].contains(&color_by.as_str()) {
                return Err(invalid(&format!("color_by {color_by}")));
            }
            let spacing = t.spacing.unwrap_or(base.spacing);
            if !spacing.is_finite() || spacing <= 0.0 {
                return Err(invalid("spacing"));
            }
            Some(TuftsSpec {
                spacing: spacing.clamp(1.0 / 50.0, 1.0 / 12.0),
                color_by,
                animate: t.animate.unwrap_or(base.animate),
                seed: t.seed.unwrap_or(base.seed),
            })
        }
    };
    let speed = p.speed.unwrap_or(old.speed);
    if !speed.is_finite() || speed <= 0.0 {
        return Err(invalid("speed"));
    }
    Ok(SurfaceFlowSpec {
        lines,
        tufts,
        speed: speed.clamp(0.1, 5.0),
        paused: p.paused.unwrap_or(old.paused),
    })
}

fn info_of(project: &Project) -> Value {
    let geo = geometry(project);
    json!({
        "surface_flow": project.manifest.view.surface_flow,
        "approximate": geo.as_ref().is_some_and(|g| g.1.approximate),
        "lines": geo.as_ref().map_or(0, |g| g.1.lines),
        "tufts": geo.as_ref().map_or(0, |g| g.1.tufts),
        "compute_ms": geo.as_ref().map_or(0, |g| g.1.compute_ms),
    })
}

fn set(s: &mut Session, p: SetParams) -> Result<Value, FoamError> {
    let project = s.project()?;
    crate::view::job_of(project)?;
    let spec = merged(project.manifest.view.surface_flow.clone(), p)?;
    project.manifest.view.surface_flow =
        (spec.lines.is_some() || spec.tufts.is_some()).then_some(spec);
    project.save()?;
    let out = info_of(project);
    s.events().emit("project.changed", Value::Null);
    Ok(out)
}

#[derive(Debug, Default, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct NoParams {}

pub fn register(r: &mut Registry) {
    r.add(
        "view.set_surface_flow",
        "Show the flow on the model surface: lines like an oil film (spacing, colour plain or by \
         wall shear stress, running lights) and wool tufts (spacing, colour plain or by state, \
         flutter). Parts left out keep their settings; on: false hides a part.",
        Options {
            undoable: false,
            errors: &[
                "project.none_open",
                "results.none",
                "view.invalid_surface_flow",
            ],
            example: json!({"lines": {"on": true, "density": 0.0167}, "tufts": {"on": false}}),
        },
        set,
    );
    r.add(
        "view.surface_flow_off",
        "Hide the lines and tufts on the model surface.",
        Options {
            undoable: false,
            errors: &["project.none_open"],
            example: json!({}),
        },
        |s: &mut Session, _: NoParams| {
            let project = s.project()?;
            project.manifest.view.surface_flow = None;
            project.save()?;
            s.events().emit("project.changed", Value::Null);
            Ok(json!({"surface_flow": null}))
        },
    );
    r.add(
        "view.surface_flow_info",
        "Settings of the surface flow and facts about it: approximate (run without wall shear \
         stress vector), number of lines and tufts, compute time.",
        Options {
            undoable: false,
            errors: &["project.none_open"],
            example: json!({}),
        },
        |s: &mut Session, _: NoParams| Ok(info_of(s.project()?)),
    );
}
