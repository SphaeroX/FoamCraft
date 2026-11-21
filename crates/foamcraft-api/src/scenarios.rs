// SPDX-License-Identifier: GPL-3.0-or-later
//! Several named scenarios per project: `scenario.list`, `scenario.add`, `scenario.rename`,
//! `scenario.remove`, `scenario.activate`. The existing `scenario.set_*` commands work on the
//! active scenario (the manifest's `scenario` and `mesh`); activating writes the active one back
//! into the list and loads the other. A scenario without a fitting mesh takes the mesh of another
//! scenario when the mesh inputs are the same.

use foamcraft_core::command::{Options, Registry, Session};
use foamcraft_core::error::FoamError;
use foamcraft_core::project::{MeshRecord, Project, Scenario};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::mesh::mesh_is_current;

#[derive(Debug, Serialize, JsonSchema)]
pub struct ListedScenario {
    pub id: String,
    /// Empty: the interface shows "Scenario N".
    pub name: String,
    pub scenario: Option<Scenario>,
    pub mesh: Option<MeshRecord>,
    /// Job ids of the runs of this scenario, oldest first.
    pub runs: Vec<String>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct ScenarioList {
    pub active: String,
    pub scenarios: Vec<ListedScenario>,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AddParams {
    #[serde(default)]
    pub name: String,
    /// Start as a copy of the active scenario (default) or empty.
    #[serde(default = "yes")]
    pub copy: bool,
}

fn yes() -> bool {
    true
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct IdParams {
    pub id: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RenameParams {
    pub id: String,
    pub name: String,
}

pub fn list(project: &mut Project) -> ScenarioList {
    project.manifest.normalize();
    let m = &project.manifest;
    ScenarioList {
        active: m.active_id(),
        scenarios: m
            .scenarios
            .iter()
            .map(|e| ListedScenario {
                id: e.id.clone(),
                name: e.name.clone(),
                scenario: e.scenario.clone(),
                mesh: e.mesh.clone(),
                runs: m
                    .runs
                    .iter()
                    .filter(|r| m.run_scenarios.get(*r) == Some(&e.id))
                    .cloned()
                    .collect(),
            })
            .collect(),
    }
}

fn exists(project: &Project, id: &str) -> Result<(), FoamError> {
    if project.manifest.scenarios.iter().any(|e| e.id == id) {
        Ok(())
    } else {
        Err(FoamError::new("scenario.not_found", id))
    }
}

/// Makes `id` active; without a fitting own mesh it takes a fitting one of another scenario.
pub fn activate(project: &mut Project, id: &str) -> Result<(), FoamError> {
    project.manifest.normalize();
    exists(project, id)?;
    project.manifest.activate_scenario(id);
    if !mesh_is_current(project) {
        let own = project.manifest.mesh.clone();
        let others: Vec<MeshRecord> = project
            .manifest
            .scenarios
            .iter()
            .filter(|e| e.id != id)
            .filter_map(|e| e.mesh.clone())
            .collect();
        for m in others {
            project.manifest.mesh = Some(m);
            if mesh_is_current(project) {
                project.manifest.normalize();
                return Ok(());
            }
        }
        project.manifest.mesh = own;
    }
    project.manifest.normalize();
    Ok(())
}

pub fn register(r: &mut Registry) {
    let errors: &'static [&'static str] = &["project.none_open", "scenario.not_found"];
    r.add(
        "scenario.list",
        "The named scenarios of the project with their mesh and runs, and the active one.",
        Options {
            undoable: false,
            errors: &["project.none_open"],
            example: json!({}),
        },
        |s: &mut Session, _: crate::results::JobParams| Ok(list(s.project()?)),
    );
    r.add(
        "scenario.add",
        "Add a named scenario, as a copy of the active one (default) or empty; returns its id. \
         It does not become active.",
        Options {
            undoable: true,
            errors: &["project.none_open"],
            example: json!({"name": "Fast", "copy": true}),
        },
        |s: &mut Session, p: AddParams| {
            let id = s.project()?.manifest.add_scenario(&p.name, p.copy);
            Ok(json!({"id": id}))
        },
    );
    r.add(
        "scenario.rename",
        "Give a scenario a name.",
        Options {
            undoable: true,
            errors,
            example: json!({"id": "scenario-1", "name": "Slow"}),
        },
        |s: &mut Session, p: RenameParams| {
            let project = s.project()?;
            project.manifest.normalize();
            exists(project, &p.id)?;
            if let Some(e) = project.manifest.scenarios.iter_mut().find(|e| e.id == p.id) {
                e.name = p.name.trim().to_string();
            }
            Ok(list(project))
        },
    );
    r.add(
        "scenario.remove",
        "Remove a scenario that is not active (its runs stay).",
        Options {
            undoable: true,
            errors: &[
                "project.none_open",
                "scenario.not_found",
                "scenario.remove_active",
            ],
            example: json!({"id": "scenario-2"}),
        },
        |s: &mut Session, p: IdParams| {
            let project = s.project()?;
            project.manifest.normalize();
            exists(project, &p.id)?;
            if project.manifest.active_id() == p.id {
                return Err(FoamError::new("scenario.remove_active", p.id));
            }
            project.manifest.scenarios.retain(|e| e.id != p.id);
            Ok(list(project))
        },
    );
    r.add(
        "scenario.activate",
        "Make a scenario the active one: scenario.set_*, mesh.generate and run.start work on it.",
        Options {
            undoable: true,
            errors,
            example: json!({"id": "scenario-1"}),
        },
        |s: &mut Session, p: IdParams| {
            let project = s.project()?;
            activate(project, &p.id)?;
            Ok(list(project))
        },
    );
}
