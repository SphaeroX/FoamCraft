// SPDX-License-Identifier: GPL-3.0-or-later
//! `run.start`, `run.cancel`, `run.status`: simulation jobs of the open project.
//!
//! Jobs of a project live in `<project>/runs/<job id>/`. `run.start` answers at once with the id
//! of the run; a background thread then meshes (when the project has no mesh for its current
//! geometry and scenario), solves and evaluates. The thread works on a copy of the project and
//! hands changes to the session through its update queue, so other commands are answered
//! meanwhile. Progress arrives as `mesh.stage`, `mesh.progress`, `mesh.ready`, `run.stage`,
//! `run.progress`, `run.result` and `run.failed` events.

use std::collections::HashMap;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;

use foamcraft_core::case::REFERENCE_CUBE;
use foamcraft_core::command::{EventBus, Options, Registry, Session, Updates};
use foamcraft_core::error::{FoamError, Language, Message, message};
use foamcraft_core::job::{JobEvent, JobRecord, JobState, JobStore, RunOptions, Stage};
use foamcraft_core::project::{Manifest, Project};
use foamcraft_runtime::Container;
use foamcraft_runtime::jobs::{core_limit, execute_job, prepare_job, remove_job_resources};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::SharedRunner;
use crate::mesh::{GenerateParams, MESH_JOBS_FILE, MeshRun, mesh_is_current, mesh_project};
use crate::solve;

/// Job name of a run that only builds the mesh.
const MESH_ONLY: &str = "mesh_only";

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct StartParams {
    /// Solver iterations (default 200).
    #[serde(default)]
    pub iterations: Option<u32>,
    /// Upper limit for parallel processes (default: physical cores).
    #[serde(default)]
    pub cores: Option<u32>,
    /// Block until the run ends and return its result (wind tunnel projects).
    #[serde(default)]
    pub wait: bool,
    /// Only build the mesh (a new one, even if the project has one) and stop.
    #[serde(default)]
    pub mesh_only: bool,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AttachParams {
    pub job_id: String,
    /// Block until the run ends and return its result.
    #[serde(default)]
    pub wait: bool,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct LogParams {
    pub job_id: String,
    /// Number of lines from the end (default 200).
    #[serde(default)]
    pub lines: Option<usize>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct LogTail {
    pub lines: Vec<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DescribeParams {
    pub codes: Vec<String>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct Described {
    pub en: Message,
    pub de: Message,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct ActiveJob {
    pub job_id: String,
    pub case: String,
    pub state: JobState,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ResultParams {
    /// Job of the result; default: the latest result of the project.
    #[serde(default)]
    pub job_id: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct JobParams {
    pub job_id: String,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct Started {
    pub job_id: String,
    /// Result of a wind tunnel run started with `wait`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct Status {
    pub job_id: String,
    pub state: JobState,
    pub cells: Option<u64>,
    pub cores: u32,
    /// This process still works on the job: solving, or evaluating and extracting the fields
    /// after the solver ended. A run is complete when its state is finished and it is not busy.
    pub busy: bool,
}

/// Cancel flags of jobs followed by this process.
fn running() -> &'static Mutex<HashMap<String, Arc<AtomicBool>>> {
    static RUNNING: OnceLock<Mutex<HashMap<String, Arc<AtomicBool>>>> = OnceLock::new();
    RUNNING.get_or_init(Default::default)
}

fn store(session: &mut Session) -> Result<JobStore, FoamError> {
    Ok(JobStore::new(session.project()?.root.join("runs")))
}

fn status_of(record: &JobRecord) -> Status {
    let busy = running()
        .lock()
        .is_ok_and(|map| map.contains_key(&record.id));
    Status {
        job_id: record.id.clone(),
        state: record.state.clone(),
        cells: record.cells,
        cores: record.cores,
        busy,
    }
}

pub(crate) fn event_payload(
    job_id: &str,
    event: &JobEvent,
) -> Option<(&'static str, serde_json::Value)> {
    match event {
        JobEvent::StageStarted(stage) => {
            Some(("run.stage", json!({"job_id": job_id, "stage": stage})))
        }
        JobEvent::Phase { stage, phase } => Some((
            "run.phase",
            json!({"job_id": job_id, "stage": stage, "phase": phase}),
        )),
        JobEvent::Progress {
            stage,
            iteration,
            residuals,
        } => Some((
            "run.progress",
            json!({"job_id": job_id, "stage": stage, "iteration": iteration, "residuals": residuals}),
        )),
        JobEvent::Finished(state) => {
            Some(("run.finished", json!({"job_id": job_id, "state": state})))
        }
        _ => None,
    }
}

fn cancel_flag(job_id: &str) -> Arc<AtomicBool> {
    let cancel = Arc::new(AtomicBool::new(false));
    if let Ok(mut map) = running().lock() {
        map.insert(job_id.to_string(), cancel.clone());
    }
    cancel
}

fn forget(job_id: &str) {
    if let Ok(mut map) = running().lock() {
        map.remove(job_id);
    }
}

/// Applies `change` to the manifest of the project at `root` and saves it: in the session when
/// that project is open there, else in its folder (the user opened another one meanwhile).
fn change_project(session: &mut Session, root: &Path, change: impl FnOnce(&mut Manifest)) {
    match session.project() {
        Ok(project) if project.root == root => {
            change(&mut project.manifest);
            let _ = project.save();
        }
        _ => {
            if let Ok(mut project) = Project::open(root) {
                change(&mut project.manifest);
                let _ = project.save();
            }
        }
    }
}

/// The background work of one run: a copy of the project, the record of the run and the handles
/// to report progress and to hand changes to the session.
struct Pipeline {
    runner: SharedRunner,
    project: Project,
    /// Id of the scenario this run belongs to (its mesh is recorded there).
    scenario_id: String,
    store: JobStore,
    record: JobRecord,
    events: EventBus,
    updates: Updates,
    iterations: Option<u32>,
    cores: Option<u32>,
    mesh_only: bool,
}

impl Pipeline {
    /// Meshes when the project has no mesh for its current geometry and scenario (or only a mesh
    /// was asked for), then solves and evaluates. `None` for a run that only meshes.
    fn work(&mut self, cancel: &AtomicBool) -> Result<Option<Value>, FoamError> {
        if self.mesh_only || !mesh_is_current(&self.project) {
            let ctx = MeshRun {
                runner: &self.runner,
                events: &self.events,
                cancel,
                run_id: Some(&self.record.id),
            };
            let meshed = mesh_project(&ctx, &self.project, GenerateParams { max_cells: None })?;
            let mesh = meshed.record;
            // The volume of the mesh this one replaces is dropped, unless another scenario
            // still uses it; its record stays in runs/.
            let id = self.scenario_id.clone();
            let shared = |job: &str| {
                self.project
                    .manifest
                    .scenarios
                    .iter()
                    .any(|e| e.id != id && e.mesh.as_ref().is_some_and(|m| m.job_id == job))
            };
            if let Some(old) = self.project.manifest.mesh.replace(mesh.clone())
                && old.job_id != mesh.job_id
                && !shared(&old.job_id)
                && let Ok(old_record) = self.store.load(&old.job_id)
            {
                let _ = remove_job_resources(self.runner.as_ref(), &old_record);
            }
            let root = self.project.root.clone();
            let ready = json!({
                "run_id": self.record.id,
                "job_id": mesh.job_id,
                "cells": mesh.cells,
                "verdict": mesh.verdict,
            });
            self.updates.push(move |s| {
                change_project(s, &root, |m| m.set_scenario_mesh(&id, mesh));
            });
            self.events.emit("project.changed", Value::Null);
            self.events.emit("mesh.ready", ready);
        }
        if self.mesh_only {
            self.record.state = JobState::Succeeded;
            self.store.save(&self.record)?;
            return Ok(None);
        }
        if cancel.load(Ordering::SeqCst) {
            return Err(FoamError::new("run.cancelled", ""));
        }
        let job = solve::prepare(
            &self.runner,
            &self.project,
            self.record.clone(),
            self.iterations,
            self.cores,
        )?;
        solve::run(job, cancel, &self.events, false).map(Some)
    }

    /// Runs the pipeline; a failure is recorded in the job of the run.
    fn run(&mut self, cancel: &AtomicBool) -> Result<Option<Value>, FoamError> {
        let out = self.work(cancel);
        if let Err(error) = &out
            && let Ok(mut record) = self.store.load(&self.record.id)
            && !record.state.is_finished()
        {
            record.state = if error.code == "run.cancelled" {
                JobState::Cancelled
            } else {
                JobState::Failed {
                    stage: Stage::Mesh,
                    error: error.clone(),
                }
            };
            let _ = self.store.save(&record);
        }
        out
    }
}

/// Projects with a scenario: mesh (if needed), solve, evaluate, all in the background.
fn start_scenario(
    runner: &SharedRunner,
    session: &mut Session,
    p: StartParams,
) -> Result<Started, FoamError> {
    let project = session.project()?;
    let scenario = project
        .manifest
        .scenario
        .clone()
        .ok_or_else(|| FoamError::new("scenario.none", ""))?;
    let needs_mesh = p.mesh_only || !mesh_is_current(project);
    // Problems that need no container show at once, as the answer of the command.
    if needs_mesh && matches!(scenario, foamcraft_core::project::Scenario::WindTunnel(_)) {
        crate::mesh::prepare_models(project)?;
    }
    let store = JobStore::new(project.root.join("runs"));
    let record = store.create(if p.mesh_only {
        MESH_ONLY
    } else {
        solve::case_name(&scenario)
    })?;
    let job_id = record.id.clone();
    project.manifest.normalize();
    let scenario_id = project.manifest.active_id();
    if !p.mesh_only {
        project.manifest.runs.push(job_id.clone());
        project
            .manifest
            .run_scenarios
            .insert(job_id.clone(), scenario_id.clone());
        project.save()?;
    }
    let mut pipeline = Pipeline {
        runner: runner.clone(),
        scenario_id,
        project: project.clone(),
        store,
        record,
        events: session.events().clone(),
        updates: session.updates(),
        iterations: p.iterations,
        cores: p.cores,
        mesh_only: p.mesh_only,
    };
    let events = session.events().clone();
    if !p.mesh_only {
        events.emit("project.changed", Value::Null);
    }
    let cancel = cancel_flag(&job_id);
    if p.wait {
        let result = pipeline.run(&cancel);
        forget(&job_id);
        // What the pipeline handed in (the mesh) belongs to the state this command leaves.
        session.apply_updates();
        return Ok(Started {
            job_id,
            result: result?,
        });
    }
    let id = job_id.clone();
    std::thread::spawn(move || {
        let mut pipeline = pipeline;
        if let Err(e) = pipeline.run(&cancel) {
            events.emit(
                "run.failed",
                json!({"job_id": id, "code": e.code, "detail": e.detail}),
            );
        }
        forget(&id);
    });
    Ok(Started {
        job_id,
        result: None,
    })
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct StartAllParams {
    /// Scenario ids to run, in this order; default: all scenarios that are set up.
    #[serde(default)]
    pub scenarios: Option<Vec<String>>,
    #[serde(default)]
    pub iterations: Option<u32>,
    #[serde(default)]
    pub cores: Option<u32>,
    /// Block until the whole queue has ended.
    #[serde(default)]
    pub wait: bool,
}

#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct QueuedRun {
    pub job_id: String,
    pub scenario: String,
    /// Code of the failure of this run; null while pending or when it succeeded.
    pub error: Option<String>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct Queue {
    pub runs: Vec<QueuedRun>,
}

/// Cancel flag and job ids of the running queue, by project folder.
type QueueEntry = (Arc<AtomicBool>, Vec<String>);
fn queues() -> &'static Mutex<HashMap<std::path::PathBuf, QueueEntry>> {
    static QUEUES: OnceLock<Mutex<HashMap<std::path::PathBuf, QueueEntry>>> = OnceLock::new();
    QUEUES.get_or_init(Default::default)
}

/// Runs scenarios one after another in the background: each on a copy of the project with that
/// scenario active, reusing a mesh of an earlier scenario when it fits. A failure is recorded
/// and the queue goes on; `run.cancel_queue` stops it.
fn start_all(
    runner: &SharedRunner,
    session: &mut Session,
    p: StartAllParams,
) -> Result<Queue, FoamError> {
    let project = session.project()?;
    project.manifest.normalize();
    let ids: Vec<String> = match p.scenarios {
        Some(ids) => ids,
        None => project
            .manifest
            .scenarios
            .iter()
            .filter(|e| e.scenario.is_some())
            .map(|e| e.id.clone())
            .collect(),
    };
    if ids.is_empty() {
        return Err(FoamError::new("scenario.none", ""));
    }
    let store = JobStore::new(project.root.join("runs"));
    let mut items = Vec::new();
    for id in &ids {
        let m = project
            .manifest
            .with_active(id)
            .ok_or_else(|| FoamError::new("scenario.not_found", id.clone()))?;
        let scenario = m
            .scenario
            .clone()
            .ok_or_else(|| FoamError::new("scenario.none", id.clone()))?;
        if matches!(scenario, foamcraft_core::project::Scenario::WindTunnel(_)) {
            let mut copy = project.clone();
            copy.manifest = m;
            crate::mesh::prepare_models(&copy)?;
        }
        items.push((id.clone(), solve::case_name(&scenario)));
    }
    let mut queued = Vec::new();
    for (id, case) in items {
        let record = store.create(case)?;
        project.manifest.runs.push(record.id.clone());
        project
            .manifest
            .run_scenarios
            .insert(record.id.clone(), id.clone());
        queued.push((id, record));
    }
    project.save()?;
    let base = project.clone();
    let flag = Arc::new(AtomicBool::new(false));
    if let Ok(mut map) = queues().lock() {
        map.insert(
            base.root.clone(),
            (
                flag.clone(),
                queued.iter().map(|(_, r)| r.id.clone()).collect(),
            ),
        );
    }
    let answer: Vec<QueuedRun> = queued
        .iter()
        .map(|(id, r)| QueuedRun {
            job_id: r.id.clone(),
            scenario: id.clone(),
            error: None,
        })
        .collect();
    let events = session.events().clone();
    events.emit("project.changed", Value::Null);
    let updates = session.updates();
    let runner = runner.clone();
    let (iterations, cores) = (p.iterations, p.cores);
    let work = move || -> Vec<QueuedRun> {
        let total = queued.len();
        let mut produced: Vec<foamcraft_core::project::MeshRecord> = Vec::new();
        let mut out = Vec::new();
        let mut cancelled = false;
        for (index, (id, record)) in queued.into_iter().enumerate() {
            if cancelled || flag.load(Ordering::SeqCst) {
                cancelled = true;
                let mut r = record;
                r.state = JobState::Cancelled;
                let _ = store.save(&r);
                out.push(QueuedRun {
                    job_id: r.id,
                    scenario: id,
                    error: Some("run.cancelled".into()),
                });
                continue;
            }
            events.emit(
                "run.queue",
                json!({"index": index, "total": total, "run_id": record.id, "scenario": id}),
            );
            let mut copy = base.clone();
            if let Some(m) = base.manifest.with_active(&id) {
                copy.manifest = m;
            }
            // A mesh an earlier scenario of this queue built may fit this one.
            if !mesh_is_current(&copy) {
                let own = copy.manifest.mesh.clone();
                for m in &produced {
                    copy.manifest.mesh = Some(m.clone());
                    if mesh_is_current(&copy) {
                        break;
                    }
                    copy.manifest.mesh = own.clone();
                }
            }
            let job_id = record.id.clone();
            let mut pipeline = Pipeline {
                runner: runner.clone(),
                scenario_id: id.clone(),
                project: copy,
                store: JobStore::new(base.root.join("runs")),
                record,
                events: events.clone(),
                updates: updates.clone(),
                iterations,
                cores,
                mesh_only: false,
            };
            let cancel = cancel_flag(&job_id);
            let result = pipeline.run(&cancel);
            forget(&job_id);
            if let Some(mesh) = pipeline.project.manifest.mesh.clone() {
                if !produced.iter().any(|m| m.job_id == mesh.job_id) {
                    produced.push(mesh.clone());
                }
                let root = base.root.clone();
                let sid = id.clone();
                updates.push(move |s| {
                    change_project(s, &root, |m| m.set_scenario_mesh(&sid, mesh));
                });
            }
            let error = result.err().map(|e| {
                if e.code == "run.cancelled" {
                    cancelled = true;
                }
                events.emit(
                    "run.failed",
                    json!({"job_id": job_id, "code": e.code, "detail": e.detail}),
                );
                e.code
            });
            out.push(QueuedRun {
                job_id,
                scenario: id,
                error,
            });
        }
        if let Ok(mut map) = queues().lock() {
            map.remove(&base.root);
        }
        events.emit("project.changed", Value::Null);
        events.emit(
            "run.queue_finished",
            json!({"cancelled": cancelled || flag.load(Ordering::SeqCst), "runs": out}),
        );
        out
    };
    if p.wait {
        let runs = work();
        session.apply_updates();
        return Ok(Queue { runs });
    }
    std::thread::spawn(work);
    Ok(Queue { runs: answer })
}

/// Stops the running queue of the open project: the current run is cancelled, the rest skipped.
fn cancel_queue(session: &mut Session) -> Result<Value, FoamError> {
    let root = session.project()?.root.clone();
    let entry = queues().lock().ok().and_then(|m| m.get(&root).cloned());
    let Some((flag, jobs)) = entry else {
        return Ok(json!({"cancelled": false}));
    };
    flag.store(true, Ordering::SeqCst);
    let flags = running().lock().ok().map(|m| {
        jobs.iter()
            .filter_map(|j| m.get(j).cloned())
            .collect::<Vec<_>>()
    });
    for f in flags.into_iter().flatten() {
        f.store(true, Ordering::SeqCst);
    }
    Ok(json!({"cancelled": true}))
}

/// The run lost its background work (the app ended while its mesh was built): its meshing jobs
/// are stopped and the run is marked as lost.
fn mark_lost(runner: &SharedRunner, store: &JobStore, record: &mut JobRecord) -> FoamError {
    let attempts = std::fs::read_to_string(store.dir(&record.id).join(MESH_JOBS_FILE));
    for id in attempts.unwrap_or_default().lines() {
        if let Ok(mut attempt) = store.load(id)
            && !attempt.state.is_finished()
        {
            let _ = remove_job_resources(runner.as_ref(), &attempt);
            attempt.state = JobState::Cancelled;
            let _ = store.save(&attempt);
        }
    }
    let error = FoamError::new("run.lost", "the app ended while the mesh was built");
    record.state = JobState::Failed {
        stage: Stage::Mesh,
        error: error.clone(),
    };
    let _ = store.save(record);
    error
}

/// The log of a job; for a run, the logs of its meshing attempts come first.
fn full_log(store: &JobStore, job_id: &str) -> Result<String, FoamError> {
    let own = store.read_log(job_id)?;
    let attempts = std::fs::read_to_string(store.dir(job_id).join(MESH_JOBS_FILE));
    let mut text = String::new();
    for id in attempts.unwrap_or_default().lines() {
        text.push_str(&store.read_log(id).unwrap_or_default());
    }
    text.push_str(&own);
    Ok(text)
}

fn start(
    runner: &SharedRunner,
    session: &mut Session,
    p: StartParams,
) -> Result<Started, FoamError> {
    if session.project()?.manifest.scenario.is_some() {
        return start_scenario(runner, session, p);
    }
    let store = store(session)?;
    let iterations = p.iterations.unwrap_or(200).to_string();
    let runner_ref = runner.as_ref();
    let mut record = prepare_job(runner_ref, &store, REFERENCE_CUBE.name, |dir| {
        REFERENCE_CUBE.write(dir, &[("END_TIME", &iterations)])
    })?;
    let project = session.project()?;
    project.manifest.runs.push(record.id.clone());
    project.save()?;
    session
        .events()
        .emit("project.changed", serde_json::Value::Null);

    let options = RunOptions {
        cores: p
            .cores
            .unwrap_or_else(|| core_limit(runner_ref, &record))
            .max(1),
        poll_interval: Duration::from_millis(500),
        stop_after: None,
    };
    let cancel = cancel_flag(&record.id);
    let events = session.events().clone();
    let runner = runner.clone();
    let job_id = record.id.clone();
    std::thread::spawn(move || {
        let id = record.id.clone();
        let runner_ref = runner.as_ref();
        execute_job(
            runner_ref,
            &store,
            &mut record,
            &options,
            &cancel,
            &mut |e| {
                if let Some((name, payload)) = event_payload(&id, &e) {
                    events.emit(name, payload);
                }
            },
        );
        if let Ok(mut map) = running().lock() {
            map.remove(&id);
        }
    });
    Ok(Started {
        job_id,
        result: None,
    })
}

/// Unfinished jobs of the open project (for example after a restart of the app).
fn active(session: &mut Session) -> Result<Vec<ActiveJob>, FoamError> {
    Ok(store(session)?
        .list()?
        .into_iter()
        .filter(|r| !r.state.is_finished())
        .map(|r| ActiveJob {
            job_id: r.id,
            case: r.case,
            state: r.state,
        })
        .collect())
}

/// Follows a running job again: events, convergence automation and result.
fn attach(
    runner: &SharedRunner,
    session: &mut Session,
    p: AttachParams,
) -> Result<Started, FoamError> {
    let root = session.project()?.root.clone();
    let store = store(session)?;
    let mut record = store.load(&p.job_id)?;
    let events = session.events().clone();
    if running().lock().is_ok_and(|m| m.contains_key(&p.job_id)) {
        // Already followed by this process.
        return Ok(Started {
            job_id: p.job_id,
            result: None,
        });
    }
    // A run that never got its solver setup was still meshing when the app ended.
    if record.state == JobState::Pending
        && record.container.is_none()
        && !store.dir(&p.job_id).join(solve::SETUP_FILE).is_file()
    {
        return Err(mark_lost(runner, &store, &mut record));
    }
    if store.dir(&p.job_id).join(solve::SETUP_FILE).is_file() {
        let job = solve::SolveJob::load(runner, &root, &p.job_id)?;
        let cancel = cancel_flag(&p.job_id);
        let resume = !record.state.is_finished();
        if p.wait {
            let result = solve::run(job, &cancel, &events, resume);
            forget(&p.job_id);
            return Ok(Started {
                job_id: p.job_id,
                result: Some(result?),
            });
        }
        let id = p.job_id.clone();
        std::thread::spawn(move || {
            if let Err(e) = solve::run(job, &cancel, &events, resume) {
                events.emit(
                    "run.failed",
                    json!({"job_id": id, "code": e.code, "detail": e.detail}),
                );
            }
            forget(&id);
        });
        return Ok(Started {
            job_id: p.job_id,
            result: None,
        });
    }
    // Other jobs (reference case): continue following the stages.
    let options = RunOptions {
        cores: record.cores.max(1),
        poll_interval: Duration::from_millis(500),
        stop_after: None,
    };
    let cancel = cancel_flag(&record.id);
    let runner = runner.clone();
    let id = record.id.clone();
    let mut follow = move || {
        let job = record.id.clone();
        execute_job(
            runner.as_ref(),
            &store,
            &mut record,
            &options,
            &cancel,
            &mut |e| {
                if let Some((name, payload)) = event_payload(&job, &e) {
                    events.emit(name, payload);
                }
            },
        );
        forget(&job);
    };
    if p.wait {
        follow();
    } else {
        std::thread::spawn(follow);
    }
    Ok(Started {
        job_id: id,
        result: None,
    })
}

fn cancel(runner: &SharedRunner, session: &mut Session, p: JobParams) -> Result<Status, FoamError> {
    let store = store(session)?;
    let mut record = store.load(&p.job_id)?;
    let flag = running()
        .lock()
        .ok()
        .and_then(|m| m.get(&p.job_id).cloned());
    if let Some(flag) = flag {
        // The thread that follows the job stops the container and records the state.
        flag.store(true, Ordering::SeqCst);
    } else if !record.state.is_finished() {
        if let Some(id) = &record.container {
            runner.stop(&Container { id: id.clone() })?;
        }
        record.state = JobState::Cancelled;
        store.save(&record)?;
    }
    Ok(status_of(&record))
}

pub fn register(r: &mut Registry, runner: SharedRunner) {
    let run_errors: &'static [&'static str] = &[
        "project.none_open",
        "runtime.docker_missing",
        "runtime.daemon_not_running",
        "runtime.image_missing",
        "scenario.no_geometry",
        "mesh.leak",
        "mesh.failed",
        "run.diverged",
        "run.cancelled",
        "run.foam_fatal_error",
        "run.stage_failed",
    ];
    let starter = runner.clone();
    r.add(
        "run.start",
        "Run the wind tunnel of the open project (meshing first if needed; without a scenario: the          reference case). Returns at once, or with `wait` the result when the run has ended.",
        Options {
            undoable: false,
            errors: run_errors,
            example: json!({"iterations": 200}),
        },
        move |s, p: StartParams| start(&starter, s, p),
    );
    let queue_runner = runner.clone();
    r.add(
        "run.start_all",
        "Run several scenarios one after another in the background (all set-up scenarios, or the \
         given ids). A scenario reuses the mesh of an earlier one when it fits; a failure is \
         recorded and the queue goes on. Events run.queue and run.queue_finished.",
        Options {
            undoable: false,
            errors: &[
                "project.none_open",
                "scenario.none",
                "scenario.not_found",
                "scenario.no_geometry",
                "runtime.docker_missing",
                "runtime.daemon_not_running",
            ],
            example: json!({"scenarios": ["scenario-1", "scenario-2"]}),
        },
        move |s, p: StartAllParams| start_all(&queue_runner, s, p),
    );
    r.add(
        "run.cancel_queue",
        "Stop the running queue of scenarios: the current run is cancelled, the others skipped.",
        Options {
            undoable: false,
            errors: &["project.none_open"],
            example: json!({}),
        },
        |s, _: crate::results::JobParams| cancel_queue(s),
    );
    r.add(
        "run.log",
        "Return the last lines of the log of a job of the open project.",
        Options {
            undoable: false,
            errors: &["project.none_open", "job.not_found"],
            example: json!({"job_id": "job-1795221439-15240-0", "lines": 200}),
        },
        |s, p: LogParams| {
            let text = full_log(&store(s)?, &p.job_id)?;
            let all: Vec<&str> = text.lines().collect();
            let n = p.lines.unwrap_or(200);
            Ok(LogTail {
                lines: all[all.len().saturating_sub(n)..]
                    .iter()
                    .map(|l| l.to_string())
                    .collect(),
            })
        },
    );
    r.add(
        "error.describe",
        "Return the English and German text and suggestion of message codes (unknown codes are left out).",
        Options {
            undoable: false,
            errors: &[],
            example: json!({"codes": ["run.diverged"]}),
        },
        |_, p: DescribeParams| {
            Ok(p.codes
                .iter()
                .filter_map(|c| {
                    Some((
                        c.clone(),
                        Described {
                            en: message(c, Language::En)?.clone(),
                            de: message(c, Language::De)?.clone(),
                        },
                    ))
                })
                .collect::<std::collections::BTreeMap<_, _>>())
        },
    );
    r.add(
        "run.active",
        "List the unfinished jobs of the open project.",
        Options {
            undoable: false,
            errors: &["project.none_open"],
            example: json!({}),
        },
        |s, _: foamcraft_sim::commands::NoParams| active(s),
    );
    let attacher = runner.clone();
    r.add(
        "run.attach",
        "Follow a running job of the open project again (after a restart of the app).",
        Options {
            undoable: false,
            errors: &[
                "project.none_open",
                "job.not_found",
                "run.lost",
                "run.diverged",
            ],
            example: json!({"job_id": "job-1795221439-15240-0"}),
        },
        move |s, p: AttachParams| attach(&attacher, s, p),
    );
    r.add(
        "run.result",
        "Return the stored result of a wind tunnel run: forces, coefficients, convergence, y+.",
        Options {
            undoable: false,
            errors: &["project.none_open", "run.no_result"],
            example: json!({}),
        },
        |s, p: ResultParams| {
            let root = s.project()?.root.join(solve::RESULTS_DIR);
            solve::load_result(&root, p.job_id.as_deref())
        },
    );
    r.add(
        "run.status",
        "Report the state of a job of the open project.",
        Options {
            undoable: false,
            errors: &["job.not_found", "project.none_open"],
            example: json!({"job_id": "job-1795221439-15240-0"}),
        },
        |s, p: JobParams| Ok(status_of(&store(s)?.load(&p.job_id)?)),
    );
    r.add(
        "run.cancel",
        "Cancel a job of the open project; all its processes end.",
        Options {
            undoable: false,
            errors: &["job.not_found", "project.none_open"],
            example: json!({"job_id": "job-1795221439-15240-0"}),
        },
        move |s, p: JobParams| cancel(&runner, s, p),
    );
}
