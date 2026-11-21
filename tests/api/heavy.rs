// SPDX-License-Identifier: GPL-3.0-or-later
//! Heavy particles (Task_34) on a result cache written by the test: the 20 mm cube in a uniform
//! flow of 10 m/s along x (points inside the cube masked). Materials, settings, the impact map
//! with its figures and cache, cancelling, and a ride on a raindrop that hits the cube.

use std::path::{Path, PathBuf};

use foamcraft_core::command::{Registry, Session};
use foamcraft_sim::results::{GridSpec, Surface, grid_from_rows};
use serde_json::{Value, json};

fn run(reg: &Registry, s: &mut Session, name: &str, params: Value) -> Value {
    reg.execute(s, name, params)
        .unwrap_or_else(|e| panic!("{name}: {e}"))
}

fn write_cache(root: &Path) {
    let job = "job-h";
    let dir = root.join("cache/results").join(job);
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
        fields: vec![("p".into(), vec![1.0; n])],
    };
    std::fs::write(dir.join("surface.bin"), surface.to_bytes()).unwrap();
    let spec = GridSpec {
        min: [-0.04, -0.03, -0.03],
        max: [0.08, 0.05, 0.05],
        dims: [48, 32, 32],
    };
    let mut rows = Vec::new();
    for k in 0..spec.dims[2] {
        for j in 0..spec.dims[1] {
            for i in 0..spec.dims[0] {
                let p = spec.point(i, j, k);
                if (0..3).all(|a| p[a] > 0.0 && p[a] < 0.02) {
                    continue;
                }
                rows.push(vec![p[0], p[1], p[2], 0.0, 10.0, 0.0, 0.0]);
            }
        }
    }
    let grid = grid_from_rows(&spec, &["p", "Ux", "Uy", "Uz"], &rows);
    std::fs::write(dir.join("grid_0.bin"), grid.to_bytes()).unwrap();
    let meta = json!({
        "job_id": job,
        "fields": [
            {"name": "p", "unit": "Pa", "location": "surface", "min": 1.0, "max": 1.0},
            {"name": "Ux", "unit": "m/s", "location": "volume", "min": 10.0, "max": 10.0}
        ],
        "levels": [{"min": spec.min, "max": spec.max, "dims": spec.dims}],
        "yaw_deg": 0.0, "bytes": 1, "colormap": []
    });
    std::fs::write(dir.join("meta.json"), meta.to_string()).unwrap();
    std::fs::create_dir_all(root.join("results")).unwrap();
    let r = json!({
        "job_id": job,
        "reference": {"area_m2": 0.0004, "length_m": 0.02, "speed_m_s": 10.0, "density_kg_m3": 1.2, "dynamic_pressure_pa": 60.0},
        "scenario": {"speed_m_s": 10.0, "direction_deg": 0.0, "ground": "none", "medium": {"kind": "air", "temperature_c": 20.0}, "quality": "draft"}
    });
    std::fs::write(root.join("results/job-h.json"), r.to_string()).unwrap();
}

fn setup(name: &str) -> (Registry, Session, PathBuf) {
    let reg = foamcraft_api::registry();
    let base = std::env::temp_dir().join(format!("fc-heavy-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&base);
    let mut s = Session::new(base.join("data"));
    let root = base.join("p");
    run(
        &reg,
        &mut s,
        "project.new",
        json!({"path": root, "name": "H"}),
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
        json!({"field": "p", "job_id": "job-h"}),
    );
    (reg, s, root)
}

#[test]
fn materials_come_with_their_stokes_number() {
    let (reg, mut s, _) = setup("materials");
    let list = run(&reg, &mut s, "particles.materials", json!({}));
    let list = list.as_array().unwrap();
    assert_eq!(list.len(), 5);
    let st = |id: &str| {
        list.iter().find(|m| m["id"] == id).unwrap()["stokes"]
            .as_f64()
            .unwrap()
    };
    // Rain hardly notices the flow around a 20 mm cube, fine dust follows it.
    assert!(st("rain") > 2.0, "{}", st("rain"));
    assert!(st("fine_dust") < st("spray"));
    let dust = list.iter().find(|m| m["id"] == "fine_dust").unwrap();
    assert_eq!(dust["sentence"], "particles.stokes.middle");
    assert_eq!(
        list.iter().find(|m| m["id"] == "rain").unwrap()["sentence"],
        "particles.stokes.high"
    );
}

#[test]
fn inertia_is_stored_checked_and_switched_off() {
    let (reg, mut s, root) = setup("settings");
    let v = run(
        &reg,
        &mut s,
        "view.set_particles",
        json!({"preset": "wind_tunnel_smoke", "inertia": {"material": "sand"}, "color_by": "slip"}),
    );
    assert_eq!(v["particles"]["inertia"]["material"], "sand");
    assert_eq!(v["particles"]["inertia"]["on_impact"], "bounce");
    assert_eq!(v["particles"]["color_by"], "slip");
    run(&reg, &mut s, "project.close", json!({}));
    run(&reg, &mut s, "project.open", json!({"path": root}));
    let state = run(&reg, &mut s, "view.state", json!({}));
    assert_eq!(state["particles"]["inertia"]["density_kg_m3"], 2650.0);
    let code =
        |s: &mut Session, p: Value| reg.execute(s, "view.set_particles", p).unwrap_err().code;
    assert_eq!(
        code(&mut s, json!({"inertia": {"density_kg_m3": 1.0}})),
        "particles.too_light"
    );
    assert_eq!(
        code(&mut s, json!({"inertia": {"material": "lava"}})),
        "particles.invalid_material"
    );
    let v = run(&reg, &mut s, "view.set_particles", json!({"inertia": null}));
    assert!(v["particles"]["inertia"].is_null());
    assert_eq!(v["particles"]["color_by"], "speed");
}

fn impact(reg: &Registry, s: &mut Session, wait: bool, count: u32) -> Value {
    run(
        reg,
        s,
        "results.impact",
        json!({"inertia": {"material": "rain", "gravity": false}, "count": count, "seed": 3, "wait": wait}),
    )
}

#[test]
fn the_impact_map_counts_the_hits_and_comes_from_the_cache() {
    let (reg, mut s, root) = setup("map");
    let out = impact(&reg, &mut s, true, 3000);
    assert_eq!(out["cached"], false);
    let map = &out["status"]["map"];
    let hits = map["hits"].as_f64().unwrap();
    assert!(hits > 100.0, "{out}");
    // Rain flies straight: all particles aimed at the frontal area hit it.
    let share = map["share_percent"].as_f64().unwrap();
    let unc = map["uncertainty_percent"].as_f64().unwrap();
    assert!((share - 100.0).abs() <= 3.0 * unc + 2.0, "{share} ± {unc}");
    // The impact field: its sum over the surface gives back the number of hits.
    let surface = foamcraft_api::results::load_surface(&root, "job-h").unwrap();
    let field = surface.field("impact").unwrap();
    let total_area: f64 = surface
        .triangles
        .iter()
        .map(|t| {
            let [a, b, c] = t.map(|i| glam::DVec3::from(surface.points[i as usize].map(f64::from)));
            0.5 * (b - a).cross(c - a).length()
        })
        .sum();
    assert!(total_area > 0.0);
    let fields = run(&reg, &mut s, "results.list_fields", json!({}));
    assert!(
        fields["fields"]
            .as_array()
            .unwrap()
            .iter()
            .any(|f| f["name"] == "impact")
    );
    // On a cube of 12 triangles the smoothing spreads the front hits over the sides, so the
    // front value is well below 1; conservation is checked in tests/sim/heavy_particles.rs.
    assert!(field.iter().any(|v| *v > 0.1));
    // The map can be shown on the model.
    run(
        &reg,
        &mut s,
        "view.set_surface_field",
        json!({"field": "impact"}),
    );
    // The same inputs again: from the cache, the same map.
    let again = impact(&reg, &mut s, true, 3000);
    assert_eq!(again["cached"], true);
    assert_eq!(again["status"]["map"]["impact"], map["impact"]);
}

#[test]
fn cancelling_writes_nothing() {
    let (reg, mut s, root) = setup("cancel");
    let out = impact(&reg, &mut s, false, 2_000_000);
    let id = out["impact_id"].as_str().unwrap().to_string();
    run(
        &reg,
        &mut s,
        "results.impact_cancel",
        json!({"impact_id": id}),
    );
    let mut state = String::new();
    for _ in 0..600 {
        let st = run(
            &reg,
            &mut s,
            "results.impact_status",
            json!({"impact_id": id}),
        );
        state = st["state"].as_str().unwrap().to_string();
        if state != "running" {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
    assert_eq!(state, "cancelled");
    let dir = root.join("cache/results/job-h");
    let maps = std::fs::read_dir(dir)
        .unwrap()
        .flatten()
        .filter(|e| e.file_name().to_string_lossy().starts_with("impact-"))
        .count();
    assert_eq!(maps, 0);
}

#[test]
fn a_ride_on_a_raindrop_ends_with_the_impact() {
    let (reg, mut s, _) = setup("ride");
    let info = run(
        &reg,
        &mut s,
        "ride.start",
        json!({"from": [-0.02, 0.01, 0.012], "particle": {"material": "rain", "gravity": false}}),
    );
    assert_eq!(info["ends"][1], "blocked", "{}", info["ends"]);
    let last = info["events"].as_array().unwrap().last().unwrap().clone();
    assert_eq!(last["id"], "impact", "{}", info["events"]);
    assert!(last["value"].as_f64().unwrap() > 5.0);
    assert!(
        (last["extra"].as_f64().unwrap() - 90.0).abs() < 5.0,
        "{last}"
    );
    assert_eq!(info["columns"][8], "slip");
    // Screen time stays monotonic up to the impact.
    let t: Vec<f64> = info["rows"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r[0].as_f64().unwrap())
        .collect();
    assert!(t.windows(2).all(|w| w[1] > w[0]));
}

#[test]
fn a_manifest_of_version_15_opens_with_massless_particles() {
    let (reg, mut s, root) = setup("migrate");
    run(
        &reg,
        &mut s,
        "view.set_particles",
        json!({"preset": "wind_tunnel_smoke"}),
    );
    run(&reg, &mut s, "project.close", json!({}));
    let file = root.join("foamcraft.json");
    let mut doc: Value = serde_json::from_str(&std::fs::read_to_string(&file).unwrap()).unwrap();
    doc["schema_version"] = json!(15);
    doc["view"]["particles"]
        .as_object_mut()
        .unwrap()
        .remove("inertia");
    std::fs::write(&file, doc.to_string()).unwrap();
    run(&reg, &mut s, "project.open", json!({"path": root}));
    let state = run(&reg, &mut s, "view.state", json!({}));
    assert!(state["particles"]["inertia"].is_null());
}
