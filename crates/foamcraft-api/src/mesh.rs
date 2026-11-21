// SPDX-License-Identifier: GPL-3.0-or-later
//! Meshing of the project's scenario with the fallback ladder. The meshing itself works on a copy
//! of the project and reports through events, so it can run in the background of a run
//! (`run.start`); `mesh.generate` is the blocking command for the CLI and agents.

use std::collections::HashMap;
use std::io::Write as _;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use foamcraft_core::command::{EventBus, Options, Registry, Session};
use foamcraft_core::error::{FoamError, Language, Message, message};
use foamcraft_core::job::{JobEvent, JobRecord, JobState, JobStore, RunOptions, Stage};
use foamcraft_core::project::{MeshRecord, Project, Scenario};
use foamcraft_geometry::commands::placement;
use foamcraft_geometry::diagnose;
use foamcraft_geometry::surfaces::{face_key, segmented_models};
use foamcraft_runtime::jobs::{core_limit, execute_job, prepare_job, remove_job_resources};
use foamcraft_runtime::{Container, Mount, StartOptions};
use foamcraft_sim::case::{DEFAULT_REGION, ModelSurface, patch_word, write_wind_tunnel_case};
use foamcraft_sim::mesh::{MeshPlan, plan_wind_tunnel};
use foamcraft_sim::mesh_progress::MeshProgress;
use foamcraft_sim::mesh_report::{LayerReport, MeshQuality, layer_report, mesh_quality};
use foamcraft_sim::mesh_steps::{Attempt, AttemptOutcome, accept, next_attempt};
use foamcraft_sim::{cell_budget, medium_properties};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::SharedRunner;

/// Name of the mesh preview in the project's cache folder.
pub const PREVIEW_FILE: &str = "mesh_preview.obj";
/// File in the folder of a run that lists the jobs of its meshing attempts, one id per line.
pub const MESH_JOBS_FILE: &str = "mesh_jobs";

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct GenerateParams {
    /// Upper limit for the cell count; default: the budget of the quality level.
    #[serde(default)]
    pub max_cells: Option<u64>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct Hint {
    pub code: String,
    pub en: Message,
    pub de: Message,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct MeshResult {
    pub job_id: String,
    pub budget: u64,
    pub cells: u64,
    pub quality: MeshQuality,
    pub layers: Option<LayerReport>,
    /// Fallbacks that were needed, in order.
    pub fallbacks: Vec<Hint>,
    pub plan: MeshPlan,
}

pub(crate) fn hint(code: &str) -> Hint {
    let get = |lang| {
        message(code, lang).cloned().unwrap_or(Message {
            text: code.into(),
            fix: String::new(),
        })
    };
    Hint {
        code: code.into(),
        en: get(Language::En),
        de: get(Language::De),
    }
}

/// Lines of one stage from a job log (`[stage] line`).
pub(crate) fn stage_log(log: &str, stage: Stage) -> String {
    let prefix = format!("[{}] ", stage.id());
    log.lines()
        .filter_map(|l| l.strip_prefix(&prefix))
        .collect::<Vec<_>>()
        .join("\n")
}

pub(crate) struct Prepared {
    pub(crate) worlds: Vec<foamcraft_geometry::Mesh>,
    pub(crate) regions: Vec<Vec<String>>,
}

/// World meshes and the patch region of every triangle (first group of its face, else "body").
pub(crate) fn prepare_models(project: &Project) -> Result<Prepared, FoamError> {
    let models = segmented_models(project)?;
    if models.is_empty() {
        return Err(FoamError::new("scenario.no_geometry", ""));
    }
    let mut group_of: HashMap<String, String> = HashMap::new();
    for g in &project.manifest.groups {
        for k in &g.keys {
            group_of
                .entry(k.clone())
                .or_insert_with(|| patch_word(&g.name));
        }
    }
    let mut worlds = Vec::new();
    let mut regions = Vec::new();
    for m in &models {
        let d = diagnose(&m.mesh);
        if !d.watertight {
            return Err(FoamError::new(
                "mesh.leak",
                format!(
                    "{} ({} open boundary loops)",
                    m.entry.name, d.boundary_loops
                ),
            ));
        }
        worlds.push(placement(&m.entry).apply(&m.mesh));
        regions.push(
            m.segmentation
                .face_of_triangle
                .iter()
                .map(|f| {
                    group_of
                        .get(&face_key(&m.id, *f))
                        .cloned()
                        .unwrap_or(DEFAULT_REGION.into())
                })
                .collect(),
        );
    }
    Ok(Prepared { worlds, regions })
}

/// Copies the extracted model surface out of the job's volume into the project cache.
pub(crate) fn fetch_preview(runner: &SharedRunner, project: &Project, volume: &str) {
    let r = runner.as_ref();
    let Ok(c) = r.start(&StartOptions {
        mount: Some(Mount::Volume(volume.into())),
        ..Default::default()
    }) else {
        return;
    };
    let list = r.exec(&c, &["sh", "-c", "ls /work/case/preview*.obj"], &mut |_| {});
    if let Some(file) = list
        .ok()
        .and_then(|o| o.stdout.lines().next().map(str::to_string))
    {
        let _ = r.copy_out(&c, &file, &project.root.join("cache").join(PREVIEW_FILE));
    }
    let _ = r.stop(&Container { id: c.id });
}

/// What a meshing needs besides the project. It does not touch the session, so it can run in the
/// background while other commands are answered.
pub(crate) struct MeshRun<'a> {
    pub runner: &'a SharedRunner,
    pub events: &'a EventBus,
    pub cancel: &'a AtomicBool,
    /// The run this meshing belongs to: events carry its id and its folder lists the meshing
    /// jobs, so `run.log` shows them.
    pub run_id: Option<&'a str>,
}

/// A finished mesh: the record for the project and the answer of `mesh.generate`.
pub(crate) struct Meshed {
    pub record: MeshRecord,
    pub result: serde_json::Value,
}

impl MeshRun<'_> {
    /// Runs one meshing attempt to the mesh check, reporting stages and progress as events.
    pub(crate) fn attempt(
        &self,
        store: &JobStore,
        record: &mut JobRecord,
        attempt: u32,
        with_layers: bool,
    ) -> JobState {
        let r = self.runner.as_ref();
        if let Some(run) = self.run_id {
            let list = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(store.dir(run).join(MESH_JOBS_FILE));
            if let Ok(mut file) = list {
                let _ = writeln!(file, "{}", record.id);
            }
        }
        let options = RunOptions {
            cores: core_limit(r, record),
            poll_interval: Duration::from_millis(500),
            stop_after: Some(Stage::Check),
        };
        let id = record.id.clone();
        let started = Instant::now();
        let mut progress = MeshProgress::new(with_layers);
        let report = |progress: &MeshProgress| {
            self.events.emit(
                "mesh.progress",
                json!({
                    "job_id": id,
                    "run_id": self.run_id,
                    "attempt": attempt,
                    "stage": progress.stage(),
                    "phase": progress.phase(),
                    "fraction": progress.fraction(),
                    "remaining_s": progress.remaining_s(started.elapsed().as_secs_f64()),
                }),
            );
        };
        let state = execute_job(r, store, record, &options, self.cancel, &mut |e| match e {
            JobEvent::StageStarted(stage) => {
                self.events.emit(
                    "mesh.stage",
                    json!({
                        "job_id": id,
                        "run_id": self.run_id,
                        "stage": stage,
                        "attempt": attempt,
                    }),
                );
                if progress.stage_started(stage) {
                    report(&progress);
                }
            }
            JobEvent::Log { line, .. } if progress.line(&line) => report(&progress),
            _ => {}
        });
        if state == JobState::Succeeded && progress.finished() {
            report(&progress);
        }
        state
    }

    /// `run.cancelled` once the user cancelled; the resources of the attempt are removed.
    pub(crate) fn check_cancelled(&self, record: &JobRecord) -> Result<(), FoamError> {
        if self.cancel.load(Ordering::SeqCst) {
            let _ = remove_job_resources(self.runner.as_ref(), record);
            return Err(FoamError::new("run.cancelled", ""));
        }
        Ok(())
    }
}

/// File in the folder of a mesh job with the project inputs the mesh was built from.
const MESH_INPUTS_FILE: &str = "mesh_inputs.json";

/// Everything of the project a mesh depends on: models with their placement, patch groups, caps,
/// boundary roles and the scenario. A mesh built from other inputs must not be reused.
fn mesh_inputs(project: &Project) -> serde_json::Value {
    let m = &project.manifest;
    json!({
        "geometry": m.geometry,
        "groups": m.groups,
        "caps": m.caps,
        "boundary_roles": m.boundary_roles,
        "scenario": m.scenario,
    })
}

/// Whether the mesh of the project was built from its current geometry and scenario. A mesh
/// without recorded inputs (made by an older version) counts as outdated.
pub(crate) fn mesh_is_current(project: &Project) -> bool {
    let Some(mesh) = &project.manifest.mesh else {
        return false;
    };
    let file = project
        .root
        .join("runs")
        .join(&mesh.job_id)
        .join(MESH_INPUTS_FILE);
    std::fs::read_to_string(file)
        .ok()
        .and_then(|text| serde_json::from_str::<serde_json::Value>(&text).ok())
        .is_some_and(|stored| stored == mesh_inputs(project))
}

/// Meshes the scenario of `project` (wind tunnel or internal flow) without changing it.
pub(crate) fn mesh_project(
    ctx: &MeshRun<'_>,
    project: &Project,
    p: GenerateParams,
) -> Result<Meshed, FoamError> {
    let internal = matches!(project.manifest.scenario, Some(Scenario::InternalFlow(_)));
    let io = |e: serde_json::Error| FoamError::new("command.internal", e.to_string());
    let meshed = if internal {
        let (record, result) = crate::mesh_internal::mesh_internal(ctx, project, p)?;
        Meshed {
            record,
            result: serde_json::to_value(result).map_err(io)?,
        }
    } else {
        let (record, result) = mesh_wind(ctx, project, p)?;
        Meshed {
            record,
            result: serde_json::to_value(result).map_err(io)?,
        }
    };
    let inputs = project
        .root
        .join("runs")
        .join(&meshed.record.job_id)
        .join(MESH_INPUTS_FILE);
    std::fs::write(inputs, mesh_inputs(project).to_string())
        .map_err(|e| FoamError::new("job.storage_failed", e.to_string()))?;
    Ok(meshed)
}

/// Makes `record` the mesh of the project: the volume of an older mesh is dropped (its record
/// stays in runs/) and the project is saved.
pub(crate) fn store_mesh(
    runner: &SharedRunner,
    project: &mut Project,
    record: MeshRecord,
) -> Result<(), FoamError> {
    let store = JobStore::new(project.root.join("runs"));
    if let Some(old) = project.manifest.mesh.take()
        && old.job_id != record.job_id
        && let Ok(old_record) = store.load(&old.job_id)
    {
        let _ = remove_job_resources(runner.as_ref(), &old_record);
    }
    project.manifest.mesh = Some(record);
    project.save()
}

/// Meshes the scenario of the open project (wind tunnel or internal flow); blocks until done.
pub(crate) fn generate(
    runner: &SharedRunner,
    session: &mut Session,
    p: GenerateParams,
) -> Result<serde_json::Value, FoamError> {
    let project = session.project()?.clone();
    let events = session.events().clone();
    let ctx = MeshRun {
        runner,
        events: &events,
        cancel: &AtomicBool::new(false),
        run_id: None,
    };
    let meshed = mesh_project(&ctx, &project, p)?;
    store_mesh(runner, session.project()?, meshed.record)?;
    events.emit("project.changed", serde_json::Value::Null);
    Ok(meshed.result)
}

fn mesh_wind(
    ctx: &MeshRun<'_>,
    project: &Project,
    p: GenerateParams,
) -> Result<(MeshRecord, MeshResult), FoamError> {
    let runner = ctx.runner;
    let Some(Scenario::WindTunnel(wind)) = project.manifest.scenario.clone() else {
        return Err(FoamError::new("scenario.none", ""));
    };
    let prepared = prepare_models(project)?;
    let refs: Vec<&foamcraft_geometry::Mesh> = prepared.worlds.iter().collect();
    let medium = medium_properties(&wind.medium);
    let r = runner.as_ref();
    let env = r.check()?;
    let budget = p.max_cells.unwrap_or_else(|| {
        let machine = sysinfo::System::new().map(|mut s| {
            s.refresh_memory();
            s.total_memory()
        });
        let memory = machine.map_or(env.memory_bytes, |m| m.min(env.memory_bytes));
        let cores = sysinfo::System::physical_core_count().unwrap_or(1) as u32;
        cell_budget(memory, cores, wind.quality).cells
    });
    let store = JobStore::new(project.root.join("runs"));
    let base_plan = plan_wind_tunnel(
        &refs,
        wind.direction_deg,
        wind.ground,
        wind.speed_m_s,
        &medium,
        wind.quality,
        budget,
    );
    let mut attempt = Attempt {
        layers: base_plan.layers,
        budget_factor: 1.0,
        hint: None,
    };
    let mut hints = Vec::new();
    let mut number = 0;
    loop {
        number += 1;
        let cells = (budget as f64 * attempt.budget_factor) as u64;
        let mut plan = plan_wind_tunnel(
            &refs,
            wind.direction_deg,
            wind.ground,
            wind.speed_m_s,
            &medium,
            wind.quality,
            cells,
        );
        plan.layers = attempt.layers;
        let models: Vec<ModelSurface<'_>> = prepared
            .worlds
            .iter()
            .zip(&prepared.regions)
            .map(|(mesh, regions)| ModelSurface {
                mesh,
                regions: regions.clone(),
            })
            .collect();
        let mut record = prepare_job(r, &store, "wind_tunnel_mesh", |dir| {
            write_wind_tunnel_case(dir, &plan, &models, &wind, &medium, 500)
        })?;
        let state = ctx.attempt(&store, &mut record, number, plan.layers > 0);
        ctx.check_cancelled(&record)?;
        let log = store.read_log(&record.id).unwrap_or_default();
        let outcome = AttemptOutcome {
            meshed: state == JobState::Succeeded,
            quality: (state == JobState::Succeeded)
                .then(|| mesh_quality(&stage_log(&log, Stage::Check))),
            // Planned layers without a layer table in the log count as no layers built.
            layers: (plan.layers > 0).then(|| {
                layer_report(&stage_log(&log, Stage::Mesh)).unwrap_or(LayerReport {
                    target: plan.layers,
                    achieved: 0.0,
                    thickness_percent: 0.0,
                })
            }),
        };
        if accept(&outcome) {
            if let Some(volume) = &record.volume {
                fetch_preview(runner, project, volume);
            }
            let quality = outcome.quality.unwrap_or_else(|| mesh_quality(""));
            let cells = quality.cells.or(record.cells).unwrap_or(0);
            let fallbacks: Vec<String> = hints.clone();
            let mesh = MeshRecord {
                job_id: record.id.clone(),
                cells,
                verdict: serde_json::to_value(quality.verdict)
                    .ok()
                    .and_then(|v| v.as_str().map(String::from))
                    .unwrap_or_default(),
                layers_target: plan.layers,
                layers_achieved: outcome.layers.as_ref().map_or(0.0, |l| l.achieved),
                fallbacks: fallbacks.clone(),
            };
            let result = MeshResult {
                job_id: record.id,
                budget,
                cells,
                quality,
                layers: outcome.layers,
                fallbacks: fallbacks.iter().map(|c| hint(c)).collect(),
                plan,
            };
            return Ok((mesh, result));
        }
        let _ = remove_job_resources(r, &record);
        match next_attempt(&attempt) {
            Some(next) => {
                if let Some(code) = &next.hint {
                    hints.push(code.clone());
                }
                attempt = next;
            }
            None => {
                let detail = match state {
                    JobState::Failed { error, .. } => error.to_string(),
                    _ => "mesh quality or layers not acceptable".into(),
                };
                return Err(FoamError::new("mesh.failed", detail));
            }
        }
    }
}

pub fn register(r: &mut Registry, runner: SharedRunner) {
    r.add(
        "mesh.generate",
        "Mesh the scenario of the open project within the cell budget (blocks until done; in the app, run.start meshes in the background).",
        Options {
            undoable: false,
            errors: &[
                "project.none_open",
                "scenario.none",
                "scenario.no_geometry",
                "mesh.leak",
                "mesh.failed",
                "geometry.not_sealed",
                "runtime.daemon_not_running",
                "runtime.image_missing",
            ],
            example: json!({}),
        },
        move |s, p: GenerateParams| generate(&runner, s, p),
    );
    r.add(
        "mesh.info",
        "Describe the latest mesh of the open project.",
        Options {
            undoable: false,
            errors: &["project.none_open", "mesh.none"],
            example: json!({}),
        },
        |s, _: foamcraft_sim::commands::NoParams| {
            s.project()?
                .manifest
                .mesh
                .clone()
                .ok_or_else(|| FoamError::new("mesh.none", ""))
        },
    );
}
