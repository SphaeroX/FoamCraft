// SPDX-License-Identifier: GPL-3.0-or-later
//! Messages between the host and the agent inside the runtime.
//!
//! The host runs `foamcraft-agent <command>` through the runner and reads one JSON document from
//! standard output.

use serde::{Deserialize, Serialize};

/// OpenFOAM release information from `$WM_PROJECT_DIR/META-INFO`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct OpenFoamInfo {
    pub api: String,
    pub patch: String,
    pub build: String,
}

impl OpenFoamInfo {
    /// Release name as OpenCFD writes it, for example `v2606`.
    pub fn version(&self) -> String {
        if self.api.is_empty() {
            String::new()
        } else {
            format!("v{}", self.api)
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct GpuInfo {
    pub available: bool,
    /// Device or reason, in English (technical detail).
    pub detail: String,
}

/// Answer of `foamcraft-agent status`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentStatus {
    pub agent_version: String,
    pub openfoam: OpenFoamInfo,
    /// Cores the container may use.
    pub cpus: u32,
    /// Memory the container may use, in bytes.
    pub memory_bytes: Option<u64>,
    pub gpu: GpuInfo,
}

/// Answer of `foamcraft-agent tail <log> <exit-file> <offset>`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TailResult {
    /// New log text; while the stage runs, only complete lines.
    pub text: String,
    /// Offset to pass to the next call.
    pub next_offset: u64,
    /// Exit code once the stage has ended.
    pub exit: Option<i32>,
}
