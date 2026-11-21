// SPDX-License-Identifier: GPL-3.0-or-later
//! Human-readable runtime report in English or German.

use foamcraft_core::error::{FoamError, Language};
use foamcraft_core::runtime::AgentStatus;

use crate::{Backend, EnvironmentReport};

struct Labels {
    engine: &'static str,
    backend: [&'static str; 3],
    cores: &'static str,
    memory: &'static str,
    unknown: &'static str,
    gpu_yes: &'static str,
    gpu_no: &'static str,
    passthrough_no: &'static str,
    agent: &'static str,
    warning: &'static str,
    fix: &'static str,
}

const EN: Labels = Labels {
    engine: "Docker",
    backend: ["WSL 2", "virtual machine", "native"],
    cores: "Cores",
    memory: "Memory",
    unknown: "unknown",
    gpu_yes: "GPU: available",
    gpu_no: "GPU: not available",
    passthrough_no: "the container engine passes no GPU through",
    agent: "Agent",
    warning: "Warning",
    fix: "Fix",
};

const DE: Labels = Labels {
    engine: "Docker",
    backend: ["WSL 2", "virtuelle Maschine", "nativ"],
    cores: "Kerne",
    memory: "Arbeitsspeicher",
    unknown: "unbekannt",
    gpu_yes: "GPU: verfügbar",
    gpu_no: "GPU: nicht verfügbar",
    passthrough_no: "die Container-Laufzeit reicht keine GPU durch",
    agent: "Agent",
    warning: "Warnung",
    fix: "Abhilfe",
};

fn labels(lang: Language) -> &'static Labels {
    match lang {
        Language::En => &EN,
        Language::De => &DE,
    }
}

pub fn gib(bytes: u64) -> String {
    format!("{:.1} GiB", bytes as f64 / (1024.0 * 1024.0 * 1024.0))
}

/// One error as "Label [code]: text" plus the fix on the next line.
pub fn render_error(err: &FoamError, lang: Language, label: &str) -> String {
    let m = err.localized(lang);
    let fix = labels(lang).fix;
    let mut out = format!("{label} [{}]: {}", err.code, m.text);
    if !err.detail.is_empty() {
        out.push_str(&format!(" ({})", err.detail));
    }
    if !m.fix.is_empty() {
        out.push_str(&format!("\n  {fix}: {}", m.fix));
    }
    out
}

/// Report of `foamcraft runtime check`.
pub fn render_check(
    env: &EnvironmentReport,
    status: &AgentStatus,
    gpu_passthrough: bool,
    lang: Language,
) -> String {
    let l = labels(lang);
    let backend = match env.backend {
        Backend::Wsl2 => l.backend[0],
        Backend::VirtualMachine => l.backend[1],
        Backend::Native => l.backend[2],
    };
    let mut lines = vec![
        format!("{} {} ({backend})", l.engine, env.server_version),
        format!(
            "OpenFOAM {} (build {})",
            status.openfoam.version(),
            if status.openfoam.build.is_empty() {
                l.unknown
            } else {
                &status.openfoam.build
            }
        ),
        format!("{} {}", l.agent, status.agent_version),
        format!("{}: {}", l.cores, status.cpus),
        format!(
            "{}: {}",
            l.memory,
            status.memory_bytes.map_or(l.unknown.to_string(), gib)
        ),
    ];
    lines.push(if status.gpu.available {
        format!("{} ({})", l.gpu_yes, status.gpu.detail)
    } else if !gpu_passthrough {
        format!("{} ({})", l.gpu_no, l.passthrough_no)
    } else {
        format!("{} ({})", l.gpu_no, status.gpu.detail)
    });
    for w in &env.warnings {
        lines.push(render_error(w, lang, l.warning));
    }
    lines.join("\n")
}
