// SPDX-License-Identifier: GPL-3.0-or-later
//! export.report on a project with stored results: PDF and HTML in English and German, a
//! comparison report, the OpenFOAM release matches the runtime image, and a run without result
//! fields gives a report without images.

use std::path::{Path, PathBuf};

use foamcraft_core::command::{Registry, Session};
use serde_json::{Value, json};

fn result(job: &str, drag: f64) -> Value {
    json!({
        "job_id": job, "reason": "stable", "iterations": 300, "decided_at_iteration": 290,
        "max_iterations": 900, "fluctuation_percent": 0.5,
        "coefficients": {"cd": drag / 160.0, "cl": 0.1, "cs": 0.0, "cm_pitch": 0.0, "cm_roll": 0.0, "cm_yaw": 0.0},
        "forces": {"drag_n": drag, "lift_n": 1.0, "side_n": 0.0},
        "parts": [],
        "reference": {"area_m2": 0.5, "length_m": 1.0, "speed_m_s": 20.0, "density_kg_m3": 1.2, "dynamic_pressure_pa": 240.0},
        "y_plus": [{"patch": "model", "min": 1.0, "max": 90.0, "average": 45.0}],
        "hints": ["run.stable"], "restarted": false, "cells": 1000, "cores": 2, "timings_s": {"solve": 10.0},
        "scenario": {"speed_m_s": 20.0, "direction_deg": 0.0, "ground": "fixed", "medium": {"kind": "air", "temperature_c": 20.0}, "quality": "draft"},
        "mesh": {"job_id": "job-mesh", "cells": 1000, "verdict": "good", "layers_target": 3, "layers_achieved": 3.0, "fallbacks": []},
        "history": {"iterations": [1, 2, 3, 4], "cd": [0.5, 0.4, 0.31, 0.3], "cl": [], "residual_iterations": [], "residuals": {}}
    })
}

fn setup(name: &str) -> (Registry, Session, PathBuf) {
    let reg = foamcraft_api::registry();
    let base = std::env::temp_dir().join(format!("fc-report-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&base);
    let mut s = Session::new(base.join("data"));
    let root = base.join("p");
    reg.execute(
        &mut s,
        "project.new",
        json!({"path": root, "name": "Helmet"}),
    )
    .unwrap();
    let model =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/data/geometry/cube_binary.stl");
    reg.execute(
        &mut s,
        "geometry.import",
        json!({"path": model, "unit": "mm"}),
    )
    .unwrap();
    std::fs::create_dir_all(root.join("results")).unwrap();
    for (job, drag) in [("job-a", 48.0), ("job-b", 43.2)] {
        std::fs::write(
            root.join("results").join(format!("{job}.json")),
            result(job, drag).to_string(),
        )
        .unwrap();
    }
    (reg, s, root)
}

fn read(path: &Path) -> String {
    std::fs::read_to_string(path).unwrap()
}

#[test]
fn a_report_in_both_languages_and_formats() {
    let (reg, mut s, root) = setup("both");
    for lang in ["en", "de"] {
        let base = root.join(format!("report-{lang}"));
        let out = reg
            .execute(
                &mut s,
                "export.report",
                json!({"path": base, "language": lang, "job_id": "job-a", "title": "Helmet", "author": "Tester"}),
            )
            .unwrap();
        let pdf = PathBuf::from(out["pdf"].as_str().unwrap());
        let html = PathBuf::from(out["html"].as_str().unwrap());
        assert_eq!(pdf.extension().unwrap(), "pdf");
        assert_eq!(&std::fs::read(&pdf).unwrap()[..5], b"%PDF-");
        let page = read(&html);
        assert!(page.contains("OpenCFD Limited"));
        assert!(page.contains("v2606"), "solver release");
        // No result fields in the cache: a hint instead of images.
        assert!(!page.contains("<img"));
        let hint = if lang == "en" {
            "No images"
        } else {
            "Keine Bilder"
        };
        assert!(page.contains(hint), "{lang}");
    }
    assert_eq!(foamcraft_runtime::OPENFOAM_RELEASE, "v2606");
}

#[test]
fn a_comparison_report_and_chosen_formats() {
    let (reg, mut s, root) = setup("compare");
    reg.execute(
        &mut s,
        "runs.rename",
        json!({"job_id": "job-b", "name": "Visor down"}),
    )
    .unwrap();
    let out = reg
        .execute(
            &mut s,
            "export.report",
            json!({"path": root.join("cmp.html"), "formats": ["html"], "job_id": "job-a", "compare_with": "job-b"}),
        )
        .unwrap();
    assert!(out["pdf"].is_null());
    let page = read(&PathBuf::from(out["html"].as_str().unwrap()));
    assert!(page.contains("Comparison report"));
    assert!(page.contains("Visor down"));
    assert!(!root.join("cmp.pdf").exists());
}
