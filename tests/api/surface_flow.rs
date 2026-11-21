// SPDX-License-Identifier: GPL-3.0-or-later
//! Flow on the surface (Task_33) on result caches written by the test: the 20 mm cube with the
//! wall shear stress vector (front half attached, back half in reverse flow) and an old cache
//! without it; settings, field list, the reverse flow figure, presets, approximation, images.

use std::path::{Path, PathBuf};

use foamcraft_core::command::{Registry, Session};
use foamcraft_sim::results::{GridSpec, Surface, grid_from_rows};
use serde_json::{Value, json};

fn run(reg: &Registry, s: &mut Session, name: &str, params: Value) -> Value {
    reg.execute(s, name, params)
        .unwrap_or_else(|e| panic!("{name}: {e}"))
}

fn cube() -> foamcraft_geometry::Mesh {
    foamcraft_geometry::read_mesh(
        &PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/data/geometry/cube_binary.stl"),
    )
    .unwrap()
}

/// Cache of `job` around the cube; with `vector` the wall shear stress points against +X on the
/// front half (attached flow) and along +X on the back half (reverse flow).
fn write_cache(root: &Path, job: &str, vector: bool) {
    let dir = root.join("cache/results").join(job);
    std::fs::create_dir_all(&dir).unwrap();
    let cube = cube();
    let points: Vec<[f32; 3]> = cube
        .positions
        .iter()
        .map(|p| p.map(|c| c / 1000.0))
        .collect();
    let n = cube.triangles.len();
    let mut fields = vec![("p".to_string(), vec![1.0f32; n])];
    if vector {
        let sx: Vec<f32> = cube
            .triangles
            .iter()
            .map(|t| {
                let cx: f32 = t.iter().map(|i| points[*i as usize][0]).sum::<f32>() / 3.0;
                if cx > 0.01 { 0.5 } else { -0.5 }
            })
            .collect();
        fields.push(("wss_x".into(), sx));
        fields.push(("wss_y".into(), vec![0.0; n]));
        fields.push(("wss_z".into(), vec![0.0; n]));
    }
    let surface = Surface {
        points,
        triangles: cube.triangles.clone(),
        fields,
    };
    std::fs::write(dir.join("surface.bin"), surface.to_bytes()).unwrap();
    let spec = GridSpec {
        min: [-0.04, -0.04, -0.03],
        max: [0.08, 0.06, 0.05],
        dims: [24, 20, 16],
    };
    let mut rows = Vec::new();
    for k in 0..spec.dims[2] {
        for j in 0..spec.dims[1] {
            for i in 0..spec.dims[0] {
                let p = spec.point(i, j, k);
                if (0..3).all(|a| p[a] > 0.0 && p[a] < 0.02) {
                    continue;
                }
                rows.push(vec![p[0], p[1], p[2], 0.0, 10.0, 0.0, 0.0, 0.5]);
            }
        }
    }
    let grid = grid_from_rows(&spec, &["p", "Ux", "Uy", "Uz", "k"], &rows);
    std::fs::write(dir.join("grid_0.bin"), grid.to_bytes()).unwrap();
    let meta = json!({
        "job_id": job,
        "fields": [
            {"name": "p", "unit": "Pa", "location": "surface", "min": 1.0, "max": 1.0},
            {"name": "p", "unit": "Pa", "location": "volume", "min": 0.0, "max": 0.0},
            {"name": "Ux", "unit": "m/s", "location": "volume", "min": 10.0, "max": 10.0}
        ],
        "levels": [{"min": spec.min, "max": spec.max, "dims": spec.dims}],
        "yaw_deg": 0.0,
        "bytes": 1,
        "colormap": []
    });
    std::fs::write(dir.join("meta.json"), meta.to_string()).unwrap();
}

fn write_result(root: &Path, job: &str) {
    std::fs::create_dir_all(root.join("results")).unwrap();
    let r = json!({
        "job_id": job, "reason": "stable", "iterations": 300, "decided_at_iteration": 290,
        "max_iterations": 900, "fluctuation_percent": 0.5,
        "coefficients": {"cd": 1.0, "cl": 0.0, "cs": 0.0, "cm_pitch": 0.0, "cm_roll": 0.0, "cm_yaw": 0.0},
        "forces": {"drag_n": 1.0, "lift_n": 0.0, "side_n": 0.0},
        "drag_split": {"pressure_n": 0.9, "viscous_n": 0.1},
        "parts": [],
        "reference": {"area_m2": 0.0004, "length_m": 0.02, "speed_m_s": 10.0, "density_kg_m3": 1.2, "dynamic_pressure_pa": 60.0},
        "y_plus": [], "hints": [], "restarted": false, "cells": 1000, "cores": 2,
        "timings_s": {"solve": 1.0},
        "scenario": {"speed_m_s": 10.0, "direction_deg": 0.0, "ground": "none", "medium": {"kind": "air", "temperature_c": 20.0}, "quality": "draft"},
        "mesh": {"job_id": "job-mesh", "cells": 1000, "verdict": "good", "layers_target": 3, "layers_achieved": 3.0, "fallbacks": []},
        "history": {"iterations": [], "cd": [], "cl": [], "residual_iterations": [], "residuals": {}}
    });
    std::fs::write(
        root.join("results").join(format!("{job}.json")),
        r.to_string(),
    )
    .unwrap();
}

fn setup(name: &str) -> (Registry, Session, PathBuf) {
    let reg = foamcraft_api::registry();
    let base = std::env::temp_dir().join(format!("fc-sflow-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&base);
    let mut s = Session::new(base.join("data"));
    let root = base.join("p");
    run(
        &reg,
        &mut s,
        "project.new",
        json!({"path": root, "name": "S"}),
    );
    let model =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/data/geometry/cube_binary.stl");
    run(
        &reg,
        &mut s,
        "geometry.import",
        json!({"path": model, "unit": "mm"}),
    );
    write_cache(&root, "job-vec", true);
    write_cache(&root, "job-old", false);
    write_result(&root, "job-vec");
    run(
        &reg,
        &mut s,
        "view.set_surface_field",
        json!({"field": "p", "job_id": "job-vec"}),
    );
    (reg, s, root)
}

#[test]
fn the_vector_stays_out_of_the_field_list_and_gives_the_attached_map() {
    let (reg, mut s, _) = setup("fields");
    let meta = run(
        &reg,
        &mut s,
        "results.list_fields",
        json!({"job_id": "job-vec"}),
    );
    let names: Vec<&str> = meta["fields"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|f| f["location"] == "surface")
        .map(|f| f["name"].as_str().unwrap())
        .collect();
    assert!(names.contains(&"attached"), "{names:?}");
    run(
        &reg,
        &mut s,
        "view.set_surface_field",
        json!({"field": "attached", "job_id": "job-vec"}),
    );
    assert!(!names.iter().any(|n| n.starts_with("wss_")), "{names:?}");
    // The probe reads the map: +1 in front, -1 behind on a side face.
    let front = run(
        &reg,
        &mut s,
        "results.probe",
        json!({"point": [0.004, 0.0, 0.01], "field": "attached"}),
    );
    let back = run(
        &reg,
        &mut s,
        "results.probe",
        json!({"point": [0.016, 0.0, 0.01], "field": "attached"}),
    );
    assert!(
        (front["value"].as_f64().unwrap() - 1.0).abs() < 1e-6
            || (back["value"].as_f64().unwrap() + 1.0).abs() < 1e-6,
        "{front} {back}"
    );
    // Old cache: no map.
    let old = run(
        &reg,
        &mut s,
        "results.list_fields",
        json!({"job_id": "job-old"}),
    );
    assert!(
        !old["fields"]
            .as_array()
            .unwrap()
            .iter()
            .any(|f| f["name"] == "attached")
    );
}

#[test]
fn the_summary_reports_the_surface_in_reverse_flow() {
    let (reg, mut s, _) = setup("figure");
    let sum = run(
        &reg,
        &mut s,
        "results.summary",
        json!({"job_id": "job-vec"}),
    );
    let fig = sum["figures"]
        .as_array()
        .unwrap()
        .iter()
        .find(|f| f["figure"]["id"] == "reverse_flow_area" || f["id"] == "reverse_flow_area")
        .unwrap_or_else(|| panic!("{sum}"));
    let value = fig["figure"]["value"]
        .as_f64()
        .or(fig["value"].as_f64())
        .unwrap();
    // Of the 12 equal triangles, the 4 side triangles behind the middle run against the flow
    // (front and back faces are across the flow: neither attached nor reverse).
    assert!((value - 100.0 / 3.0).abs() < 1e-3, "{value}");
}

#[test]
fn settings_are_stored_validated_and_migrated() {
    let (reg, mut s, root) = setup("store");
    let out = run(
        &reg,
        &mut s,
        "view.set_surface_flow",
        json!({"lines": {"on": true, "density": 0.02, "color_by": "wall_shear_stress"},
               "tufts": {"on": true, "color_by": "state"}}),
    );
    assert_eq!(out["approximate"], false);
    assert!(out["lines"].as_u64().unwrap() > 0, "{out}");
    assert!(out["tufts"].as_u64().unwrap() > 0, "{out}");
    run(&reg, &mut s, "project.close", json!({}));
    run(&reg, &mut s, "project.open", json!({"path": root}));
    let info = run(&reg, &mut s, "view.surface_flow_info", json!({}));
    assert_eq!(
        info["surface_flow"]["lines"]["color_by"],
        "wall_shear_stress"
    );
    assert_eq!(info["surface_flow"]["tufts"]["color_by"], "state");
    // Hiding the lines keeps the tufts.
    let out = run(
        &reg,
        &mut s,
        "view.set_surface_flow",
        json!({"lines": {"on": false}}),
    );
    assert!(out["surface_flow"]["lines"].is_null());
    assert!(!out["surface_flow"]["tufts"].is_null());
    let err = reg
        .execute(
            &mut s,
            "view.set_surface_flow",
            json!({"lines": {"on": true, "color_by": "rainbow"}}),
        )
        .unwrap_err();
    assert_eq!(err.code, "view.invalid_surface_flow");
    run(&reg, &mut s, "view.surface_flow_off", json!({}));
    let info = run(&reg, &mut s, "view.surface_flow_info", json!({}));
    assert!(info["surface_flow"].is_null());
    // A manifest of version 14 opens without surface flow.
    run(&reg, &mut s, "project.close", json!({}));
    let file = root.join("foamcraft.json");
    let mut doc: Value = serde_json::from_str(&std::fs::read_to_string(&file).unwrap()).unwrap();
    doc["schema_version"] = json!(14);
    doc["view"].as_object_mut().unwrap().remove("surface_flow");
    std::fs::write(&file, doc.to_string()).unwrap();
    let info = run(&reg, &mut s, "project.open", json!({"path": root}));
    assert_eq!(
        info["schema_version"],
        foamcraft_core::project::SCHEMA_VERSION
    );
}

#[test]
fn an_old_run_without_the_vector_is_approximate() {
    let (reg, mut s, _) = setup("old");
    run(
        &reg,
        &mut s,
        "view.set_surface_field",
        json!({"field": "p", "job_id": "job-old"}),
    );
    let out = run(
        &reg,
        &mut s,
        "view.set_surface_flow",
        json!({"lines": {"on": true}}),
    );
    assert_eq!(out["approximate"], true);
    assert!(out["lines"].as_u64().unwrap() > 0, "{out}");
}

#[test]
fn the_oil_film_and_tufts_presets_build_valid_scenes() {
    let (reg, mut s, _) = setup("presets");
    let ids: Vec<String> = run(&reg, &mut s, "view.presets", json!({}))
        .as_array()
        .unwrap()
        .iter()
        .map(|p| p["id"].as_str().unwrap().to_string())
        .collect();
    assert!(ids.contains(&"oil_film".into()) && ids.contains(&"tufts".into()));
    let v = run(&reg, &mut s, "view.apply_preset", json!({"id": "oil_film"}));
    assert!(!v["surface_flow"]["lines"].is_null());
    assert!(v["surface_field"].is_null());
    let v = run(&reg, &mut s, "view.apply_preset", json!({"id": "tufts"}));
    assert!(!v["surface_flow"]["tufts"].is_null());
    assert_eq!(v["surface_field"], "attached");
    let texts = run(
        &reg,
        &mut s,
        "error.describe",
        json!({"codes": ["preset.oil_film", "preset.tufts"]}),
    );
    assert!(texts["preset.oil_film"]["de"]["text"].is_string());
}

fn gpu() -> bool {
    if foamcraft_render::Renderer::new_headless().is_none() {
        assert!(
            std::env::var_os("FOAMCRAFT_REQUIRE_GPU").is_none(),
            "no GPU adapter"
        );
        eprintln!("SKIPPED: no GPU adapter available");
        return false;
    }
    true
}

#[test]
fn images_show_the_lines_and_videos_repeat() {
    if !gpu() {
        return;
    }
    let (reg, mut s, root) = setup("images");
    let plain = root.join("plain.png");
    run(
        &reg,
        &mut s,
        "view.render",
        json!({"path": plain, "width": 240, "height": 160}),
    );
    run(
        &reg,
        &mut s,
        "view.set_surface_flow",
        json!({"lines": {"on": true}, "tufts": {"on": true}}),
    );
    let with = root.join("with.png");
    run(
        &reg,
        &mut s,
        "view.render",
        json!({"path": with, "width": 240, "height": 160}),
    );
    assert_ne!(
        std::fs::read(&plain).unwrap(),
        std::fs::read(&with).unwrap()
    );
    let video = |p: &Path, s: &mut Session| {
        run(
            &reg,
            s,
            "export.video",
            json!({"path": p, "width": 96, "height": 64, "fps": 10, "duration_s": 1.0,
                   "camera": {"preset": "still"}, "wait": true}),
        )
    };
    let (a, b) = (root.join("a.avi"), root.join("b.avi"));
    assert_eq!(video(&a, &mut s)["state"], "done");
    video(&b, &mut s);
    assert_eq!(std::fs::read(&a).unwrap(), std::fs::read(&b).unwrap());
}
