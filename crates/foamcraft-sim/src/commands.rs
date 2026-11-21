// SPDX-License-Identifier: GPL-3.0-or-later
//! scenario.* commands.

use foamcraft_core::command::{Options, Registry, Session};
use foamcraft_core::error::{FoamError, Language, Message, message};
use foamcraft_core::project::{MediumSpec, Project, Scenario, WindTunnel};
use foamcraft_geometry::Mesh;
use foamcraft_geometry::commands::world_meshes;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::{
    CellBudget, MediumProperties, TunnelBox, cell_budget, flow_class, flow_direction, frontal_area,
    length_along, mach_number, medium_properties, tunnel_box, unit_suspect,
};

/// Mach numbers above this make the incompressible solvers invalid.
pub const MACH_LIMIT: f64 = 0.3;

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct NoParams {}

#[derive(Debug, Serialize, JsonSchema)]
pub struct Localized {
    pub code: String,
    pub en: Message,
    pub de: Message,
}

pub(crate) fn localized(code: &str) -> Localized {
    let get = |lang| {
        message(code, lang).cloned().unwrap_or(Message {
            text: code.into(),
            fix: String::new(),
        })
    };
    Localized {
        code: code.into(),
        en: get(Language::En),
        de: get(Language::De),
    }
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct WindTunnelInfo {
    pub scenario: Scenario,
    pub medium: MediumProperties,
    /// Projected area of all models normal to the flow, m².
    pub frontal_area_m2: f64,
    /// Length of the models along the flow, m.
    pub reference_length_m: f64,
    pub reynolds: f64,
    pub mach: Option<f64>,
    pub flow: Localized,
    pub tunnel: TunnelBox,
    pub budget: CellBudget,
    /// Memory and cores the budget was computed for.
    pub memory_bytes: u64,
    pub cores: u32,
    pub warnings: Vec<Localized>,
    /// Sources of air (empty: the whole inlet wall blows).
    pub jets: Vec<foamcraft_core::project::JetSource>,
}

/// Total memory and physical cores of this computer.
pub(crate) fn machine() -> (u64, u32) {
    let cores = sysinfo::System::physical_core_count().unwrap_or(1) as u32;
    let memory = sysinfo::System::new()
        .map(|mut sys| {
            sys.refresh_memory();
            sys.total_memory()
        })
        // Without system information assume a modest 8 GB.
        .unwrap_or(8 * 1024 * 1024 * 1024);
    (memory, cores)
}

pub(crate) fn invalid(what: &str) -> FoamError {
    FoamError::new("scenario.invalid_input", what)
}

fn validate(w: &WindTunnel) -> Result<(), FoamError> {
    if !(w.speed_m_s.is_finite() && w.speed_m_s > 0.0) {
        return Err(invalid("speed_m_s must be positive"));
    }
    if !w.direction_deg.is_finite() {
        return Err(invalid("direction_deg"));
    }
    validate_medium(&w.medium)
}

/// Temperatures inside the range of the medium tables, positive custom values.
pub(crate) fn validate_medium(medium: &MediumSpec) -> Result<(), FoamError> {
    match *medium {
        MediumSpec::Air { temperature_c } if !(-50.0..=150.0).contains(&temperature_c) => {
            Err(invalid("air temperature must lie between -50 and 150 °C"))
        }
        MediumSpec::Water { temperature_c } if !(0.5..=99.0).contains(&temperature_c) => {
            Err(invalid("water temperature must lie between 0.5 and 99 °C"))
        }
        MediumSpec::Custom {
            density,
            kinematic_viscosity,
        } if !(density > 0.0 && kinematic_viscosity > 0.0) => {
            Err(invalid("density and kinematic viscosity must be positive"))
        }
        _ => Ok(()),
    }
}

fn evaluate(project: &Project, scenario: &Scenario) -> Result<WindTunnelInfo, FoamError> {
    let Scenario::WindTunnel(w) = scenario else {
        return Err(FoamError::new("scenario.none", "not a wind tunnel"));
    };
    let models: Vec<(String, Mesh)> = world_meshes(project);
    if models.is_empty() {
        return Err(FoamError::new("scenario.no_geometry", ""));
    }
    let meshes: Vec<&Mesh> = models.iter().map(|(_, m)| m).collect();
    let dir = flow_direction(w.direction_deg);
    let mut min = [f64::MAX; 3];
    let mut max = [f64::MIN; 3];
    for m in &meshes {
        let (lo, hi) = m.bounds();
        for k in 0..3 {
            min[k] = min[k].min(f64::from(lo[k]));
            max[k] = max[k].max(f64::from(hi[k]));
        }
    }
    let largest = (0..3).map(|k| max[k] - min[k]).fold(0.0, f64::max);
    let medium = medium_properties(&w.medium);
    let reference_length_m = length_along(&meshes, dir);
    let reynolds = w.speed_m_s * reference_length_m / medium.kinematic_viscosity;
    let mach = mach_number(w.speed_m_s, &medium);
    let (memory_bytes, cores) = machine();
    let mut warnings = Vec::new();
    if mach.is_some_and(|m| m > MACH_LIMIT) {
        warnings.push(localized("scenario.mach_high"));
    }
    if unit_suspect(largest) {
        warnings.push(localized("scenario.unit_suspect"));
    }
    let jets = project.manifest.jets.clone();
    if !jets.is_empty() {
        let refs: Vec<&Mesh> = meshes.clone();
        for code in crate::jets::check_jets(&refs, &jets) {
            warnings.push(localized(code));
        }
    }
    Ok(WindTunnelInfo {
        scenario: scenario.clone(),
        frontal_area_m2: frontal_area(&meshes, dir),
        reference_length_m,
        reynolds,
        mach,
        flow: localized(flow_class(reynolds)),
        tunnel: tunnel_box(min, max, w.direction_deg, w.ground),
        budget: cell_budget(memory_bytes, cores, w.quality),
        memory_bytes,
        cores,
        medium,
        warnings,
        jets,
    })
}

/// Parameters of `scenario.set_jets`.
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct JetsParams {
    /// Up to four sources; an empty list makes the whole inlet wall blow again.
    pub jets: Vec<foamcraft_core::project::JetSource>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct MoveJetParams {
    /// Position of the source in the list (from 0).
    pub index: usize,
    /// Shift in metres along world X, Y, Z.
    pub delta: [f64; 3],
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RotateJetParams {
    pub index: usize,
    /// World axis: x, y or z.
    pub axis: String,
    pub angle_deg: f64,
}

/// Changes one source of the active scenario and evaluates it again.
fn change_jet(
    s: &mut Session,
    index: usize,
    change: impl FnOnce(&mut foamcraft_core::project::JetSource),
) -> Result<WindTunnelInfo, FoamError> {
    let project = s.project()?;
    let scenario = project
        .manifest
        .scenario
        .clone()
        .ok_or_else(|| FoamError::new("scenario.none", ""))?;
    let jet = project
        .manifest
        .jets
        .get_mut(index)
        .ok_or_else(|| invalid("no source with this index"))?;
    change(jet);
    check_jet(jet)?;
    evaluate(project, &scenario)
}

fn check_jet(j: &foamcraft_core::project::JetSource) -> Result<(), FoamError> {
    let finite = j
        .center
        .iter()
        .chain(&j.direction)
        .chain(&j.size)
        .all(|x| x.is_finite());
    let length = j.direction.iter().map(|x| x * x).sum::<f64>().sqrt();
    if !finite || length < 1e-9 {
        return Err(invalid("jet position and direction"));
    }
    if !["circle", "rectangle"].contains(&j.shape.as_str()) {
        return Err(invalid("jet shape must be circle or rectangle"));
    }
    let size_ok = j.size[0] > 0.0 && (j.shape == "circle" || j.size[1] > 0.0);
    if !size_ok || !(j.speed_m_s.is_finite() && j.speed_m_s > 0.0) {
        return Err(invalid("jet size and speed must be positive"));
    }
    Ok(())
}

/// Parameters of `scenario.set_wind_direction`.
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct WindDirection {
    /// Direction the wind blows towards, degrees in the ground plane: 0 = +X, 90 = +Y.
    pub direction_deg: f64,
}

pub fn register(r: &mut Registry) {
    r.add(
        "scenario.set_wind_tunnel",
        "Set up external flow in a virtual wind tunnel; returns reference values and the cell budget.",
        Options {
            undoable: true,
            errors: &["project.none_open", "scenario.no_geometry", "scenario.invalid_input"],
            example: json!({
                "speed_m_s": 20.0,
                "direction_deg": 0.0,
                "ground": "fixed",
                "medium": {"kind": "air", "temperature_c": 20.0},
                "quality": "standard"
            }),
        },
        |s: &mut Session, p: WindTunnel| {
            validate(&p)?;
            let project = s.project()?;
            let scenario = Scenario::WindTunnel(p);
            let info = evaluate(project, &scenario)?;
            project.manifest.scenario = Some(scenario);
            Ok(info)
        },
    );
    r.add(
        "scenario.set_wind_direction",
        "Turn the wind of the wind tunnel; everything else stays. Degrees, 0 = towards +X, 90 = towards +Y.",
        Options {
            undoable: true,
            errors: &[
                "project.none_open",
                "scenario.not_wind_tunnel",
                "scenario.no_geometry",
                "scenario.invalid_input",
            ],
            example: json!({"direction_deg": 30.0}),
        },
        |s: &mut Session, p: WindDirection| {
            let project = s.project()?;
            let Some(Scenario::WindTunnel(w)) = project.manifest.scenario.clone() else {
                return Err(FoamError::new("scenario.not_wind_tunnel", ""));
            };
            if !p.direction_deg.is_finite() {
                return Err(FoamError::new("scenario.invalid_input", "direction_deg"));
            }
            let mut w = w;
            // Keep the angle in -180..180 so the same direction is stored the same way.
            w.direction_deg = (p.direction_deg + 180.0).rem_euclid(360.0) - 180.0;
            validate(&w)?;
            let scenario = Scenario::WindTunnel(w);
            let info = evaluate(project, &scenario)?;
            project.manifest.scenario = Some(scenario);
            Ok(info)
        },
    );
    r.add(
        "scenario.set_jets",
        "Let the air come from up to four sources (round or rectangular openings with centre, \
         direction, size and speed, like fans or nozzles) in still surroundings instead of the \
         whole inlet wall; an empty list goes back to the wind tunnel.",
        Options {
            undoable: true,
            errors: &[
                "project.none_open",
                "scenario.not_wind_tunnel",
                "scenario.no_geometry",
                "scenario.invalid_input",
            ],
            example: json!({"jets": [{"shape": "circle", "center": [-0.3, 0.0, 0.1],
                "direction": [1.0, 0.0, 0.0], "size": [0.08, 0.08], "speed_m_s": 15.0}]}),
        },
        |s: &mut Session, p: JetsParams| {
            if p.jets.len() > 4 {
                return Err(invalid("at most four jets"));
            }
            for j in &p.jets {
                check_jet(j)?;
            }
            let project = s.project()?;
            let Some(Scenario::WindTunnel(mut w)) = project.manifest.scenario.clone() else {
                return Err(FoamError::new("scenario.not_wind_tunnel", ""));
            };
            // The reference speed is the fastest jet; the case lines up with the first one.
            if let Some(v) = p.jets.iter().map(|j| j.speed_m_s).reduce(f64::max) {
                w.speed_m_s = v;
            }
            if let Some(dir) = crate::jets::direction_of(&p.jets) {
                w.direction_deg = (dir + 180.0).rem_euclid(360.0) - 180.0;
            }
            let scenario = Scenario::WindTunnel(w);
            project.manifest.jets = p.jets;
            let info = evaluate(project, &scenario)?;
            project.manifest.scenario = Some(scenario);
            Ok(info)
        },
    );
    r.add(
        "scenario.move_jet",
        "Move one source of air by a distance along the world axes (metres).",
        Options {
            undoable: true,
            errors: &[
                "project.none_open",
                "scenario.none",
                "scenario.invalid_input",
            ],
            example: json!({"index": 0, "delta": [0.0, 0.0, 0.05]}),
        },
        |s: &mut Session, p: MoveJetParams| {
            change_jet(s, p.index, |j| {
                for k in 0..3 {
                    j.center[k] += p.delta[k];
                }
            })
        },
    );
    r.add(
        "scenario.rotate_jet",
        "Turn the direction of one source of air about a world axis (degrees).",
        Options {
            undoable: true,
            errors: &[
                "project.none_open",
                "scenario.none",
                "scenario.invalid_input",
            ],
            example: json!({"index": 0, "axis": "z", "angle_deg": 15.0}),
        },
        |s: &mut Session, p: RotateJetParams| {
            let axis = match p.axis.as_str() {
                "x" => glam::DVec3::X,
                "y" => glam::DVec3::Y,
                "z" => glam::DVec3::Z,
                _ => return Err(invalid("axis must be x, y or z")),
            };
            let turn = glam::DMat3::from_axis_angle(axis, p.angle_deg.to_radians());
            change_jet(s, p.index, |j| {
                j.direction = (turn * glam::DVec3::from(j.direction)).into();
            })
        },
    );
    r.add(
        "scenario.info",
        "Describe the scenario of the open project with its reference values.",
        Options {
            undoable: false,
            errors: &["project.none_open", "scenario.none", "scenario.no_geometry"],
            example: json!({}),
        },
        |s: &mut Session, _: NoParams| {
            let project = s.project()?;
            let scenario = project
                .manifest
                .scenario
                .clone()
                .ok_or_else(|| FoamError::new("scenario.none", ""))?;
            let value = match &scenario {
                Scenario::InternalFlow(flow) => {
                    serde_json::to_value(crate::internal::evaluate(project, flow)?)
                }
                Scenario::WindTunnel(_) => serde_json::to_value(evaluate(project, &scenario)?),
            };
            value.map_err(|e| FoamError::new("command.internal", e.to_string()))
        },
    );
    r.add(
        "scenario.set_internal_flow",
        "Set up flow through the models from inlets to outlets (caps or faces with a boundary role);          returns hydraulic diameters, Reynolds number and laminar or turbulent.",
        Options {
            undoable: true,
            errors: &[
                "project.none_open",
                "scenario.invalid_input",
                "scenario.no_inlet",
                "scenario.no_outlet",
                "scenario.duplicate_boundary",
                "scenario.role_mismatch",
                "scenario.no_driving_pressure",
                "surface.not_found",
            ],
            example: json!({
                "medium": {"kind": "water", "temperature_c": 20.0},
                "quality": "draft",
                "inlets": [{"key": "model-1:cap-1", "condition": {"type": "velocity", "m_s": 0.5}}],
                "outlets": [{"key": "model-1:cap-2", "pressure_pa": 0.0}]
            }),
        },
        |s: &mut Session, p: foamcraft_core::project::InternalFlow| {
            let project = s.project()?;
            crate::internal::validate(project, &p)?;
            let info = crate::internal::evaluate(project, &p)?;
            project.manifest.scenario = Some(Scenario::InternalFlow(p));
            Ok(info)
        },
    );
}
