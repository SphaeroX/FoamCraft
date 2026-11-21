// SPDX-License-Identifier: GPL-3.0-or-later
//! Job state machine against a fake stage executor: success, failure, cancel, resume.

use std::collections::{BTreeMap, VecDeque};
use std::sync::atomic::{AtomicBool, Ordering};

use foamcraft_core::error::FoamError;
use foamcraft_core::job::{
    JobEvent, JobState, JobStore, RunOptions, Stage, StageExecutor, StagePoll, choose_cores,
    run_job, stage_script, stages_for,
};

/// Answers polls from a per-stage script of (log text, exit code) chunks.
#[derive(Default)]
struct FakeExecutor {
    script: BTreeMap<Stage, VecDeque<StagePoll>>,
    started: Vec<Stage>,
    cancelled: bool,
    offsets_seen: Vec<(Stage, u64)>,
}

impl FakeExecutor {
    fn stage(mut self, stage: Stage, polls: &[(&str, Option<i32>)]) -> Self {
        self.script.insert(
            stage,
            polls
                .iter()
                .map(|(t, e)| StagePoll {
                    text: t.to_string(),
                    exit: *e,
                })
                .collect(),
        );
        self
    }

    fn all_ok() -> Self {
        Self::default()
            .stage(Stage::Background, &[("Creating block mesh\nEnd\n", Some(0))])
            .stage(Stage::Mesh, &[("Refinement phase\n", None), ("Finished meshing\nEnd\n", Some(0))])
            .stage(Stage::Check, &[("Time = 0\n    cells:            40000\nMesh OK.\nEnd\n", Some(0))])
            .stage(Stage::Decompose, &[("End\n", Some(0))])
            .stage(
                Stage::Solve,
                &[
                    ("Time = 1\nGAMG:  Solving for p, Initial residual = 1, Final residual = 0.1, No Iterations 2\n", None),
                    ("Time = 2\nGAMG:  Solving for p, Initial residual = 0.5, Final residual = 0.1, No Iterations 2\nEnd\n", Some(0)),
                ],
            )
            .stage(Stage::Reconstruct, &[("End\n", Some(0))])
    }
}

impl StageExecutor for FakeExecutor {
    fn start(&mut self, stage: Stage, _script: &str) -> Result<(), FoamError> {
        self.started.push(stage);
        Ok(())
    }

    fn poll(&mut self, stage: Stage, offset: u64) -> Result<StagePoll, FoamError> {
        self.offsets_seen.push((stage, offset));
        let queue = self.script.get_mut(&stage).expect("stage scripted");
        Ok(queue.pop_front().unwrap_or(StagePoll {
            text: String::new(),
            exit: None,
        }))
    }

    fn cancel(&mut self) -> Result<(), FoamError> {
        self.cancelled = true;
        Ok(())
    }
}

fn temp_store(name: &str) -> JobStore {
    let dir = std::env::temp_dir().join(format!("fc-jobs-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    JobStore::new(dir)
}

fn options() -> RunOptions {
    RunOptions {
        cores: 4,
        poll_interval: std::time::Duration::ZERO,
        stop_after: None,
    }
}

#[test]
fn successful_job_runs_every_stage_and_persists_log() {
    let store = temp_store("ok");
    let mut record = store.create("reference_cube").unwrap();
    let mut exec = FakeExecutor::all_ok();
    let mut events = Vec::new();
    let cancel = AtomicBool::new(false);
    let state = run_job(
        &mut record,
        &mut exec,
        &store,
        &options(),
        &cancel,
        &mut |e| events.push(e),
    );
    assert_eq!(state, JobState::Succeeded);
    assert_eq!(exec.started, stages_for(4));
    let loaded = store.load(&record.id).unwrap();
    assert_eq!(loaded.state, JobState::Succeeded);
    assert_eq!(loaded.cells, Some(40000));
    let log = store.read_log(&record.id).unwrap();
    assert!(log.contains("[mesh] Refinement phase"), "{log}");
    assert!(log.contains("[solve] Time = 2"), "{log}");
    assert!(events.contains(&JobEvent::StageStarted(Stage::Solve)));
    assert!(events.iter().any(|e| matches!(
        e,
        JobEvent::Progress {
            stage: Stage::Solve,
            ..
        }
    )));
    assert!(
        !events
            .iter()
            .any(|e| matches!(e, JobEvent::Progress { stage, .. } if *stage != Stage::Solve)),
        "progress only comes from the solver"
    );
    assert_eq!(
        events.last(),
        Some(&JobEvent::Finished(JobState::Succeeded))
    );
}

#[test]
fn failing_stage_ends_with_its_error_code() {
    let store = temp_store("fail");
    let mut record = store.create("reference_cube").unwrap();
    let mut exec = FakeExecutor::all_ok().stage(
        Stage::Mesh,
        &[(
            "--> FOAM FATAL IO ERROR: (openfoam-2606)\nEntry 'x' not found\n\nFOAM exiting\n",
            Some(1),
        )],
    );
    let cancel = AtomicBool::new(false);
    let state = run_job(
        &mut record,
        &mut exec,
        &store,
        &options(),
        &cancel,
        &mut |_| {},
    );
    match &state {
        JobState::Failed { stage, error } => {
            assert_eq!(*stage, Stage::Mesh);
            assert_eq!(error.code, "run.foam_fatal_error");
        }
        other => panic!("unexpected {other:?}"),
    }
    assert_eq!(exec.started, [Stage::Background, Stage::Mesh]);
    assert_eq!(store.load(&record.id).unwrap().state, state);
}

#[test]
fn non_zero_exit_without_message_is_stage_failed() {
    let store = temp_store("exit");
    let mut record = store.create("reference_cube").unwrap();
    let mut exec = FakeExecutor::all_ok().stage(Stage::Background, &[("", Some(137))]);
    let cancel = AtomicBool::new(false);
    let state = run_job(
        &mut record,
        &mut exec,
        &store,
        &options(),
        &cancel,
        &mut |_| {},
    );
    assert!(
        matches!(state, JobState::Failed { ref error, .. } if error.code == "run.stage_failed")
    );
}

#[test]
fn divergence_stops_the_job() {
    let store = temp_store("div");
    let mut record = store.create("reference_cube").unwrap();
    let mut exec = FakeExecutor::all_ok().stage(
        Stage::Solve,
        &[("Time = 1\nsmoothSolver:  Solving for Ux, Initial residual = nan, Final residual = nan, No Iterations 1000\n", None)],
    );
    let cancel = AtomicBool::new(false);
    let state = run_job(
        &mut record,
        &mut exec,
        &store,
        &options(),
        &cancel,
        &mut |_| {},
    );
    assert!(matches!(state, JobState::Failed { ref error, .. } if error.code == "run.diverged"));
    assert!(exec.cancelled, "a diverged run must be stopped");
}

#[test]
fn cancel_stops_the_executor_and_marks_the_job() {
    let store = temp_store("cancel");
    let mut record = store.create("reference_cube").unwrap();
    let mut exec = FakeExecutor::all_ok();
    let cancel = AtomicBool::new(false);
    let state = run_job(
        &mut record,
        &mut exec,
        &store,
        &options(),
        &cancel,
        &mut |e| {
            if e == JobEvent::StageStarted(Stage::Solve) {
                cancel.store(true, Ordering::SeqCst);
            }
        },
    );
    assert_eq!(state, JobState::Cancelled);
    assert!(exec.cancelled);
    assert_eq!(store.load(&record.id).unwrap().state, JobState::Cancelled);
}

#[test]
fn resume_continues_a_running_stage_from_its_offset() {
    let store = temp_store("resume");
    let mut record = store.create("reference_cube").unwrap();
    record.state = JobState::Running(Stage::Solve);
    record.cores = 4;
    record.offsets.insert(Stage::Solve, 120);
    store.save(&record).unwrap();

    let mut record = store.load(&record.id).unwrap();
    let mut exec = FakeExecutor::all_ok();
    let cancel = AtomicBool::new(false);
    let state = run_job(
        &mut record,
        &mut exec,
        &store,
        &options(),
        &cancel,
        &mut |_| {},
    );
    assert_eq!(state, JobState::Succeeded);
    assert_eq!(
        exec.started,
        [Stage::Reconstruct],
        "solve must not be restarted"
    );
    assert_eq!(exec.offsets_seen[0], (Stage::Solve, 120));
}

#[test]
fn jobs_can_be_listed_and_missing_ones_report_not_found() {
    let store = temp_store("list");
    let a = store.create("a").unwrap();
    let b = store.create("b").unwrap();
    let ids: Vec<_> = store.list().unwrap().into_iter().map(|r| r.id).collect();
    assert!(ids.contains(&a.id) && ids.contains(&b.id));
    assert_eq!(store.load("nope").unwrap_err().code, "job.not_found");
}

#[test]
fn cores_follow_cells_and_limits() {
    assert_eq!(choose_cores(15_000, 24, 32), 1);
    assert_eq!(choose_cores(95_000, 24, 32), 9);
    assert_eq!(choose_cores(5_000_000, 24, 32), 24);
    assert_eq!(choose_cores(5_000_000, 24, 8), 8);
    assert_eq!(choose_cores(0, 0, 0), 1);
}

#[test]
fn serial_runs_skip_decompose_and_reconstruct() {
    assert_eq!(
        stages_for(1),
        [Stage::Background, Stage::Mesh, Stage::Check, Stage::Solve]
    );
    assert_eq!(stages_for(2).len(), 6);
}

#[test]
fn solve_script_runs_parallel_with_the_chosen_cores() {
    assert!(stage_script(Stage::Solve, 8).contains("-np 8"));
    assert!(stage_script(Stage::Solve, 8).contains("-parallel"));
    assert!(!stage_script(Stage::Solve, 1).contains("mpirun"));
    assert!(stage_script(Stage::Decompose, 8).contains("numberOfSubdomains"));
}

#[test]
fn a_job_can_stop_after_the_mesh_check() {
    let store = temp_store("stop");
    let mut record = store.create("wind_tunnel").unwrap();
    let mut exec = FakeExecutor::all_ok();
    let options = RunOptions {
        stop_after: Some(Stage::Check),
        ..options()
    };
    let state = run_job(
        &mut record,
        &mut exec,
        &store,
        &options,
        &AtomicBool::new(false),
        &mut |_| {},
    );
    assert_eq!(state, JobState::Succeeded);
    assert_eq!(exec.started, [Stage::Background, Stage::Mesh, Stage::Check]);
    assert_eq!(record.cells, Some(40000));
}

#[test]
fn a_job_can_start_on_an_existing_mesh() {
    // A solve job reuses the mesh of an earlier job: it starts at the solver stages.
    let store = temp_store("start-at");
    let mut record = store.create("wind_tunnel").unwrap();
    record.start_at = Some(Stage::Decompose);
    record.cores = 4;
    record.cells = Some(40000);
    let mut exec = FakeExecutor::all_ok();
    let state = run_job(
        &mut record,
        &mut exec,
        &store,
        &options(),
        &AtomicBool::new(false),
        &mut |_| {},
    );
    assert_eq!(state, JobState::Succeeded);
    assert_eq!(
        exec.started,
        [Stage::Decompose, Stage::Solve, Stage::Reconstruct]
    );
    // The start stage survives a reload of the record.
    assert_eq!(
        store.load(&record.id).unwrap().start_at,
        Some(Stage::Decompose)
    );
}

#[test]
fn the_solver_stage_initialises_the_flow_with_potential_flow() {
    let serial = stage_script(Stage::Solve, 1);
    assert!(serial.contains("potentialFoam") && serial.contains("simpleFoam"));
    let parallel = stage_script(Stage::Solve, 4);
    assert!(parallel.contains("-np 4 potentialFoam -parallel"));
}
