// SPDX-License-Identifier: GPL-3.0-or-later
//! Fallback ladder for meshing: when layers fail or the mesh is poor, try fewer layers, then no
//! layers, then a coarser mesh.

use crate::mesh_report::{LayerReport, MeshQuality, Verdict};

/// Share of the target layers that must be built.
pub const MIN_LAYER_COVERAGE: f64 = 0.5;

#[derive(Debug, Clone, PartialEq)]
pub struct Attempt {
    pub layers: u32,
    /// Factor on the cell budget.
    pub budget_factor: f64,
    /// Catalog code telling the user what was changed.
    pub hint: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AttemptOutcome {
    /// The meshing stages ran to the end.
    pub meshed: bool,
    pub quality: Option<MeshQuality>,
    pub layers: Option<LayerReport>,
}

pub fn accept(o: &AttemptOutcome) -> bool {
    let quality_ok = o
        .quality
        .as_ref()
        .is_some_and(|q| q.verdict != Verdict::Poor);
    let layers_ok = o
        .layers
        .as_ref()
        .is_none_or(|l| l.coverage() >= MIN_LAYER_COVERAGE);
    o.meshed && quality_ok && layers_ok
}

pub fn next_attempt(previous: &Attempt) -> Option<Attempt> {
    let step = |layers, budget_factor, hint: &str| {
        Some(Attempt {
            layers,
            budget_factor,
            hint: Some(hint.to_string()),
        })
    };
    if previous.layers > 1 && previous.hint.is_none() {
        step(
            previous.layers / 2,
            previous.budget_factor,
            "mesh.fallback_fewer_layers",
        )
    } else if previous.layers > 0 {
        step(0, previous.budget_factor, "mesh.fallback_no_layers")
    } else if previous.budget_factor > 0.5 {
        step(0, previous.budget_factor * 0.5, "mesh.fallback_coarser")
    } else {
        None
    }
}
