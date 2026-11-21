// SPDX-License-Identifier: GPL-3.0-or-later
//! Project and history commands.

use std::path::{Path, PathBuf};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::json;

use super::recent::{self, RecentProject};
use super::{Options, Registry, Session};
use crate::error::FoamError;
use crate::project::Project;
use crate::units::DisplayUnits;

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct NoParams {}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct NewParams {
    /// Folder of the new project (created if missing; must not hold a project). Default: a free
    /// folder named after the project in the default projects folder.
    #[serde(default)]
    pub path: Option<String>,
    pub name: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct LocationParams {
    /// Project name to find a free folder for.
    #[serde(default)]
    pub name: Option<String>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct DefaultLocation {
    /// The default projects folder.
    pub folder: String,
    /// Folder a project with the given name would get (free at the time of the call).
    pub path: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct OpenParams {
    /// Folder that contains `foamcraft.json`.
    pub path: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RenameParams {
    pub name: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DisplayUnitsParams {
    pub units: DisplayUnits,
}

#[derive(Debug, Clone, PartialEq, Serialize, JsonSchema)]
pub struct ProjectInfo {
    pub path: String,
    pub name: String,
    pub schema_version: u32,
    pub display_units: DisplayUnits,
    /// Job ids of the runs, oldest first.
    pub runs: Vec<String>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct HistoryState {
    pub can_undo: bool,
    pub can_redo: bool,
    /// Command that undo would revert.
    pub undo: Option<String>,
    /// Command that redo would repeat.
    pub redo: Option<String>,
}

impl ProjectInfo {
    pub fn of(project: &Project) -> Self {
        Self {
            path: project.root.to_string_lossy().into_owned(),
            name: project.manifest.name.clone(),
            schema_version: project.manifest.schema_version,
            display_units: project.manifest.display_units,
            runs: project.manifest.runs.clone(),
        }
    }
}

/// Makes `project` the open project of the session and remembers it in the recent list.
pub fn opened(session: &mut Session, project: Project) -> ProjectInfo {
    let info = ProjectInfo::of(&project);
    recent::remember(
        session.data_dir(),
        RecentProject {
            path: info.path.clone(),
            name: info.name.clone(),
        },
    );
    session.set_project(Some(project));
    info
}

fn history_state(session: &Session) -> HistoryState {
    HistoryState {
        can_undo: !session.undo.is_empty(),
        can_redo: !session.redo.is_empty(),
        undo: session.undo.last().map(|e| e.command.to_string()),
        redo: session.redo.last().map(|e| e.command.to_string()),
    }
}

fn step(session: &mut Session, undo: bool) -> Result<ProjectInfo, FoamError> {
    let entry = if undo {
        session.undo.pop()
    } else {
        session.redo.pop()
    };
    let Some(entry) = entry else {
        let code = if undo {
            "history.nothing_to_undo"
        } else {
            "history.nothing_to_redo"
        };
        return Err(FoamError::new(code, ""));
    };
    let project = session.project()?;
    project.manifest = if undo {
        entry.before.clone()
    } else {
        entry.after.clone()
    };
    project.save()?;
    let info = ProjectInfo::of(project);
    if undo {
        session.redo.push(entry)
    } else {
        session.undo.push(entry)
    }
    session
        .events
        .emit("project.changed", serde_json::Value::Null);
    session
        .events
        .emit("history.changed", serde_json::Value::Null);
    Ok(info)
}

const NO_ERRORS: &[&str] = &[];

pub fn register_builtin(r: &mut Registry) {
    let plain = |example| Options {
        undoable: false,
        errors: NO_ERRORS,
        example,
    };
    r.add(
        "project.new",
        "Create a project in a folder and open it.",
        Options {
            undoable: false,
            errors: &["project.already_exists", "project.write_failed"],
            example: json!({"path": "C:/Projects/Wing", "name": "Wing"}),
        },
        |s, p: NewParams| {
            let path = match &p.path {
                Some(path) => PathBuf::from(path),
                None => crate::paths::free_folder(s.projects_dir(), &p.name),
            };
            Ok(opened(s, Project::create(&path, &p.name)?))
        },
    );
    r.add(
        "project.default_location",
        "Report the default projects folder and, for a name, the free folder a new project would get.",
        plain(json!({"name": "Wing"})),
        |s, p: LocationParams| {
            Ok(DefaultLocation {
                folder: s.projects_dir().to_string_lossy().into_owned(),
                path: p.name.map(|name| {
                    crate::paths::free_folder(s.projects_dir(), &name)
                        .to_string_lossy()
                        .into_owned()
                }),
            })
        },
    );
    r.add(
        "project.open",
        "Open the project in a folder.",
        Options {
            undoable: false,
            errors: &[
                "project.not_found",
                "project.schema_too_new",
                "project.invalid_manifest",
            ],
            example: json!({"path": "C:/Projects/Wing"}),
        },
        |s, p: OpenParams| Ok(opened(s, Project::open(Path::new(&p.path))?)),
    );
    r.add(
        "project.close",
        "Close the open project.",
        plain(json!({})),
        |s, _: NoParams| {
            s.set_project(None);
            Ok(())
        },
    );
    r.add(
        "project.info",
        "Describe the open project.",
        Options {
            undoable: false,
            errors: &["project.none_open"],
            example: json!({}),
        },
        |s, _: NoParams| Ok(ProjectInfo::of(s.project()?)),
    );
    r.add(
        "project.recent",
        "List recently opened projects, newest first.",
        plain(json!({})),
        |s, _: NoParams| Ok(recent::load(s.data_dir())),
    );
    r.add(
        "project.rename",
        "Rename the open project.",
        Options {
            undoable: true,
            errors: &["project.none_open"],
            example: json!({"name": "Renamed"}),
        },
        |s, p: RenameParams| {
            let project = s.project()?;
            project.manifest.name = p.name;
            Ok(ProjectInfo::of(project))
        },
    );
    r.add(
        "project.set_display_units",
        "Choose metric or imperial units for display and input.",
        Options {
            undoable: true,
            errors: &["project.none_open"],
            example: json!({"units": "imperial"}),
        },
        |s, p: DisplayUnitsParams| {
            let project = s.project()?;
            project.manifest.display_units = p.units;
            Ok(ProjectInfo::of(project))
        },
    );
    r.add(
        "history.undo",
        "Revert the last change to the project.",
        Options {
            undoable: false,
            errors: &["history.nothing_to_undo", "project.none_open"],
            example: json!({}),
        },
        |s, _: NoParams| step(s, true),
    );
    r.add(
        "history.redo",
        "Repeat the last reverted change.",
        Options {
            undoable: false,
            errors: &["history.nothing_to_redo", "project.none_open"],
            example: json!({}),
        },
        |s, _: NoParams| step(s, false),
    );
    r.add(
        "history.state",
        "Report whether undo and redo are possible.",
        plain(json!({})),
        |s, _: NoParams| Ok(history_state(s)),
    );
}
