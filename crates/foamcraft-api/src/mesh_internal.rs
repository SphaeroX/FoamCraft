// SPDX-License-Identifier: GPL-3.0-or-later
//! Meshing the fluid region of an internal flow: walls, caps and faces with a role become regions
//! of one surface; the point inside comes from the flood test.

use std::collections::HashMap;

use foamcraft_core::error::FoamError;
use foamcraft_core::job::{JobState, JobStore, Stage};
use foamcraft_core::project::{InternalFlow, MeshRecord, Project, Scenario};
use foamcraft_geometry::Mesh;
use foamcraft_geometry::caps::{cap_meshes, fluid_report};
use foamcraft_geometry::commands::placement;
use foamcraft_geometry::surfaces::{face_key, segmented_models};
use foamcraft_runtime::jobs::{prepare_job, remove_job_resources};
use foamcraft_sim::case::ModelSurface;
use foamcraft_sim::internal::{BoundaryValues, OpeningRole, evaluate};
use foamcraft_sim::internal_case::{
    Boundary, InternalPlan, MIN_CELLS_ACROSS, WALL_REGION, plan_internal, write_internal_case,
};
use foamcraft_sim::medium_properties;
use foamcraft_sim::mesh_report::{MeshQuality, mesh_quality};
use schemars::JsonSchema;
use serde::Serialize;

use crate::mesh::{GenerateParams, Hint, MeshRun, fetch_preview, hint, stage_log};

#[derive(Debug, Serialize, JsonSchema)]
pub struct InternalMeshResult {
    pub job_id: String,
    pub budget: u64,
    pub cells: u64,
    pub quality: MeshQuality,
    pub fallbacks: Vec<Hint>,
    pub hints: Vec<Hint>,
    pub plan: InternalPlan,
}

/// Region name of every inlet and outlet key.
pub(crate) fn regions_of(flow: &InternalFlow) -> HashMap<String, String> {
    let mut map = HashMap::new();
    for (i, inlet) in flow.inlets.iter().enumerate() {
        map.insert(inlet.key.clone(), format!("in_{}", i + 1));
    }
    for (i, outlet) in flow.outlets.iter().enumerate() {
        map.insert(outlet.key.clone(), format!("out_{}", i + 1));
    }
    map
}

/// The surfaces of the fluid region (world coordinates) with the region of every triangle.
pub(crate) struct FluidSurfaces {
    pub meshes: Vec<Mesh>,
    pub regions: Vec<Vec<String>>,
}

pub(crate) fn fluid_surfaces(
    project: &Project,
    flow: &InternalFlow,
) -> Result<FluidSurfaces, FoamError> {
    let map = regions_of(flow);
    let mut meshes = Vec::new();
    let mut regions = Vec::new();
    for m in segmented_models(project)? {
        meshes.push(placement(&m.entry).apply(&m.mesh));
        regions.push(
            m.segmentation
                .face_of_triangle
                .iter()
                .map(|f| {
                    map.get(&face_key(&m.id, *f))
                        .cloned()
                        .unwrap_or_else(|| WALL_REGION.into())
                })
                .collect(),
        );
    }
    for (key, cap) in cap_meshes(project) {
        let region = map.get(&key).cloned().unwrap_or_else(|| WALL_REGION.into());
        regions.push(vec![region; cap.triangles.len()]);
        meshes.push(cap);
    }
    Ok(FluidSurfaces { meshes, regions })
}

/// Inlets and outlets of the case and for the evaluation.
pub(crate) fn boundaries(
    project: &Project,
    flow: &InternalFlow,
) -> Result<(Vec<Boundary>, Vec<BoundaryValues>), FoamError> {
    let info = evaluate(project, flow)?;
    let map = regions_of(flow);
    let mut case = Vec::new();
    let mut values = Vec::new();
    for o in &info.openings {
        let region = map[&o.key].clone();
        let inlet = flow
            .inlets
            .iter()
            .find(|i| i.key == o.key)
            .map(|i| i.condition);
        let outlet_pa = flow
            .outlets
            .iter()
            .find(|x| x.key == o.key)
            .map(|x| x.pressure_pa);
        case.push(Boundary {
            region: region.clone(),
            inlet,
            outlet_pa,
            area_m2: o.area_m2,
            hydraulic_diameter_m: o.hydraulic_diameter_m,
        });
        values.push(BoundaryValues {
            key: o.key.clone(),
            region,
            role: if inlet.is_some() {
                OpeningRole::Inlet
            } else {
                OpeningRole::Outlet
            },
            area_m2: o.area_m2,
        });
    }
    Ok((case, values))
}

fn bounds(meshes: &[Mesh]) -> ([f64; 3], [f64; 3]) {
    let mut lo = [f64::MAX; 3];
    let mut hi = [f64::MIN; 3];
    for m in meshes {
        let (a, b) = m.bounds();
        for k in 0..3 {
            lo[k] = lo[k].min(f64::from(a[k]));
            hi[k] = hi[k].max(f64::from(b[k]));
        }
    }
    (lo, hi)
}

/// Meshes the fluid region of the project's internal flow (one retry with half the cells) without
/// changing the project.
pub(crate) fn mesh_internal(
    ctx: &MeshRun<'_>,
    project: &Project,
    p: GenerateParams,
) -> Result<(MeshRecord, InternalMeshResult), FoamError> {
    let runner = ctx.runner;
    let Some(Scenario::InternalFlow(flow)) = project.manifest.scenario.clone() else {
        return Err(FoamError::new("scenario.none", ""));
    };
    let seal = fluid_report(project)?;
    let (Some(location), true) = (seal.fluid_point_m, seal.sealed) else {
        return Err(FoamError::new("geometry.not_sealed", ""));
    };
    let info = evaluate(project, &flow)?;
    let surfaces = fluid_surfaces(project, &flow)?;
    let (case_boundaries, _) = boundaries(project, &flow)?;
    let medium = medium_properties(&flow.medium);
    let r = runner.as_ref();
    let env = r.check()?;
    let budget = p.max_cells.unwrap_or_else(|| {
        let machine = sysinfo::System::new().map(|mut s| {
            s.refresh_memory();
            s.total_memory()
        });
        let memory = machine.map_or(env.memory_bytes, |m| m.min(env.memory_bytes));
        let cores = sysinfo::System::physical_core_count().unwrap_or(1) as u32;
        foamcraft_sim::cell_budget(memory, cores, flow.quality).cells
    });
    let smallest = info
        .openings
        .iter()
        .map(|o| o.hydraulic_diameter_m)
        .fold(f64::MAX, f64::min);
    let (lo, hi) = bounds(&surfaces.meshes);
    let store = JobStore::new(project.root.join("runs"));
    let mut factor = 1.0;
    let mut fallbacks = Vec::new();
    let mut number = 0;
    loop {
        number += 1;
        let cells = (budget as f64 * factor) as u64;
        let plan = plan_internal(
            lo,
            hi,
            seal.fluid_volume_m3,
            location,
            smallest,
            cells,
            info.regime.turbulent(),
        );
        let models: Vec<ModelSurface<'_>> = surfaces
            .meshes
            .iter()
            .zip(&surfaces.regions)
            .map(|(mesh, regions)| ModelSurface {
                mesh,
                regions: regions.clone(),
            })
            .collect();
        let mut record = prepare_job(r, &store, "internal_flow_mesh", |dir| {
            write_internal_case(dir, &plan, &models, &case_boundaries, &medium, 1000)
        })?;
        let state = ctx.attempt(&store, &mut record, number, false);
        ctx.check_cancelled(&record)?;
        let log = store.read_log(&record.id).unwrap_or_default();
        let quality =
            (state == JobState::Succeeded).then(|| mesh_quality(&stage_log(&log, Stage::Check)));
        let acceptable = quality
            .as_ref()
            .is_some_and(|q| q.verdict != foamcraft_sim::mesh_report::Verdict::Poor);
        if acceptable {
            let quality = quality.unwrap_or_else(|| mesh_quality(""));
            if let Some(volume) = &record.volume {
                fetch_preview(runner, project, volume);
            }
            let cells = quality.cells.or(record.cells).unwrap_or(0);
            let mesh = MeshRecord {
                job_id: record.id.clone(),
                cells,
                verdict: serde_json::to_value(quality.verdict)
                    .ok()
                    .and_then(|v| v.as_str().map(String::from))
                    .unwrap_or_default(),
                layers_target: 0,
                layers_achieved: 0.0,
                fallbacks: fallbacks.clone(),
            };
            let mut hints = Vec::new();
            if plan.cells_across < MIN_CELLS_ACROSS {
                hints.push(hint("mesh.coarse_openings"));
            }
            let result = InternalMeshResult {
                job_id: record.id,
                budget,
                cells,
                quality,
                fallbacks: fallbacks.iter().map(|c| hint(c)).collect(),
                hints,
                plan,
            };
            return Ok((mesh, result));
        }
        let _ = remove_job_resources(r, &record);
        if factor < 1.0 {
            let detail = match state {
                JobState::Failed { error, .. } => error.to_string(),
                _ => "mesh quality not acceptable".into(),
            };
            return Err(FoamError::new("mesh.failed", detail));
        }
        factor = 0.5;
        fallbacks.push("mesh.fallback_coarser".to_string());
    }
}
