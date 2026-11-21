// SPDX-License-Identifier: GPL-3.0-or-later
//! State change events for subscribers (the app interface, the MCP server).

use std::sync::{Arc, Mutex};

use serde::Serialize;
use serde_json::Value;

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Event {
    /// For example `project.changed`, `history.changed`, `run.progress`, `run.finished`.
    pub name: String,
    pub payload: Value,
}

type Listener = Arc<dyn Fn(&Event) + Send + Sync>;

/// Cloneable handle; background work (running jobs) keeps a clone to report progress.
#[derive(Clone, Default)]
pub struct EventBus {
    listeners: Arc<Mutex<Vec<Listener>>>,
}

impl EventBus {
    pub fn subscribe(&self, listener: impl Fn(&Event) + Send + Sync + 'static) {
        if let Ok(mut l) = self.listeners.lock() {
            l.push(Arc::new(listener));
        }
    }

    pub fn emit(&self, name: &str, payload: Value) {
        let listeners = self.listeners.lock().map(|l| l.clone()).unwrap_or_default();
        let event = Event {
            name: name.into(),
            payload,
        };
        for listener in listeners {
            listener(&event);
        }
    }
}
