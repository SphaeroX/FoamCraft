// SPDX-License-Identifier: GPL-3.0-or-later
//! Writes the OpenFOAM case of a wind tunnel scenario: templates plus the planned mesh settings and
//! the model surface as STL in the flow frame.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::Path;

use foamcraft_core::case::WIND_TUNNEL;
use foamcraft_core::error::FoamError;
use foamcraft_core::project::WindTunnel;
use foamcraft_geometry::Mesh;
use glam::DVec3;

use crate::mesh::{MeshPlan, to_flow_frame};
use crate::solve::{
    INLET_INTENSITY, InletTurbulence, LENGTH_SCALE_SHARE, inlet_turbulence, solver_functions,
};
use crate::{MediumProperties, flow_direction, frontal_area};

/// Default surface region for faces that belong to no group.
pub const DEFAULT_REGION: &str = "body";

/// A model in world coordinates and the region (patch) of every triangle.
pub struct ModelSurface<'a> {
    pub mesh: &'a Mesh,
    pub regions: Vec<String>,
}

/// A valid OpenFOAM word: letters, digits and underscores, starting with a letter.
pub fn patch_word(name: &str) -> String {
    let mut out: String = name
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect();
    if !out.starts_with(|c: char| c.is_ascii_alphabetic()) {
        out.insert(0, 'g');
    }
    out
}

fn io(e: std::io::Error) -> FoamError {
    FoamError::new("job.storage_failed", e.to_string())
}

fn v3(v: [f64; 3]) -> String {
    format!("{:.6} {:.6} {:.6}", v[0], v[1], v[2])
}

/// ASCII STL with one solid per region, rotated into the flow frame.
pub fn surface_stl(models: &[ModelSurface<'_>], yaw_deg: f64) -> String {
    let r = to_flow_frame(yaw_deg);
    let mut by_region: BTreeMap<&str, Vec<[DVec3; 3]>> = BTreeMap::new();
    for m in models {
        for (i, t) in m.mesh.triangles.iter().enumerate() {
            let tri = t.map(|k| {
                let p = m.mesh.positions[k as usize];
                r * DVec3::new(f64::from(p[0]), f64::from(p[1]), f64::from(p[2]))
            });
            let region = m.regions.get(i).map_or(DEFAULT_REGION, String::as_str);
            by_region.entry(region).or_default().push(tri);
        }
    }
    let mut out = String::new();
    for (region, tris) in by_region {
        let _ = writeln!(out, "solid {region}");
        for [a, b, c] in tris {
            let n = (b - a).cross(c - a).normalize_or_zero();
            let _ = writeln!(
                out,
                "facet normal {:e} {:e} {:e}\nouter loop",
                n.x, n.y, n.z
            );
            for p in [a, b, c] {
                let _ = writeln!(out, "vertex {:e} {:e} {:e}", p.x, p.y, p.z);
            }
            let _ = writeln!(out, "endloop\nendfacet");
        }
        let _ = writeln!(out, "endsolid {region}");
    }
    out
}

/// Placeholder values for the wind tunnel templates.
pub fn template_values(
    plan: &MeshPlan,
    regions: &[String],
    wind: &WindTunnel,
    medium: &MediumProperties,
    iterations: u32,
    frontal_area: f64,
) -> Vec<(String, String)> {
    let [x0, y0, z0] = plan.tunnel_min;
    let [x1, y1, z1] = plan.tunnel_max;
    let [nx, ny, nz] = plan.background_cells;
    let mut region_block = String::new();
    for r in regions {
        let _ = writeln!(
            region_block,
            "                {r} {{ level ({0} {0}); patchInfo {{ type wall; inGroups (model); }} }}",
            plan.surface_level
        );
    }
    // Inflow turbulence: 1 % intensity, length scale a tenth of the model length.
    let u = wind.speed_m_s;
    let InletTurbulence { k, omega } = inlet_turbulence(
        u,
        INLET_INTENSITY,
        LENGTH_SCALE_SHARE * plan.reference_length.max(1e-6),
    );
    let estimated = plan.estimated_cells.max(1);
    let values: Vec<(&str, String)> = vec![
        ("X0", format!("{x0:.6}")),
        ("Y0", format!("{y0:.6}")),
        ("Z0", format!("{z0:.6}")),
        ("X1", format!("{x1:.6}")),
        ("Y1", format!("{y1:.6}")),
        ("Z1", format!("{z1:.6}")),
        ("NX", nx.to_string()),
        ("NY", ny.to_string()),
        ("NZ", nz.to_string()),
        (
            "FLOOR",
            if plan.ground {
                "ground { type wall; faces ((0 3 2 1)); }".to_string()
            } else {
                String::new()
            },
        ),
        (
            "FLOOR_AS_SIDE",
            if plan.ground {
                String::new()
            } else {
                " (0 3 2 1)".to_string()
            },
        ),
        ("ADD_LAYERS", (plan.layers > 0).to_string()),
        ("NEAR_MIN", v3(plan.near_box[0])),
        ("NEAR_MAX", v3(plan.near_box[1])),
        ("WAKE_MIN", v3(plan.wake_box[0])),
        ("WAKE_MAX", v3(plan.wake_box[1])),
        // Generous limits: the plan, not these, controls the size.
        ("MAX_LOCAL", (estimated * 4).max(1_000_000).to_string()),
        ("MAX_GLOBAL", (estimated * 8).max(2_000_000).to_string()),
        ("SURFACE_LEVEL", plan.surface_level.to_string()),
        ("NEAR_LEVEL", plan.near_level.to_string()),
        ("WAKE_LEVEL", plan.wake_level.to_string()),
        ("REGIONS", region_block),
        ("LOCATION", v3(plan.location_in_mesh)),
        ("LAYERS", plan.layers.to_string()),
        ("EXPANSION", format!("{:.3}", plan.expansion_ratio)),
        ("FIRST_LAYER", format!("{:e}", plan.first_layer)),
        ("MIN_THICKNESS", format!("{:e}", plan.first_layer * 0.1)),
        ("NU", format!("{:e}", medium.kinematic_viscosity)),
        ("U", format!("{u}")),
        ("K", format!("{k:e}")),
        ("OMEGA", format!("{omega:e}")),
        ("END_TIME", iterations.to_string()),
        (
            "FUNCTIONS",
            solver_functions(
                regions,
                frontal_area,
                plan.reference_length,
                u,
                medium.density,
            ),
        ),
    ];
    values
        .into_iter()
        .map(|(k, v)| (k.to_string(), v))
        .collect()
}

/// Writes the complete case into `dir`.
pub fn write_wind_tunnel_case(
    dir: &Path,
    plan: &MeshPlan,
    models: &[ModelSurface<'_>],
    wind: &WindTunnel,
    medium: &MediumProperties,
    iterations: u32,
) -> Result<(), FoamError> {
    let mut regions: Vec<String> = models
        .iter()
        .flat_map(|m| m.regions.iter().cloned())
        .collect();
    regions.sort();
    regions.dedup();
    if regions.is_empty() {
        regions.push(DEFAULT_REGION.into());
    }
    let meshes: Vec<&Mesh> = models.iter().map(|m| m.mesh).collect();
    let area = frontal_area(&meshes, flow_direction(plan.yaw_deg));
    let values = template_values(plan, &regions, wind, medium, iterations, area);
    let refs: Vec<(&str, &str)> = values
        .iter()
        .map(|(k, v)| (k.as_str(), v.as_str()))
        .collect();
    WIND_TUNNEL.write(dir, &refs)?;
    let surfaces = dir.join("constant").join("triSurface");
    std::fs::create_dir_all(&surfaces).map_err(io)?;
    std::fs::write(
        surfaces.join("model.stl"),
        surface_stl(models, plan.yaw_deg),
    )
    .map_err(io)?;
    let json = serde_json::to_string_pretty(plan)
        .map_err(|e| FoamError::new("job.storage_failed", e.to_string()))?;
    std::fs::write(dir.join("foamcraft-mesh-plan.json"), json).map_err(io)
}
