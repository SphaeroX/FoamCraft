// SPDX-License-Identifier: GPL-3.0-or-later
//! `foamcraft-agent`: commands the host runs inside the runtime container.
//!
//! - `version`: prints the agent version.
//! - `status`: prints the runtime status as JSON.
//! - `idle`: keeps the container alive until it is stopped (container main process).
//! - `run-stage <case> <stage> <script>`: runs a job stage, writes `log.<stage>` and
//!   `.exit.<stage>` in the case folder.
//! - `tail <case> <stage> <offset>`: prints new stage log text and the exit code as JSON.

use std::path::Path;
use std::process::ExitCode;

use foamcraft_agent::{run_stage, stage_exit, stage_log, tail_stage};

fn print_json<T: serde::Serialize>(value: &T) -> ExitCode {
    match serde_json::to_string(value) {
        Ok(json) => {
            println!("{json}");
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("foamcraft-agent: {e}");
            ExitCode::FAILURE
        }
    }
}

const USAGE: &str = "usage: foamcraft-agent version|status|idle|run-stage <case> <stage> <script>|tail <case> <stage> <offset>";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let arg = |i: usize| args.get(i).map(String::as_str);
    match arg(0) {
        Some("version") => {
            println!("{}", foamcraft_core::VERSION);
            ExitCode::SUCCESS
        }
        Some("status") => print_json(&foamcraft_agent::status()),
        Some("idle") => loop {
            // Main process of the container; `docker rm -f` ends it (docker-init forwards
            // signals).
            std::thread::park();
        },
        Some("run-stage") => match (arg(1), arg(2), arg(3)) {
            (Some(case), Some(stage), Some(script)) => {
                match run_stage(Path::new(case), stage, script) {
                    Ok(code) => ExitCode::from(u8::try_from(code).unwrap_or(1)),
                    Err(e) => {
                        eprintln!("foamcraft-agent: {e}");
                        let _ = std::fs::write(stage_exit(Path::new(case), stage), "127\n");
                        ExitCode::FAILURE
                    }
                }
            }
            _ => {
                eprintln!("{USAGE}");
                ExitCode::from(2)
            }
        },
        Some("tail") => match (arg(1), arg(2), arg(3).and_then(|o| o.parse::<u64>().ok())) {
            (Some(case), Some(stage), Some(offset)) => {
                let case = Path::new(case);
                print_json(&tail_stage(
                    &stage_log(case, stage),
                    &stage_exit(case, stage),
                    offset,
                ))
            }
            _ => {
                eprintln!("{USAGE}");
                ExitCode::from(2)
            }
        },
        _ => {
            eprintln!("{USAGE}");
            ExitCode::from(2)
        }
    }
}
