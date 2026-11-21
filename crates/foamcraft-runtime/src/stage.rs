// SPDX-License-Identifier: GPL-3.0-or-later
//! Runs job stages in a runtime container through the agent.

use foamcraft_core::error::FoamError;
use foamcraft_core::job::{Stage, StageExecutor, StagePoll};
use foamcraft_core::runtime::TailResult;

use crate::{Container, Runner};

pub struct DockerStageExecutor<'r, R: Runner + ?Sized> {
    runner: &'r R,
    container: Container,
    case_dir: String,
}

impl<'r, R: Runner + ?Sized> DockerStageExecutor<'r, R> {
    /// `case_dir` is the case folder inside the container, for example `/work/case`.
    pub fn new(runner: &'r R, container: Container, case_dir: &str) -> Self {
        Self {
            runner,
            container,
            case_dir: case_dir.into(),
        }
    }

    pub fn container(&self) -> &Container {
        &self.container
    }
}

fn lost_or(stderr: &str, code: &str) -> FoamError {
    if stderr.contains("No such container") || stderr.contains("is not running") {
        FoamError::new("run.lost", stderr.trim())
    } else {
        FoamError::new(code, stderr.trim())
    }
}

impl<R: Runner + ?Sized> StageExecutor for DockerStageExecutor<'_, R> {
    fn start(&mut self, stage: Stage, script: &str) -> Result<(), FoamError> {
        let out = self.runner.exec_detached(
            &self.container,
            &[
                "foamcraft-agent",
                "run-stage",
                &self.case_dir,
                stage.id(),
                script,
            ],
        )?;
        if out.status == 0 {
            Ok(())
        } else {
            Err(lost_or(&out.stderr, "runtime.command_failed"))
        }
    }

    fn poll(&mut self, stage: Stage, offset: u64) -> Result<StagePoll, FoamError> {
        let offset = offset.to_string();
        let out = self.runner.exec(
            &self.container,
            &[
                "foamcraft-agent",
                "tail",
                &self.case_dir,
                stage.id(),
                &offset,
            ],
            &mut |_| {},
        )?;
        if out.status != 0 {
            return Err(lost_or(&out.stderr, "runtime.agent_failed"));
        }
        let tail: TailResult = serde_json::from_str(out.stdout.trim())
            .map_err(|e| FoamError::new("runtime.agent_failed", e.to_string()))?;
        Ok(StagePoll {
            text: tail.text,
            exit: tail.exit,
        })
    }

    fn cancel(&mut self) -> Result<(), FoamError> {
        // Removing the container ends every process of the job at once; case files stay on the
        // volume.
        self.runner.stop(&self.container)
    }
}
