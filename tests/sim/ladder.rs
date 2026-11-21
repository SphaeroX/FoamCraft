// SPDX-License-Identifier: GPL-3.0-or-later
//! Fallback ladder: fewer layers, no layers, coarser mesh.

use foamcraft_sim::mesh_report::{LayerReport, MeshQuality, Verdict};
use foamcraft_sim::mesh_steps::{Attempt, AttemptOutcome, accept, next_attempt};

fn quality(verdict: Verdict) -> MeshQuality {
    MeshQuality {
        cells: Some(1000),
        failed_checks: 0,
        max_non_orthogonality: Some(40.0),
        max_skewness: Some(1.0),
        verdict,
    }
}

fn outcome(verdict: Verdict, layers: Option<(u32, f64)>) -> AttemptOutcome {
    AttemptOutcome {
        meshed: true,
        quality: Some(quality(verdict)),
        layers: layers.map(|(target, achieved)| LayerReport {
            target,
            achieved,
            thickness_percent: 80.0,
        }),
    }
}

#[test]
fn good_meshes_with_enough_layers_are_accepted() {
    assert!(accept(&outcome(Verdict::Good, Some((5, 4.2)))));
    assert!(accept(&outcome(Verdict::Acceptable, Some((5, 3.0)))));
    assert!(accept(&outcome(Verdict::Good, None)));
    assert!(!accept(&outcome(Verdict::Poor, None)));
    assert!(
        !accept(&outcome(Verdict::Good, Some((5, 1.0)))),
        "20 % of the layers is a layer failure"
    );
    assert!(!accept(&AttemptOutcome {
        meshed: false,
        quality: None,
        layers: None
    }));
}

#[test]
fn the_ladder_reduces_layers_then_coarsens_then_gives_up() {
    let first = Attempt {
        layers: 6,
        budget_factor: 1.0,
        hint: None,
    };
    let a1 = next_attempt(&first).unwrap();
    assert_eq!(
        (a1.layers, a1.budget_factor, a1.hint.as_deref()),
        (3, 1.0, Some("mesh.fallback_fewer_layers"))
    );
    let a2 = next_attempt(&a1).unwrap();
    assert_eq!(
        (a2.layers, a2.hint.as_deref()),
        (0, Some("mesh.fallback_no_layers"))
    );
    let a3 = next_attempt(&a2).unwrap();
    assert_eq!(
        (a3.layers, a3.budget_factor, a3.hint.as_deref()),
        (0, 0.5, Some("mesh.fallback_coarser"))
    );
    assert!(next_attempt(&a3).is_none());
    // With one layer, "fewer layers" is skipped.
    let single = Attempt {
        layers: 1,
        budget_factor: 1.0,
        hint: None,
    };
    assert_eq!(
        next_attempt(&single).unwrap().hint.as_deref(),
        Some("mesh.fallback_no_layers")
    );
}
