// SPDX-License-Identifier: GPL-3.0-or-later
//! The complete FoamCraft command API: core commands (project, history) plus commands that need
//! the runtime (runs). The app, the CLI and the MCP server all use [`registry`].

pub mod bridge;
mod cad;
pub mod export;
pub mod heavy;
pub mod mesh;
mod mesh_internal;
pub mod particles;
pub mod presets;
mod report;
pub mod results;
pub mod ride;
mod run;
pub mod runs;
mod samples;
pub mod scenarios;
pub mod scene;
pub mod solve;
pub mod surface_flow;
pub mod system;
pub mod view;

use std::sync::Arc;

use foamcraft_core::command::{Registry, core_registry};
use foamcraft_runtime::{DockerRunner, Runner};

pub use cad::DockerCad;
pub use foamcraft_core::command::{CommandSpec, Event, Session};

/// Runner shared by background jobs.
pub type SharedRunner = Arc<dyn Runner + Send + Sync>;

/// All commands, running simulations with the local Docker runtime.
pub fn registry() -> Registry {
    registry_with(Arc::new(DockerRunner::local()))
}

/// All commands with a given runner (tests use a fake engine).
pub fn registry_with(runner: SharedRunner) -> Registry {
    let mut registry = core_registry();
    let cad = Arc::new(cad::DockerCad {
        runner: runner.clone(),
    });
    foamcraft_geometry::commands::register_with_cad(&mut registry, Some(cad));
    foamcraft_sim::commands::register(&mut registry);
    mesh::register(&mut registry, runner.clone());
    results::register(&mut registry);
    view::register(&mut registry);
    particles::register(&mut registry);
    presets::register(&mut registry);
    ride::register(&mut registry);
    surface_flow::register(&mut registry);
    heavy::register(&mut registry);
    scenarios::register(&mut registry);
    export::register(&mut registry);
    report::register(&mut registry);
    samples::register(&mut registry);
    system::register(&mut registry, runner.clone());
    runs::register(&mut registry, runner.clone());
    run::register(&mut registry, runner);
    registry
}
