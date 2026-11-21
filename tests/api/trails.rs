// SPDX-License-Identifier: GPL-3.0-or-later
//! Light trails, ribbons and tubes in the commands (Task_35): the new styles and fields are
//! stored and reopened, a manifest of version 16 opens, unknown styles stay rejected, the memory
//! limit shortens the trails, the presets build valid scenes and a long exposure repeats.

use std::path::{Path, PathBuf};

use foamcraft_core::command::{Registry, Session};
use foamcraft_sim::results::{GridSpec, Surface, grid_from_rows};
use serde_json::{Value, json};

fn run(reg: &Registry, s: &mut Session, name: &str, params: Value) -> Value {
    reg.execute(s, name, params)
        .unwrap_or_else(|e| panic!("{name}: {e}"))
}

/// A wind tunnel cache around the 20 mm cube at the origin: 10 m/s along x with a swirl
/// around the x axis behind the cube.
fn write_cache(root: &Path) {
    let dir = root.join("cache/results/job-test");
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
        fields: vec![("p".into(), (0..n).map(|i| i as f32).collect())],
    };
    std::fs::write(dir.join("surface.bin"), surface.to_bytes()).unwrap();
    let spec = GridSpec {
        min: [-0.04, -0.04, -0.01],
        max: [0.08, 0.06, 0.05],
        dims: [24, 20, 12],
    };
    let mut rows = Vec::new();
    for k in 0..12 {
        for j in 0..20 {
            for i in 0..24 {
                let p = spec.point(i, j, k);
                if (0..3).all(|a| p[a] > 0.0 && p[a] < 0.02) {
                    continue;
                }
                let d =
                    ((p[0] - 0.01).powi(2) + (p[1] - 0.01).powi(2) + (p[2] - 0.01).powi(2)).sqrt();
                let u = 10.0 * (1.0 - (-d / 0.01).exp());
                // Swirl behind the cube, around the x axis through its middle.
                let w = if p[0] > 0.02 { 300.0 } else { 0.0 };
                rows.push(vec![
                    p[0],
                    p[1],
                    p[2],
                    0.5 * 1.2 * (100.0 - u * u),
                    u,
                    -w * (p[2] - 0.01),
                    w * (p[1] - 0.01),
                ]);
            }
        }
    }
    let grid = grid_from_rows(&spec, &["p", "Ux", "Uy", "Uz"], &rows);
    std::fs::write(dir.join("grid_0.bin"), grid.to_bytes()).unwrap();
    let meta = json!({
        "job_id": "job-test",
        "fields": [
            {"name": "p", "unit": "Pa", "location": "surface", "min": 0.0, "max": n as f64},
            {"name": "p", "unit": "Pa", "location": "volume", "min": 0.0, "max": 60.0},
            {"name": "Ux", "unit": "m/s", "location": "volume", "min": 0.0, "max": 10.0},
            {"name": "Uy", "unit": "m/s", "location": "volume", "min": -3.0, "max": 3.0},
            {"name": "Uz", "unit": "m/s", "location": "volume", "min": -3.0, "max": 3.0}
        ],
        "levels": [{"min": spec.min, "max": spec.max, "dims": spec.dims}],
        "yaw_deg": 0.0,
        "bytes": 1,
        "colormap": []
    });
    std::fs::write(dir.join("meta.json"), meta.to_string()).unwrap();
}

fn setup(name: &str) -> (Registry, Session, PathBuf) {
    let reg = foamcraft_api::registry();
    let base = std::env::temp_dir().join(format!("fc-trails-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&base);
    let mut s = Session::new(base.join("data"));
    let root = base.join("p");
    run(
        &reg,
        &mut s,
        "project.new",
        json!({"path": root, "name": "T"}),
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
    (reg, s, root)
}

fn has_gpu() -> bool {
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
fn trails_and_their_fields_are_stored_and_reopened() {
    let (reg, mut s, root) = setup("fields");
    let view = run(
        &reg,
        &mut s,
        "view.set_particles",
        json!({"style": "trails", "count": 20000, "trail_length": 0.001, "width": 3.0,
               "glow": false, "exposure": 2.0, "long_exposure": 2.0}),
    );
    let p = &view["particles"];
    assert_eq!(p["style"], "trails");
    assert_eq!(p["width"], 3.0);
    assert_eq!(p["glow"], false);
    assert_eq!(p["long_exposure"], 2.0);
    let setup = foamcraft_api::particles::setup(s.project().unwrap()).expect("setup");
    let t = setup.trails.as_ref().expect("trail settings");
    assert_eq!(t.k, 24);
    assert!(!setup.trails_shortened);
    assert_eq!(t.width_px, 3.0);
    assert!(!t.glow);
    assert_eq!(t.exposure, 2.0);
    assert!(t.every >= 1);
    assert_eq!(setup.long_exposure, Some(2.0));
    // A longer trail writes a point less often.
    let every = t.every;
    run(
        &reg,
        &mut s,
        "view.set_particles",
        json!({"trail_length": 0.003}),
    );
    let longer = foamcraft_api::particles::setup(s.project().unwrap()).unwrap();
    assert!(longer.trails.unwrap().every > every);
    // Null switches the long exposure off.
    let view = run(
        &reg,
        &mut s,
        "view.set_particles",
        json!({"long_exposure": null}),
    );
    assert!(view["particles"]["long_exposure"].is_null());
    // Reopened, the fields are still there.
    run(&reg, &mut s, "project.close", json!({}));
    run(&reg, &mut s, "project.open", json!({"path": root}));
    let state = run(&reg, &mut s, "view.state", json!({}));
    assert_eq!(state["particles"]["style"], "trails");
    assert_eq!(state["particles"]["width"], 3.0);
    // Other styles have no trails.
    run(
        &reg,
        &mut s,
        "view.set_particles",
        json!({"style": "comets"}),
    );
    let comets = foamcraft_api::particles::setup(s.project().unwrap()).unwrap();
    assert!(comets.trails.is_none());
}

#[test]
fn too_many_points_shorten_the_trails_not_the_particles() {
    let (reg, mut s, _) = setup("limit");
    run(
        &reg,
        &mut s,
        "view.set_particles",
        json!({"style": "trails", "count": 200000}),
    );
    let setup = foamcraft_api::particles::setup(s.project().unwrap()).unwrap();
    assert_eq!(setup.settings.count, 200_000);
    assert_eq!(setup.trails.unwrap().k, 12);
    assert!(setup.trails_shortened);
}

#[test]
fn unknown_styles_and_bad_values_are_rejected() {
    let (reg, mut s, _) = setup("bad");
    for bad in [
        json!({"style": "sparks"}),
        json!({"style": "trails", "width": -1.0}),
        json!({"style": "trails", "trail_length": 0.0}),
        json!({"style": "trails", "exposure": f64::NAN.to_string()}),
        json!({"style": "trails", "long_exposure": 100.0}),
    ] {
        let err = reg
            .execute(&mut s, "view.set_particles", bad.clone())
            .unwrap_err();
        assert!(
            ["view.invalid_particles", "command.invalid_params"].contains(&err.code.as_str()),
            "{bad}: {}",
            err.code
        );
    }
}

#[test]
fn a_manifest_of_version_16_opens_with_the_old_behaviour() {
    let (reg, mut s, root) = setup("migrate");
    run(
        &reg,
        &mut s,
        "view.set_particles",
        json!({"style": "comets"}),
    );
    run(&reg, &mut s, "project.close", json!({}));
    let file = root.join("foamcraft.json");
    let mut doc: Value = serde_json::from_str(&std::fs::read_to_string(&file).unwrap()).unwrap();
    doc["schema_version"] = json!(16);
    let p = doc["view"]["particles"].as_object_mut().unwrap();
    for f in ["trail_length", "width", "glow", "exposure", "long_exposure"] {
        p.remove(f);
    }
    std::fs::write(&file, doc.to_string()).unwrap();
    let info = run(&reg, &mut s, "project.open", json!({"path": root}));
    assert_eq!(
        info["schema_version"],
        foamcraft_core::project::SCHEMA_VERSION
    );
    let setup = foamcraft_api::particles::setup(s.project().unwrap()).unwrap();
    assert!(setup.trails.is_none());
    assert_eq!(setup.long_exposure, None);
}

#[test]
fn the_presets_light_trails_and_ribbons_build_valid_scenes() {
    let (reg, mut s, _) = setup("presets");
    let ids: Vec<String> = run(&reg, &mut s, "view.presets", json!({}))
        .as_array()
        .unwrap()
        .iter()
        .map(|p| p["id"].as_str().unwrap().to_string())
        .collect();
    assert!(ids.iter().any(|i| i == "light_trails"), "{ids:?}");
    assert!(ids.iter().any(|i| i == "ribbons"), "{ids:?}");
    let v = run(
        &reg,
        &mut s,
        "view.apply_preset",
        json!({"id": "light_trails"}),
    );
    assert_eq!(v["particles"]["style"], "trails");
    let setup = foamcraft_api::particles::setup(s.project().unwrap()).unwrap();
    assert!(setup.trails.is_some());
    let v = run(&reg, &mut s, "view.apply_preset", json!({"id": "ribbons"}));
    assert_eq!(v["particles"]["style"], "ribbons");
    assert_eq!(v["particles"]["color_by"], "swirl");
    assert_eq!(v["particles"]["region"]["kind"], "rake");
    let geometry =
        foamcraft_api::particles::ribbon_geometry(s.project().unwrap()).expect("ribbon geometry");
    assert!(!geometry.ribbons.is_empty());
}

#[test]
fn a_long_exposure_repeats_and_needs_particles() {
    if !has_gpu() {
        return;
    }
    let (reg, mut s, root) = setup("exposure");
    let a = root.join("a.png");
    let b = root.join("b.png");
    let plain = root.join("plain.png");
    // Without particles there is nothing to expose.
    let err = reg
        .execute(
            &mut s,
            "export.image",
            json!({"path": a, "width": 320, "height": 200, "exposure_s": 1.0}),
        )
        .unwrap_err();
    assert_eq!(err.code, "export.invalid");
    run(
        &reg,
        &mut s,
        "view.set_particles",
        json!({"style": "trails", "count": 5000,
               "region": {"kind": "area", "origin": [-0.025, -0.02, 0.0], "u": [0.0, 0.06, 0.0], "v": [0.0, 0.0, 0.04]}}),
    );
    for path in [&a, &b] {
        run(
            &reg,
            &mut s,
            "export.image",
            json!({"path": path, "width": 320, "height": 200, "exposure_s": 1.0}),
        );
    }
    run(
        &reg,
        &mut s,
        "export.image",
        json!({"path": plain, "width": 320, "height": 200}),
    );
    let read = |p: &PathBuf| std::fs::read(p).unwrap();
    assert_eq!(read(&a), read(&b));
    assert_ne!(read(&a), read(&plain));
}

#[test]
fn the_trail_length_can_be_given_in_comet_tails_and_a_width_does_not_carry_over() {
    let (reg, mut s, _) = setup("factor");
    let two = run(
        &reg,
        &mut s,
        "view.set_particles",
        json!({"style": "trails", "trail_factor": 2.0, "width": 3.5}),
    );
    let four = run(
        &reg,
        &mut s,
        "view.set_particles",
        json!({"trail_factor": 4.0}),
    );
    let (a, b) = (
        two["particles"]["trail_length"].as_f64().unwrap(),
        four["particles"]["trail_length"].as_f64().unwrap(),
    );
    assert!((b / a - 2.0).abs() < 1e-9, "{a} {b}");
    let err = reg
        .execute(&mut s, "view.set_particles", json!({"trail_factor": 20.0}))
        .unwrap_err();
    assert_eq!(err.code, "view.invalid_particles");
    // Pixels of a trail are no share of the model: ribbons start with their own width.
    let v = run(
        &reg,
        &mut s,
        "view.set_particles",
        json!({"style": "ribbons"}),
    );
    assert!(v["particles"]["width"].is_null());
}

#[test]
fn inside_a_part_the_new_presets_cut_it_open() {
    let (reg, mut s, _) = setup("internal");
    s.project().unwrap().manifest.scenario = Some(
        serde_json::from_value(json!({
            "kind": "internal_flow",
            "medium": {"kind": "water", "temperature_c": 20.0},
            "quality": "draft",
            "inlets": [],
            "outlets": []
        }))
        .unwrap(),
    );
    for id in ["light_trails", "ribbons"] {
        let v = run(&reg, &mut s, "view.apply_preset", json!({"id": id}));
        assert!(v["clip"].is_object(), "{id}: {v}");
        assert_eq!(v["clip_targets"], json!(["model"]), "{id}");
        assert_eq!(v["particles"]["region"]["kind"], "inlets", "{id}");
    }
}
