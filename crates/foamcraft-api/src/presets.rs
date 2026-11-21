// SPDX-License-Identifier: GPL-3.0-or-later
//! Ready-made views of a result (`view.presets`, `view.apply_preset`): smoke lines, a smoke
//! curtain, the mid slice, pressure on the model, vortices, the wake, and for internal flow the
//! path through the part and a lengthwise slice. A preset replaces the scene of the view in one
//! undoable step and adapts to the run (scenario, model size, wind direction).

use foamcraft_core::command::{Options, Registry, Session};
use foamcraft_core::error::FoamError;
use foamcraft_core::project::{
    IsoSpec, ParticleSpec, PlaneSpec, Project, Scenario, SeedRegionSpec, SliceSpec, ViewSettings,
};
use foamcraft_sim::results::Grid;
use glam::{DMat3, DVec3};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::json;

/// Presets of a wind tunnel run, in the order the demo shows them.
pub const WIND_PRESETS: [&str; 11] = [
    "smoke_lines",
    "smoke_curtain",
    "mid_slice",
    "surface_pressure",
    "vortices",
    "wake",
    "oil_film",
    "tufts",
    "rain",
    "light_trails",
    "ribbons",
];
/// Presets of an internal flow run.
pub const INTERNAL_PRESETS: [&str; 7] = [
    "internal_path",
    "internal_slice",
    "surface_pressure",
    "oil_film",
    "tufts",
    "light_trails",
    "ribbons",
];
/// Nozzles of the smoke rake.
pub const RAKE_NOZZLES: u32 = 13;

/// A preset as `view.presets` lists it.
#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct PresetInfo {
    pub id: String,
    /// Standard view the camera turns to (see `view.render`).
    pub view: String,
    /// Catalog code of the one-sentence explanation (`preset.<id>`).
    pub explanation: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ApplyParams {
    /// A preset id from `view.presets`.
    pub id: String,
}

fn is_internal(project: &Project) -> bool {
    matches!(project.manifest.scenario, Some(Scenario::InternalFlow(_)))
}

/// The presets that fit the project's scenario.
pub fn presets(project: &Project) -> Vec<PresetInfo> {
    let ids: &[&str] = if is_internal(project) {
        &INTERNAL_PRESETS
    } else {
        &WIND_PRESETS
    };
    ids.iter()
        .map(|id| PresetInfo {
            id: (*id).into(),
            view: camera_of(id).into(),
            explanation: format!("preset.{id}"),
        })
        .collect()
}

fn camera_of(id: &str) -> &'static str {
    match id {
        "smoke_lines" | "smoke_curtain" | "mid_slice" => "right",
        "vortices" => "top_back_right",
        "oil_film" | "tufts" => "top_front_left",
        "rain" | "light_trails" => "front_left",
        "ribbons" => "top_back_right",
        "wake" => "back",
        _ => "default",
    }
}

/// Model box in the flow frame (x along the flow), from the models placed in the world.
fn model_box(project: &Project, yaw_deg: f64) -> Option<([f64; 3], [f64; 3])> {
    let to_flow = DMat3::from_rotation_z(-yaw_deg.to_radians());
    let mut lo = [f64::MAX; 3];
    let mut hi = [f64::MIN; 3];
    for (_, m) in foamcraft_geometry::commands::world_meshes(project) {
        for p in &m.positions {
            let q = to_flow * DVec3::from(p.map(f64::from));
            for k in 0..3 {
                lo[k] = lo[k].min(q[k]);
                hi[k] = hi[k].max(q[k]);
            }
        }
    }
    (lo[0] <= hi[0]).then_some((lo, hi))
}

/// A high percentile of the positive Q values of the finest grid: the strong vortices.
fn q_value(grids: &[Grid]) -> f64 {
    let Some(g) = grids.last() else { return 1.0 };
    let Some(f) = g.names.iter().position(|n| n == "Q") else {
        return 1.0;
    };
    let mut q: Vec<f32> = g.values[f]
        .iter()
        .zip(&g.mask)
        .filter(|(v, m)| !**m && **v > 0.0)
        .map(|(v, _)| *v)
        .collect();
    if q.is_empty() {
        return 1.0;
    }
    q.sort_by(f32::total_cmp);
    f64::from(q[((q.len() - 1) as f64 * 0.9) as usize])
}

/// The view settings of preset `id` (the rest of the view stays).
fn build(project: &Project, id: &str) -> Result<ViewSettings, FoamError> {
    let allowed = presets(project);
    if !allowed.iter().any(|p| p.id == id) {
        return Err(FoamError::new("view.unknown_preset", id));
    }
    let (grids, yaw) = crate::view::load_grids(project)?;
    let coarse = grids
        .first()
        .ok_or_else(|| FoamError::new("results.none", ""))?;
    let fine = grids.last().unwrap_or(coarse);
    let (dom_lo, dom_hi) = (coarse.spec.min, coarse.spec.max);
    let (lo, hi) = model_box(project, yaw).unwrap_or((fine.spec.min, fine.spec.max));
    let c = [0, 1, 2].map(|k| (lo[k] + hi[k]) / 2.0);
    let size = [0, 1, 2].map(|k| (hi[k] - lo[k]).max(1e-9));
    let step = fine.spec.step();
    let to_world = DMat3::from_rotation_z(yaw.to_radians());
    let w = |v: [f64; 3]| -> [f64; 3] { (to_world * DVec3::from(v)).into() };
    // Inside the domain, a little away from its walls.
    let inside = |k: usize, x: f64| {
        let m = 0.02 * (dom_hi[k] - dom_lo[k]);
        x.clamp(dom_lo[k] + m, dom_hi[k] - m)
    };
    let upstream = inside(0, lo[0] - 0.5 * size[0]);
    let z0 = inside(2, c[2] - 0.6 * size[2]);
    let height = inside(2, c[2] + 0.6 * size[2]) - z0;

    let mut view = project.manifest.view.clone();
    // The scene is built from the shown run; colouring the model needs it set.
    if view.result_job.is_none() {
        view.result_job = Some(crate::view::job_of(project)?);
    }
    view.slices.clear();
    view.isosurfaces.clear();
    view.clip = None;
    view.particles = None;
    view.surface_field = None;
    view.surface_flow = None;
    let smoke = |count: u32, region: SeedRegionSpec| ParticleSpec {
        style: "smoke".into(),
        count,
        seed: 1,
        color_by: "speed".into(),
        speed: 1.0,
        paused: false,
        region,
        inertia: None,
        trail_length: None,
        width: None,
        glow: None,
        exposure: None,
        long_exposure: None,
    };
    let slice = |origin: [f64; 3], normal: [f64; 3]| SliceSpec {
        id: "slice-1".into(),
        plane: PlaneSpec {
            origin: w(origin),
            normal: w(normal),
        },
        field: "U_mag".into(),
        range: None,
        hidden: false,
    };
    match id {
        "smoke_lines" => {
            view.particles = Some(smoke(
                20_000,
                SeedRegionSpec {
                    kind: "rake".into(),
                    origin: w([upstream, c[1], z0]),
                    u: w([0.0, 0.0, height]),
                    // Scatter radius of each nozzle: half a cell of the finest grid.
                    v: [0.5 * step[0], 0.0, 0.0],
                },
            ));
        }
        "smoke_curtain" => {
            view.particles = Some(smoke(
                40_000,
                SeedRegionSpec {
                    kind: "area".into(),
                    origin: w([upstream, c[1] - 0.5 * step[1], z0]),
                    u: w([0.0, step[1], 0.0]),
                    v: w([0.0, 0.0, height]),
                },
            ));
        }
        "mid_slice" => view.slices.push(slice(c, [0.0, 1.0, 0.0])),
        "wake" => view.slices.push(slice(
            [inside(0, hi[0] + 0.5 * size[0]), c[1], c[2]],
            [1.0, 0.0, 0.0],
        )),
        "surface_pressure" => view.surface_field = Some("p".into()),
        "vortices" => view.isosurfaces.push(IsoSpec {
            id: "iso-1".into(),
            field: "Q".into(),
            value: q_value(&grids),
        }),
        "internal_path" => {
            let mut path = smoke(
                200,
                SeedRegionSpec {
                    kind: "inlets".into(),
                    origin: [0.0; 3],
                    u: [0.0; 3],
                    v: [0.0; 3],
                },
            );
            path.style = "streamlines".into();
            view.particles = Some(path);
        }
        "internal_slice" => {
            // Through the longest extent: the normal is the shortest axis of the box.
            let k = (0..3)
                .min_by(|a, b| size[*a].total_cmp(&size[*b]))
                .unwrap_or(2);
            let mut n = [0.0; 3];
            n[k] = 1.0;
            view.slices.push(slice(c, n));
        }
        // Lines on a dark model like an oil film (Task_33).
        "oil_film" => {
            view.surface_flow = Some(crate::surface_flow::merged(
                None,
                crate::surface_flow::SetParams {
                    lines: Some(crate::surface_flow::LinesParams {
                        on: Some(true),
                        ..Default::default()
                    }),
                    ..Default::default()
                },
            )?);
        }
        // Raindrops from upstream; the impact map on the model when it has been computed.
        "rain" => {
            let job = view.result_job.clone().unwrap_or_default();
            let medium = crate::heavy::medium_of(project, &job);
            let inertia = crate::heavy::resolve(
                &crate::heavy::InertiaParams {
                    material: Some("rain".into()),
                    ..Default::default()
                },
                &medium,
            )?;
            // Raindrops sink on their way to the model: start that much higher.
            let p = crate::heavy::particle_of(&inertia);
            let sink = foamcraft_sim::heavy::terminal_velocity(&p, &medium).z.abs();
            let speed = crate::ride::load_flow(project).map_or(1.0, |rf| rf.flow.u_ref);
            let drop = sink / speed.max(1e-9) * (hi[0] - upstream);
            let mut spec = smoke(
                20_000,
                SeedRegionSpec {
                    kind: "area".into(),
                    origin: w([upstream, c[1] - 0.6 * size[1], z0]),
                    u: w([0.0, 1.2 * size[1], 0.0]),
                    v: w([0.0, 0.0, inside(2, z0 + height + drop) - z0]),
                },
            );
            spec.style = "comets".into();
            spec.inertia = Some(inertia);
            view.particles = Some(spec);
            if crate::heavy::latest_map(&project.root, &job).is_some() {
                view.surface_field = Some("impact".into());
            }
        }
        // Long glowing trails (Task_35): from the inlets inside a part, else from an area
        // upstream of the model.
        "light_trails" => {
            let region = if is_internal(project) {
                SeedRegionSpec {
                    kind: "inlets".into(),
                    origin: [0.0; 3],
                    u: [0.0; 3],
                    v: [0.0; 3],
                }
            } else {
                SeedRegionSpec {
                    kind: "area".into(),
                    origin: w([upstream, c[1] - 0.6 * size[1], z0]),
                    u: w([0.0, 1.2 * size[1], 0.0]),
                    v: w([0.0, 0.0, height]),
                }
            };
            let mut spec = smoke(12_000, region);
            spec.style = "trails".into();
            spec.glow = Some(true);
            view.particles = Some(spec);
        }
        // Ribbons from the rake (or the inlets), coloured by the swirl around them.
        "ribbons" => {
            let region = if is_internal(project) {
                SeedRegionSpec {
                    kind: "inlets".into(),
                    origin: [0.0; 3],
                    u: [0.0; 3],
                    v: [0.0; 3],
                }
            } else {
                SeedRegionSpec {
                    kind: "rake".into(),
                    origin: w([upstream, c[1], z0]),
                    u: w([0.0, 0.0, height]),
                    v: [0.5 * step[0], 0.0, 0.0],
                }
            };
            let mut spec = smoke(200, region);
            spec.style = "ribbons".into();
            spec.color_by = "swirl".into();
            view.particles = Some(spec);
        }
        // Wool tufts on the map "attached / reverse flow" (wind tunnel with the vector).
        "tufts" => {
            view.surface_flow = Some(crate::surface_flow::merged(
                None,
                crate::surface_flow::SetParams {
                    tufts: Some(crate::surface_flow::TuftsParams {
                        on: Some(true),
                        ..Default::default()
                    }),
                    ..Default::default()
                },
            )?);
            let has_map = view.result_job.as_deref().is_some_and(|job| {
                crate::results::load_surface(&project.root, job)
                    .is_ok_and(|s| s.field("attached").is_some())
            });
            if has_map && !is_internal(project) {
                view.surface_field = Some("attached".into());
            }
        }
        _ => return Err(FoamError::new("view.unknown_preset", id)),
    }
    // Inside a closed part nothing is visible: cut the part open along its middle plane, across
    // its shortest extent (hiding the half the normal points to, the upper one for a flat part).
    if id.starts_with("internal_")
        || (is_internal(project) && ["oil_film", "tufts", "light_trails", "ribbons"].contains(&id))
    {
        let k = (0..3)
            .min_by(|a, b| size[*a].total_cmp(&size[*b]))
            .unwrap_or(2);
        let mut n = [0.0; 3];
        n[k] = 1.0;
        view.clip = Some(PlaneSpec {
            origin: w(c),
            normal: w(n),
        });
        view.clip_targets = vec!["model".into()];
    }
    Ok(view)
}

pub fn register(r: &mut Registry) {
    r.add(
        "view.presets",
        "Ready-made views that fit the run (wind tunnel or internal flow), each with the standard \
         view for the camera and the catalog code of a one-sentence explanation.",
        Options {
            undoable: false,
            errors: &["project.none_open"],
            example: json!({}),
        },
        |s: &mut Session, _: crate::results::JobParams| Ok(presets(s.project()?)),
    );
    r.add(
        "view.apply_preset",
        "Replace the scene of the view with a ready-made view: smoke_lines, smoke_curtain, \
         mid_slice, surface_pressure, vortices, wake, internal_path, internal_slice. One undo step.",
        Options {
            undoable: true,
            errors: &["project.none_open", "results.none", "view.unknown_preset"],
            example: json!({"id": "smoke_lines"}),
        },
        |s: &mut Session, p: ApplyParams| {
            let project = s.project()?;
            let view = build(project, &p.id)?;
            project.manifest.view = view.clone();
            Ok(view)
        },
    );
}
