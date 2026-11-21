// SPDX-License-Identifier: GPL-3.0-or-later
//! The meshing progress estimate against recorded snappyHexMesh output.

use std::path::PathBuf;

use foamcraft_core::job::Stage;
use foamcraft_sim::mesh_progress::MeshProgress;

fn log(name: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/data/logs")
        .join(name);
    std::fs::read_to_string(path).unwrap()
}

#[test]
fn a_recorded_run_moves_forward_through_refine_and_snap() {
    let mut p = MeshProgress::new(false);
    assert_eq!(p.fraction(), 0.0);
    p.stage_started(Stage::Background);
    p.stage_started(Stage::Mesh);
    let after_background = p.fraction();
    assert!(after_background > 0.0 && after_background < 0.1);
    let mut last = after_background;
    let mut phases = Vec::new();
    for line in log("snappyHexMesh_ok.log").lines() {
        if p.line(line) {
            assert!(p.fraction() >= last, "went backwards at {line:?}");
            last = p.fraction();
        }
        if let Some(phase) = p.phase()
            && phases.last() != Some(&phase)
        {
            phases.push(phase);
        }
    }
    assert_eq!(phases, ["refine", "snap"]);
    // Without layers the snapping ends the body-fitted mesh; the check follows.
    assert!((0.6..0.95).contains(&p.fraction()), "{}", p.fraction());
    assert!(p.stage_started(Stage::Check));
    assert!(p.fraction() >= 0.95);
    assert!(p.finished());
    assert_eq!(p.fraction(), 1.0);
}

#[test]
fn layers_get_their_own_share_only_when_planned() {
    let lines = [
        "Refinement phase",
        "Surface refinement iteration 0",
        "Morphing phase",
        "Morph iteration 0",
        "Shrinking and layer addition phase",
        "Layer addition iteration 0",
    ];
    let run = |with_layers: bool| {
        let mut p = MeshProgress::new(with_layers);
        p.stage_started(Stage::Mesh);
        let mut at_snap = 0.0;
        for line in lines {
            p.line(line);
            if line == "Morph iteration 0" {
                at_snap = p.fraction();
            }
        }
        (at_snap, p.fraction(), p.phase())
    };
    let (snap_with, end_with, phase_with) = run(true);
    let (snap_without, end_without, phase_without) = run(false);
    assert_eq!(phase_with, Some("layers"));
    assert_eq!(phase_without, Some("snap"));
    // With layers planned the snapping is reached earlier on the scale, and layers move it on.
    assert!(snap_with < snap_without);
    assert!(end_with > snap_with);
    assert_eq!(end_without, snap_without);
}

#[test]
fn the_remaining_time_follows_the_fraction() {
    let mut p = MeshProgress::new(false);
    assert_eq!(p.remaining_s(10.0), None, "nothing known at the start");
    p.stage_started(Stage::Mesh);
    p.line("Refinement phase");
    for i in 0..3 {
        p.line(&format!("Surface refinement iteration {i}"));
    }
    let f = p.fraction();
    let remaining = p.remaining_s(60.0).unwrap();
    assert!((remaining - 60.0 * (1.0 - f) / f).abs() < 1e-9);
    p.finished();
    assert_eq!(p.remaining_s(60.0), None);
}

#[test]
fn lines_of_other_stages_do_not_count() {
    let mut p = MeshProgress::new(true);
    p.stage_started(Stage::Background);
    assert!(!p.line("Refinement phase"));
    assert_eq!(p.phase(), None);
}
