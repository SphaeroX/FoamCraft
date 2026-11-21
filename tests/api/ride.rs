// SPDX-License-Identifier: GPL-3.0-or-later
//! ride.* commands (Task_32) on a result cache written by the test: potential flow around the
//! 20 mm cube, suggestions, start, telemetry, storage, migration, errors, still and video.

use std::path::{Path, PathBuf};

use foamcraft_core::command::{Registry, Session};
use foamcraft_sim::results::{GridSpec, Surface, grid_from_rows};
use serde_json::{Value, json};

fn run(reg: &Registry, s: &mut Session, name: &str, params: Value) -> Value {
    reg.execute(s, name, params)
        .unwrap_or_else(|e| panic!("{name}: {e}"))
}

const C: [f64; 3] = [0.01, 0.01, 0.01];
const R: f64 = 0.0125;
const U: f64 = 10.0;

/// Potential flow around a sphere of radius R at the cube centre; inside the cube or the sphere
/// is solid.
fn flow(p: [f64; 3]) -> Option<([f64; 3], f64)> {
    if (0..3).all(|a| p[a] > 0.0 && p[a] < 0.02) {
        return None;
    }
    let d = [p[0] - C[0], p[1] - C[1], p[2] - C[2]];
    let r = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt();
    if r < R {
        return None;
    }
    let r3 = (R / r).powi(3);
    let k = 1.5 * r3 * d[0] / (r * r);
    let u = [
        U * (1.0 + 0.5 * r3) - U * k * d[0],
        -U * k * d[1],
        -U * k * d[2],
    ];
    let p = 0.5 * 1.2 * (U * U - (u[0] * u[0] + u[1] * u[1] + u[2] * u[2]));
    Some((u, p))
}

fn level(dir: &Path, n: usize, spec: &GridSpec) {
    let mut rows = Vec::new();
    for k in 0..spec.dims[2] {
        for j in 0..spec.dims[1] {
            for i in 0..spec.dims[0] {
                let p = spec.point(i, j, k);
                if let Some((u, pr)) = flow(p) {
                    rows.push(vec![p[0], p[1], p[2], pr, u[0], u[1], u[2]]);
                }
            }
        }
    }
    let grid = grid_from_rows(spec, &["p", "Ux", "Uy", "Uz"], &rows);
    std::fs::write(dir.join(format!("grid_{n}.bin")), grid.to_bytes()).unwrap();
}

fn write_cache(root: &Path) {
    let dir = root.join("cache/results/job-ride");
    std::fs::create_dir_all(&dir).unwrap();
    let cube = foamcraft_geometry::read_mesh(
        &PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/data/geometry/cube_binary.stl"),
    )
    .unwrap();
    let n = cube.triangles.len();
    let surface = Surface {
        points: cube
            .positions
            .iter()
            .map(|p| p.map(|c| c / 1000.0))
            .collect(),
        triangles: cube.triangles.clone(),
        fields: vec![("p".into(), vec![0.0; n])],
    };
    std::fs::write(dir.join("surface.bin"), surface.to_bytes()).unwrap();
    let coarse = GridSpec {
        min: [-0.06, -0.04, -0.04],
        max: [0.1, 0.06, 0.06],
        dims: [48, 30, 30],
    };
    let fine = GridSpec {
        min: [-0.015, -0.015, -0.015],
        max: [0.035, 0.035, 0.035],
        dims: [40, 40, 40],
    };
    level(&dir, 0, &coarse);
    level(&dir, 1, &fine);
    let meta = json!({
        "job_id": "job-ride",
        "fields": [
            {"name": "p", "unit": "Pa", "location": "surface", "min": 0.0, "max": 1.0},
            {"name": "p", "unit": "Pa", "location": "volume", "min": -100.0, "max": 60.0},
            {"name": "Ux", "unit": "m/s", "location": "volume", "min": 0.0, "max": 15.0},
            {"name": "Uy", "unit": "m/s", "location": "volume", "min": -5.0, "max": 5.0},
            {"name": "Uz", "unit": "m/s", "location": "volume", "min": -5.0, "max": 5.0}
        ],
        "levels": [
            {"min": coarse.min, "max": coarse.max, "dims": coarse.dims},
            {"min": fine.min, "max": fine.max, "dims": fine.dims}
        ],
        "yaw_deg": 0.0,
        "bytes": 1,
        "colormap": []
    });
    std::fs::write(dir.join("meta.json"), meta.to_string()).unwrap();
}

fn setup(name: &str) -> (Registry, Session, PathBuf) {
    let reg = foamcraft_api::registry();
    let base = std::env::temp_dir().join(format!("fc-ride-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&base);
    let mut s = Session::new(base.join("data"));
    let root = base.join("p");
    run(
        &reg,
        &mut s,
        "project.new",
        json!({"path": root, "name": "R"}),
    );
    let model =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/data/geometry/cube_binary.stl");
    run(
        &reg,
        &mut s,
        "geometry.import",
        json!({"path": model, "unit": "mm"}),
    );
    write_cache(&root);
    run(
        &reg,
        &mut s,
        "view.set_surface_field",
        json!({"field": "p", "job_id": "job-ride"}),
    );
    (reg, s, root)
}

#[test]
fn suggestions_start_a_ride_with_readings_and_events() {
    let (reg, mut s, _) = setup("start");
    let list = run(&reg, &mut s, "ride.suggest", json!({}));
    let ids: Vec<&str> = list
        .as_array()
        .unwrap()
        .iter()
        .map(|x| x["id"].as_str().unwrap())
        .collect();
    assert!(ids.contains(&"stagnation"), "{list}");
    assert!(ids.len() <= 5);
    assert_eq!(list[0]["title"], format!("ride.suggestion.{}", ids[0]));
    // Same seed, same list.
    assert_eq!(run(&reg, &mut s, "ride.suggest", json!({})), list);
    let info = run(
        &reg,
        &mut s,
        "ride.start",
        json!({"suggestion": "stagnation"}),
    );
    assert_eq!(info["ride"]["mode"], "calm");
    assert_eq!(info["ride"]["view"], "first_person");
    let rows = info["rows"].as_array().unwrap();
    assert!(rows.len() > 10 && rows.len() <= 512);
    assert_eq!(info["columns"][2], "speed");
    // Time runs forward; the speed falls towards the stagnation point.
    let t: Vec<f64> = rows.iter().map(|r| r[0].as_f64().unwrap()).collect();
    assert!(t.windows(2).all(|w| w[1] > w[0]));
    let slowest = rows
        .iter()
        .map(|r| r[2].as_f64().unwrap())
        .fold(f64::MAX, f64::min);
    // The stagnation ride passes the model: on this coarse grid it slows to about half.
    assert!(slowest < 0.6 * U, "{slowest}");
    let events: Vec<&str> = info["events"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| e["id"].as_str().unwrap())
        .collect();
    // It meets the model (slowing down in front of it or gliding along it) and calms down after.
    assert!(
        events
            .iter()
            .any(|e| ["approach", "stagnation", "near_wall"].contains(e)),
        "{events:?}"
    );
    assert_eq!(events.last(), Some(&"calm"), "{events:?}");
    assert!((info["reference_speed_m_s"].as_f64().unwrap() - U).abs() < 1.0);
    // Telemetry repeats the stored ride.
    let again = run(&reg, &mut s, "ride.telemetry", json!({}));
    assert_eq!(again["rows"], info["rows"]);
}

#[test]
fn the_ride_is_stored_and_comes_back_after_opening() {
    let (reg, mut s, root) = setup("store");
    run(
        &reg,
        &mut s,
        "ride.start",
        json!({"from": [-0.03, 0.012, 0.011], "mode": "coaster", "view": "chase", "speed": 2.0}),
    );
    run(&reg, &mut s, "project.close", json!({}));
    run(&reg, &mut s, "project.open", json!({"path": root}));
    let info = run(&reg, &mut s, "ride.telemetry", json!({}));
    assert_eq!(info["ride"]["start"], json!([-0.03, 0.012, 0.011]));
    assert_eq!(info["ride"]["mode"], "coaster");
    assert_eq!(info["ride"]["view"], "chase");
    assert_eq!(info["ride"]["speed"], 2.0);
    // Switching the mode keeps the start point.
    let calm = run(&reg, &mut s, "ride.start", json!({"mode": "calm"}));
    assert_eq!(calm["ride"]["start"], json!([-0.03, 0.012, 0.011]));
    assert_eq!(calm["ride"]["view"], "chase");
    run(&reg, &mut s, "ride.stop", json!({}));
    let err = reg
        .execute(&mut s, "ride.telemetry", json!({}))
        .unwrap_err();
    assert_eq!(err.code, "ride.none");
}

#[test]
fn a_manifest_of_the_previous_version_opens_without_a_ride() {
    let (reg, mut s, root) = setup("migrate");
    run(&reg, &mut s, "project.close", json!({}));
    let file = root.join("foamcraft.json");
    let mut doc: Value = serde_json::from_str(&std::fs::read_to_string(&file).unwrap()).unwrap();
    doc["schema_version"] = json!(13);
    doc["view"].as_object_mut().unwrap().remove("ride");
    std::fs::write(&file, doc.to_string()).unwrap();
    let info = run(&reg, &mut s, "project.open", json!({"path": root}));
    assert_eq!(
        info["schema_version"],
        foamcraft_core::project::SCHEMA_VERSION
    );
    let err = reg
        .execute(&mut s, "ride.telemetry", json!({}))
        .unwrap_err();
    assert_eq!(err.code, "ride.none");
}

#[test]
fn bad_starts_give_clear_errors() {
    let (reg, mut s, _) = setup("errors");
    let code = |s: &mut Session, p: Value| reg.execute(s, "ride.start", p).unwrap_err().code;
    assert_eq!(
        code(&mut s, json!({"from": [0.01, 0.01, 0.01]})),
        "ride.no_path"
    );
    assert_eq!(
        code(&mut s, json!({"from": [5.0, 0.0, 0.0]})),
        "ride.no_path"
    );
    assert_eq!(
        code(&mut s, json!({"suggestion": "nonsense"})),
        "ride.unknown_suggestion"
    );
    assert_eq!(
        code(&mut s, json!({"from": [-0.03, 0.0, 0.0], "mode": "wild"})),
        "ride.invalid_option"
    );
    assert_eq!(code(&mut s, json!({})), "ride.no_path");
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
fn a_still_from_the_ride_differs_from_the_standard_view() {
    if !gpu() {
        return;
    }
    let (reg, mut s, root) = setup("still");
    run(
        &reg,
        &mut s,
        "ride.start",
        json!({"from": [-0.03, 0.004, 0.006]}),
    );
    let a = root.join("ride.png");
    let b = root.join("plain.png");
    run(
        &reg,
        &mut s,
        "view.render",
        json!({"path": a, "width": 320, "height": 200, "ride_t": 1.0}),
    );
    run(
        &reg,
        &mut s,
        "view.render",
        json!({"path": b, "width": 320, "height": 200}),
    );
    assert_ne!(std::fs::read(&a).unwrap(), std::fs::read(&b).unwrap());
    let img = root.join("export.png");
    let out = run(
        &reg,
        &mut s,
        "export.image",
        json!({"path": img, "width": 320, "height": 200, "ride_t": 1.0, "legend": false}),
    );
    assert_eq!(out["width"], 320);
}

#[test]
fn a_video_of_the_ride_repeats_exactly() {
    if !gpu() {
        return;
    }
    let (reg, mut s, root) = setup("video");
    run(
        &reg,
        &mut s,
        "view.set_particles",
        json!({"preset": "wind_tunnel_smoke", "count": 2000}),
    );
    run(
        &reg,
        &mut s,
        "ride.start",
        json!({"from": [-0.03, 0.004, 0.006], "view": "chase"}),
    );
    let video = |path: &Path, s: &mut Session| {
        run(
            &reg,
            s,
            "export.video",
            json!({"path": path, "width": 160, "height": 96, "fps": 10, "duration_s": 1.5,
                   "camera": {"preset": "ride"}, "wait": true}),
        )
    };
    let (a, b) = (root.join("a.avi"), root.join("b.avi"));
    let out = video(&a, &mut s);
    assert_eq!(out["state"], "done", "{out}");
    assert_eq!(out["frames"], 15);
    video(&b, &mut s);
    assert_eq!(std::fs::read(&a).unwrap(), std::fs::read(&b).unwrap());
    // Without a duration the video lasts as long as the ride.
    let info = run(&reg, &mut s, "ride.telemetry", json!({}));
    let full = run(
        &reg,
        &mut s,
        "export.video",
        json!({"path": root.join("c.avi"), "width": 64, "height": 48, "fps": 2,
               "camera": {"preset": "ride"}, "wait": true}),
    );
    let want = (info["duration_s"].as_f64().unwrap() * 2.0).round() as u64;
    assert_eq!(full["frames"].as_u64().unwrap(), want.max(1));
}

#[test]
fn a_new_start_counts_up_and_a_change_of_settings_does_not() {
    let (reg, mut s, _) = setup("run");
    let a = run(
        &reg,
        &mut s,
        "ride.start",
        json!({"from": [-0.03, 0.012, 0.011]}),
    );
    let b = run(&reg, &mut s, "ride.start", json!({"mode": "coaster"}));
    assert_eq!(a["ride"]["run"], b["ride"]["run"]);
    let c = run(
        &reg,
        &mut s,
        "ride.start",
        json!({"from": [-0.03, 0.012, 0.011]}),
    );
    assert_eq!(
        c["ride"]["run"].as_u64().unwrap(),
        b["ride"]["run"].as_u64().unwrap() + 1
    );
}
