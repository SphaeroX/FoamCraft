// SPDX-License-Identifier: GPL-3.0-or-later
//! The container engine command line.

use std::io::{BufRead, BufReader, Read};
use std::process::{Command, Stdio};
use std::sync::mpsc;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LogLine {
    Stdout(String),
    Stderr(String),
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CmdOutput {
    pub status: i32,
    pub stdout: String,
    pub stderr: String,
}

impl CmdOutput {
    pub fn ok(stdout: &str) -> Self {
        Self {
            status: 0,
            stdout: stdout.into(),
            stderr: String::new(),
        }
    }

    pub fn failed(status: i32, stderr: &str) -> Self {
        Self {
            status,
            stdout: String::new(),
            stderr: stderr.into(),
        }
    }

    pub fn with_lines(status: i32, stdout: &[&str], stderr: &[&str]) -> Self {
        let join = |l: &[&str]| l.iter().map(|s| format!("{s}\n")).collect::<String>();
        Self {
            status,
            stdout: join(stdout),
            stderr: join(stderr),
        }
    }
}

/// Runs the engine binary (`docker`) with arguments.
pub trait Engine {
    /// `Err` only when the binary cannot be started (for example not installed). `on_line`
    /// receives output lines while the command runs.
    fn run(&self, args: &[String], on_line: &mut dyn FnMut(LogLine)) -> std::io::Result<CmdOutput>;
}

/// The real `docker` command line.
pub struct ProcessEngine {
    program: String,
}

impl Default for ProcessEngine {
    fn default() -> Self {
        Self {
            program: "docker".into(),
        }
    }
}

fn pump(
    reader: impl Read + Send + 'static,
    tx: mpsc::Sender<LogLine>,
    wrap: fn(String) -> LogLine,
) -> std::thread::JoinHandle<()> {
    std::thread::spawn(move || {
        for line in BufReader::new(reader).lines() {
            let Ok(line) = line else { break };
            if tx.send(wrap(line)).is_err() {
                break;
            }
        }
    })
}

impl Engine for ProcessEngine {
    fn run(&self, args: &[String], on_line: &mut dyn FnMut(LogLine)) -> std::io::Result<CmdOutput> {
        let mut child = Command::new(&self.program)
            .args(args)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;
        let (tx, rx) = mpsc::channel();
        let mut threads = Vec::new();
        if let Some(out) = child.stdout.take() {
            threads.push(pump(out, tx.clone(), LogLine::Stdout));
        }
        if let Some(err) = child.stderr.take() {
            threads.push(pump(err, tx.clone(), LogLine::Stderr));
        }
        drop(tx);
        let mut output = CmdOutput::default();
        for line in rx {
            match &line {
                LogLine::Stdout(s) => {
                    output.stdout.push_str(s);
                    output.stdout.push('\n');
                }
                LogLine::Stderr(s) => {
                    output.stderr.push_str(s);
                    output.stderr.push('\n');
                }
            }
            on_line(line);
        }
        for t in threads {
            let _ = t.join();
        }
        output.status = child.wait()?.code().unwrap_or(-1);
        Ok(output)
    }
}
