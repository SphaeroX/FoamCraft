// SPDX-License-Identifier: GPL-3.0-or-later
//! How far a meshing job is: an estimate from the job stages and the snappyHexMesh log, for a
//! progress bar and a rough remaining time. It never goes backwards within one attempt.

use foamcraft_core::job::Stage;

/// Start of each part on the scale 0..1, with and without boundary layers. The shares follow the
/// times measured on the sample cases (the body-fitted mesh takes nearly all of it).
struct Spans {
    refine: f64,
    snap: f64,
    layers: f64,
}

const BACKGROUND_END: f64 = 0.03;
const MESH_END: f64 = 0.95;
const WITH_LAYERS: Spans = Spans {
    refine: 0.06,
    snap: 0.50,
    layers: 0.72,
};
const WITHOUT_LAYERS: Spans = Spans {
    refine: 0.06,
    snap: 0.60,
    layers: MESH_END,
};
/// Every further step of a phase covers this share of what is left of the phase.
const STEP_SHARE: f64 = 0.25;

#[derive(Debug, Clone)]
pub struct MeshProgress {
    with_layers: bool,
    stage: Option<Stage>,
    phase: Option<&'static str>,
    /// Steps seen in the current phase.
    steps: u32,
    fraction: f64,
}

impl MeshProgress {
    /// `with_layers`: the plan asks for boundary layers (their phase takes a share of the time).
    pub fn new(with_layers: bool) -> Self {
        Self {
            with_layers,
            stage: None,
            phase: None,
            steps: 0,
            fraction: 0.0,
        }
    }

    /// Share of the meshing that is done, 0 to 1.
    pub fn fraction(&self) -> f64 {
        self.fraction
    }

    /// snappyHexMesh phase: `refine`, `snap` or `layers`.
    pub fn phase(&self) -> Option<&'static str> {
        self.phase
    }

    pub fn stage(&self) -> Option<Stage> {
        self.stage
    }

    fn spans(&self) -> &'static Spans {
        if self.with_layers {
            &WITH_LAYERS
        } else {
            &WITHOUT_LAYERS
        }
    }

    fn raise(&mut self, to: f64) -> bool {
        if to > self.fraction {
            self.fraction = to.min(1.0);
            true
        } else {
            false
        }
    }

    /// A stage of the job started. Returns whether the estimate changed.
    pub fn stage_started(&mut self, stage: Stage) -> bool {
        let changed = self.stage != Some(stage);
        self.stage = Some(stage);
        let at = match stage {
            Stage::Background => 0.0,
            Stage::Mesh => BACKGROUND_END,
            Stage::Check => MESH_END,
            // Later stages belong to the solver: the mesh is done.
            _ => 1.0,
        };
        if stage != Stage::Mesh {
            self.phase = None;
        }
        self.raise(at) || changed
    }

    /// The job ended after the mesh check.
    pub fn finished(&mut self) -> bool {
        self.phase = None;
        self.raise(1.0)
    }

    /// One log line of the current stage. Returns whether the estimate changed.
    pub fn line(&mut self, line: &str) -> bool {
        if self.stage != Some(Stage::Mesh) {
            return false;
        }
        let line = line.trim_start();
        let spans = self.spans();
        let entered = if line.starts_with("Refinement phase") {
            Some(("refine", spans.refine))
        } else if line.starts_with("Morphing phase") || line.starts_with("Snapping to features") {
            Some(("snap", spans.snap))
        } else if self.with_layers
            && (line.starts_with("Shrinking and layer addition phase")
                || line.starts_with("Adding layers"))
        {
            Some(("layers", spans.layers))
        } else {
            None
        };
        if let Some((phase, start)) = entered {
            if self.phase == Some(phase) {
                return false;
            }
            self.phase = Some(phase);
            self.steps = 0;
            self.raise(start);
            return true;
        }
        let step = match self.phase {
            Some("refine") => line.contains("refinement iteration"),
            Some("snap") => {
                line.starts_with("Morph iteration") || line.starts_with("Smoothing iteration")
            }
            Some("layers") => line.starts_with("Layer addition iteration"),
            _ => false,
        };
        if !step {
            return false;
        }
        self.steps += 1;
        let (start, end) = match self.phase {
            Some("refine") => (spans.refine, spans.snap),
            Some("snap") => (spans.snap, spans.layers),
            _ => (spans.layers, MESH_END),
        };
        // Approaches the end of the phase without reaching it: the number of steps is not known.
        let done = 1.0 - (1.0 - STEP_SHARE).powi(self.steps as i32);
        self.raise(start + (end - start) * done)
    }

    /// Rough remaining time from the time spent so far; `None` while too little is known.
    pub fn remaining_s(&self, elapsed_s: f64) -> Option<f64> {
        (self.fraction >= 0.08 && self.fraction < 1.0 && elapsed_s > 0.0)
            .then(|| elapsed_s * (1.0 - self.fraction) / self.fraction)
    }
}
