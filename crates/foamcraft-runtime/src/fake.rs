// SPDX-License-Identifier: GPL-3.0-or-later
//! A scripted engine for tests: answers commands by argument prefix and records every call.

use std::sync::Mutex;

use crate::engine::{CmdOutput, Engine, LogLine};

#[derive(Default)]
pub struct FakeEngine {
    responses: Vec<(Vec<String>, CmdOutput)>,
    calls: Mutex<Vec<Vec<String>>>,
    missing: bool,
}

impl FakeEngine {
    pub fn new() -> Self {
        Self::default()
    }

    /// An engine whose binary is not installed.
    pub fn missing_binary() -> Self {
        Self {
            missing: true,
            ..Self::default()
        }
    }

    /// Answers every call whose arguments start with `prefix`. Earlier rules win.
    pub fn on(mut self, prefix: &[&str], output: CmdOutput) -> Self {
        self.responses
            .push((prefix.iter().map(|s| s.to_string()).collect(), output));
        self
    }

    pub fn calls(&self) -> Vec<Vec<String>> {
        self.calls.lock().map(|c| c.clone()).unwrap_or_default()
    }
}

impl Engine for FakeEngine {
    fn run(&self, args: &[String], on_line: &mut dyn FnMut(LogLine)) -> std::io::Result<CmdOutput> {
        if let Ok(mut calls) = self.calls.lock() {
            calls.push(args.to_vec());
        }
        if self.missing {
            return Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "program not found",
            ));
        }
        let output = self
            .responses
            .iter()
            .find(|(prefix, _)| args.starts_with(prefix))
            .map(|(_, out)| out.clone())
            .unwrap_or_else(|| CmdOutput::failed(1, "fake engine: no response configured"));
        output
            .stdout
            .lines()
            .for_each(|l| on_line(LogLine::Stdout(l.into())));
        output
            .stderr
            .lines()
            .for_each(|l| on_line(LogLine::Stderr(l.into())));
        Ok(output)
    }
}
