// SPDX-License-Identifier: GPL-3.0-or-later
//! The command API. Every user-visible action is a command with a JSON schema; the interface, the
//! CLI and the MCP server are equal clients of the same [`Registry`].
//!
//! Commands that change the project are undoable: the registry stores the manifest before and
//! after the command, so undo and redo restore the state exactly.

mod builtin;
mod events;
pub mod recent;

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use schemars::JsonSchema;
use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::Value;

use crate::error::FoamError;
use crate::project::{Manifest, Project};

pub use builtin::{ProjectInfo, opened, register_builtin};
pub use events::{Event, EventBus};
pub use recent::RecentProject;

/// Public description of a command.
#[derive(Debug, Clone, Serialize)]
pub struct CommandSpec {
    pub name: &'static str,
    /// One English sentence (developer-facing; the interface uses translated labels).
    pub summary: &'static str,
    /// JSON schema of the parameters.
    pub params: Value,
    /// JSON schema of the result.
    pub result: Value,
    /// Error codes the command can return besides `command.*`.
    pub errors: &'static [&'static str],
    /// The command changes the project and takes part in undo and redo.
    pub undoable: bool,
    /// Valid example parameters.
    pub example: Value,
}

/// Options when registering a command.
pub struct Options {
    pub undoable: bool,
    pub errors: &'static [&'static str],
    pub example: Value,
}

type Handler = Box<dyn Fn(&mut Session, Value) -> Result<Value, FoamError> + Send + Sync>;

#[derive(Default)]
pub struct Registry {
    commands: BTreeMap<&'static str, (CommandSpec, Handler)>,
}

fn schema_of<T: JsonSchema>() -> Value {
    serde_json::to_value(schemars::schema_for!(T)).unwrap_or(Value::Null)
}

impl Registry {
    pub fn add<P, R, F>(&mut self, name: &'static str, summary: &'static str, opts: Options, f: F)
    where
        P: DeserializeOwned + JsonSchema,
        R: Serialize + JsonSchema,
        F: Fn(&mut Session, P) -> Result<R, FoamError> + Send + Sync + 'static,
    {
        let spec = CommandSpec {
            name,
            summary,
            params: schema_of::<P>(),
            result: schema_of::<R>(),
            errors: opts.errors,
            undoable: opts.undoable,
            example: opts.example,
        };
        let handler: Handler = Box::new(move |session, params| {
            let params: P = serde_json::from_value(params)
                .map_err(|e| FoamError::new("command.invalid_params", format!("{name}: {e}")))?;
            let result = f(session, params)?;
            serde_json::to_value(result)
                .map_err(|e| FoamError::new("command.invalid_params", e.to_string()))
        });
        self.commands.insert(name, (spec, handler));
    }

    /// All commands, sorted by name.
    pub fn specs(&self) -> impl Iterator<Item = &CommandSpec> {
        self.commands.values().map(|(spec, _)| spec)
    }

    pub fn describe(&self, name: &str) -> Option<&CommandSpec> {
        self.commands.get(name).map(|(spec, _)| spec)
    }

    /// Runs a command. Undoable commands are recorded in the history and the project is saved.
    pub fn execute(
        &self,
        session: &mut Session,
        name: &str,
        params: Value,
    ) -> Result<Value, FoamError> {
        let (spec, handler) = self
            .commands
            .get(name)
            .ok_or_else(|| FoamError::new("command.unknown", name))?;
        // Changes handed in by background work come first, so every command sees them.
        session.apply_updates();
        if !spec.undoable {
            let result = handler(session, params);
            // A command that waited for background work applies what that work handed in.
            session.apply_updates();
            return result;
        }
        let before = session.project()?.manifest.clone();
        let result = handler(session, params)?;
        let after = session.project()?.manifest.clone();
        if before != after {
            session.undo.push(HistoryEntry {
                command: spec.name,
                before,
                after,
            });
            session.redo.clear();
            session.project()?.save()?;
            session.events.emit("project.changed", Value::Null);
            session.events.emit("history.changed", Value::Null);
        }
        Ok(result)
    }
}

/// Registry with the commands of the core library (project and history).
pub fn core_registry() -> Registry {
    let mut registry = Registry::default();
    register_builtin(&mut registry);
    registry
}

#[derive(Debug, Clone)]
struct HistoryEntry {
    command: &'static str,
    before: Manifest,
    after: Manifest,
}

type Update = Box<dyn FnOnce(&mut Session) + Send>;

/// Changes to the session handed in by background work (a running job). A command holds the
/// session only while it runs; work that outlives it keeps this handle instead and pushes its
/// changes, which are applied before the next command.
#[derive(Clone, Default)]
pub struct Updates {
    pending: Arc<Mutex<Vec<Update>>>,
}

impl Updates {
    pub fn push(&self, update: impl FnOnce(&mut Session) + Send + 'static) {
        if let Ok(mut pending) = self.pending.lock() {
            pending.push(Box::new(update));
        }
    }
}

/// State of one client: the open project, its history and the event bus.
pub struct Session {
    project: Option<Project>,
    /// Selected face keys (`model-1:face-3`); session state, not saved.
    selection: Vec<String>,
    undo: Vec<HistoryEntry>,
    redo: Vec<HistoryEntry>,
    data_dir: PathBuf,
    /// Folder for projects created without a path.
    projects_dir: PathBuf,
    events: EventBus,
    updates: Updates,
}

impl Session {
    /// `data_dir` holds app-wide data such as the list of recent projects.
    pub fn new(data_dir: impl Into<PathBuf>) -> Self {
        Self {
            project: None,
            selection: Vec::new(),
            undo: Vec::new(),
            redo: Vec::new(),
            data_dir: data_dir.into(),
            projects_dir: crate::paths::projects_dir(),
            events: EventBus::default(),
            updates: Updates::default(),
        }
    }

    pub fn data_dir(&self) -> &std::path::Path {
        &self.data_dir
    }

    /// Folder in which projects without a given path are created.
    pub fn projects_dir(&self) -> &std::path::Path {
        &self.projects_dir
    }

    pub fn set_projects_dir(&mut self, dir: impl Into<PathBuf>) {
        self.projects_dir = dir.into();
    }

    /// Handle for background work to hand in changes (see [`Updates`]).
    pub fn updates(&self) -> Updates {
        self.updates.clone()
    }

    /// Applies the changes handed in by background work, oldest first.
    pub fn apply_updates(&mut self) {
        let pending = match self.updates.pending.lock() {
            Ok(mut pending) => std::mem::take(&mut *pending),
            Err(_) => return,
        };
        for update in pending {
            update(self);
        }
    }

    pub fn events(&self) -> &EventBus {
        &self.events
    }

    pub fn project(&mut self) -> Result<&mut Project, FoamError> {
        self.project
            .as_mut()
            .ok_or_else(|| FoamError::new("project.none_open", ""))
    }

    pub fn project_ref(&self) -> Option<&Project> {
        self.project.as_ref()
    }

    pub fn selection(&self) -> &[String] {
        &self.selection
    }

    pub fn set_selection(&mut self, keys: Vec<String>) {
        if keys != self.selection {
            self.selection = keys;
            self.events
                .emit("selection.changed", Value::from(self.selection.clone()));
        }
    }

    /// Replaces the open project and clears the history and the selection.
    pub fn set_project(&mut self, project: Option<Project>) {
        self.project = project;
        self.selection.clear();
        self.undo.clear();
        self.redo.clear();
        self.events.emit("project.changed", Value::Null);
        self.events.emit("history.changed", Value::Null);
    }
}
