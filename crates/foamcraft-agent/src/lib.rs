// SPDX-License-Identifier: GPL-3.0-or-later
//! FoamCraft agent: runs inside the Linux runtime next to OpenFOAM and is the only component
//! that touches case files there. This module holds the platform-independent parts.

use std::path::{Path, PathBuf};

use foamcraft_core::runtime::{AgentStatus, GpuInfo, OpenFoamInfo, TailResult};

/// `MemTotal` from `/proc/meminfo`, in bytes.
pub fn parse_meminfo(text: &str) -> Option<u64> {
    let line = text.lines().find(|l| l.starts_with("MemTotal:"))?;
    let kib: u64 = line.split_whitespace().nth(1)?.parse().ok()?;
    Some(kib * 1024)
}

/// cgroup v2 `memory.max`; `max` means no limit.
pub fn parse_cgroup_memory_max(text: &str) -> Option<u64> {
    text.trim().parse().ok()
}

/// The smaller of physical memory and the cgroup limit.
pub fn effective_memory(physical: Option<u64>, limit: Option<u64>) -> Option<u64> {
    match (physical, limit) {
        (Some(p), Some(l)) => Some(p.min(l)),
        (p, l) => p.or(l),
    }
}

fn key_value<'a>(text: &'a str, key: &str) -> &'a str {
    text.lines()
        .filter_map(|l| l.split_once('='))
        .find(|(k, _)| k.trim() == key)
        .map_or("", |(_, v)| v.trim())
}

/// Contents of `META-INFO/api-info` and `META-INFO/build-info`.
pub fn parse_api_info(api_info: &str, build_info: &str) -> OpenFoamInfo {
    OpenFoamInfo {
        api: key_value(api_info, "api").into(),
        patch: key_value(api_info, "patch").into(),
        build: key_value(build_info, "build").into(),
    }
}

/// OpenFOAM installation: `$WM_PROJECT_DIR` if set, otherwise the first `openfoam*` folder with
/// `META-INFO` below `root` (the agent does not run in a login shell).
pub fn openfoam_project_dir(env: Option<&str>, root: &Path) -> Option<PathBuf> {
    if let Some(dir) = env.filter(|d| !d.is_empty()) {
        return Some(PathBuf::from(dir));
    }
    let mut dirs: Vec<PathBuf> = std::fs::read_dir(root)
        .ok()?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| {
            p.file_name()
                .is_some_and(|n| n.to_string_lossy().starts_with("openfoam"))
                && p.join("META-INFO").is_dir()
        })
        .collect();
    dirs.sort();
    dirs.pop()
}

fn read(path: impl AsRef<Path>) -> String {
    std::fs::read_to_string(path).unwrap_or_default()
}

/// GPU visible in the container: NVIDIA device nodes or the WSL GPU bridge.
fn detect_gpu() -> GpuInfo {
    for dev in ["/dev/nvidia0", "/dev/dxg"] {
        if Path::new(dev).exists() {
            return GpuInfo {
                available: true,
                detail: dev.into(),
            };
        }
    }
    GpuInfo {
        available: false,
        detail: "no GPU device in the container".into(),
    }
}

/// Collects the status of the runtime this agent runs in.
pub fn status() -> AgentStatus {
    let env = std::env::var("WM_PROJECT_DIR").ok();
    let project =
        openfoam_project_dir(env.as_deref(), Path::new("/usr/lib/openfoam")).unwrap_or_default();
    let meta = project.join("META-INFO");
    let openfoam = parse_api_info(&read(meta.join("api-info")), &read(meta.join("build-info")));
    let memory = effective_memory(
        parse_meminfo(&read("/proc/meminfo")),
        parse_cgroup_memory_max(&read("/sys/fs/cgroup/memory.max")),
    );
    AgentStatus {
        agent_version: foamcraft_core::VERSION.into(),
        openfoam,
        cpus: std::thread::available_parallelism().map_or(1, |n| n.get() as u32),
        memory_bytes: memory,
        gpu: detect_gpu(),
    }
}

/// Reads a stage log from byte `offset`. The exit file is read first, so once it exists the log
/// is complete and returned in full; before that only complete lines are returned.
pub fn tail_stage(log: &Path, exit_file: &Path, offset: u64) -> TailResult {
    let exit = std::fs::read_to_string(exit_file)
        .ok()
        .and_then(|s| s.trim().parse().ok());
    let bytes = std::fs::read(log).unwrap_or_default();
    let start = (offset as usize).min(bytes.len());
    let mut chunk = &bytes[start..];
    if exit.is_none() {
        let end = chunk.iter().rposition(|b| *b == b'\n').map_or(0, |i| i + 1);
        chunk = &chunk[..end];
    }
    TailResult {
        text: String::from_utf8_lossy(chunk).into_owned(),
        next_offset: (start + chunk.len()) as u64,
        exit,
    }
}

/// Log file of a stage inside the case folder.
pub fn stage_log(case: &Path, stage: &str) -> PathBuf {
    case.join(format!("log.{stage}"))
}

/// File that receives the exit code of a stage when it ends.
pub fn stage_exit(case: &Path, stage: &str) -> PathBuf {
    case.join(format!(".exit.{stage}"))
}

/// Runs `script` in `case` with the OpenFOAM environment (login shell), writes its output to the
/// stage log and its exit code to the exit file. Blocks until the script ends; the host starts this
/// detached (`docker exec -d`), so the stage survives the host process.
pub fn run_stage(case: &Path, stage: &str, script: &str) -> std::io::Result<i32> {
    let exit_file = stage_exit(case, stage);
    if exit_file.exists() {
        std::fs::remove_file(&exit_file)?;
    }
    let log = std::fs::File::create(stage_log(case, stage))?;
    let status = std::process::Command::new("bash")
        .arg("-lc")
        .arg(script)
        .current_dir(case)
        // The container runs as root; Open MPI refuses that unless told otherwise.
        .env("OMPI_ALLOW_RUN_AS_ROOT", "1")
        .env("OMPI_ALLOW_RUN_AS_ROOT_CONFIRM", "1")
        .stdin(std::process::Stdio::null())
        .stdout(log.try_clone()?)
        .stderr(log)
        .status()?;
    let code = status.code().unwrap_or_else(|| killed_code(status));
    std::fs::write(&exit_file, format!("{code}\n"))?;
    Ok(code)
}

/// Exit code for a process ended by a signal, as shells report it (128 + signal).
#[cfg(unix)]
fn killed_code(status: std::process::ExitStatus) -> i32 {
    use std::os::unix::process::ExitStatusExt;
    128 + status.signal().unwrap_or(0)
}

#[cfg(not(unix))]
fn killed_code(_status: std::process::ExitStatus) -> i32 {
    -1
}
