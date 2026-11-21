// SPDX-License-Identifier: GPL-3.0-or-later
//! Internal flow: hydraulic diameter, laminar or turbulent, plausibility of the boundary
//! conditions and `scenario.set_internal_flow`.

use foamcraft_core::error::FoamError;
use foamcraft_core::project::{BoundaryRole, InletCondition, InternalFlow, Project, Scenario};
use foamcraft_geometry::caps::{BoundarySize, boundary_size};
use std::collections::BTreeMap;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::commands::{Localized, invalid, localized, machine, validate_medium};
use crate::{CellBudget, MediumProperties, cell_budget, medium_properties};

/// Below this Reynolds number pipe flow is laminar.
pub const LAMINAR_LIMIT: f64 = 2300.0;
/// Above this Reynolds number pipe flow is fully turbulent.
pub const TURBULENT_LIMIT: f64 = 4000.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum FlowRegime {
    Laminar,
    /// Between laminar and turbulent; computed as turbulent.
    Transitional,
    Turbulent,
}

impl FlowRegime {
    /// Turbulence model used for the regime.
    pub fn turbulent(self) -> bool {
        self != FlowRegime::Laminar
    }
}

/// D_h = 4 A / P.
pub fn hydraulic_diameter(area: f64, perimeter: f64) -> f64 {
    4.0 * area / perimeter.max(1e-300)
}

pub fn regime(reynolds: f64) -> FlowRegime {
    if reynolds < LAMINAR_LIMIT {
        FlowRegime::Laminar
    } else if reynolds <= TURBULENT_LIMIT {
        FlowRegime::Transitional
    } else {
        FlowRegime::Turbulent
    }
}

/// Mean velocity through an inlet; for a total pressure inlet the velocity the pressure
/// difference to the lowest outlet pressure could reach at most.
pub fn inlet_velocity(
    condition: &InletCondition,
    area: f64,
    density: f64,
    lowest_outlet_pa: f64,
) -> f64 {
    match *condition {
        InletCondition::Velocity { m_s } => m_s,
        InletCondition::VolumeFlow { m3_s } => m3_s / area.max(1e-300),
        InletCondition::MassFlow { kg_s } => kg_s / (density * area.max(1e-300)),
        InletCondition::TotalPressure { pa } => {
            (2.0 * (pa - lowest_outlet_pa).max(0.0) / density).sqrt()
        }
    }
}

#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct OpeningValues {
    pub key: String,
    pub role: BoundaryRole,
    pub area_m2: f64,
    pub hydraulic_diameter_m: f64,
    /// Mean velocity (inlets), m/s.
    pub velocity_m_s: Option<f64>,
    #[serde(flatten)]
    pub size: BoundarySize,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct InternalFlowInfo {
    pub scenario: Scenario,
    pub medium: MediumProperties,
    pub openings: Vec<OpeningValues>,
    /// Reynolds number of the inlet with the largest value (velocity and hydraulic diameter).
    pub reynolds: f64,
    pub regime: FlowRegime,
    pub flow: Localized,
    pub budget: CellBudget,
    pub memory_bytes: u64,
    pub cores: u32,
    pub warnings: Vec<Localized>,
}

fn positive(v: f64, what: &str) -> Result<(), FoamError> {
    if v.is_finite() && v > 0.0 {
        Ok(())
    } else {
        Err(invalid(&format!("{what} must be positive")))
    }
}

/// Checks the scenario against the boundary roles of the project.
pub fn validate(project: &Project, flow: &InternalFlow) -> Result<(), FoamError> {
    validate_medium(&flow.medium)?;
    if flow.inlets.is_empty() {
        return Err(FoamError::new("scenario.no_inlet", ""));
    }
    if flow.outlets.is_empty() {
        return Err(FoamError::new("scenario.no_outlet", ""));
    }
    let roles = &project.manifest.boundary_roles;
    let mut seen = std::collections::BTreeSet::new();
    let keys = flow
        .inlets
        .iter()
        .map(|i| (&i.key, BoundaryRole::Inlet))
        .chain(flow.outlets.iter().map(|o| (&o.key, BoundaryRole::Outlet)));
    for (key, role) in keys {
        if !seen.insert(key.clone()) {
            return Err(FoamError::new("scenario.duplicate_boundary", key.clone()));
        }
        if boundary_size(project, key).is_none() {
            return Err(FoamError::new("surface.not_found", key.clone()));
        }
        if roles.get(key) != Some(&role) {
            return Err(FoamError::new("scenario.role_mismatch", key.clone()));
        }
    }
    for o in &flow.outlets {
        if !o.pressure_pa.is_finite() {
            return Err(invalid("outlet pressure"));
        }
    }
    let highest_outlet = flow
        .outlets
        .iter()
        .map(|o| o.pressure_pa)
        .fold(f64::MIN, f64::max);
    for i in &flow.inlets {
        match i.condition {
            InletCondition::Velocity { m_s } => positive(m_s, "velocity")?,
            InletCondition::VolumeFlow { m3_s } => positive(m3_s, "volume flow")?,
            InletCondition::MassFlow { kg_s } => positive(kg_s, "mass flow")?,
            InletCondition::TotalPressure { pa } => {
                if !pa.is_finite() {
                    return Err(invalid("total pressure"));
                }
                if pa <= highest_outlet {
                    return Err(FoamError::new(
                        "scenario.no_driving_pressure",
                        format!("{pa} Pa at {} is not above {highest_outlet} Pa", i.key),
                    ));
                }
            }
        }
    }
    Ok(())
}

/// Reference values of an internal flow scenario.
pub fn evaluate(project: &Project, flow: &InternalFlow) -> Result<InternalFlowInfo, FoamError> {
    let medium = medium_properties(&flow.medium);
    let lowest_outlet = flow
        .outlets
        .iter()
        .map(|o| o.pressure_pa)
        .fold(f64::MAX, f64::min);
    let mut openings = Vec::new();
    let mut reynolds: f64 = 0.0;
    for i in &flow.inlets {
        let size = boundary_size(project, &i.key)
            .ok_or_else(|| FoamError::new("surface.not_found", i.key.clone()))?;
        let dh = hydraulic_diameter(size.area_m2, size.perimeter_m);
        let u = inlet_velocity(&i.condition, size.area_m2, medium.density, lowest_outlet);
        reynolds = reynolds.max(u * dh / medium.kinematic_viscosity);
        openings.push(OpeningValues {
            key: i.key.clone(),
            role: BoundaryRole::Inlet,
            area_m2: size.area_m2,
            hydraulic_diameter_m: dh,
            velocity_m_s: Some(u),
            size,
        });
    }
    for o in &flow.outlets {
        let size = boundary_size(project, &o.key)
            .ok_or_else(|| FoamError::new("surface.not_found", o.key.clone()))?;
        openings.push(OpeningValues {
            key: o.key.clone(),
            role: BoundaryRole::Outlet,
            area_m2: size.area_m2,
            hydraulic_diameter_m: hydraulic_diameter(size.area_m2, size.perimeter_m),
            velocity_m_s: None,
            size,
        });
    }
    let r = regime(reynolds);
    let mut warnings = Vec::new();
    if r == FlowRegime::Transitional {
        warnings.push(localized("scenario.transitional"));
    }
    let (memory_bytes, cores) = machine();
    Ok(InternalFlowInfo {
        scenario: Scenario::InternalFlow(flow.clone()),
        medium,
        openings,
        reynolds,
        regime: r,
        flow: localized(match r {
            FlowRegime::Laminar => "scenario.internal_laminar",
            FlowRegime::Transitional => "scenario.internal_transitional",
            FlowRegime::Turbulent => "scenario.internal_turbulent",
        }),
        budget: cell_budget(memory_bytes, cores, flow.quality),
        memory_bytes,
        cores,
        warnings,
    })
}

/// Values of `surfaceFieldValue` and `volFieldValue` objects (`log true`) per iteration: object
/// name → value.
#[derive(Debug, Default)]
pub struct FieldValueLog {
    /// Objects per iteration; a sample is complete when all have reported.
    expected: usize,
    current: BTreeMap<String, f64>,
    pending: Option<String>,
    pub samples: Vec<BTreeMap<String, f64>>,
    pub iterations: Vec<f64>,
    iteration: f64,
}

impl FieldValueLog {
    pub fn new(expected: usize) -> Self {
        Self {
            expected,
            ..Default::default()
        }
    }

    /// Feeds one log line; returns true when it completed a sample.
    pub fn feed(&mut self, line: &str) -> bool {
        let t = line.trim();
        if let Some(rest) = t.strip_prefix("Time = ") {
            self.iteration = rest.trim().parse().unwrap_or(self.iteration);
            self.current.clear();
            return false;
        }
        if let Some(rest) = t
            .strip_prefix("surfaceFieldValue ")
            .or_else(|| t.strip_prefix("volFieldValue "))
            && let Some(name) = rest.strip_suffix(" write:")
        {
            self.pending = Some(name.trim().to_string());
            return false;
        }
        if let Some(name) = self.pending.clone()
            && t.contains(" of ")
            && let Some((_, value)) = t.rsplit_once('=')
        {
            self.pending = None;
            let v = value.trim().parse().unwrap_or(f64::NAN);
            self.current.insert(name, v);
            if self.current.len() == self.expected {
                self.samples.push(std::mem::take(&mut self.current));
                self.iterations.push(self.iteration);
                return true;
            }
        }
        false
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum OpeningRole {
    Inlet,
    Outlet,
}

/// An opening of the case for the evaluation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct BoundaryValues {
    pub key: String,
    /// Region of the surface file (`in_1`); objects are `flow_<region>` and `pressure_<region>`.
    pub region: String,
    pub role: OpeningRole,
    pub area_m2: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct OpeningResult {
    pub key: String,
    pub role: OpeningRole,
    /// Volume flow through the opening in its direction (in at inlets, out at outlets), m³/s.
    pub flow_m3_s: f64,
    pub mass_flow_kg_s: f64,
    /// Share of the total outflow (outlets).
    pub share: Option<f64>,
    pub velocity_m_s: f64,
    /// Area-averaged static pressure, Pa above the reference.
    pub static_pressure_pa: f64,
    /// Static pressure plus the dynamic pressure of the mean velocity, Pa.
    pub total_pressure_pa: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct OpeningsEvaluation {
    pub openings: Vec<OpeningResult>,
    /// Flow-weighted total pressure of the inlets minus that of the outlets, Pa.
    pub pressure_loss_pa: f64,
    /// The same for the static pressure, Pa.
    pub static_pressure_loss_pa: f64,
    /// |inflow - outflow| / inflow.
    pub mass_balance_error: f64,
}

/// Means over the last `span` samples, per opening, and the derived values. Pressures in the
/// samples are kinematic (p / rho).
pub fn evaluate_openings(
    samples: &[BTreeMap<String, f64>],
    boundaries: &[BoundaryValues],
    density: f64,
    span: usize,
) -> OpeningsEvaluation {
    let tail = &samples[samples.len().saturating_sub(span.max(1))..];
    let mean = |name: &str| {
        let v: Vec<f64> = tail.iter().filter_map(|s| s.get(name).copied()).collect();
        v.iter().sum::<f64>() / v.len().max(1) as f64
    };
    let mut openings: Vec<OpeningResult> = boundaries
        .iter()
        .map(|b| {
            let sum = mean(&format!("flow_{}", b.region));
            let flow = match b.role {
                OpeningRole::Inlet => -sum,
                OpeningRole::Outlet => sum,
            };
            let velocity = flow.abs() / b.area_m2.max(1e-300);
            let p = mean(&format!("pressure_{}", b.region)) * density;
            OpeningResult {
                key: b.key.clone(),
                role: b.role,
                flow_m3_s: flow,
                mass_flow_kg_s: flow * density,
                share: None,
                velocity_m_s: velocity,
                static_pressure_pa: p,
                total_pressure_pa: p + 0.5 * density * velocity * velocity,
            }
        })
        .collect();
    let total = |role| -> f64 {
        openings
            .iter()
            .filter(|o| o.role == role)
            .map(|o| o.flow_m3_s)
            .sum()
    };
    let (inflow, outflow) = (total(OpeningRole::Inlet), total(OpeningRole::Outlet));
    for o in openings
        .iter_mut()
        .filter(|o| o.role == OpeningRole::Outlet)
    {
        o.share = Some(o.flow_m3_s / outflow.abs().max(1e-300));
    }
    let weighted = |role, f: &dyn Fn(&OpeningResult) -> f64| {
        let (mut sum, mut weight) = (0.0, 0.0);
        for o in openings.iter().filter(|o| o.role == role) {
            let w = o.flow_m3_s.abs().max(1e-300);
            sum += f(o) * w;
            weight += w;
        }
        sum / weight.max(1e-300)
    };
    let total_p = |o: &OpeningResult| o.total_pressure_pa;
    let static_p = |o: &OpeningResult| o.static_pressure_pa;
    OpeningsEvaluation {
        pressure_loss_pa: weighted(OpeningRole::Inlet, &total_p)
            - weighted(OpeningRole::Outlet, &total_p),
        static_pressure_loss_pa: weighted(OpeningRole::Inlet, &static_p)
            - weighted(OpeningRole::Outlet, &static_p),
        mass_balance_error: (inflow - outflow).abs() / inflow.abs().max(1e-300),
        openings,
    }
}

/// History of an internal flow run.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct InternalHistory {
    pub iterations: Vec<f64>,
    pub pressure_loss_pa: Vec<f64>,
    pub flow_m3_s: Vec<f64>,
    pub residual_iterations: Vec<f64>,
    pub residuals: BTreeMap<String, Vec<f64>>,
}

/// Result of an internal flow run, stored in `<project>/results/<job id>.json`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct InternalFlowResult {
    /// Always `internal_flow`.
    pub kind: String,
    pub job_id: String,
    pub reason: crate::solve::Verdict,
    pub iterations: u32,
    pub decided_at_iteration: u32,
    pub max_iterations: u32,
    #[serde(flatten)]
    pub evaluation: OpeningsEvaluation,
    /// Largest band of pressure loss or flow in the last window, percent of the mean.
    pub fluctuation_percent: f64,
    pub reynolds: f64,
    pub regime: FlowRegime,
    pub hints: Vec<String>,
    pub restarted: bool,
    pub cells: u64,
    pub cores: u32,
    pub timings_s: BTreeMap<String, f64>,
    pub scenario: InternalFlow,
    pub mesh: Option<foamcraft_core::project::MeshRecord>,
    pub history: InternalHistory,
}
