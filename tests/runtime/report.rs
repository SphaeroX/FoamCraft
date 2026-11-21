// SPDX-License-Identifier: GPL-3.0-or-later
//! Text of `foamcraft runtime check`.

use foamcraft_core::error::{FoamError, Language};
use foamcraft_core::runtime::{AgentStatus, GpuInfo, OpenFoamInfo};
use foamcraft_runtime::report::render_check;
use foamcraft_runtime::{Backend, EnvironmentReport};

const GIB: u64 = 1024 * 1024 * 1024;

fn env(warnings: Vec<FoamError>) -> EnvironmentReport {
    EnvironmentReport {
        server_version: "29.9.2".into(),
        backend: Backend::Wsl2,
        cpus: 32,
        memory_bytes: 16 * GIB,
        warnings,
    }
}

fn status(gpu: bool) -> AgentStatus {
    AgentStatus {
        agent_version: "0.1.0".into(),
        openfoam: OpenFoamInfo {
            api: "2606".into(),
            patch: "0".into(),
            build: "_abc".into(),
        },
        cpus: 8,
        memory_bytes: Some(8 * GIB),
        gpu: GpuInfo {
            available: gpu,
            detail: "/dev/dxg".into(),
        },
    }
}

#[test]
fn english_report_names_version_cores_memory_and_gpu() {
    let text = render_check(&env(vec![]), &status(true), true, Language::En);
    for part in [
        "OpenFOAM v2606",
        "Cores: 8",
        "Memory: 8.0 GiB",
        "GPU: available",
        "WSL 2",
    ] {
        assert!(text.contains(part), "{part} missing in:\n{text}");
    }
}

#[test]
fn german_report_and_missing_gpu() {
    let text = render_check(&env(vec![]), &status(false), false, Language::De);
    for part in [
        "OpenFOAM v2606",
        "Kerne: 8",
        "Arbeitsspeicher: 8.0 GiB",
        "GPU: nicht verfügbar",
    ] {
        assert!(text.contains(part), "{part} missing in:\n{text}");
    }
}

#[test]
fn warnings_show_code_text_and_fix() {
    let w = FoamError::new("runtime.insufficient_memory", "2 GiB");
    let text = render_check(&env(vec![w]), &status(false), false, Language::En);
    assert!(text.contains("runtime.insufficient_memory"));
    assert!(text.contains("less memory than recommended"));
    assert!(text.contains("Settings > Resources"));
}
