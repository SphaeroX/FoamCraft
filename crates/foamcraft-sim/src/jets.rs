// SPDX-License-Identifier: GPL-3.0-or-later
//! Free jets (Task_31): sources of air in still surroundings. A source is not a boundary but a
//! small cylinder in which the velocity is held at the jet velocity (`vectorFixedValueConstraint`,
//! selected geometrically), so it can sit anywhere and point anywhere. The tunnel box becomes a
//! box with open boundaries (`pressureInletOutletVelocity` / `totalPressure`), the mesh is refined
//! along each jet, and the solver runs with plain SIMPLE and calmer relaxation, which a start
//! from rest with a sudden jet needs (SIMPLEC at 0.9 diverged in the trial runs).

use std::fmt::Write as _;
use std::path::Path;

use foamcraft_core::error::FoamError;
use foamcraft_core::project::JetSource;
use foamcraft_geometry::Mesh;
use glam::DVec3;

use crate::mesh::{MeshPlan, inside_any};

/// Free-jet half angle used to size the refinement around a jet (about 11° for a round jet).
const SPREAD: f64 = 0.2;
/// Room left around a source, in source diameters.
const MARGIN_DIAMETERS: f64 = 10.0;
/// Cells across the source diameter at the source.
const CELLS_ACROSS: f64 = 10.0;
/// Function object that prints the mean speed in the domain every iteration.
pub const FLOW_OBJECT: &str = "jetFlow";

/// A source in the flow frame (x along the wind direction of the case).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct JetFlow {
    pub center: [f64; 3],
    /// Unit vector.
    pub direction: [f64; 3],
    /// Diameter; a rectangle counts with the diameter of the same area.
    pub diameter: f64,
    pub speed_m_s: f64,
}

/// Diameter of a source (rectangles: equal area).
pub fn diameter(j: &JetSource) -> f64 {
    if j.shape == "rectangle" {
        (4.0 * j.size[0] * j.size[1] / std::f64::consts::PI).sqrt()
    } else {
        j.size[0]
    }
}

/// The source in the flow frame of a case turned by `yaw_deg`.
pub fn to_flow(j: &JetSource, yaw_deg: f64) -> JetFlow {
    let r = crate::mesh::to_flow_frame(yaw_deg);
    let c = r * DVec3::from(j.center);
    let d = (r * DVec3::from(j.direction)).normalize_or_zero();
    JetFlow {
        center: c.into(),
        direction: d.into(),
        diameter: diameter(j),
        speed_m_s: j.speed_m_s,
    }
}

/// Wind direction that lines the case up with the first source (degrees in the ground plane).
pub fn direction_of(jets: &[JetSource]) -> Option<f64> {
    let d = jets.first()?.direction;
    (d[0].hypot(d[1]) > 1e-9).then(|| d[1].atan2(d[0]).to_degrees())
}

/// Refinement level that gives `CELLS_ACROSS` cells over a source of this diameter.
pub fn jet_level(plan: &MeshPlan, diameter: f64) -> u32 {
    let ratio = plan.background_cell * CELLS_ACROSS / diameter.max(1e-9);
    if ratio <= 1.0 {
        0
    } else {
        ratio.log2().ceil() as u32
    }
}

/// Grows the box so every source has room around it; the background cell size stays.
pub fn extend_plan(plan: &mut MeshPlan, jets: &[JetFlow]) {
    let h = plan.background_cell.max(1e-9);
    let mut lo = plan.tunnel_min;
    let mut hi = plan.tunnel_max;
    for j in jets {
        let m = MARGIN_DIAMETERS * j.diameter;
        for k in 0..3 {
            lo[k] = lo[k].min(j.center[k] - m);
            hi[k] = hi[k].max(j.center[k] + m);
        }
    }
    if plan.ground {
        // The floor stays where the model stands.
        lo[2] = plan.tunnel_min[2];
    }
    // A source far from a small model would need very many background cells: coarsen the
    // background in steps of two and refine every level one more, so the cells at the model
    // stay as they were.
    let before: f64 = plan
        .background_cells
        .iter()
        .map(|&c| f64::from(c))
        .product();
    let limit = 2.0 * before.max(25_000.0);
    let count = |h: f64| -> f64 {
        (0..3)
            .map(|k| ((hi[k] - lo[k]) / h).ceil().max(1.0))
            .product()
    };
    let mut steps = 0u32;
    let mut h = h;
    while count(h) > limit && steps < 8 {
        h *= 2.0;
        steps += 1;
    }
    plan.background_cell = h;
    plan.surface_level += steps;
    plan.near_level += steps;
    plan.wake_level += steps;
    for k in 0..3 {
        // Whole cells: the box ends on grid lines through the old corner.
        let below = ((plan.tunnel_min[k] - lo[k]) / h).ceil().max(0.0);
        lo[k] = plan.tunnel_min[k] - below * h;
        let cells = ((hi[k] - lo[k]) / h).ceil().max(1.0);
        hi[k] = lo[k] + cells * h;
        plan.background_cells[k] = cells as u32;
    }
    plan.tunnel_min = lo;
    plan.tunnel_max = hi;
}

/// Codes of plausibility warnings: a source inside a model, or one that blows past all models.
pub fn check_jets(models: &[&Mesh], jets: &[JetSource]) -> Vec<&'static str> {
    let mut lo = DVec3::splat(f64::MAX);
    let mut hi = DVec3::splat(f64::MIN);
    for m in models {
        let (a, b) = m.bounds();
        lo = lo.min(DVec3::from(a.map(f64::from)));
        hi = hi.max(DVec3::from(b.map(f64::from)));
    }
    let centre = (lo + hi) / 2.0;
    let radius = (hi - lo).length() / 2.0;
    let mut out = Vec::new();
    for j in jets {
        let code = if inside_any(models, j.center) {
            "scenario.jet_inside_model"
        } else {
            // Does the axis pass the bounding sphere ahead of the source?
            let o = DVec3::from(j.center);
            let d = DVec3::from(j.direction).normalize_or_zero();
            let t = (centre - o).dot(d);
            let miss = (o + d * t - centre).length();
            if t > 0.0 && miss <= radius + diameter(j) / 2.0 {
                continue;
            }
            "scenario.jet_misses_model"
        };
        if !out.contains(&code) {
            out.push(code);
        }
    }
    out
}

fn io(e: std::io::Error) -> FoamError {
    FoamError::new("job.storage_failed", e.to_string())
}

fn v3(p: DVec3) -> String {
    format!("({:.6} {:.6} {:.6})", p.x, p.y, p.z)
}

/// Rewrites the boundaries `inlet`, `outlet` and `sides` of a field file.
fn open_boundaries(text: &str, bc: impl Fn(&str) -> String) -> String {
    let mut out = String::new();
    for line in text.lines() {
        let t = line.trim_start();
        let name = ["inlet", "outlet", "sides"].into_iter().find(|n| {
            t.strip_prefix(n)
                .is_some_and(|r| r.starts_with(' ') || r.starts_with('{'))
        });
        match name {
            Some(n) => {
                let _ = writeln!(out, "    {n:<6} {}", bc(n));
            }
            None => {
                out.push_str(line);
                out.push('\n');
            }
        }
    }
    out
}

fn internal_value(text: &str) -> String {
    text.lines()
        .find_map(|l| l.trim_start().strip_prefix("internalField"))
        .map(|v| v.trim().trim_end_matches(';').trim().to_string())
        .unwrap_or_else(|| "uniform 0".into())
}

/// Changes a written wind tunnel case into a free jet case.
pub fn apply_jets(dir: &Path, plan: &MeshPlan, jets: &[JetFlow]) -> Result<(), FoamError> {
    if jets.is_empty() {
        return Ok(());
    }
    let read = |p: &str| std::fs::read_to_string(dir.join(p)).map_err(io);
    let write = |p: &str, s: String| std::fs::write(dir.join(p), s).map_err(io);
    // Still air: no inflow, open boundaries everywhere.
    let u = read("0/U")?;
    let u = u
        .lines()
        .map(|l| {
            if l.trim_start().starts_with("internalField") {
                "internalField   uniform (0 0 0);".to_string()
            } else {
                l.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
        + "\n";
    write(
        "0/U",
        open_boundaries(&u, |_| {
            "{ type pressureInletOutletVelocity; value uniform (0 0 0); }".into()
        }),
    )?;
    write(
        "0/p",
        open_boundaries(&read("0/p")?, |_| {
            "{ type totalPressure; p0 uniform 0; value uniform 0; }".into()
        }),
    )?;
    for field in ["k", "omega"] {
        let path = format!("0/{field}");
        if !dir.join(&path).is_file() {
            continue;
        }
        let text = read(&path)?;
        let v = internal_value(&text);
        write(
            &path,
            open_boundaries(&text, |_| {
                format!("{{ type inletOutlet; inletValue {v}; value {v}; }}")
            }),
        )?;
    }
    // Calmer pressure-velocity coupling for a start from rest.
    let sol = read("system/fvSolution")?
        .replace("consistent      true;", "consistent      false;")
        .replace(
            "equations { U 0.9; \".*\" 0.9; }",
            "equations { U 0.5; \".*\" 0.5; }",
        )
        .replace("fields { p 1; }", "fields { p 0.3; }");
    write("system/fvSolution", sol)?;
    // The run also watches the mean speed in the domain (see `solve::assess_with_flow`): |U| as
    // a field, then its volume average in the solver log, both ahead of the forces.
    if dir.join("system/controlDict").is_file() {
        let objects = format!(
            "    jetSpeed {{ type mag; libs (fieldFunctionObjects); field U; result jetSpeed; \
             executeControl timeStep; executeInterval 1; writeControl none; log false; }}\n    \
             {FLOW_OBJECT} {{ type volFieldValue; libs (fieldFunctionObjects); fields (jetSpeed); \
             operation volAverage; regionType all; writeControl timeStep; writeInterval 1; \
             writeFields false; log true; }}\n"
        );
        let control = read("system/controlDict")?;
        let open = "functions\n{\n";
        let at = control
            .find(open)
            .map(|i| i + open.len())
            .ok_or_else(|| FoamError::new("job.storage_failed", "controlDict without functions"))?;
        write(
            "system/controlDict",
            format!("{}{objects}{}", &control[..at], &control[at..]),
        )?;
    }
    // The sources and the refinement along each jet.
    let mut options = String::from(
        "FoamFile { version 2.0; format ascii; class dictionary; object fvOptions; }\n",
    );
    let mut geometry = String::new();
    let mut regions = String::new();
    let near_lo = DVec3::from(plan.near_box[0]);
    for (i, j) in jets.iter().enumerate() {
        let n = i + 1;
        let c = DVec3::from(j.center);
        let d = DVec3::from(j.direction).normalize_or_zero();
        let level = jet_level(plan, j.diameter);
        let h = plan.background_cell / 2f64.powi(level as i32);
        let len = (0.5 * j.diameter).max(2.0 * h);
        let _ = writeln!(
            options,
            "jet{n}\n{{\n    type            vectorFixedValueConstraint;\n    selectionMode   geometric;\n    selection\n    {{\n        nozzle {{ action use; source cylinder; point1 {}; point2 {}; radius {:.6}; }}\n    }}\n    fieldValues {{ U ({} {} {}); }}\n}}",
            v3(c - d * len),
            v3(c),
            j.diameter / 2.0,
            j.speed_m_s * d.x,
            j.speed_m_s * d.y,
            j.speed_m_s * d.z,
        );
        // Along the jet up to the middle of the region around the model (at least 5 d); the
        // refinement around the model takes over from there. Going further made the mesh too
        // big for the memory of the container in the first real run.
        let middle = (near_lo + DVec3::from(plan.near_box[1])) / 2.0;
        let reach = (middle - c).dot(d).max(5.0 * j.diameter);
        // Three steps: the core (CELLS_ACROSS over d), up to 10 d one level coarser, beyond that
        // two levels coarser; the jet widens with SPREAD, so the cells may grow along it.
        let near = reach.min(10.0 * j.diameter);
        let _ = write!(
            geometry,
            "
    jet{n} {{ type searchableCylinder; point1 {}; point2 {}; radius {:.6}; }}
    jet{n}_near {{ type searchableCylinder; point1 {}; point2 {}; radius {:.6}; }}
    jet{n}_core {{ type searchableCylinder; point1 {}; point2 {}; radius {:.6}; }}",
            v3(c),
            v3(c + d * reach),
            j.diameter / 2.0 + SPREAD * reach,
            v3(c),
            v3(c + d * near),
            j.diameter / 2.0 + SPREAD * near,
            v3(c - d * j.diameter),
            v3(c + d * 2.0 * j.diameter),
            0.75 * j.diameter,
        );
        let cap = plan.near_level + 1;
        let _ = write!(
            regions,
            "
        jet{n} {{ mode inside; levels ((1E15 {})); }}
        jet{n}_near {{ mode inside; levels ((1E15 {})); }}
        jet{n}_core {{ mode inside; levels ((1E15 {level})); }}",
            level.saturating_sub(2).min(cap),
            level.saturating_sub(1).min(cap),
        );
    }
    write("system/fvOptions", options)?;
    let snappy = read("system/snappyHexMeshDict")?;
    let mut out = String::new();
    for line in snappy.lines() {
        out.push_str(line);
        let t = line.trim_start();
        if t.starts_with("wake { type searchableBox") {
            out.push_str(&geometry);
        } else if t.starts_with("wake { mode inside") {
            out.push_str(&regions);
        }
        out.push('\n');
    }
    write("system/snappyHexMeshDict", out)
}
