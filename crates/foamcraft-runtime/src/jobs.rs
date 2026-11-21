// SPDX-License-Identifier: GPL-3.0-or-later
//! Runs jobs in the container runtime: one volume and one container per job.

use std::path::Path;
use std::sync::atomic::AtomicBool;

use foamcraft_core::error::FoamError;
use foamcraft_core::job::{JobEvent, JobRecord, JobState, JobStore, RunOptions, run_job};

use crate::{Container, DockerStageExecutor, Mount, Runner, StartOptions, WORK_DIR};

/// Case folder inside the container.
pub fn case_dir() -> String {
    format!("{WORK_DIR}/case")
}

/// Physical cores of this computer (the default upper limit for parallel runs).
pub fn host_physical_cores() -> u32 {
    num_cpus::get_physical().max(1) as u32
}

/// Creates a job: writes the case with `write_case` into the job folder, creates a volume and a
/// container and copies the case in. The record names the container and volume.
pub fn prepare_job<R: Runner + ?Sized>(
    runner: &R,
    store: &JobStore,
    case_name: &str,
    write_case: impl FnOnce(&Path) -> Result<(), FoamError>,
) -> Result<JobRecord, FoamError> {
    let mut record = store.create(case_name)?;
    let input = store.dir(&record.id).join("case");
    write_case(&input)?;
    let volume = format!("foamcraft-{}", record.id);
    runner.create_volume(&volume)?;
    record.volume = Some(volume.clone());
    store.save(&record)?;
    let container = runner.start(&StartOptions {
        mount: Some(Mount::Volume(volume)),
        ..Default::default()
    })?;
    record.container = Some(container.id.clone());
    store.save(&record)?;
    runner.copy_into(&container, &input, &case_dir())?;
    Ok(record)
}

/// Upper core limit for a job: physical host cores and what the container may use.
pub fn core_limit<R: Runner + ?Sized>(runner: &R, record: &JobRecord) -> u32 {
    let container_cpus = record
        .container
        .as_ref()
        .and_then(|id| runner.agent_status(&Container { id: id.clone() }).ok())
        .map_or(0, |s| s.cpus);
    let physical = host_physical_cores();
    if container_cpus == 0 {
        physical
    } else {
        physical.min(container_cpus)
    }
}

/// Runs or resumes a prepared job. When it ends, the container is removed; the volume with the
/// results stays until [`remove_job_resources`].
pub fn execute_job<R: Runner + ?Sized>(
    runner: &R,
    store: &JobStore,
    record: &mut JobRecord,
    options: &RunOptions,
    cancel: &AtomicBool,
    on_event: &mut dyn FnMut(JobEvent),
) -> JobState {
    let Some(id) = record.container.clone() else {
        record.state = JobState::Failed {
            stage: foamcraft_core::job::Stage::Background,
            error: FoamError::new("run.lost", "job has no container"),
        };
        let _ = store.save(record);
        return record.state.clone();
    };
    let container = Container { id };
    let mut exec = DockerStageExecutor::new(runner, container.clone(), &case_dir());
    let state = run_job(record, &mut exec, store, options, cancel, on_event);
    if state.is_finished() {
        let _ = runner.stop(&container);
    }
    state
}

/// Removes the container and the volume of a job (its record and log stay).
pub fn remove_job_resources<R: Runner + ?Sized>(
    runner: &R,
    record: &JobRecord,
) -> Result<(), FoamError> {
    if let Some(id) = &record.container {
        let _ = runner.stop(&Container { id: id.clone() });
    }
    if let Some(volume) = &record.volume {
        runner.remove_volume(volume)?;
    }
    Ok(())
}
