// SPDX-License-Identifier: GPL-3.0-or-later
//! Job pipeline against the real runtime (needs Docker and the runtime image):
//!     cargo test --workspace -- --ignored runtime_

use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use foamcraft_core::case::REFERENCE_CUBE;
use foamcraft_core::job::{JobEvent, JobState, JobStore, RunOptions, Stage};
use foamcraft_runtime::jobs::{core_limit, execute_job, prepare_job, remove_job_resources};
use foamcraft_runtime::{DockerRunner, Runner};

fn runner() -> DockerRunner {
    let r = DockerRunner::local();
    r.check().expect("docker must be running");
    assert!(
        r.image_present().expect("image check"),
        "run `foamcraft runtime build` first"
    );
    r
}

fn store(name: &str) -> JobStore {
    JobStore::new(
        std::env::temp_dir().join(format!("fc-runtime-jobs-{name}-{}", std::process::id())),
    )
}

fn options(cores: u32) -> RunOptions {
    RunOptions {
        cores,
        poll_interval: Duration::from_millis(300),
        stop_after: None,
    }
}

fn container_exists(id: &str) -> bool {
    std::process::Command::new("docker")
        .args(["container", "inspect", id])
        .output()
        .is_ok_and(|o| o.status.success())
}

#[test]
#[ignore]
fn runtime_reference_case_runs_in_parallel_in_under_five_minutes() {
    let r = runner();
    let store = store("ok");
    let mut record = prepare_job(&r, &store, REFERENCE_CUBE.name, |dir| {
        REFERENCE_CUBE.write(dir, &[])
    })
    .unwrap();
    let limit = core_limit(&r, &record);
    let t0 = Instant::now();
    let mut iterations = 0;
    let state = execute_job(
        &r,
        &store,
        &mut record,
        &options(limit),
        &AtomicBool::new(false),
        &mut |e| {
            if matches!(
                e,
                JobEvent::Progress {
                    stage: Stage::Solve,
                    ..
                }
            ) {
                iterations += 1;
            }
        },
    );
    let elapsed = t0.elapsed();
    eprintln!(
        "reference case: {:?}, {} cells, {} cores, {:.1} s",
        state,
        record.cells.unwrap_or(0),
        record.cores,
        elapsed.as_secs_f64()
    );
    assert_eq!(state, JobState::Succeeded);
    assert!(elapsed < Duration::from_secs(300), "took {elapsed:?}");
    assert!(
        record.cores > 1,
        "expected a parallel run, cells {:?}",
        record.cells
    );
    assert!(iterations > 10);
    let log = store.read_log(&record.id).unwrap();
    for part in [
        "[background] End",
        "[mesh] Finished meshing",
        "[check] Mesh OK.",
        "[solve] End",
        "[reconstruct] End",
    ] {
        assert!(log.contains(part), "{part} missing in log");
    }
    assert!(!container_exists(record.container.as_deref().unwrap()));
    remove_job_resources(&r, &record).unwrap();
}

#[test]
#[ignore]
fn runtime_cancel_during_solve_ends_all_processes() {
    let r = runner();
    let store = store("cancel");
    let mut record = prepare_job(&r, &store, REFERENCE_CUBE.name, |dir| {
        // Many iterations so the solver is surely still running when we cancel.
        REFERENCE_CUBE.write(dir, &[("END_TIME", "100000")])
    })
    .unwrap();
    let container = record.container.clone().unwrap();
    let limit = core_limit(&r, &record);
    let cancel = AtomicBool::new(false);
    let mut solver_was_running = false;
    let state = execute_job(
        &r,
        &store,
        &mut record,
        &options(limit),
        &cancel,
        &mut |e| {
            if let JobEvent::Progress {
                stage: Stage::Solve,
                iteration,
                ..
            } = e
                && iteration >= 5.0
                && !cancel.load(Ordering::SeqCst)
            {
                let ps = std::process::Command::new("docker")
                    .args(["exec", &container, "ps", "-eo", "comm"])
                    .output()
                    .unwrap();
                solver_was_running = String::from_utf8_lossy(&ps.stdout).contains("simpleFoam");
                cancel.store(true, Ordering::SeqCst);
            }
        },
    );
    assert_eq!(state, JobState::Cancelled);
    assert!(
        solver_was_running,
        "simpleFoam was not running before the cancel"
    );
    assert!(
        !container_exists(&container),
        "container still exists after cancel"
    );
    assert_eq!(store.load(&record.id).unwrap().state, JobState::Cancelled);
    remove_job_resources(&r, &record).unwrap();
}

#[test]
#[ignore]
fn runtime_broken_case_ends_with_an_error_code() {
    let r = runner();
    let store = store("broken");
    let mut record = prepare_job(&r, &store, REFERENCE_CUBE.name, |dir| {
        REFERENCE_CUBE.write(dir, &[])?;
        std::fs::write(
            dir.join("system/blockMeshDict"),
            "FoamFile { class dictionary; }\nblocks (;\n",
        )
        .map_err(|e| foamcraft_core::error::FoamError::new("job.storage_failed", e.to_string()))
    })
    .unwrap();
    let state = execute_job(
        &r,
        &store,
        &mut record,
        &options(2),
        &AtomicBool::new(false),
        &mut |_| {},
    );
    match state {
        JobState::Failed { stage, error } => {
            assert_eq!(stage, Stage::Background);
            assert_eq!(error.code, "run.foam_fatal_error", "{error:?}");
        }
        other => panic!("expected failure, got {other:?}"),
    }
    remove_job_resources(&r, &record).unwrap();
}
