// SPDX-License-Identifier: GPL-3.0-or-later
//! Simulation jobs: stages, state machine, persistent record and log, event stream.
//!
//! The host drives the stages; a [`StageExecutor`] starts each stage in the runtime and reports
//! new log text and the exit code. Each stage runs detached in the runtime, so a job keeps running
//! when the host process ends and can be resumed from the record on disk.

use std::collections::BTreeMap;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::error::FoamError;
use crate::log::{LogEvent, LogParser, LogSummary};

/// Below this many cells per core, parallel runs get slower instead of faster.
pub const MIN_CELLS_PER_CORE: u64 = 10_000;

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Serialize,
    Deserialize,
    schemars::JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum Stage {
    /// Background mesh (blockMesh).
    Background,
    /// Body-fitted mesh (snappyHexMesh).
    Mesh,
    /// Mesh quality check (checkMesh).
    Check,
    /// Split the case for parallel runs (decomposePar).
    Decompose,
    /// Solver.
    Solve,
    /// Merge parallel results (reconstructPar).
    Reconstruct,
}

impl Stage {
    pub fn id(self) -> &'static str {
        match self {
            Stage::Background => "background",
            Stage::Mesh => "mesh",
            Stage::Check => "check",
            Stage::Decompose => "decompose",
            Stage::Solve => "solve",
            Stage::Reconstruct => "reconstruct",
        }
    }
}

/// Stages of a run with `cores` processes.
pub fn stages_for(cores: u32) -> Vec<Stage> {
    if cores > 1 {
        vec![
            Stage::Background,
            Stage::Mesh,
            Stage::Check,
            Stage::Decompose,
            Stage::Solve,
            Stage::Reconstruct,
        ]
    } else {
        vec![Stage::Background, Stage::Mesh, Stage::Check, Stage::Solve]
    }
}

/// Shell command of a stage, run in the case folder with the OpenFOAM environment loaded.
pub fn stage_script(stage: Stage, cores: u32) -> String {
    match stage {
        Stage::Background => "blockMesh".into(),
        Stage::Mesh => "snappyHexMesh -overwrite".into(),
        // The surface of the model patches is extracted for the mesh preview; failing that is fine.
        Stage::Check => "checkMesh && (surfaceMeshExtract -constant -patches '(\"model.*\")' \
                         preview.obj > log.preview 2>&1 || true)"
            .into(),
        Stage::Decompose => format!(
            "foamDictionary -entry numberOfSubdomains -set {cores} system/decomposeParDict \
             > /dev/null && decomposePar -force"
        ),
        // potentialFoam initialises the flow; its log is shown only when it fails.
        Stage::Solve if cores > 1 => format!(
            "mpirun --oversubscribe -np {cores} potentialFoam -parallel -writep -initialiseUBCs \
             > log.potentialFoam 2>&1 || {{ tail -n 30 log.potentialFoam; exit 1; }}; \
             mpirun --oversubscribe -np {cores} simpleFoam -parallel"
        ),
        Stage::Solve => "potentialFoam -writep -initialiseUBCs > log.potentialFoam 2>&1 \
                         || { tail -n 30 log.potentialFoam; exit 1; }; simpleFoam"
            .into(),
        Stage::Reconstruct => "reconstructPar -latestTime".into(),
    }
}

/// Process count: one per [`MIN_CELLS_PER_CORE`] cells, at most the smaller of the physical
/// cores and the cores the container may use (0 means unknown).
pub fn choose_cores(cells: u64, physical: u32, container: u32) -> u32 {
    let limit = match (physical, container) {
        (0, 0) => 1,
        (0, c) => c,
        (p, 0) => p,
        (p, c) => p.min(c),
    };
    let wanted = (cells / MIN_CELLS_PER_CORE).min(u64::from(u32::MAX)) as u32;
    wanted.clamp(1, limit.max(1))
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum JobState {
    Pending,
    Running(Stage),
    Succeeded,
    Failed { stage: Stage, error: FoamError },
    Cancelled,
}

impl JobState {
    pub fn is_finished(&self) -> bool {
        matches!(
            self,
            JobState::Succeeded | JobState::Failed { .. } | JobState::Cancelled
        )
    }
}

/// What is stored in `<jobs>/<id>/job.json`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JobRecord {
    pub id: String,
    /// Name of the case template or project.
    pub case: String,
    pub state: JobState,
    /// Process count of the solve stage (decided after the mesh check).
    pub cores: u32,
    pub cells: Option<u64>,
    /// Bytes of each stage log already read.
    pub offsets: BTreeMap<Stage, u64>,
    /// Runtime resources, set by the host that runs the job.
    pub container: Option<String>,
    pub volume: Option<String>,
    /// First stage when the job reuses an existing mesh (for example `Decompose` or `Solve`).
    #[serde(default)]
    pub start_at: Option<Stage>,
    /// Unix time in seconds.
    pub created: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub enum JobEvent {
    StageStarted(Stage),
    Log {
        stage: Stage,
        line: String,
    },
    /// snappyHexMesh phase or solver iteration.
    Phase {
        stage: Stage,
        phase: String,
    },
    Progress {
        stage: Stage,
        iteration: f64,
        residuals: BTreeMap<String, f64>,
    },
    StageFinished(Stage),
    Finished(JobState),
}

/// New output of a stage since the last poll.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct StagePoll {
    /// Complete lines only (ends with a line break or is empty).
    pub text: String,
    /// Exit code once the stage has finished.
    pub exit: Option<i32>,
}

/// Runs stages in the runtime.
pub trait StageExecutor {
    /// Starts `script` for `stage` in the background.
    fn start(&mut self, stage: Stage, script: &str) -> Result<(), FoamError>;
    /// Log text from byte `offset` on, and the exit code if the stage has ended.
    fn poll(&mut self, stage: Stage, offset: u64) -> Result<StagePoll, FoamError>;
    /// Stops all processes of the job.
    fn cancel(&mut self) -> Result<(), FoamError>;
}

#[derive(Debug, Clone)]
pub struct RunOptions {
    /// Upper limit for parallel processes.
    pub cores: u32,
    /// Pause between polls while a stage produces no output.
    pub poll_interval: Duration,
    /// End the job successfully after this stage (for example only meshing: `Check`).
    pub stop_after: Option<Stage>,
}

/// Job records and logs in a folder.
#[derive(Debug, Clone)]
pub struct JobStore {
    root: PathBuf,
}

fn storage(e: impl std::fmt::Display) -> FoamError {
    FoamError::new("job.storage_failed", e.to_string())
}

impl JobStore {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn dir(&self, id: &str) -> PathBuf {
        self.root.join(id)
    }

    pub fn create(&self, case: &str) -> Result<JobRecord, FoamError> {
        static COUNTER: AtomicU32 = AtomicU32::new(0);
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default();
        let id = format!(
            "job-{}-{}-{}",
            now.as_secs(),
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::Relaxed)
        );
        let record = JobRecord {
            id,
            case: case.into(),
            state: JobState::Pending,
            cores: 1,
            cells: None,
            offsets: BTreeMap::new(),
            container: None,
            volume: None,
            start_at: None,
            created: now.as_secs(),
        };
        std::fs::create_dir_all(self.dir(&record.id)).map_err(storage)?;
        self.save(&record)?;
        Ok(record)
    }

    pub fn save(&self, record: &JobRecord) -> Result<(), FoamError> {
        let dir = self.dir(&record.id);
        std::fs::create_dir_all(&dir).map_err(storage)?;
        let json = serde_json::to_string_pretty(record).map_err(storage)?;
        // Write and rename so a crash never leaves a half-written record.
        let tmp = dir.join("job.json.tmp");
        std::fs::write(&tmp, json).map_err(storage)?;
        std::fs::rename(&tmp, dir.join("job.json")).map_err(storage)
    }

    pub fn load(&self, id: &str) -> Result<JobRecord, FoamError> {
        let path = self.dir(id).join("job.json");
        if !path.is_file() {
            return Err(FoamError::new("job.not_found", id));
        }
        let text = std::fs::read_to_string(path).map_err(storage)?;
        serde_json::from_str(&text).map_err(storage)
    }

    /// All jobs, oldest first.
    pub fn list(&self) -> Result<Vec<JobRecord>, FoamError> {
        let Ok(entries) = std::fs::read_dir(&self.root) else {
            return Ok(vec![]);
        };
        let mut jobs: Vec<JobRecord> = entries
            .flatten()
            .filter_map(|e| self.load(&e.file_name().to_string_lossy()).ok())
            .collect();
        jobs.sort_by(|a, b| (a.created, &a.id).cmp(&(b.created, &b.id)));
        Ok(jobs)
    }

    pub fn append_log(&self, id: &str, stage: Stage, lines: &[&str]) -> Result<(), FoamError> {
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(self.dir(id).join("job.log"))
            .map_err(storage)?;
        for line in lines {
            writeln!(file, "[{}] {line}", stage.id()).map_err(storage)?;
        }
        Ok(())
    }

    pub fn read_log(&self, id: &str) -> Result<String, FoamError> {
        let path = self.dir(id).join("job.log");
        if !self.dir(id).join("job.json").is_file() {
            return Err(FoamError::new("job.not_found", id));
        }
        Ok(std::fs::read_to_string(path).unwrap_or_default())
    }
}

enum StageOutcome {
    Done(LogSummary),
    Failed(FoamError),
    Cancelled,
}

struct Ctx<'a, E: StageExecutor> {
    exec: &'a mut E,
    store: &'a JobStore,
    options: &'a RunOptions,
    cancel: &'a AtomicBool,
    on_event: &'a mut dyn FnMut(JobEvent),
}

impl<E: StageExecutor> Ctx<'_, E> {
    fn run_stage(&mut self, record: &mut JobRecord, stage: Stage, resume: bool) -> StageOutcome {
        if !resume {
            record.offsets.insert(stage, 0);
            if let Err(e) = self.exec.start(stage, &stage_script(stage, record.cores)) {
                return StageOutcome::Failed(e);
            }
        }
        record.state = JobState::Running(stage);
        if let Err(e) = self.store.save(record) {
            return StageOutcome::Failed(e);
        }
        (self.on_event)(JobEvent::StageStarted(stage));

        let mut parser = LogParser::default();
        let mut summary = LogSummary::default();
        loop {
            if self.cancel.load(Ordering::SeqCst) {
                let _ = self.exec.cancel();
                return StageOutcome::Cancelled;
            }
            let offset = record.offsets.get(&stage).copied().unwrap_or(0);
            let poll = match self.exec.poll(stage, offset) {
                Ok(p) => p,
                Err(e) => return StageOutcome::Failed(e),
            };
            let before = summary.last_iteration;
            let lines: Vec<&str> = poll.text.lines().collect();
            if !lines.is_empty()
                && let Err(e) = self.store.append_log(&record.id, stage, &lines)
            {
                return StageOutcome::Failed(e);
            }
            for line in &lines {
                (self.on_event)(JobEvent::Log {
                    stage,
                    line: line.to_string(),
                });
                for event in parser.feed(line) {
                    if let LogEvent::Phase(p) = &event {
                        (self.on_event)(JobEvent::Phase {
                            stage,
                            phase: p.clone(),
                        });
                    }
                    summary.apply(&event);
                }
            }
            if poll.exit.is_some() {
                parser.finish().iter().for_each(|e| summary.apply(e));
            }
            let new_iteration = summary.last_iteration.filter(|i| Some(*i) != before);
            // Utilities also print "Time = 0"; only solver iterations are progress.
            if let Some(iteration) = new_iteration.filter(|_| stage == Stage::Solve) {
                (self.on_event)(JobEvent::Progress {
                    stage,
                    iteration,
                    residuals: summary.residuals.clone(),
                });
            }
            record
                .offsets
                .insert(stage, offset + poll.text.len() as u64);
            if let Err(e) = self.store.save(record) {
                return StageOutcome::Failed(e);
            }
            if let Some(error) = summary.errors.first().cloned() {
                if poll.exit.is_none() {
                    // A diverged or crashed process may hang; stop the job.
                    let _ = self.exec.cancel();
                }
                return StageOutcome::Failed(error);
            }
            match poll.exit {
                Some(0) => return StageOutcome::Done(summary),
                Some(code) => {
                    return StageOutcome::Failed(FoamError::new(
                        "run.stage_failed",
                        format!("{} exited with code {code}", stage.id()),
                    ));
                }
                None if poll.text.is_empty() => std::thread::sleep(self.options.poll_interval),
                None => {}
            }
        }
    }
}

fn finish(
    record: &mut JobRecord,
    store: &JobStore,
    state: JobState,
    on_event: &mut dyn FnMut(JobEvent),
) -> JobState {
    record.state = state.clone();
    let _ = store.save(record);
    on_event(JobEvent::Finished(state.clone()));
    state
}

/// Runs (or resumes) a job until it ends. A record in `Running(stage)` continues that stage from
/// its stored log offset without starting it again.
pub fn run_job<E: StageExecutor>(
    record: &mut JobRecord,
    exec: &mut E,
    store: &JobStore,
    options: &RunOptions,
    cancel: &AtomicBool,
    on_event: &mut dyn FnMut(JobEvent),
) -> JobState {
    if record.state.is_finished() {
        return record.state.clone();
    }
    let resume_stage = match record.state {
        JobState::Running(stage) => Some(stage),
        _ => None,
    };
    let mut ctx = Ctx {
        exec,
        store,
        options,
        cancel,
        on_event,
    };
    let mut stage = resume_stage
        .or(record.start_at)
        .unwrap_or(Stage::Background);
    let mut resume = resume_stage.is_some();
    loop {
        match ctx.run_stage(record, stage, resume) {
            StageOutcome::Cancelled => {
                return finish(record, store, JobState::Cancelled, ctx.on_event);
            }
            StageOutcome::Failed(error) => {
                return finish(
                    record,
                    store,
                    JobState::Failed { stage, error },
                    ctx.on_event,
                );
            }
            StageOutcome::Done(summary) => {
                (ctx.on_event)(JobEvent::StageFinished(stage));
                if stage == Stage::Check {
                    record.cells = summary.cells;
                    record.cores = choose_cores(summary.cells.unwrap_or(0), options.cores, 0);
                }
                if options.stop_after == Some(stage) {
                    return finish(record, store, JobState::Succeeded, ctx.on_event);
                }
            }
        }
        let stages = stages_for(record.cores);
        let next = stages
            .iter()
            .position(|s| *s == stage)
            .and_then(|i| stages.get(i + 1));
        match next {
            Some(next) => {
                stage = *next;
                resume = false;
            }
            None => return finish(record, store, JobState::Succeeded, ctx.on_event),
        }
    }
}
