// SPDX-License-Identifier: GPL-3.0-or-later
//! Runtime layer: the [`Runner`] abstraction that starts and controls OpenFOAM, and its Docker
//! implementation.
//!
//! Docker is driven through its command line (`docker ...`) instead of a client library: no extra
//! dependency, the same behaviour as a user typing the command, and stable error texts that map to
//! FoamCraft error codes.

mod docker;
mod engine;
pub mod fake;
pub mod jobs;
pub mod report;
mod stage;

pub use docker::DockerRunner;
pub use engine::{CmdOutput, Engine, LogLine, ProcessEngine};
pub use stage::DockerStageExecutor;

use foamcraft_core::error::FoamError;
use foamcraft_core::runtime::AgentStatus;

/// OpenFOAM release in the runtime image (`OPENFOAM_API` in `runtime/Dockerfile`).
pub const OPENFOAM_RELEASE: &str = "v2606";
/// Name and tag of the runtime image built from `runtime/Dockerfile`.
pub const IMAGE_TAG: &str = concat!("foamcraft-runtime:", env!("CARGO_PKG_VERSION"));
/// Label on every container FoamCraft starts, so `stop_all` finds them.
pub const MANAGED_LABEL: &str = "org.foamcraft.managed=true";
/// Working directory inside the container.
pub const WORK_DIR: &str = "/work";
/// Below this the runtime works but large meshes will fail.
pub const RECOMMENDED_MEMORY_BYTES: u64 = 4 * 1024 * 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Backend {
    /// Docker Desktop on the WSL 2 engine (Windows).
    Wsl2,
    /// Any other virtual machine (Docker Desktop Hyper-V, macOS).
    VirtualMachine,
    /// Docker Engine running directly on Linux.
    Native,
}

/// Result of an environment check. Warnings do not stop simulations.
#[derive(Debug, Clone)]
pub struct EnvironmentReport {
    pub server_version: String,
    pub backend: Backend,
    pub cpus: u32,
    pub memory_bytes: u64,
    pub warnings: Vec<FoamError>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Mount {
    /// A named container volume (fast, the agent owns the files).
    Volume(String),
    /// A host directory (slow across the Windows/Linux boundary).
    Bind(std::path::PathBuf),
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct StartOptions {
    pub cpus: Option<f64>,
    pub memory_bytes: Option<u64>,
    pub mount: Option<Mount>,
    pub gpu: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Container {
    pub id: String,
}

/// Starts and controls the OpenFOAM runtime.
pub trait Runner {
    /// Checks that the container engine is installed, running and suitable.
    fn check(&self) -> Result<EnvironmentReport, FoamError>;
    fn image_present(&self) -> Result<bool, FoamError>;
    /// Builds the runtime image from the repository root `context`.
    fn build_image(
        &self,
        context: &std::path::Path,
        on_line: &mut dyn FnMut(LogLine),
    ) -> Result<(), FoamError>;
    fn start(&self, options: &StartOptions) -> Result<Container, FoamError>;
    /// Runs `command` in the container. A non-zero exit code is returned, not an error.
    fn exec(
        &self,
        container: &Container,
        command: &[&str],
        on_line: &mut dyn FnMut(LogLine),
    ) -> Result<CmdOutput, FoamError>;
    /// Starts `command` in the container and returns at once (`docker exec -d`).
    fn exec_detached(
        &self,
        container: &Container,
        command: &[&str],
    ) -> Result<CmdOutput, FoamError>;
    fn stop(&self, container: &Container) -> Result<(), FoamError>;
    /// Stops every container FoamCraft started; returns how many.
    fn stop_all(&self) -> Result<usize, FoamError>;
    /// True when containers can see the GPU.
    fn gpu_passthrough(&self) -> bool;
    /// Creates a named volume for case files (labelled as managed by FoamCraft).
    fn create_volume(&self, name: &str) -> Result<(), FoamError>;
    fn remove_volume(&self, name: &str) -> Result<(), FoamError>;
    /// Copies a host file or folder into the container (into volumes mounted there).
    fn copy_into(
        &self,
        container: &Container,
        host: &std::path::Path,
        target: &str,
    ) -> Result<(), FoamError>;
    /// Copies a file or folder from the container to the host.
    fn copy_out(
        &self,
        container: &Container,
        source: &str,
        host: &std::path::Path,
    ) -> Result<(), FoamError>;

    fn agent_status(&self, container: &Container) -> Result<AgentStatus, FoamError> {
        let out = self.exec(container, &["foamcraft-agent", "status"], &mut |_| {})?;
        if out.status != 0 {
            return Err(FoamError::new("runtime.agent_failed", out.stderr.trim()));
        }
        serde_json::from_str(out.stdout.trim())
            .map_err(|e| FoamError::new("runtime.agent_failed", e.to_string()))
    }

    /// Runs a shell command with the OpenFOAM environment loaded, in the working directory.
    fn exec_openfoam(
        &self,
        container: &Container,
        script: &str,
        on_line: &mut dyn FnMut(LogLine),
    ) -> Result<CmdOutput, FoamError> {
        let script = format!("cd {WORK_DIR} && {script}");
        self.exec(container, &["bash", "-lc", &script], on_line)
    }
}
