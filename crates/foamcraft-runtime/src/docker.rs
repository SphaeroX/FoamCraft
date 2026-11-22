// SPDX-License-Identifier: GPL-3.0-or-later
//! Docker implementation of the runner.

use std::path::Path;
use std::sync::atomic::{AtomicU32, Ordering};

use foamcraft_core::error::FoamError;
use serde_json::Value;

use crate::engine::{CmdOutput, Engine, LogLine, ProcessEngine};
use crate::{
    Backend, Container, EnvironmentReport, IMAGE_TAG, MANAGED_LABEL, Mount, PUBLISHED_IMAGE,
    RECOMMENDED_MEMORY_BYTES, Runner, StartOptions, WORK_DIR,
};

pub struct DockerRunner<E: Engine = ProcessEngine> {
    engine: E,
    host_is_windows: bool,
}

impl DockerRunner<ProcessEngine> {
    /// Runner on the local `docker` command line.
    pub fn local() -> Self {
        Self::with_host(ProcessEngine::default(), cfg!(windows))
    }
}

/// Texts Docker prints when the client cannot reach the service.
const DAEMON_DOWN: [&str; 6] = [
    "cannot connect to the docker daemon",
    "error during connect",
    "is the docker daemon running",
    "dockerdesktoplinuxengine",
    // Docker 28 and newer.
    "failed to connect to the docker api",
    "if the daemon is running",
];

fn map_failure(out: &CmdOutput) -> FoamError {
    let err = out.stderr.to_lowercase();
    let code = if DAEMON_DOWN.iter().any(|t| err.contains(t)) {
        "runtime.daemon_not_running"
    } else if err.contains("unable to find image") || err.contains("no such image") {
        "runtime.image_missing"
    } else {
        "runtime.command_failed"
    };
    FoamError::new(code, out.stderr.trim())
}

fn container_name() -> String {
    static COUNTER: AtomicU32 = AtomicU32::new(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    let t = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_millis());
    format!("foamcraft-{}-{t}-{n}", std::process::id())
}

impl<E: Engine> DockerRunner<E> {
    pub fn with_host(engine: E, host_is_windows: bool) -> Self {
        Self {
            engine,
            host_is_windows,
        }
    }

    pub fn engine(&self) -> &E {
        &self.engine
    }

    fn call(
        &self,
        args: &[&str],
        on_line: &mut dyn FnMut(LogLine),
    ) -> Result<CmdOutput, FoamError> {
        let args: Vec<String> = args.iter().map(|s| s.to_string()).collect();
        self.engine.run(&args, on_line).map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                FoamError::new("runtime.docker_missing", e.to_string())
            } else {
                FoamError::new("runtime.command_failed", e.to_string())
            }
        })
    }

    /// Runs a command and turns a non-zero exit into a mapped error.
    fn call_ok(&self, args: &[&str]) -> Result<CmdOutput, FoamError> {
        let out = self.call(args, &mut |_| {})?;
        if out.status == 0 {
            Ok(out)
        } else {
            Err(map_failure(&out))
        }
    }
}

impl<E: Engine> Runner for DockerRunner<E> {
    fn check(&self) -> Result<EnvironmentReport, FoamError> {
        let out = self.call_ok(&["info", "--format", "{{json .}}"])?;
        let info: Value = serde_json::from_str(out.stdout.trim())
            .map_err(|e| FoamError::new("runtime.command_failed", e.to_string()))?;
        let text = |key: &str| {
            info.get(key)
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string()
        };
        let kernel = text("KernelVersion").to_lowercase();
        let os = text("OperatingSystem");
        let backend = if kernel.contains("microsoft-standard-wsl2") {
            Backend::Wsl2
        } else if os.contains("Docker Desktop") || self.host_is_windows {
            Backend::VirtualMachine
        } else {
            Backend::Native
        };
        let memory_bytes = info.get("MemTotal").and_then(Value::as_u64).unwrap_or(0);
        let cpus = info.get("NCPU").and_then(Value::as_u64).unwrap_or(0) as u32;
        let mut warnings = Vec::new();
        if self.host_is_windows && backend != Backend::Wsl2 {
            warnings.push(FoamError::new(
                "runtime.no_wsl2_backend",
                format!("kernel {kernel}"),
            ));
        }
        if memory_bytes < RECOMMENDED_MEMORY_BYTES {
            warnings.push(FoamError::new(
                "runtime.insufficient_memory",
                format!("{memory_bytes} bytes available, {RECOMMENDED_MEMORY_BYTES} recommended"),
            ));
        }
        Ok(EnvironmentReport {
            server_version: text("ServerVersion"),
            backend,
            cpus,
            memory_bytes,
            warnings,
        })
    }

    fn image_present(&self) -> Result<bool, FoamError> {
        let out = self.call(&["image", "inspect", IMAGE_TAG], &mut |_| {})?;
        if out.status == 0 {
            return Ok(true);
        }
        let err = map_failure(&out);
        match err.code.as_str() {
            "runtime.image_missing" | "runtime.command_failed" => Ok(false),
            _ => Err(err),
        }
    }

    fn build_image(
        &self,
        context: &Path,
        on_line: &mut dyn FnMut(LogLine),
    ) -> Result<(), FoamError> {
        let dockerfile = context.join("runtime").join("Dockerfile");
        let dockerfile = dockerfile.to_string_lossy();
        let context = context.to_string_lossy();
        let out = self.call(
            &[
                "build",
                "--progress",
                "plain",
                "-f",
                &dockerfile,
                "-t",
                IMAGE_TAG,
                &context,
            ],
            on_line,
        )?;
        if out.status == 0 {
            Ok(())
        } else {
            Err(map_failure(&out))
        }
    }

    fn pull_image(&self, on_line: &mut dyn FnMut(LogLine)) -> Result<(), FoamError> {
        let out = self.call(&["pull", PUBLISHED_IMAGE], on_line)?;
        if out.status != 0 {
            return Err(map_failure(&out));
        }
        let out = self.call(&["tag", PUBLISHED_IMAGE, IMAGE_TAG], on_line)?;
        if out.status == 0 {
            Ok(())
        } else {
            Err(map_failure(&out))
        }
    }

    fn start(&self, options: &StartOptions) -> Result<Container, FoamError> {
        let name = container_name();
        let mut args: Vec<String> = [
            "run",
            "-d",
            "--init",
            "--name",
            &name,
            "--label",
            MANAGED_LABEL,
        ]
        .iter()
        .map(|s| s.to_string())
        .collect();
        if let Some(cpus) = options.cpus {
            args.extend(["--cpus".into(), format!("{cpus}")]);
        }
        if let Some(mem) = options.memory_bytes {
            args.extend(["--memory".into(), mem.to_string()]);
        }
        match &options.mount {
            Some(Mount::Volume(v)) => args.extend(["-v".into(), format!("{v}:{WORK_DIR}")]),
            Some(Mount::Bind(p)) => {
                args.extend(["-v".into(), format!("{}:{WORK_DIR}", p.to_string_lossy())])
            }
            None => {}
        }
        if options.gpu {
            args.extend(["--gpus".into(), "all".into()]);
        }
        args.extend([IMAGE_TAG.into(), "idle".into()]);
        let refs: Vec<&str> = args.iter().map(String::as_str).collect();
        let out = self.call_ok(&refs)?;
        Ok(Container {
            id: out.stdout.trim().to_string(),
        })
    }

    fn exec(
        &self,
        container: &Container,
        command: &[&str],
        on_line: &mut dyn FnMut(LogLine),
    ) -> Result<CmdOutput, FoamError> {
        let mut args = vec!["exec", container.id.as_str()];
        args.extend_from_slice(command);
        let out = self.call(&args, on_line)?;
        // Exit codes 125-127 come from `docker exec` itself (no container, no program).
        if (125..=127).contains(&out.status) && out.stdout.is_empty() {
            let err = map_failure(&out);
            if err.code == "runtime.daemon_not_running" {
                return Err(err);
            }
        }
        Ok(out)
    }

    fn exec_detached(
        &self,
        container: &Container,
        command: &[&str],
    ) -> Result<CmdOutput, FoamError> {
        let mut args = vec!["exec", "-d", container.id.as_str()];
        args.extend_from_slice(command);
        self.call(&args, &mut |_| {})
    }

    fn stop(&self, container: &Container) -> Result<(), FoamError> {
        self.call_ok(&["rm", "-f", &container.id]).map(|_| ())
    }

    fn stop_all(&self) -> Result<usize, FoamError> {
        let filter = format!("label={MANAGED_LABEL}");
        let out = self.call_ok(&["ps", "-aq", "--filter", &filter])?;
        let ids: Vec<&str> = out.stdout.split_whitespace().collect();
        if ids.is_empty() {
            return Ok(0);
        }
        let mut args = vec!["rm", "-f"];
        args.extend(&ids);
        self.call_ok(&args)?;
        Ok(ids.len())
    }

    fn create_volume(&self, name: &str) -> Result<(), FoamError> {
        self.call_ok(&["volume", "create", "--label", MANAGED_LABEL, name])
            .map(|_| ())
    }

    fn remove_volume(&self, name: &str) -> Result<(), FoamError> {
        self.call_ok(&["volume", "rm", "-f", name]).map(|_| ())
    }

    fn copy_into(&self, container: &Container, host: &Path, target: &str) -> Result<(), FoamError> {
        let dest = format!("{}:{target}", container.id);
        self.call_ok(&["cp", &host.to_string_lossy(), &dest])
            .map(|_| ())
    }

    fn copy_out(&self, container: &Container, source: &str, host: &Path) -> Result<(), FoamError> {
        let src = format!("{}:{source}", container.id);
        self.call_ok(&["cp", &src, &host.to_string_lossy()])
            .map(|_| ())
    }

    fn gpu_passthrough(&self) -> bool {
        self.call(
            &["run", "--rm", "--gpus", "all", IMAGE_TAG, "version"],
            &mut |_| {},
        )
        .is_ok_and(|o| o.status == 0)
    }
}
