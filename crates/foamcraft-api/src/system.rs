// SPDX-License-Identifier: GPL-3.0-or-later
//! System check of the setup assistant: Docker installed and running, WSL 2 backend, memory of
//! the container runtime, runtime image, free disk space and GPU, each with text, suggestion and
//! step-by-step help in English and German; the hardware profile with default cores and cell
//! budget; a diagnostic report to copy; the "setup done" state; providing the runtime image.

use std::path::Path;

use foamcraft_core::command::{Options, Registry, Session};
use foamcraft_core::error::{FoamError, Language, message};
use foamcraft_runtime::{Backend, Runner};
use foamcraft_sim::{BYTES_PER_MILLION_CELLS, USABLE_MEMORY_SHARE};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::SharedRunner;

const GIB: u64 = 1024 * 1024 * 1024;
/// Below this much free space the assistant warns (image, meshes and results need room).
pub const MIN_FREE_DISK_BYTES: u64 = 20 * GIB;

/// Facts the check cannot get from the container engine.
#[derive(Debug, Clone, Default)]
pub struct Probe {
    /// Name of an NVIDIA GPU of this computer, if any.
    pub gpu: Option<String>,
    /// Free space where projects and the runtime live.
    pub free_disk_bytes: Option<u64>,
}

impl Probe {
    /// The facts of this computer.
    pub fn local(data_dir: &Path) -> Self {
        let free_disk_bytes = sysinfo::Disks::new_with_refreshed_list()
            .ok()
            .and_then(|disks| {
                disks
                    .iter()
                    .filter(|d| data_dir.starts_with(d.mount_point()))
                    .max_by_key(|d| d.mount_point().as_os_str().len())
                    .map(|d| d.available_space())
            });
        let gpu = std::process::Command::new("nvidia-smi")
            .args(["--query-gpu=name", "--format=csv,noheader"])
            .output()
            .ok()
            .filter(|o| o.status.success())
            .map(|o| {
                String::from_utf8_lossy(&o.stdout)
                    .lines()
                    .next()
                    .unwrap_or("")
                    .trim()
                    .to_string()
            })
            .filter(|n| !n.is_empty());
        Self {
            gpu,
            free_disk_bytes,
        }
    }
}

fn help() -> &'static Value {
    static HELP: std::sync::OnceLock<Value> = std::sync::OnceLock::new();
    HELP.get_or_init(|| {
        serde_json::from_str(include_str!("../data/system_help.json")).unwrap_or(Value::Null)
    })
}

/// One step: `ok`, `warning` (works, but worse), `action` (the user must do something),
/// `info`, or `skipped` (an earlier step must be done first).
fn step(id: &str, state: &str, code: &str, detail: impl Into<String>) -> Value {
    let text = |lang: Language, key: &str| {
        message(code, lang)
            .map(|m| json!({"text": m.text, "fix": m.fix, "help": help()[code][key].clone()}))
    };
    let (en, de) = (
        text(Language::En, "en").unwrap_or(json!({"text": code, "fix": "", "help": []})),
        text(Language::De, "de").unwrap_or(json!({"text": code, "fix": "", "help": []})),
    );
    json!({"id": id, "state": state, "code": code, "detail": detail.into(), "en": en, "de": de})
}

/// The steps of the check, in order.
pub fn steps(runner: &dyn Runner, probe: &Probe) -> Vec<Value> {
    let mut out = Vec::new();
    let skipped = |id: &str| step(id, "skipped", "system.skipped", "");
    match runner.check() {
        Err(e) if e.code == "runtime.docker_missing" => {
            out.push(step("docker", "action", &e.code, e.detail));
            for id in ["service", "backend", "memory", "image"] {
                out.push(skipped(id));
            }
        }
        Err(e) => {
            out.push(step("docker", "ok", "system.docker_installed", ""));
            out.push(step("service", "action", &e.code, e.detail));
            for id in ["backend", "memory", "image"] {
                out.push(skipped(id));
            }
        }
        Ok(report) => {
            out.push(step("docker", "ok", "system.docker_installed", ""));
            out.push(step(
                "service",
                "ok",
                "system.service_running",
                report.server_version.clone(),
            ));
            let warn = |code: &str| report.warnings.iter().find(|w| w.code == code);
            out.push(match warn("runtime.no_wsl2_backend") {
                Some(w) => step("backend", "warning", &w.code, w.detail.clone()),
                None => step(
                    "backend",
                    "ok",
                    "system.backend_ok",
                    match report.backend {
                        Backend::Wsl2 => "WSL 2",
                        Backend::Native => "native",
                        Backend::VirtualMachine => "virtual machine",
                    },
                ),
            });
            let memory = format!(
                "{:.1} GiB, {} cores",
                report.memory_bytes as f64 / GIB as f64,
                report.cpus
            );
            out.push(match warn("runtime.insufficient_memory") {
                Some(w) => step("memory", "warning", &w.code, memory),
                None => step("memory", "ok", "system.memory_ok", memory),
            });
            out.push(match runner.image_present() {
                Ok(true) => step(
                    "image",
                    "ok",
                    "system.image_present",
                    foamcraft_runtime::IMAGE_TAG,
                ),
                Ok(false) => step(
                    "image",
                    "action",
                    "runtime.image_missing",
                    foamcraft_runtime::IMAGE_TAG,
                ),
                Err(e) => step("image", "action", &e.code, e.detail),
            });
        }
    }
    out.push(match probe.free_disk_bytes {
        Some(free) if free < MIN_FREE_DISK_BYTES => step(
            "disk",
            "warning",
            "system.disk_low",
            format!("{:.0} GiB free", free as f64 / GIB as f64),
        ),
        Some(free) => step(
            "disk",
            "ok",
            "system.disk_ok",
            format!("{:.0} GiB free", free as f64 / GIB as f64),
        ),
        None => step("disk", "info", "system.disk_unknown", ""),
    });
    out.push(match &probe.gpu {
        Some(name) => step("gpu", "ok", "system.gpu_found", name.clone()),
        None => step("gpu", "info", "system.gpu_none", ""),
    });
    out
}

/// Defaults derived from this computer and the container runtime.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct HardwareProfile {
    pub physical_cores: u32,
    pub host_memory_bytes: u64,
    pub container_cpus: u32,
    pub container_memory_bytes: u64,
    /// Parallel processes of a run.
    pub default_cores: u32,
    /// Largest mesh the memory allows (60 % of it, 1.5 GiB per million cells).
    pub max_cells: u64,
    /// Highest quality level whose cell count fits: `draft`, `standard` or `fine`.
    pub quality: String,
}

pub fn profile(
    physical_cores: u32,
    host_memory: u64,
    container_cpus: u32,
    container_memory: u64,
) -> HardwareProfile {
    let cores = physical_cores.max(1).min(container_cpus.max(1));
    let memory = if container_memory > 0 {
        container_memory
    } else {
        host_memory
    };
    let max_cells = (memory as f64 * USABLE_MEMORY_SHARE / BYTES_PER_MILLION_CELLS * 1e6).round()
        as u64
        / 100_000
        * 100_000;
    let quality = if max_cells >= 8_000_000 {
        "fine"
    } else if max_cells >= 2_000_000 {
        "standard"
    } else {
        "draft"
    };
    HardwareProfile {
        physical_cores,
        host_memory_bytes: host_memory,
        container_cpus,
        container_memory_bytes: container_memory,
        default_cores: cores,
        max_cells,
        quality: quality.into(),
    }
}

fn local_profile(runner: &dyn Runner) -> HardwareProfile {
    let host_memory = sysinfo::System::new()
        .map(|mut sys| {
            sys.refresh_memory();
            sys.total_memory()
        })
        .unwrap_or(0);
    let physical = sysinfo::System::physical_core_count().unwrap_or(1) as u32;
    let (cpus, memory) = runner
        .check()
        .map_or((physical, 0), |r| (r.cpus, r.memory_bytes));
    profile(physical, host_memory, cpus, memory)
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct SystemCheck {
    /// Steps with id, state, code, detail and texts (`en`, `de`: text, fix, help).
    pub steps: Vec<Value>,
    pub profile: HardwareProfile,
    /// No step needs action.
    pub ready: bool,
}

fn check(runner: &dyn Runner, data_dir: &Path) -> SystemCheck {
    let steps = steps(runner, &Probe::local(data_dir));
    let ready = steps.iter().all(|s| s["state"] != "action");
    SystemCheck {
        steps,
        profile: local_profile(runner),
        ready,
    }
}

/// Plain text for a bug report; the home folder is replaced by `~`.
fn diagnostics(runner: &dyn Runner, data_dir: &Path) -> String {
    let c = check(runner, data_dir);
    let mut t = format!(
        "FoamCraft {}\nOS: {}\nRuntime image: {}\n\nChecks:\n",
        foamcraft_core::VERSION,
        sysinfo::System::long_os_version().unwrap_or_default(),
        foamcraft_runtime::IMAGE_TAG
    );
    for s in &c.steps {
        t.push_str(&format!(
            "- {:<8} {:<8} {} {}\n",
            s["id"].as_str().unwrap_or(""),
            s["state"].as_str().unwrap_or(""),
            s["code"].as_str().unwrap_or(""),
            s["detail"].as_str().unwrap_or("")
        ));
    }
    let p = &c.profile;
    t.push_str(&format!(
        "\nHardware: {} physical cores, {:.1} GiB memory; container: {} cores, {:.1} GiB\nDefaults: {} cores, up to {} cells, quality {}\nData folder: {}\n",
        p.physical_cores,
        p.host_memory_bytes as f64 / GIB as f64,
        p.container_cpus,
        p.container_memory_bytes as f64 / GIB as f64,
        p.default_cores,
        p.max_cells,
        p.quality,
        data_dir.display()
    ));
    for var in ["USERPROFILE", "HOME"] {
        if let Some(home) = std::env::var_os(var).map(|h| h.to_string_lossy().to_string())
            && home.len() > 3
        {
            t = t.replace(&home, "~").replace(&home.replace('\\', "/"), "~");
        }
    }
    t
}

const SETUP_FILE: &str = "setup.json";

#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
pub struct SetupState {
    /// The assistant has been completed once.
    pub done: bool,
    #[serde(default)]
    pub profile: Option<HardwareProfile>,
}

fn load_setup(data_dir: &Path) -> SetupState {
    std::fs::read_to_string(data_dir.join(SETUP_FILE))
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default()
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct NoParams {}

/// Building the runtime image in the background (until releases provide it ready-made).
/// Provides the runtime image: built from `runtime/Dockerfile` when `context` is the repository
/// (a development build), otherwise the image published for this version is pulled.
pub fn fetch_image(
    runner: &dyn Runner,
    context: &Path,
    on_line: &mut dyn FnMut(foamcraft_runtime::LogLine),
) -> Result<(), FoamError> {
    if context.join("runtime/Dockerfile").is_file() {
        runner.build_image(context, on_line)
    } else {
        runner.pull_image(on_line)
    }
}

fn provide_image(runner: SharedRunner, s: &mut Session) -> Result<Value, FoamError> {
    static BUSY: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
    if BUSY.swap(true, std::sync::atomic::Ordering::SeqCst) {
        return Ok(json!({"started": false, "running": true}));
    }
    // A development build finds the repository it was built from; an installed app does not.
    let context = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let events = s.events().clone();
    std::thread::spawn(move || {
        let mut lines = 0u32;
        let result = fetch_image(runner.as_ref(), &context, &mut |line| {
            lines += 1;
            let text = match line {
                foamcraft_runtime::LogLine::Stdout(t) | foamcraft_runtime::LogLine::Stderr(t) => t,
            };
            events.emit(
                "system.image_progress",
                json!({"line": text, "lines": lines}),
            );
        });
        match result {
            Ok(()) => events.emit("system.image_ready", Value::Null),
            Err(e) => events.emit(
                "system.image_failed",
                json!({"code": e.code, "detail": e.detail}),
            ),
        }
        BUSY.store(false, std::sync::atomic::Ordering::SeqCst);
    });
    Ok(json!({"started": true, "running": true}))
}

pub fn register(r: &mut Registry, runner: SharedRunner) {
    let rc = runner.clone();
    r.add(
        "system.check",
        "Check this computer for simulations: Docker installed and running, WSL 2 backend, memory, \
         runtime image, disk space, GPU; with texts, suggestions, step-by-step help and the \
         hardware profile (default cores, cell budget, quality).",
        Options {
            undoable: false,
            errors: &[],
            example: json!({}),
        },
        move |s, _: NoParams| Ok(check(rc.as_ref(), s.data_dir())),
    );
    let rd = runner.clone();
    r.add(
        "system.diagnostics",
        "A plain-text diagnostic report of the system check to copy into a bug report.",
        Options {
            undoable: false,
            errors: &[],
            example: json!({}),
        },
        move |s, _: NoParams| Ok(json!({"text": diagnostics(rd.as_ref(), s.data_dir())})),
    );
    r.add(
        "system.setup_state",
        "Whether the setup assistant has been completed, and the stored hardware profile.",
        Options {
            undoable: false,
            errors: &[],
            example: json!({}),
        },
        |s, _: NoParams| Ok(load_setup(s.data_dir())),
    );
    let rf = runner.clone();
    r.add(
        "system.finish_setup",
        "Mark the setup assistant as completed and store the hardware profile.",
        Options {
            undoable: false,
            errors: &["project.write_failed"],
            example: json!({}),
        },
        move |s, _: NoParams| {
            let state = SetupState {
                done: true,
                profile: Some(local_profile(rf.as_ref())),
            };
            std::fs::create_dir_all(s.data_dir())
                .and_then(|_| {
                    std::fs::write(
                        s.data_dir().join(SETUP_FILE),
                        serde_json::to_string_pretty(&state).unwrap_or_default(),
                    )
                })
                .map_err(|e| FoamError::new("project.write_failed", e.to_string()))?;
            Ok(state)
        },
    );
    r.add(
        "system.provide_image",
        "Provide the OpenFOAM runtime image (built locally from the repository); progress as \
         system.image_progress events, then system.image_ready or system.image_failed.",
        Options {
            undoable: false,
            errors: &["system.image_unavailable"],
            example: json!({}),
        },
        move |s, _: NoParams| provide_image(runner.clone(), s),
    );
}
