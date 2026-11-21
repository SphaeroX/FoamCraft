// SPDX-License-Identifier: GPL-3.0-or-later
//! Parser for OpenFOAM logs (blockMesh, snappyHexMesh, checkMesh, decomposePar, solvers).
//!
//! Works line by line so it can follow a running process. Formats were recorded with OpenFOAM
//! v2606; samples are in `tests/data/logs`.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::error::FoamError;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum LogEvent {
    /// `Time = N`: a new solver iteration (steady) or time step (transient).
    Iteration {
        time: f64,
    },
    /// Initial residual of one linear solve. NaN when the run diverged.
    Residual {
        field: String,
        initial: f64,
    },
    ExecutionTime {
        seconds: f64,
    },
    /// snappyHexMesh phase: `refine`, `snap` or `layers`.
    Phase(String),
    /// Cell count reported by checkMesh.
    Cells(u64),
    MeshOk(bool),
    /// The solver met its residual controls.
    Converged,
    /// The program reached its regular end.
    End,
    Error(FoamError),
}

#[derive(Debug, Default)]
pub struct LogParser {
    /// Lines of a FATAL ERROR block being collected.
    fatal: Option<Vec<String>>,
    diverged: bool,
    phase: Option<&'static str>,
}

fn number_after<'a>(line: &'a str, key: &str) -> Option<&'a str> {
    let rest = &line[line.find(key)? + key.len()..];
    rest.split([',', ' ']).find(|s| !s.is_empty())
}

fn parse_f64(s: &str) -> f64 {
    // OpenFOAM prints `nan` and `-nan`; both mean divergence.
    if s.trim_start_matches('-').eq_ignore_ascii_case("nan") {
        f64::NAN
    } else {
        s.parse().unwrap_or(f64::NAN)
    }
}

impl LogParser {
    fn diverged(&mut self, detail: &str) -> Vec<LogEvent> {
        if self.diverged {
            return vec![];
        }
        self.diverged = true;
        vec![LogEvent::Error(FoamError::new(
            "run.diverged",
            detail.trim(),
        ))]
    }

    fn snappy_phase(&mut self, line: &str) -> Option<LogEvent> {
        let phase = if line.starts_with("Refinement phase") {
            "refine"
        } else if line.starts_with("Snapping to features") || line.starts_with("Morph iteration") {
            "snap"
        } else if line.starts_with("Layer addition iteration") || line.starts_with("Adding layers")
        {
            "layers"
        } else {
            return None;
        };
        if self.phase == Some(phase) {
            return None;
        }
        self.phase = Some(phase);
        Some(LogEvent::Phase(phase.into()))
    }

    /// Parses one line (without the line break).
    pub fn feed(&mut self, raw: &str) -> Vec<LogEvent> {
        let line = raw.trim_end();
        let trimmed = line.trim_start();

        if let Some(block) = self.fatal.as_mut() {
            if trimmed.starts_with("FOAM exiting") || trimmed.starts_with("FOAM aborting") {
                let detail = block
                    .iter()
                    .map(|l| l.trim())
                    .filter(|l| !l.is_empty())
                    .take(3)
                    .collect::<Vec<_>>()
                    .join(" ");
                self.fatal = None;
                return vec![LogEvent::Error(FoamError::new(
                    "run.foam_fatal_error",
                    detail,
                ))];
            }
            block.push(line.to_string());
            return vec![];
        }

        if trimmed.starts_with("--> FOAM FATAL") {
            self.fatal = Some(vec![]);
            return vec![];
        }
        // The start banner "trapFpe: Floating point exception trapping enabled" is not an error.
        if !trimmed.starts_with("trapFpe")
            && (trimmed.contains("Floating point exception")
                || trimmed.contains("sigFpe::sigHandler"))
        {
            return self.diverged("floating point exception");
        }
        if let Some(t) = trimmed.strip_prefix("Time = ") {
            return vec![LogEvent::Iteration {
                time: parse_f64(t.trim()),
            }];
        }
        if let Some(pos) = trimmed.find(":  Solving for ") {
            let rest = &trimmed[pos + ":  Solving for ".len()..];
            let field = rest.split(',').next().unwrap_or("").trim().to_string();
            let initial = number_after(trimmed, "Initial residual = ").map_or(f64::NAN, parse_f64);
            let finals = number_after(trimmed, "Final residual = ").map_or(0.0, parse_f64);
            let mut events = vec![LogEvent::Residual {
                field: field.clone(),
                initial,
            }];
            if initial.is_nan() || finals.is_nan() {
                events.extend(self.diverged(&format!("residual of {field} is not a number")));
            }
            return events;
        }
        if let Some(t) = number_after(trimmed, "ExecutionTime = ") {
            return vec![LogEvent::ExecutionTime {
                seconds: parse_f64(t),
            }];
        }
        if trimmed.contains("solution converged in") {
            return vec![LogEvent::Converged];
        }
        if let Some(n) = trimmed.strip_prefix("cells:")
            && let Ok(n) = n.trim().parse()
        {
            return vec![LogEvent::Cells(n)];
        }
        if trimmed == "Mesh OK." {
            return vec![LogEvent::MeshOk(true)];
        }
        if trimmed.starts_with("Failed ") && trimmed.ends_with("mesh checks.") {
            return vec![
                LogEvent::MeshOk(false),
                LogEvent::Error(FoamError::new("mesh.check_failed", trimmed)),
            ];
        }
        if trimmed == "End" {
            return vec![LogEvent::End];
        }
        self.snappy_phase(trimmed).into_iter().collect()
    }

    /// Flushes an unfinished fatal error block (process killed while printing it).
    pub fn finish(&mut self) -> Vec<LogEvent> {
        match self.fatal.take() {
            Some(block) => vec![LogEvent::Error(FoamError::new(
                "run.foam_fatal_error",
                block.join(" ").trim(),
            ))],
            None => vec![],
        }
    }
}

/// Parses a whole log.
pub fn parse_all(text: &str) -> Vec<LogEvent> {
    let mut parser = LogParser::default();
    let mut events: Vec<LogEvent> = text.lines().flat_map(|l| parser.feed(l)).collect();
    events.extend(parser.finish());
    events
}

/// The state a log describes so far.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct LogSummary {
    pub iterations: u64,
    pub last_iteration: Option<f64>,
    /// Latest initial residual per field.
    pub residuals: BTreeMap<String, f64>,
    pub execution_time: Option<f64>,
    pub cells: Option<u64>,
    pub mesh_ok: Option<bool>,
    pub converged: bool,
    pub ended: bool,
    pub errors: Vec<FoamError>,
}

impl LogSummary {
    pub fn apply(&mut self, event: &LogEvent) {
        match event {
            LogEvent::Iteration { time } => {
                self.iterations += 1;
                self.last_iteration = Some(*time);
            }
            LogEvent::Residual { field, initial } => {
                self.residuals.insert(field.clone(), *initial);
            }
            LogEvent::ExecutionTime { seconds } => self.execution_time = Some(*seconds),
            LogEvent::Cells(n) => self.cells = Some(*n),
            LogEvent::MeshOk(ok) => self.mesh_ok = Some(*ok),
            LogEvent::Converged => self.converged = true,
            LogEvent::End => self.ended = true,
            LogEvent::Error(e) => self.errors.push(e.clone()),
            LogEvent::Phase(_) => {}
        }
    }

    pub fn from_events(events: &[LogEvent]) -> Self {
        let mut s = Self::default();
        events.iter().for_each(|e| s.apply(e));
        s
    }
}
