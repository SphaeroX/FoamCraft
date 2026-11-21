// SPDX-License-Identifier: GPL-3.0-or-later
//! The ride along one particle (Task_32): `ride.suggest` (up to five worthwhile paths of the
//! shown result), `ride.start` (from a suggestion or a world point), `ride.telemetry` (readings
//! and events of the stored ride) and `ride.stop`. The view stores only the start point
//! (`view.ride`); the path, its screen timing and the camera track follow from it the same way
//! every time, so the app, the image and the video export show exactly the same ride.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

use foamcraft_core::command::{Options, Registry, Session};
use foamcraft_core::error::FoamError;
use foamcraft_core::project::{
    InertiaSpec, MediumSpec, Project, RideSpec, Scenario, SeedRegionSpec,
};
use foamcraft_render::camera::FreeCamera;
use foamcraft_render::glam::{Vec3, vec3};
use foamcraft_render::scene::{Mesh, cube_mesh, polylines};
use foamcraft_sim::results::Surface;
use foamcraft_sim::ride::{
    self, CamKey, End, Features, Flow, Kind, Mode, Path, RideEvent, Timing, camera_at,
};
use glam::{DMat3, DVec3};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::json;

pub const MODES: [&str; 2] = ["calm", "coaster"];
pub const VIEWS: [&str; 3] = ["first_person", "chase", "overview"];
/// Most rows of the readings table.
const MAX_ROWS: usize = 512;

/// The flow of the shown result in the flow frame, with the turn into the world.
pub struct RideFlow {
    pub flow: Flow,
    pub yaw_deg: f64,
    pub kind: Kind,
    pub job: String,
    /// Density and viscosity of the run's medium (for heavy particles).
    pub medium: foamcraft_sim::heavy::Medium,
}

impl RideFlow {
    fn to_world(&self) -> DMat3 {
        DMat3::from_rotation_z(self.yaw_deg.to_radians())
    }

    pub fn world(&self, p: DVec3) -> DVec3 {
        self.to_world() * p
    }

    pub fn flow_point(&self, world: [f64; 3]) -> [f64; 3] {
        (self.to_world().transpose() * DVec3::from(world)).into()
    }
}

/// Reference speed and density of a run from its result file.
fn references(project: &Project, job: &str) -> (Option<f64>, f64) {
    let Ok(r) = crate::solve::load_result(&project.root.join(crate::solve::RESULTS_DIR), Some(job))
    else {
        return (None, 1.2);
    };
    let medium: Option<MediumSpec> = serde_json::from_value(r["scenario"]["medium"].clone()).ok();
    let rho = r["reference"]["density_kg_m3"]
        .as_f64()
        .or_else(|| medium.map(|m| foamcraft_sim::medium_properties(&m).density))
        .unwrap_or(1.2);
    let speed = r["reference"]["speed_m_s"].as_f64().or_else(|| {
        r["openings"].as_array().and_then(|o| {
            o.iter()
                .find(|o| o["role"] == "inlet")
                .and_then(|o| o["velocity_m_s"].as_f64())
        })
    });
    (speed, rho)
}

/// The flow of the result the view shows.
pub fn load_flow(project: &Project) -> Result<RideFlow, FoamError> {
    let (grids, yaw_deg) = crate::view::load_grids(project)?;
    let job = crate::view::job_of(project)?;
    let to_flow = DMat3::from_rotation_z(-yaw_deg.to_radians());
    let surface = crate::results::load_surface(&project.root, &job)
        .ok()
        .map(|s| Surface {
            points: s
                .points
                .iter()
                .map(|p| {
                    let v = to_flow * DVec3::from(p.map(f64::from));
                    [v.x as f32, v.y as f32, v.z as f32]
                })
                .collect(),
            triangles: s.triangles,
            fields: vec![],
        });
    let (u_ref, rho) = references(project, &job);
    let mut flow = Flow::new(grids, surface.as_ref(), u_ref, rho)?;
    let kind = if matches!(project.manifest.scenario, Some(Scenario::InternalFlow(_))) {
        flow.main = None;
        Kind::Internal
    } else {
        Kind::WindTunnel
    };
    let medium = crate::heavy::medium_of(project, &job);
    Ok(RideFlow {
        flow,
        yaw_deg,
        kind,
        job,
        medium,
    })
}

/// A suggestion as `ride.suggest` lists it.
#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct SuggestionInfo {
    /// `stagnation`, `hugging`, `fast`, `wake`, `turn` or `long`.
    pub id: String,
    /// Catalog code of the title (`ride.suggestion.<id>`).
    pub title: String,
    /// Start point in the world, m.
    pub start: [f64; 3],
    /// Length of a calm ride at normal speed, s.
    pub duration_s: f64,
    #[schemars(skip)]
    pub features: Value,
}

type Value = serde_json::Value;

type CatalogKey = (PathBuf, String, u32, String);

fn catalog() -> &'static Mutex<HashMap<CatalogKey, Vec<SuggestionInfo>>> {
    static C: OnceLock<Mutex<HashMap<CatalogKey, Vec<SuggestionInfo>>>> = OnceLock::new();
    C.get_or_init(Default::default)
}

fn candidates(project: &Project, rf: &RideFlow, seed: u32) -> Vec<[f64; 3]> {
    let f = &rf.flow;
    let mut out = match rf.kind {
        Kind::WindTunnel => ride::wind_candidates(f, seed),
        Kind::Internal => {
            let region = SeedRegionSpec {
                kind: "inlets".into(),
                origin: [0.0; 3],
                u: [0.0; 3],
                v: [0.0; 3],
            };
            let all = crate::particles::seeds(project, &region, &f.grids, rf.yaw_deg, seed);
            let step = (all.len() / 300).max(1);
            all.iter()
                .step_by(step)
                .take(300)
                .map(|p| p.map(f64::from))
                .collect()
        }
    };
    out.extend(ride::surface_candidates(f, 216, seed));
    out
}

/// The catalogue of suggestions of the shown result (cached per run and seed).
/// The particle of rides: the heavy particle of the view particles, if they have mass.
pub fn view_particle(project: &Project) -> Option<InertiaSpec> {
    project
        .manifest
        .view
        .particles
        .as_ref()
        .and_then(|p| p.inertia.clone())
}

pub fn suggestions(project: &Project, seed: u32) -> Result<Vec<SuggestionInfo>, FoamError> {
    let job = crate::view::job_of(project)?;
    let particle = view_particle(project);
    let key = (
        project.root.clone(),
        job,
        seed,
        serde_json::to_string(&particle).unwrap_or_default(),
    );
    if let Some(c) = catalog().lock().ok().and_then(|m| m.get(&key).cloned()) {
        return Ok(c);
    }
    let rf = load_flow(project)?;
    let list = match &particle {
        Some(spec) => heavy_suggestions(project, &rf, spec, seed),
        None => massless_suggestions(project, &rf, seed),
    };
    if let Ok(mut m) = catalog().lock() {
        m.insert(key, list.clone());
    }
    Ok(list)
}

fn massless_suggestions(project: &Project, rf: &RideFlow, seed: u32) -> Vec<SuggestionInfo> {
    let starts = candidates(project, rf, seed);
    ride::suggest(&rf.flow, &starts, rf.kind)
        .into_iter()
        .map(|s| {
            let duration_s = rf.flow.trace(s.start).map_or(0.0, |p| {
                let p = p.trimmed(&rf.flow);
                ride::timing(&p, &rf.flow, Mode::Calm, 1.0).duration()
            });
            SuggestionInfo {
                id: s.id.into(),
                title: format!("ride.suggestion.{}", s.id),
                start: rf.world(DVec3::from(s.start)).into(),
                duration_s,
                features: features_json(&s.features),
            }
        })
        .collect()
}

/// The flight of a heavy particle from `start` (flow frame): it is not followed backwards (the
/// motion is unstable that way), so the ride begins at the release point.
fn heavy_flight(rf: &RideFlow, start: DVec3, spec: &InertiaSpec) -> Option<(Path, HeavyTrack)> {
    let f = &rf.flow;
    let wall = f.wall.as_ref()?;
    let p = crate::heavy::particle_of(spec);
    let m = rf.medium;
    let u_at = |x: DVec3| match f.velocity(x) {
        ride::Probe::Fluid(u) => Some(u),
        ride::Probe::Blocked => Some(DVec3::ZERO),
        ride::Probe::Outside => None,
    };
    let v0 = u_at(start)? + foamcraft_sim::heavy::terminal_velocity(&p, &m);
    let track = foamcraft_sim::heavy::track(
        start,
        v0,
        &u_at,
        wall,
        &p,
        &m,
        f.h,
        20.0 * f.length / f.u_ref.max(1e-9),
    );
    let raw = (0..track.points.len())
        .map(|i| {
            (
                track.points[i],
                track.times[i],
                track.velocities[i],
                track.flow[i],
            )
        })
        .collect();
    let end = match track.fate {
        foamcraft_sim::heavy::Fate::Stuck(_) => End::Blocked,
        foamcraft_sim::heavy::Fate::Left => End::Outside,
        foamcraft_sim::heavy::Fate::Timeout => End::Steps,
    };
    // Misses fly on far behind the model: keep the stretch around it, like streamlines.
    let path = f.heavy_path(raw, end)?;
    let path = if end == End::Blocked {
        path
    } else {
        path.trimmed(f)
    };
    Some((path, track))
}

type HeavyTrack = foamcraft_sim::heavy::Track;

/// Impact events of a heavy flight: each bounce and the final hit, with speed and angle.
fn impact_events(path: &Path, timing: &ride::Timing, track: &HeavyTrack) -> Vec<RideEvent> {
    let at = |p: DVec3| {
        path.points
            .iter()
            .enumerate()
            .min_by(|a, b| {
                (DVec3::from(a.1.pos) - p)
                    .length()
                    .total_cmp(&(DVec3::from(b.1.pos) - p).length())
            })
            .map_or(0, |(i, _)| i)
    };
    let mut out = Vec::new();
    for b in &track.bounces {
        let i = at(b.point);
        let n = (b.after - b.before).normalize_or_zero();
        let angle = (b.before.normalize_or_zero().dot(-n))
            .clamp(-1.0, 1.0)
            .asin()
            .to_degrees();
        out.push(RideEvent {
            id: "impact",
            s: path.points[i].s,
            t: timing.t[i],
            value: b.before.length(),
            extra: angle.abs(),
        });
    }
    if let foamcraft_sim::heavy::Fate::Stuck(hit) = track.fate {
        let i = path.points.len() - 1;
        out.push(RideEvent {
            id: "impact",
            s: path.points[i].s,
            t: timing.t[i],
            value: track.velocities.last().map_or(0.0, |v| v.length()),
            extra: hit.angle_deg,
        });
    }
    out
}

/// Suggested flights of heavy particles: a hit near the front, the flattest hit and the closest
/// miss, from about 600 release points.
fn heavy_suggestions(
    project: &Project,
    rf: &RideFlow,
    spec: &InertiaSpec,
    seed: u32,
) -> Vec<SuggestionInfo> {
    let mut starts = match rf.kind {
        Kind::WindTunnel => {
            let mut s = ride::wind_candidates(&rf.flow, seed);
            s.extend(ride::wind_candidates(&rf.flow, seed.wrapping_add(1)));
            s
        }
        Kind::Internal => candidates(project, rf, seed),
    };
    starts.truncate(600);
    let flights: Vec<Option<(Path, HeavyTrack)>> = std::thread::scope(|scope| {
        let threads = std::thread::available_parallelism().map_or(4, |n| n.get());
        let chunk = starts.len().div_ceil(threads).max(1);
        let handles: Vec<_> = starts
            .chunks(chunk)
            .map(|part| {
                scope.spawn(move || {
                    part.iter()
                        .map(|s| heavy_flight(rf, DVec3::from(*s), spec))
                        .collect::<Vec<_>>()
                })
            })
            .collect();
        handles
            .into_iter()
            .flat_map(|h| h.join().unwrap_or_default())
            .collect()
    });
    let main = DVec3::X;
    let mut picks: Vec<(&'static str, usize)> = Vec::new();
    let hit = |i: usize| match &flights[i] {
        Some((_, t)) => match t.fate {
            foamcraft_sim::heavy::Fate::Stuck(h) => Some(h),
            _ => None,
        },
        None => None,
    };
    let best = |score: &dyn Fn(usize) -> Option<f64>, taken: &[(&str, usize)]| {
        (0..flights.len())
            .filter(|i| !taken.iter().any(|(_, j)| j == i))
            .filter_map(|i| score(i).map(|v| (v, i)))
            .max_by(|a, b| a.0.total_cmp(&b.0))
            .map(|(_, i)| i)
    };
    if let Some(i) = best(&|i| hit(i).map(|h| h.normal.dot(-main)), &picks) {
        picks.push(("impact_front", i));
    }
    if let Some(i) = best(&|i| hit(i).map(|h| -h.angle_deg), &picks) {
        picks.push(("impact_grazing", i));
    }
    if let Some(i) = best(
        &|i| match &flights[i] {
            Some((p, t)) if hit(i).is_none() && t.bounces.is_empty() => {
                Some(-p.points.iter().map(|q| q.d).fold(f64::MAX, f64::min))
            }
            _ => None,
        },
        &picks,
    ) {
        picks.push(("near_miss", i));
    }
    picks
        .into_iter()
        .map(|(id, i)| {
            let duration_s = flights[i].as_ref().map_or(0.0, |(p, _)| {
                ride::timing(p, &rf.flow, Mode::Calm, 1.0).duration()
            });
            SuggestionInfo {
                id: id.into(),
                title: format!("ride.suggestion.{id}"),
                start: rf.world(DVec3::from(starts[i])).into(),
                duration_s,
                features: Value::Null,
            }
        })
        .collect()
}

fn features_json(f: &Features) -> Value {
    serde_json::to_value(f).unwrap_or_default()
}

fn mode_of(name: &str) -> Mode {
    if name == "coaster" {
        Mode::Coaster
    } else {
        Mode::Calm
    }
}

/// A computed ride: path, screen timing, camera track and events (flow frame inside).
pub struct Ride {
    pub spec: RideSpec,
    pub rf: RideFlow,
    pub path: Path,
    pub timing: Timing,
    pub track: Vec<CamKey>,
    pub events: Vec<RideEvent>,
}

fn no_path() -> FoamError {
    FoamError::new(
        "ride.no_path",
        "the point lies in a solid or outside the flow",
    )
}

impl Ride {
    pub fn new(project: &Project, spec: RideSpec) -> Result<Self, FoamError> {
        Self::with_flow(load_flow(project)?, spec)
    }

    pub fn with_flow(rf: RideFlow, spec: RideSpec) -> Result<Self, FoamError> {
        let start = DVec3::from(rf.flow_point(spec.start));
        let (path, heavy) = match &spec.particle {
            Some(particle) => {
                let (p, t) = heavy_flight(&rf, start, particle).ok_or_else(no_path)?;
                (p, Some(t))
            }
            None => (
                rf.flow
                    .trace(start.into())
                    .map(|p| p.trimmed(&rf.flow))
                    .filter(|p| p.points.len() > 1)
                    .ok_or_else(no_path)?,
                None,
            ),
        };
        let mode = mode_of(&spec.mode);
        let timing = ride::timing(&path, &rf.flow, mode, spec.speed);
        let track = ride::camera_track(&path, &timing, &rf.flow, mode);
        let mut events = ride::events(&path, &rf.flow, &timing);
        if let Some(t) = &heavy {
            events.extend(impact_events(&path, &timing, t));
            events.sort_by(|a, b| a.s.total_cmp(&b.s));
        }
        Ok(Self {
            spec,
            rf,
            path,
            timing,
            track,
            events,
        })
    }

    pub fn duration(&self) -> f64 {
        self.timing.duration()
    }

    /// Screen time of the start point (where a ride from a click begins to matter).
    pub fn start_time(&self) -> f64 {
        self.timing.t_at(&self.path, self.path.start_s)
    }

    fn v3(&self, p: DVec3) -> Vec3 {
        let w = self.rf.world(p);
        vec3(w.x as f32, w.y as f32, w.z as f32)
    }

    /// The rider in the world at screen time `t`.
    pub fn rider(&self, t: f64) -> Vec3 {
        self.v3(self.path.position_at(self.timing.s_at(&self.path, t)))
    }

    /// Camera of the ride at `t` for `first_person` or `chase` (`None` for the overview), turned
    /// by the look-around angles (radians, sideways and up) against the direction of travel.
    pub fn camera(&self, t: f64, view: &str, look: (f64, f64)) -> Option<FreeCamera> {
        let mut key = camera_at(&self.track, t)?;
        match view {
            "first_person" => {}
            "chase" => key = ride::chase(&self.rf.flow, &key),
            _ => return None,
        }
        let right = key.forward.cross(key.up).normalize_or_zero();
        let turn = DMat3::from_axis_angle(key.up, -look.0.clamp(-2.62, 2.62));
        let forward = turn * key.forward;
        let right = turn * right;
        let tilt = DMat3::from_axis_angle(right, look.1.clamp(-1.4, 1.4));
        let forward = (tilt * forward).normalize();
        let up = right.cross(forward).normalize();
        let w = self.rf.to_world();
        let f = w * forward;
        let u = w * up;
        let e = self.rf.world(key.eye);
        Some(FreeCamera {
            eye: vec3(e.x as f32, e.y as f32, e.z as f32),
            forward: vec3(f.x as f32, f.y as f32, f.z as f32),
            up: vec3(u.x as f32, u.y as f32, u.z as f32),
            fov_y: (key.fov_deg as f32).to_radians(),
            near: key.near as f32,
            far: key.far as f32,
        })
    }

    /// Flow time that passes between screen times `t` and `t + dt` (to move the particles of the
    /// view along with the rider).
    pub fn flow_dt(&self, t: f64, dt: f64) -> f64 {
        let tau = |t: f64| {
            let s = self.timing.s_at(&self.path, t);
            let (i, f) = {
                let n = self.path.points.len();
                if n < 2 {
                    return 0.0;
                }
                let i = ((s / self.path.ds).floor() as usize).min(n - 2);
                let (a, b) = (self.path.points[i].s, self.path.points[i + 1].s);
                (
                    i,
                    if b > a {
                        ((s - a) / (b - a)).clamp(0.0, 1.0)
                    } else {
                        0.0
                    },
                )
            };
            self.path.points[i].tau * (1.0 - f) + self.path.points[i + 1].tau * f
        };
        (tau(t + dt) - tau(t)).max(0.0)
    }

    /// Lines and marker of the ride at `t`: the path ahead (guide), the path behind (chase and
    /// overview) and the rider.
    pub fn meshes(&self, t: f64, view: &str) -> Vec<Mesh> {
        let s = self.timing.s_at(&self.path, t);
        let line = |from: f64, to: f64| -> Vec<[f32; 3]> {
            self.path
                .points
                .iter()
                .filter(|p| p.s >= from && p.s <= to)
                .map(|p| self.v3(DVec3::from(p.pos)).to_array())
                .collect()
        };
        let mut out = Vec::new();
        let ahead = line(s, self.path.length());
        if self.spec.guide && ahead.len() > 1 {
            out.push(polylines(&[ahead], [1.0, 0.85, 0.25], false));
        }
        if view != "first_person" {
            let behind = line(0.0, s);
            if behind.len() > 1 {
                out.push(polylines(&[behind], [0.55, 0.6, 0.7], false));
            }
            let size = (0.03 * self.rf.flow.length) as f32;
            out.push(cube_mesh(self.rider(t), size, [1.0, 0.95, 0.6]));
        }
        out
    }

    /// The readings of the whole ride (at most 512 rows) and the events.
    pub fn telemetry(&self) -> RideInfo {
        let n = self.path.points.len();
        let step = n.div_ceil(MAX_ROWS).max(1);
        let mut idx: Vec<usize> = (0..n).step_by(step).collect();
        if idx.last() != Some(&(n - 1)) {
            idx.push(n - 1);
        }
        let rows = idx
            .iter()
            .map(|&i| {
                let p = &self.path.points[i];
                [
                    self.timing.t[i],
                    p.s,
                    p.speed,
                    p.cp,
                    p.p,
                    p.d,
                    p.accel,
                    self.timing.factor[i],
                    p.slip,
                ]
            })
            .collect();
        let end = |e: End| {
            match e {
                End::Outside => "outside",
                End::Blocked => "blocked",
                End::Length => "length",
                End::Steps => "steps",
                End::Stalled => "stalled",
            }
            .to_string()
        };
        RideInfo {
            ride: self.spec.clone(),
            job: self.rf.job.clone(),
            duration_s: self.duration(),
            start_t: self.start_time(),
            length_m: self.path.length(),
            flow_time_s: self.path.points.last().map_or(0.0, |p| p.tau),
            reference_speed_m_s: self.rf.flow.u_ref,
            density_kg_m3: self.rf.flow.rho,
            model_length_m: self.rf.flow.length,
            ends: [end(self.path.back_end), end(self.path.forward_end)],
            columns: [
                "t",
                "s",
                "speed",
                "cp",
                "p",
                "d",
                "accel",
                "time_factor",
                "slip",
            ]
            .map(String::from)
            .to_vec(),
            rows,
            events: self
                .events
                .iter()
                .map(|e| EventInfo {
                    id: e.id.into(),
                    code: format!("ride.event.{}", e.id),
                    s: e.s,
                    t: e.t,
                    value: e.value,
                    extra: e.extra,
                })
                .collect(),
        }
    }
}

#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct EventInfo {
    pub id: String,
    /// Catalog code of the sentence (`ride.event.<id>`).
    pub code: String,
    /// Arc length (m) and screen time (s) where it happens.
    pub s: f64,
    pub t: f64,
    /// The number of the sentence: speed share, Cp or wall distance (m); impact speed (m/s).
    pub value: f64,
    /// A second number: the angle of an impact to the surface, degrees.
    pub extra: f64,
}

/// Readings of a ride: `rows` hold `columns` (screen time s, arc length m, speed m/s, Cp,
/// pressure Pa, wall distance m, acceleration m/s², flow time per screen time).
#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct RideInfo {
    pub ride: RideSpec,
    pub job: String,
    pub duration_s: f64,
    /// Screen time of the start point.
    pub start_t: f64,
    pub length_m: f64,
    pub flow_time_s: f64,
    pub reference_speed_m_s: f64,
    pub density_kg_m3: f64,
    pub model_length_m: f64,
    /// How the path ends upstream and downstream: `outside`, `blocked`, `length`, `steps`,
    /// `stalled`.
    pub ends: [String; 2],
    pub columns: Vec<String>,
    pub rows: Vec<[f64; 9]>,
    pub events: Vec<EventInfo>,
}

/// The stored ride of the view, computed.
pub fn current(project: &Project) -> Result<Ride, FoamError> {
    let spec = project
        .manifest
        .view
        .ride
        .clone()
        .ok_or_else(|| FoamError::new("ride.none", ""))?;
    Ride::new(project, spec)
}

#[derive(Debug, Default, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SuggestParams {
    /// Seed of the candidate points; the same seed gives the same suggestions.
    #[serde(default)]
    pub seed: Option<u32>,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct StartParams {
    /// A suggestion id from `ride.suggest` …
    #[serde(default)]
    pub suggestion: Option<String>,
    /// … or a point in the world (m) the path runs through.
    #[serde(default)]
    pub from: Option<[f64; 3]>,
    /// `calm` (default) or `coaster`.
    #[serde(default)]
    pub mode: Option<String>,
    /// `first_person` (default), `chase` or `overview`.
    #[serde(default)]
    pub view: Option<String>,
    /// Speed control 0.25 to 3, default 1.
    #[serde(default)]
    pub speed: Option<f64>,
    /// The path ahead as a thin line, default on.
    #[serde(default)]
    pub guide: Option<bool>,
    /// Seed of the suggestions.
    #[serde(default)]
    pub seed: Option<u32>,
    /// Ride on a heavy particle (material or own values); default the particles of the view.
    #[serde(default)]
    pub particle: Option<crate::heavy::InertiaParams>,
}

fn invalid(what: &str) -> FoamError {
    FoamError::new("ride.invalid_option", what)
}

fn start(s: &mut Session, p: StartParams) -> Result<RideInfo, FoamError> {
    let project = s.project()?;
    let old = project.manifest.view.ride.clone();
    let mode = p
        .mode
        .or_else(|| old.as_ref().map(|r| r.mode.clone()))
        .unwrap_or_else(|| "calm".into());
    let view = p
        .view
        .or_else(|| old.as_ref().map(|r| r.view.clone()))
        .unwrap_or_else(|| "first_person".into());
    if !MODES.contains(&mode.as_str()) {
        return Err(invalid(&format!("mode {mode}")));
    }
    if !VIEWS.contains(&view.as_str()) {
        return Err(invalid(&format!("view {view}")));
    }
    let speed = p
        .speed
        .or_else(|| old.as_ref().map(|r| r.speed))
        .unwrap_or(1.0);
    if !speed.is_finite() {
        return Err(invalid("speed"));
    }
    let start = match (&p.suggestion, p.from) {
        (Some(id), _) => {
            suggestions(project, p.seed.unwrap_or(1))?
                .into_iter()
                .find(|x| &x.id == id)
                .ok_or_else(|| FoamError::new("ride.unknown_suggestion", id.clone()))?
                .start
        }
        (None, Some(from)) => from,
        (None, None) => old
            .as_ref()
            .map(|r| r.start)
            .ok_or_else(|| FoamError::new("ride.no_path", "give a suggestion or a point"))?,
    };
    if !start.iter().all(|x| x.is_finite()) {
        return Err(no_path());
    }
    let particle = match &p.particle {
        Some(i) => {
            let job = crate::view::job_of(project)?;
            Some(crate::heavy::resolve(
                i,
                &crate::heavy::medium_of(project, &job),
            )?)
        }
        None => view_particle(project),
    };
    let restart = p.suggestion.is_some() || p.from.is_some();
    let run = old.as_ref().map_or(0, |r| r.run) + u32::from(restart);
    let spec = RideSpec {
        start,
        mode,
        view,
        speed: speed.clamp(0.25, 3.0),
        guide: p.guide.or(old.map(|r| r.guide)).unwrap_or(true),
        particle,
        run,
    };
    let ride = Ride::new(project, spec.clone())?;
    project.manifest.view.ride = Some(spec);
    project.save()?;
    s.events().emit("project.changed", Value::Null);
    Ok(ride.telemetry())
}

pub fn register(r: &mut Registry) {
    r.add(
        "ride.suggest",
        "Up to five worthwhile rides along one particle of the shown result: towards the \
         stagnation point, close along the surface, over the fastest spot, into the wake, the \
         strongest turn, the longest way (internal flow). Each with start point (world), title \
         code and duration. The same seed gives the same list.",
        Options {
            undoable: false,
            errors: &["project.none_open", "results.none", "ride.no_path"],
            example: json!({}),
        },
        |s: &mut Session, p: SuggestParams| suggestions(s.project()?, p.seed.unwrap_or(1)),
    );
    r.add(
        "ride.start",
        "Ride along one particle: from a suggestion or through a world point, calm or coaster, \
         seen first person, from behind (chase) or from outside (overview). Stores the ride in the \
         view and answers with its readings table and events.",
        Options {
            undoable: false,
            errors: &[
                "project.none_open",
                "results.none",
                "ride.no_path",
                "ride.unknown_suggestion",
                "ride.invalid_option",
            ],
            example: json!({"suggestion": "stagnation", "mode": "calm", "view": "first_person"}),
        },
        start,
    );
    r.add(
        "ride.telemetry",
        "Readings (speed, pressure, Cp, wall distance, acceleration, time factor) and events of \
         the stored ride: what the particle goes through.",
        Options {
            undoable: false,
            errors: &[
                "project.none_open",
                "results.none",
                "ride.none",
                "ride.no_path",
            ],
            example: json!({}),
        },
        |s: &mut Session, _: SuggestParams| Ok(current(s.project()?)?.telemetry()),
    );
    r.add(
        "ride.stop",
        "End the ride and remove it from the view.",
        Options {
            undoable: false,
            errors: &["project.none_open"],
            example: json!({}),
        },
        |s: &mut Session, _: SuggestParams| {
            let project = s.project()?;
            project.manifest.view.ride = None;
            project.save()?;
            s.events().emit("project.changed", Value::Null);
            Ok(json!({"stopped": true}))
        },
    );
}

/// Picture-in-picture rectangle of the overview: a quarter of the width, bottom right.
pub fn inset_rect(w: u32, h: u32) -> [u32; 4] {
    let iw = (w / 4).max(16);
    let ih = (iw * h / w.max(1)).max(16);
    let m = (w / 80).max(4);
    [w.saturating_sub(iw + m), h.saturating_sub(ih + m), iw, ih]
}

/// One frame of the ride at screen time `t` into an offscreen image: the stored view (first
/// person or chase with the overview inset, or the overview alone). `main` and `inset` are the
/// uploaded project scene; their handle layer is replaced by the ride's lines and marker.
#[allow(clippy::too_many_arguments)]
pub fn draw_frame(
    renderer: &foamcraft_render::Renderer,
    main: &mut foamcraft_render::GpuScene,
    inset: &mut foamcraft_render::GpuScene,
    ride: &Ride,
    overview: &foamcraft_render::camera::OrbitCamera,
    t: f64,
    w: u32,
    h: u32,
) -> Vec<u8> {
    let view = ride.spec.view.as_str();
    renderer.set_handles(main, &ride.meshes(t, view));
    match ride.camera(t, view, (0.0, 0.0)) {
        Some(cam) => {
            renderer.set_handles(inset, &ride.meshes(t, "overview"));
            renderer.render_offscreen_inset(main, &cam, inset, overview, w, h, inset_rect(w, h))
        }
        None => renderer.render_offscreen_gpu(main, overview, w, h),
    }
}

/// A still of the stored ride at screen time `t` (for `view.render` and `export.image`).
pub fn render_still(
    project: &Project,
    renderer: &foamcraft_render::Renderer,
    w: u32,
    h: u32,
    t: f64,
    transparent: bool,
) -> Result<Vec<u8>, FoamError> {
    let ride = current(project)?;
    let built = crate::scene::project_scene(Some(project));
    let flow = built.flow_geometry();
    let mut scene = built.scene;
    if transparent {
        scene.background[3] = 0.0;
    }
    let mut overview = foamcraft_render::camera::OrbitCamera::default();
    let (centre, radius) = built.focus.unwrap_or_else(|| scene.bounding_sphere());
    overview.fit(centre, radius * 1.1, w as f32 / h as f32);
    let mut main = renderer.upload(&scene);
    renderer.set_surface_flow(&mut main, flow.as_ref());
    let mut inset = renderer.upload(&scene);
    Ok(draw_frame(
        renderer, &mut main, &mut inset, &ride, &overview, t, w, h,
    ))
}
