// SPDX-License-Identifier: GPL-3.0-or-later
//! results.summary, results.compare, results.export and runs.* on results written by the test.

use std::path::{Path, PathBuf};

use foamcraft_core::command::{Registry, Session};
use serde_json::{Value, json};

fn run(reg: &Registry, s: &mut Session, name: &str, params: Value) -> Value {
    reg.execute(s, name, params)
        .unwrap_or_else(|e| panic!("{name}: {e}"))
}

fn result(job: &str, cd: f64, drag: f64) -> Value {
    json!({
        "job_id": job,
        "reason": "stable",
        "iterations": 300,
        "decided_at_iteration": 290,
        "max_iterations": 900,
        "fluctuation_percent": 0.5,
        "coefficients": {"cd": cd, "cl": 0.1, "cs": 0.0, "cm_pitch": 0.0, "cm_roll": 0.0, "cm_yaw": 0.0},
        "forces": {"drag_n": drag, "lift_n": 1.0, "side_n": 0.0},
        "drag_split": {"pressure_n": drag * 0.9, "viscous_n": drag * 0.1},
        "parts": [],
        "reference": {"area_m2": 0.5, "length_m": 1.0, "speed_m_s": 20.0, "density_kg_m3": 1.2, "dynamic_pressure_pa": 240.0},
        "y_plus": [{"patch": "model", "min": 1.0, "max": 90.0, "average": 45.0}],
        "hints": ["run.stable"],
        "restarted": false,
        "cells": 1000,
        "cores": 2,
        "timings_s": {"solve": 10.0},
        "scenario": {"speed_m_s": 20.0, "direction_deg": 0.0, "ground": "fixed", "medium": {"kind": "air", "temperature_c": 20.0}, "quality": "draft"},
        "mesh": {"job_id": "job-mesh", "cells": 1000, "verdict": "good", "layers_target": 3, "layers_achieved": 3.0, "fallbacks": []},
        "history": {"iterations": [], "cd": [], "cl": [], "residual_iterations": [], "residuals": {}}
    })
}

fn write_run(root: &Path, job: &str, cd: f64, drag: f64) {
    std::fs::create_dir_all(root.join("results")).unwrap();
    std::fs::write(
        root.join("results").join(format!("{job}.json")),
        result(job, cd, drag).to_string(),
    )
    .unwrap();
    let dir = root.join("runs").join(job);
    std::fs::create_dir_all(dir.join("case")).unwrap();
    std::fs::write(dir.join("case/big.bin"), vec![0u8; 100_000]).unwrap();
    std::fs::write(
        dir.join("job.json"),
        json!({"id": job, "case": "wind_tunnel_solve", "state": {"state": "succeeded"}, "cores": 2, "cells": 1000,
               "offsets": {}, "container": null, "volume": null, "created": 1}).to_string(),
    )
    .unwrap();
    let cache = root.join("cache/results").join(job);
    std::fs::create_dir_all(&cache).unwrap();
    std::fs::write(cache.join("surface.bin"), vec![0u8; 50_000]).unwrap();
}

fn setup(name: &str) -> (Registry, Session, PathBuf) {
    let reg = foamcraft_api::registry();
    let base = std::env::temp_dir().join(format!("fc-runs-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&base);
    let mut s = Session::new(base.join("data"));
    let root = base.join("p");
    run(
        &reg,
        &mut s,
        "project.new",
        json!({"path": root, "name": "Runs"}),
    );
    write_run(&root, "job-a", 0.30, 48.0);
    write_run(&root, "job-b", 0.27, 43.2);
    (reg, s, root)
}

#[test]
fn the_summary_lists_figures_with_units_texts_and_a_light() {
    let (reg, mut s, _) = setup("summary");
    let sum = run(&reg, &mut s, "results.summary", json!({"job_id": "job-a"}));
    assert_eq!(sum["schema"], 1);
    assert_eq!(sum["kind"], "wind_tunnel");
    let drag_area = sum["figures"]
        .as_array()
        .unwrap()
        .iter()
        .find(|f| f["id"] == "drag_area_m2")
        .unwrap();
    assert!((drag_area["value"].as_f64().unwrap() - 0.15).abs() < 1e-12);
    assert_eq!(drag_area["unit"], "m²");
    for f in sum["figures"].as_array().unwrap() {
        assert!(!f["en"]["text"].as_str().unwrap().is_empty(), "{f}");
        assert!(!f["de"]["text"].as_str().unwrap().is_empty(), "{f}");
    }
    assert_eq!(sum["reliability"]["level"], "green");
    assert!(
        !sum["reliability"]["checks"][0]["de"]["text"]
            .as_str()
            .unwrap()
            .is_empty()
    );
}

#[test]
fn two_runs_are_compared_and_exported() {
    let (reg, mut s, root) = setup("compare");
    let cmp = run(
        &reg,
        &mut s,
        "results.compare",
        json!({"a": "job-a", "b": "job-b"}),
    );
    let row = cmp["rows"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["id"] == "drag_n" && r["part"].is_null())
        .unwrap();
    assert_eq!(row["a"], 48.0);
    assert!((row["difference"].as_f64().unwrap() + 4.8).abs() < 1e-9);
    assert!((row["relative"].as_f64().unwrap() + 0.1).abs() < 1e-9);
    let csv = root.join("figures.csv");
    run(
        &reg,
        &mut s,
        "results.export",
        json!({"job_id": "job-a", "path": csv, "format": "csv"}),
    );
    let text = std::fs::read_to_string(&csv).unwrap();
    assert!(text.starts_with("id,part,value,unit"));
    assert!(text.contains("drag_n,,48,N"));
    let json_path = root.join("figures.json");
    run(
        &reg,
        &mut s,
        "results.export",
        json!({"job_id": "job-a", "path": json_path, "format": "json"}),
    );
    let back: Value = serde_json::from_str(&std::fs::read_to_string(&json_path).unwrap()).unwrap();
    assert_eq!(back["job_id"], "job-a");
}

#[test]
fn runs_are_listed_renamed_and_deleted_with_their_files() {
    let (reg, mut s, root) = setup("manage");
    let list = run(&reg, &mut s, "runs.list", json!({}));
    assert_eq!(list.as_array().unwrap().len(), 2);
    assert!(list[0]["bytes"].as_u64().unwrap() >= 150_000);
    run(
        &reg,
        &mut s,
        "runs.rename",
        json!({"job_id": "job-b", "name": "Spoiler flach"}),
    );
    let list = run(&reg, &mut s, "runs.list", json!({}));
    assert!(
        list.as_array()
            .unwrap()
            .iter()
            .any(|r| r["name"] == "Spoiler flach")
    );
    let deleted = run(&reg, &mut s, "runs.delete", json!({"job_id": "job-a"}));
    assert!(deleted["freed_bytes"].as_u64().unwrap() >= 150_000);
    assert!(!root.join("runs/job-a").exists());
    assert!(!root.join("results/job-a.json").exists());
    assert!(!root.join("cache/results/job-a").exists());
    let list = run(&reg, &mut s, "runs.list", json!({}));
    assert_eq!(list.as_array().unwrap().len(), 1);
    let gone = reg
        .execute(&mut s, "results.summary", json!({"job_id": "job-a"}))
        .unwrap_err();
    assert_eq!(gone.code, "run.no_result");
    // The names survive closing and opening.
    run(&reg, &mut s, "project.close", json!({}));
    run(&reg, &mut s, "project.open", json!({"path": root}));
    assert_eq!(
        run(&reg, &mut s, "runs.list", json!({}))[0]["name"],
        "Spoiler flach"
    );
}

#[test]
fn a_variant_is_a_copy_of_the_project_without_runs() {
    let (reg, mut s, root) = setup("variant");
    let out = run(
        &reg,
        &mut s,
        "runs.duplicate",
        json!({"name": "Runs Variante"}),
    );
    let copy = PathBuf::from(out["path"].as_str().unwrap());
    assert!(copy.join("foamcraft.json").is_file());
    assert!(!copy.join("runs/job-a").exists());
    assert!(!copy.join("results").join("job-a.json").exists());
    assert_ne!(copy, root);
    run(&reg, &mut s, "project.open", json!({"path": copy}));
    let info = run(&reg, &mut s, "project.info", json!({}));
    assert_eq!(info["name"], "Runs Variante");
    assert_eq!(info["runs"], json!([]));
}

// --- Task_30: comparing several runs ---

#[test]
fn several_runs_are_compared_against_a_reference() {
    let (reg, mut s, root) = setup("compare-many");
    write_run(&root, "job-c", 0.33, 52.8);
    let cmp = run(
        &reg,
        &mut s,
        "results.compare_many",
        json!({"jobs": ["job-a", "job-b", "job-c"], "reference": "job-a"}),
    );
    assert_eq!(cmp["jobs"], json!(["job-a", "job-b", "job-c"]));
    assert_eq!(cmp["names"].as_array().unwrap().len(), 3);
    let row = cmp["rows"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["id"] == "drag_n" && r["part"].is_null())
        .unwrap();
    assert_eq!(row["values"], json!([48.0, 43.2, 52.8]));
    let rel = row["relative"].as_array().unwrap();
    assert!(rel[0].as_f64().unwrap().abs() < 1e-12);
    assert!((rel[1].as_f64().unwrap() + 0.1).abs() < 1e-9);
    assert!((rel[2].as_f64().unwrap() - 0.1).abs() < 1e-9);
    // Without a reference the first run is the reference.
    let cmp = run(
        &reg,
        &mut s,
        "results.compare_many",
        json!({"jobs": ["job-b", "job-a"]}),
    );
    assert_eq!(cmp["reference"], "job-b");
    let err = reg
        .execute(&mut s, "results.compare_many", json!({"jobs": ["job-a"]}))
        .unwrap_err();
    assert_eq!(err.code, "results.compare_needs_two");
}
