// SPDX-License-Identifier: GPL-3.0-or-later
//! Internal flow case: mesh plan for the fluid region, boundary conditions per opening, function
//! objects for flow rates and mean pressures.

use std::fmt::Write as _;
use std::path::Path;

use foamcraft_core::case::INTERNAL_FLOW;
use foamcraft_core::error::FoamError;
use foamcraft_core::project::InletCondition;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::MediumProperties;
use crate::case::{ModelSurface, surface_stl};
use crate::solve::inlet_turbulence;

/// Turbulence intensity at inlets of internal flows.
pub const INTERNAL_INTENSITY: f64 = 0.05;
/// Turbulent length scale at inlets as a share of the hydraulic diameter.
pub const INTERNAL_LENGTH_SHARE: f64 = 0.07;
/// Below this many cells across the smallest opening the result gets a hint.
pub const MIN_CELLS_ACROSS: f64 = 10.0;
/// Region name of the walls in the surface file.
pub const WALL_REGION: &str = "wall";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct InternalPlan {
    /// Edge length of the cells, m.
    pub cell: f64,
    pub box_min: [f64; 3],
    pub box_max: [f64; 3],
    pub cells: [u32; 3],
    /// Point inside the fluid region.
    pub location: [f64; 3],
    /// Estimated cells in the fluid region.
    pub estimated_cells: u64,
    pub cells_across: f64,
    pub turbulent: bool,
}

/// Cell size from the fluid volume and the budget; the background box (bounds plus two cells)
/// may hold at most three times the budget.
pub fn plan_internal(
    bounds_min: [f64; 3],
    bounds_max: [f64; 3],
    fluid_volume: f64,
    location: [f64; 3],
    smallest_hydraulic_diameter: f64,
    budget: u64,
    turbulent: bool,
) -> InternalPlan {
    let budget = budget.max(1000) as f64;
    let size: Vec<f64> = (0..3)
        .map(|k| (bounds_max[k] - bounds_min[k]).max(1e-9))
        .collect();
    let box_volume: f64 = size.iter().product();
    let by_fluid = (fluid_volume.max(1e-30) / (0.95 * budget)).cbrt();
    let by_box = (box_volume / (3.0 * budget)).cbrt();
    let cell = by_fluid.max(by_box);
    let box_min = [0, 1, 2].map(|k| bounds_min[k] - 2.0 * cell);
    let cells = [0, 1, 2].map(|k| ((size[k] + 4.0 * cell) / cell).ceil().max(1.0) as u32);
    let box_max = [0, 1, 2].map(|k| box_min[k] + f64::from(cells[k]) * cell);
    InternalPlan {
        cell,
        box_min,
        box_max,
        cells,
        location,
        estimated_cells: (fluid_volume / cell.powi(3)).round() as u64,
        cells_across: smallest_hydraulic_diameter / cell,
        turbulent,
    }
}

/// An inlet or outlet of the case.
#[derive(Debug, Clone, PartialEq)]
pub struct Boundary {
    /// Region in the surface file, for example `in_1`; the patch is `model_<region>`.
    pub region: String,
    pub inlet: Option<InletCondition>,
    /// Static pressure at an outlet (Pa above the reference).
    pub outlet_pa: Option<f64>,
    pub area_m2: f64,
    pub hydraulic_diameter_m: f64,
}

impl Boundary {
    pub fn patch(&self) -> String {
        format!("model_{}", self.region)
    }
}

fn header(class: &str, object: &str) -> String {
    format!(
        "/*--------------------------------*- C++ -*----------------------------------*\\\n  \
         SPDX-License-Identifier: GPL-3.0-or-later\n  \
         FoamCraft internal flow case (generated; do not edit).\n\
         \\*---------------------------------------------------------------------------*/\n\
         FoamFile\n{{\n    version     2.0;\n    format      ascii;\n    class       {class};\n    \
         object      {object};\n}}\n\n"
    )
}

/// Mean inlet velocity of a flow inlet (for turbulence values).
fn inlet_speed(b: &Boundary, density: f64) -> f64 {
    match b.inlet {
        Some(InletCondition::Velocity { m_s }) => m_s,
        Some(InletCondition::VolumeFlow { m3_s }) => m3_s / b.area_m2.max(1e-300),
        Some(InletCondition::MassFlow { kg_s }) => kg_s / (density * b.area_m2.max(1e-300)),
        Some(InletCondition::TotalPressure { pa }) => (2.0 * pa.max(0.0) / density).sqrt(),
        None => 0.0,
    }
}

fn field(class: &str, object: &str, dims: &str, internal: &str, patches: &str) -> String {
    format!(
        "{}dimensions      {dims};\n\ninternalField   {internal};\n\nboundaryField\n{{\n    \
         \".*\" {{ type zeroGradient; }}\n{patches}}}\n",
        header(class, object)
    )
}

/// The `0/` fields; kinematic pressure (p / rho).
pub fn fields(
    boundaries: &[Boundary],
    medium: &MediumProperties,
    turbulent: bool,
) -> Vec<(String, String)> {
    let rho = medium.density;
    // Walls: the patch group of the model walls and the (normally empty) background patch.
    let walls = |entry: &str| format!("    walls {{ {entry} }}\n    background {{ {entry} }}\n");
    let mut u = walls("type noSlip;");
    let mut p = String::new();
    let mut k = walls("type kqRWallFunction; value uniform 1e-8;");
    let mut omega = walls("type omegaWallFunction; value uniform 1;");
    let nut = walls("type nutkWallFunction; value uniform 0;");
    let first_inlet = boundaries.iter().find(|b| b.inlet.is_some());
    let turb = |b: &Boundary| {
        inlet_turbulence(
            inlet_speed(b, rho).max(1e-6),
            INTERNAL_INTENSITY,
            INTERNAL_LENGTH_SHARE * b.hydraulic_diameter_m,
        )
    };
    for b in boundaries {
        let name = b.patch();
        match (b.inlet, b.outlet_pa) {
            (Some(InletCondition::TotalPressure { pa }), _) => {
                let _ = writeln!(
                    u,
                    "    {name} {{ type pressureInletOutletVelocity; value uniform (0 0 0); }}"
                );
                let _ = writeln!(
                    p,
                    "    {name} {{ type totalPressure; p0 uniform {:e}; value uniform {:e}; }}",
                    pa / rho,
                    pa / rho
                );
            }
            (Some(InletCondition::MassFlow { kg_s }), _) => {
                let _ = writeln!(
                    u,
                    "    {name} {{ type flowRateInletVelocity; massFlowRate constant {kg_s:e}; \
                     rhoInlet {rho:e}; value uniform (0 0 0); }}"
                );
            }
            (Some(c), _) => {
                let q = match c {
                    InletCondition::Velocity { m_s } => m_s * b.area_m2,
                    InletCondition::VolumeFlow { m3_s } => m3_s,
                    _ => 0.0,
                };
                let _ = writeln!(
                    u,
                    "    {name} {{ type flowRateInletVelocity; volumetricFlowRate constant {q:e}; \
                     value uniform (0 0 0); }}"
                );
            }
            (None, Some(pa)) => {
                let _ = writeln!(
                    u,
                    "    {name} {{ type inletOutlet; inletValue uniform (0 0 0); value uniform (0 0 0); }}"
                );
                let _ = writeln!(
                    p,
                    "    {name} {{ type fixedValue; value uniform {:e}; }}",
                    pa / rho
                );
            }
            (None, None) => {}
        }
        if b.inlet.is_some() {
            let t = turb(b);
            let _ = writeln!(
                k,
                "    {name} {{ type fixedValue; value uniform {:e}; }}",
                t.k
            );
            let _ = writeln!(
                omega,
                "    {name} {{ type fixedValue; value uniform {:e}; }}",
                t.omega
            );
        } else {
            let t = first_inlet
                .map(turb)
                .unwrap_or(inlet_turbulence(1.0, 0.05, 0.01));
            let _ = writeln!(
                k,
                "    {name} {{ type inletOutlet; inletValue uniform {:e}; value uniform {:e}; }}",
                t.k, t.k
            );
            let _ = writeln!(
                omega,
                "    {name} {{ type inletOutlet; inletValue uniform {:e}; value uniform {:e}; }}",
                t.omega, t.omega
            );
        }
    }
    let start = first_inlet
        .map(turb)
        .unwrap_or(inlet_turbulence(1.0, 0.05, 0.01));
    let mut out = vec![
        (
            "0/U".to_string(),
            field(
                "volVectorField",
                "U",
                "[0 1 -1 0 0 0 0]",
                "uniform (0 0 0)",
                &u,
            ),
        ),
        (
            "0/p".to_string(),
            field("volScalarField", "p", "[0 2 -2 0 0 0 0]", "uniform 0", &p),
        ),
    ];
    if turbulent {
        out.push((
            "0/k".into(),
            field(
                "volScalarField",
                "k",
                "[0 2 -2 0 0 0 0]",
                &format!("uniform {:e}", start.k),
                &k,
            ),
        ));
        out.push((
            "0/omega".into(),
            field(
                "volScalarField",
                "omega",
                "[0 0 -1 0 0 0 0]",
                &format!("uniform {:e}", start.omega),
                &omega,
            ),
        ));
        out.push((
            "0/nut".into(),
            field(
                "volScalarField",
                "nut",
                "[0 2 -1 0 0 0 0]",
                "uniform 0",
                &nut,
            ),
        ));
    }
    out
}

/// Prefix of the flow rate function objects.
pub const FLOW_PREFIX: &str = "flow_";
/// Prefix of the mean pressure function objects.
pub const PRESSURE_PREFIX: &str = "pressure_";

/// Flow rate (sum of phi) and area-averaged pressure per opening, logged every iteration.
pub fn internal_functions(boundaries: &[Boundary]) -> String {
    let mut out = String::new();
    for b in boundaries {
        let patch = b.patch();
        for (prefix, op, f) in [
            (FLOW_PREFIX, "sum", "phi"),
            (PRESSURE_PREFIX, "areaAverage", "p"),
        ] {
            let _ = write!(
                out,
                "    {prefix}{region}\n    {{\n        type surfaceFieldValue;\n        libs (fieldFunctionObjects);\n        \
                 writeControl timeStep;\n        writeInterval 1;\n        log true;\n        writeFields false;\n        \
                 regionType patch;\n        name {patch};\n        operation {op};\n        fields ({f});\n    }}\n",
                region = b.region
            );
        }
    }
    out.push_str(
        "    stop\n    {\n        type abort;\n        libs (utilityFunctionObjects);\n        file \"<case>/stop\";\n        action writeNow;\n    }\n",
    );
    out
}

/// Writes the complete case into `dir`.
pub fn write_internal_case(
    dir: &Path,
    plan: &InternalPlan,
    models: &[ModelSurface<'_>],
    boundaries: &[Boundary],
    medium: &MediumProperties,
    iterations: u32,
) -> Result<(), FoamError> {
    let v3 = |v: [f64; 3]| format!("{:.6e} {:.6e} {:.6e}", v[0], v[1], v[2]);
    let mut regions = String::new();
    let _ = writeln!(
        regions,
        "                {WALL_REGION} {{ level (0 0); patchInfo {{ type wall; inGroups (walls); }} }}"
    );
    for b in boundaries {
        let _ = writeln!(
            regions,
            "                {} {{ level (0 0); patchInfo {{ type patch; }} }}",
            b.region
        );
    }
    let turbulence = if plan.turbulent {
        "simulationType  RAS;\nRAS\n{\n    RASModel        kOmegaSST;\n    turbulence      on;\n    printCoeffs     on;\n}"
    } else {
        "simulationType  laminar;"
    };
    let [x0, y0, z0] = plan.box_min;
    let [x1, y1, z1] = plan.box_max;
    let budget = plan.estimated_cells.max(1);
    let values: Vec<(&str, String)> = vec![
        ("X0", format!("{x0:.6e}")),
        ("Y0", format!("{y0:.6e}")),
        ("Z0", format!("{z0:.6e}")),
        ("X1", format!("{x1:.6e}")),
        ("Y1", format!("{y1:.6e}")),
        ("Z1", format!("{z1:.6e}")),
        ("NX", plan.cells[0].to_string()),
        ("NY", plan.cells[1].to_string()),
        ("NZ", plan.cells[2].to_string()),
        ("MAX_LOCAL", (budget * 4).max(1_000_000).to_string()),
        ("MAX_GLOBAL", (budget * 8).max(2_000_000).to_string()),
        ("SURFACE_LEVEL", "0".into()),
        ("REGIONS", regions),
        ("LOCATION", v3(plan.location)),
        ("NU", format!("{:e}", medium.kinematic_viscosity)),
        ("TURBULENCE", turbulence.into()),
        ("END_TIME", iterations.to_string()),
        ("FUNCTIONS", internal_functions(boundaries)),
    ];
    let refs: Vec<(&str, &str)> = values.iter().map(|(k, v)| (*k, v.as_str())).collect();
    INTERNAL_FLOW.write(dir, &refs)?;
    let io = |e: std::io::Error| FoamError::new("job.storage_failed", e.to_string());
    std::fs::create_dir_all(dir.join("0")).map_err(io)?;
    for (path, text) in fields(boundaries, medium, plan.turbulent) {
        std::fs::write(dir.join(path), text).map_err(io)?;
    }
    let surfaces = dir.join("constant").join("triSurface");
    std::fs::create_dir_all(&surfaces).map_err(io)?;
    std::fs::write(surfaces.join("model.stl"), surface_stl(models, 0.0)).map_err(io)?;
    let json = serde_json::to_string_pretty(plan)
        .map_err(|e| FoamError::new("job.storage_failed", e.to_string()))?;
    std::fs::write(dir.join("foamcraft-mesh-plan.json"), json).map_err(io)
}
