// SPDX-License-Identifier: GPL-3.0-or-later
//! `foamcraft project ...` and `foamcraft command ...`: the command API from the command line.

use std::path::Path;

use foamcraft_api::Session;
use foamcraft_core::error::FoamError;
use serde_json::{Value, json};

fn session() -> Session {
    Session::new(foamcraft_core::paths::data_dir())
}

fn print_json(value: &Value) -> Result<(), FoamError> {
    let text = serde_json::to_string_pretty(value)
        .map_err(|e| FoamError::new("command.invalid_params", e.to_string()))?;
    println!("{text}");
    Ok(())
}

fn run_one(session: &mut Session, name: &str, params: Value) -> Result<Value, FoamError> {
    foamcraft_api::registry().execute(session, name, params)
}

pub fn project_new(path: &Path, name: Option<String>) -> Result<(), FoamError> {
    let name = name.unwrap_or_else(|| {
        path.file_name()
            .map_or("Project".into(), |n| n.to_string_lossy().into_owned())
    });
    let info = run_one(
        &mut session(),
        "project.new",
        json!({"path": path, "name": name}),
    )?;
    print_json(&info)
}

pub fn project_open(path: &Path) -> Result<(), FoamError> {
    let info = run_one(&mut session(), "project.open", json!({"path": path}))?;
    print_json(&info)
}

pub fn project_info(path: &Path) -> Result<(), FoamError> {
    let mut s = session();
    run_one(&mut s, "project.open", json!({"path": path}))?;
    print_json(&run_one(&mut s, "project.info", json!({}))?)
}

pub fn command_list() -> Result<(), FoamError> {
    for spec in foamcraft_api::registry().specs() {
        println!("{:<28} {}", spec.name, spec.summary);
    }
    Ok(())
}

pub fn command_describe(name: &str) -> Result<(), FoamError> {
    let registry = foamcraft_api::registry();
    let spec = registry
        .describe(name)
        .ok_or_else(|| FoamError::new("command.unknown", name))?;
    let value = serde_json::to_value(spec)
        .map_err(|e| FoamError::new("command.invalid_params", e.to_string()))?;
    print_json(&value)
}

pub fn command_run(
    name: &str,
    params: Option<&str>,
    project: Option<&Path>,
) -> Result<(), FoamError> {
    let params: Value = match params {
        Some(text) => serde_json::from_str(text)
            .map_err(|e| FoamError::new("command.invalid_params", e.to_string()))?,
        None => json!({}),
    };
    let registry = foamcraft_api::registry();
    let mut s = session();
    if let Some(path) = project {
        registry.execute(&mut s, "project.open", json!({"path": path}))?;
    }
    let result = registry.execute(&mut s, name, params)?;
    print_json(&result)
}

#[derive(serde::Deserialize)]
struct BatchItem {
    name: String,
    #[serde(default)]
    params: Option<Value>,
}

/// Runs several commands in one session (selection and history carry over); stops at the first
/// error. Prints the list of results.
pub fn command_batch(commands: &str, project: Option<&Path>) -> Result<(), FoamError> {
    let items: Vec<BatchItem> = serde_json::from_str(commands)
        .map_err(|e| FoamError::new("command.invalid_params", e.to_string()))?;
    let registry = foamcraft_api::registry();
    let mut s = session();
    if let Some(path) = project {
        registry.execute(&mut s, "project.open", json!({"path": path}))?;
    }
    let mut results = Vec::new();
    for item in items {
        results.push(registry.execute(&mut s, &item.name, item.params.unwrap_or(json!({})))?);
    }
    print_json(&Value::Array(results))
}
