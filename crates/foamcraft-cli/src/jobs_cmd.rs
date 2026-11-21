// SPDX-License-Identifier: GPL-3.0-or-later
//! `foamcraft run` and `foamcraft job ...`.

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use foamcraft_core::case::REFERENCE_CUBE;
use foamcraft_core::error::{FoamError, Language};
use foamcraft_core::job::{JobEvent, JobRecord, JobState, JobStore, RunOptions};
use foamcraft_runtime::DockerRunner;
use foamcraft_runtime::jobs::{core_limit, execute_job, prepare_job};

struct Texts {
    job: &'static str,
    stage: &'static str,
    phase: &'static str,
    iteration: &'static str,
    cells: &'static str,
    cores: &'static str,
    succeeded: &'static str,
    cancelled: &'static str,
    failed: &'static str,
    cancel_hint: &'static str,
}

const EN: Texts = Texts {
    job: "Job",
    stage: "Step",
    phase: "Phase",
    iteration: "Iteration",
    cells: "Cells",
    cores: "Cores",
    succeeded: "Finished.",
    cancelled: "Cancelled.",
    failed: "Failed in step",
    cancel_hint: "Press Ctrl+C to cancel.",
};

const DE: Texts = Texts {
    job: "Job",
    stage: "Schritt",
    phase: "Phase",
    iteration: "Iteration",
    cells: "Zellen",
    cores: "Kerne",
    succeeded: "Fertig.",
    cancelled: "Abgebrochen.",
    failed: "Fehlgeschlagen im Schritt",
    cancel_hint: "Strg+C bricht ab.",
};

fn texts(lang: Language) -> &'static Texts {
    match lang {
        Language::En => &EN,
        Language::De => &DE,
    }
}

pub fn store(dir: Option<PathBuf>) -> JobStore {
    JobStore::new(dir.unwrap_or_else(foamcraft_core::paths::jobs_dir))
}

fn cancel_flag() -> Arc<AtomicBool> {
    let flag = Arc::new(AtomicBool::new(false));
    let handler = flag.clone();
    // Without a handler Ctrl+C still ends the process; the job then keeps running in the
    // container and can be resumed.
    let _ = ctrlc::set_handler(move || handler.store(true, Ordering::SeqCst));
    flag
}

fn print_event(event: &JobEvent, t: &Texts) {
    match event {
        JobEvent::StageStarted(stage) => println!("{}: {}", t.stage, stage.id()),
        JobEvent::Phase { phase, .. } => println!("  {}: {phase}", t.phase),
        JobEvent::Progress {
            iteration,
            residuals,
            ..
        } => {
            let res: Vec<String> = residuals
                .iter()
                .map(|(f, r)| format!("{f} {r:.2e}"))
                .collect();
            println!("  {} {iteration}  {}", t.iteration, res.join("  "));
        }
        _ => {}
    }
}

fn finish(record: &JobRecord, state: JobState, lang: Language) -> Result<(), FoamError> {
    let t = texts(lang);
    if let Some(cells) = record.cells {
        println!("{}: {cells}  {}: {}", t.cells, t.cores, record.cores);
    }
    match state {
        JobState::Succeeded => {
            println!("{}", t.succeeded);
            Ok(())
        }
        JobState::Cancelled => {
            println!("{}", t.cancelled);
            Err(FoamError::new("run.cancelled", &record.id))
        }
        JobState::Failed { stage, error } => {
            println!("{} {}", t.failed, stage.id());
            Err(error)
        }
        other => Err(FoamError::new("run.stage_failed", format!("{other:?}"))),
    }
}

fn run_options(runner: &DockerRunner, record: &JobRecord, cores: Option<u32>) -> RunOptions {
    RunOptions {
        cores: cores.unwrap_or_else(|| core_limit(runner, record)).max(1),
        poll_interval: Duration::from_millis(500),
        stop_after: None,
    }
}

pub fn run(
    iterations: u32,
    cores: Option<u32>,
    jobs_dir: Option<PathBuf>,
    lang: Language,
) -> Result<(), FoamError> {
    let t = texts(lang);
    let runner = DockerRunner::local();
    use foamcraft_runtime::Runner;
    runner.check()?;
    let store = store(jobs_dir);
    let end_time = iterations.to_string();
    let mut record = prepare_job(&runner, &store, REFERENCE_CUBE.name, |dir| {
        REFERENCE_CUBE.write(dir, &[("END_TIME", &end_time)])
    })?;
    println!("{} {}", t.job, record.id);
    println!("{}", t.cancel_hint);
    let options = run_options(&runner, &record, cores);
    let cancel = cancel_flag();
    let state = execute_job(&runner, &store, &mut record, &options, &cancel, &mut |e| {
        print_event(&e, t)
    });
    finish(&record, state, lang)
}

pub fn resume(id: &str, jobs_dir: Option<PathBuf>, lang: Language) -> Result<(), FoamError> {
    let t = texts(lang);
    let runner = DockerRunner::local();
    let store = store(jobs_dir);
    let mut record = store.load(id)?;
    println!("{} {}", t.job, record.id);
    let options = run_options(&runner, &record, Some(record.cores.max(1)));
    let cancel = cancel_flag();
    let state = execute_job(&runner, &store, &mut record, &options, &cancel, &mut |e| {
        print_event(&e, t)
    });
    finish(&record, state, lang)
}

pub fn list(jobs_dir: Option<PathBuf>) -> Result<(), FoamError> {
    for record in store(jobs_dir).list()? {
        let state = match &record.state {
            JobState::Pending => "pending".to_string(),
            JobState::Running(stage) => format!("running ({})", stage.id()),
            JobState::Succeeded => "succeeded".into(),
            JobState::Failed { stage, error } => format!("failed ({}, {})", stage.id(), error.code),
            JobState::Cancelled => "cancelled".into(),
        };
        println!("{}  {}  {state}", record.id, record.case);
    }
    Ok(())
}

pub fn log(id: &str, jobs_dir: Option<PathBuf>) -> Result<(), FoamError> {
    print!("{}", store(jobs_dir).read_log(id)?);
    Ok(())
}

pub fn clean(id: &str, jobs_dir: Option<PathBuf>) -> Result<(), FoamError> {
    let record = store(jobs_dir).load(id)?;
    foamcraft_runtime::jobs::remove_job_resources(&DockerRunner::local(), &record)
}
