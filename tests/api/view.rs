// SPDX-License-Identifier: GPL-3.0-or-later
//! view.* commands on a result cache written by the test: slices, isosurfaces, clip plane, the
//! stored scene and an image without a window.

use std::path::{Path, PathBuf};

use foamcraft_core::command::{Registry, Session};
use foamcraft_sim::results::{GridSpec, Surface, grid_from_rows};
use serde_json::{Value, json};

fn run(reg: &Registry, s: &mut Session, name: &str, params: Value) -> Value {
    reg.execute(s, name, params)
        .unwrap_or_else(|e| panic!("{name}: {e}"))
}

/// A cache like the one of a wind tunnel run around the 20 mm cube at the origin: flow 10 m/s
/// along x, slower near the cube; points inside the cube masked.
fn write_cache(root: &Path) {
    let dir = root.join("cache/results/job-test");
    std::fs::create_dir_all(&dir).unwrap();
    let cube = foamcraft_geometry::read_mesh(
        &PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/data/geometry/cube_binary.stl"),
    )
    .unwrap();
    let points: Vec<[f32; 3]> = cube
        .positions
        .iter()
        .map(|p| p.map(|c| c / 1000.0))
        .collect();
    let n = cube.triangles.len();
    let surface = Surface {
        points,
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
                rows.push(vec![
                    p[0],
                    p[1],
                    p[2],
                    0.5 * 1.2 * (100.0 - u * u),
                    u,
                    0.0,
                    0.0,
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
            {"name": "Uy", "unit": "m/s", "location": "volume", "min": 0.0, "max": 0.0},
            {"name": "Uz", "unit": "m/s", "location": "volume", "min": 0.0, "max": 0.0}
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
    let base = std::env::temp_dir().join(format!("fc-view-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&base);
    let mut s = Session::new(base.join("data"));
    let root = base.join("p");
    run(
        &reg,
        &mut s,
        "project.new",
        json!({"path": root, "name": "V"}),
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

#[test]
fn slices_isosurfaces_and_clip_are_stored_as_the_scene() {
    let (reg, mut s, root) = setup("scene");
    let fields = run(&reg, &mut s, "view.volume_fields", json!({}));
    for f in ["U_mag", "Q", "p_total", "p", "Ux"] {
        assert!(
            fields.as_array().unwrap().iter().any(|x| x["name"] == f),
            "{f}: {fields}"
        );
    }
    let slice = run(
        &reg,
        &mut s,
        "view.add_slice",
        json!({"origin": [0.01, 0.01, 0.01], "normal": [0.0, 0.0, 1.0], "field": "U_mag"}),
    );
    assert_eq!(slice["slices"][0]["id"], "slice-1");
    run(
        &reg,
        &mut s,
        "view.update_slice",
        json!({"id": "slice-1", "origin": [0.01, 0.01, 0.03]}),
    );
    let iso = run(
        &reg,
        &mut s,
        "view.add_isosurface",
        json!({"field": "Q", "value": 100.0}),
    );
    assert_eq!(iso["isosurfaces"][0]["id"], "iso-1");
    run(
        &reg,
        &mut s,
        "view.set_clip",
        json!({"plane": {"origin": [0.01, 0.0, 0.0], "normal": [0.0, -1.0, 0.0]}}),
    );
    let bad = reg
        .execute(
            &mut s,
            "view.add_slice",
            json!({"origin": [0.0, 0.0, 0.0], "normal": [0.0, 0.0, 1.0], "field": "nope"}),
        )
        .unwrap_err();
    assert_eq!(bad.code, "results.unknown_field");
    let missing = reg
        .execute(&mut s, "view.remove", json!({"id": "slice-9"}))
        .unwrap_err();
    assert_eq!(missing.code, "view.not_found");
    // The scene survives closing and opening.
    run(&reg, &mut s, "project.close", json!({}));
    run(&reg, &mut s, "project.open", json!({"path": root}));
    let view = run(&reg, &mut s, "view.state", json!({}));
    assert_eq!(view["slices"][0]["origin"], json!([0.01, 0.01, 0.03]));
    assert_eq!(view["isosurfaces"][0]["field"], "Q");
    assert_eq!(view["clip"]["normal"], json!([0.0, -1.0, 0.0]));
    run(&reg, &mut s, "view.remove", json!({"id": "iso-1"}));
    run(&reg, &mut s, "view.set_clip", json!({"plane": null}));
    let view = run(&reg, &mut s, "view.state", json!({}));
    assert_eq!(view["isosurfaces"], json!([]));
    assert!(view["clip"].is_null());
}

#[test]
fn the_scene_is_rendered_without_a_window() {
    let (reg, mut s, root) = setup("render");
    run(
        &reg,
        &mut s,
        "view.add_slice",
        json!({"origin": [0.01, 0.01, 0.01], "normal": [0.0, 0.0, 1.0], "field": "U_mag"}),
    );
    run(
        &reg,
        &mut s,
        "view.add_isosurface",
        json!({"field": "U_mag", "value": 5.0}),
    );
    let path = root.join("scene.png");
    match reg.execute(
        &mut s,
        "view.render",
        json!({"path": path, "width": 320, "height": 240}),
    ) {
        Err(e) if e.code == "view.no_gpu" => {
            eprintln!("SKIPPED: no GPU adapter available");
        }
        Err(e) => panic!("view.render: {e}"),
        Ok(out) => {
            assert_eq!(out["width"], 320);
            let bytes = std::fs::read(&path).unwrap();
            assert_eq!(&bytes[1..4], b"PNG");
            assert!(bytes.len() > 2000);
        }
    }
}

#[test]
fn particles_presets_regions_and_streamlines() {
    let (reg, mut s, _) = setup("particles");
    let view = run(
        &reg,
        &mut s,
        "view.set_particles",
        json!({"preset": "wind_tunnel_smoke"}),
    );
    let p = &view["particles"];
    assert_eq!(p["style"], "smoke");
    assert_eq!(p["region"]["kind"], "area");
    // Upstream: near the inflow side of the grid (x from -0.04).
    assert!(p["region"]["origin"][0].as_f64().unwrap() < -0.03, "{p}");
    let setup = foamcraft_api::particles::setup(s.project().unwrap()).expect("particle setup");
    assert_eq!(setup.levels.len(), 1);
    assert_eq!(setup.seeds.len(), 4096);
    assert!(setup.seeds.iter().all(|q| q[0] < -0.03));
    assert!(setup.dt > 0.0);
    // Moving the start area.
    run(
        &reg,
        &mut s,
        "view.set_particles",
        json!({"region": {"kind": "area", "origin": [0.05, -0.02, 0.0], "u": [0.0, 0.04, 0.0], "v": [0.0, 0.0, 0.04]}, "count": 1000, "seed": 7}),
    );
    let setup = foamcraft_api::particles::setup(s.project().unwrap()).unwrap();
    assert!(setup.seeds.iter().all(|q| (q[0] - 0.05).abs() < 1e-6));
    assert_eq!(setup.settings.count, 1000);
    assert_eq!(setup.settings.seed, 7);
    // Streamlines are drawn as lines in the scene, not animated.
    run(
        &reg,
        &mut s,
        "view.set_particles",
        json!({"style": "streamlines", "region": {"kind": "area", "origin": [-0.035, -0.02, 0.0], "u": [0.0, 0.06, 0.0], "v": [0.0, 0.0, 0.04]}}),
    );
    assert!(foamcraft_api::particles::setup(s.project().unwrap()).is_none());
    let lines = foamcraft_api::particles::streamline_meshes(s.project().unwrap());
    assert_eq!(lines.len(), 1);
    assert!(lines[0].lines && lines[0].indices.len() > 100);
    let bad = reg
        .execute(&mut s, "view.set_particles", json!({"style": "sparks"}))
        .unwrap_err();
    assert_eq!(bad.code, "view.invalid_particles");
    let off = run(&reg, &mut s, "view.particles_off", json!({}));
    assert!(off["particles"].is_null());
}

/// A copy of the cache of `job-test` as `job`, with the surface pressure plus `add` and, when
/// `other_geometry`, one point moved.
fn copy_cache(root: &Path, job: &str, add: f32, other_geometry: bool) {
    let from = root.join("cache/results/job-test");
    let to = root.join("cache/results").join(job);
    std::fs::create_dir_all(&to).unwrap();
    for f in ["grid_0.bin", "meta.json"] {
        std::fs::copy(from.join(f), to.join(f)).unwrap();
    }
    let mut surface =
        Surface::from_bytes(&std::fs::read(from.join("surface.bin")).unwrap()).unwrap();
    for v in &mut surface.fields[0].1 {
        *v += add;
    }
    if other_geometry {
        surface.points[0][0] += 0.001;
    }
    std::fs::write(to.join("surface.bin"), surface.to_bytes()).unwrap();
}

#[test]
fn two_runs_are_shown_side_by_side_or_as_a_difference() {
    let (reg, mut s, root) = setup("compare");
    copy_cache(&root, "job-b", 5.0, false);
    copy_cache(&root, "job-c", 0.0, true);
    let out = run(
        &reg,
        &mut s,
        "view.set_compare",
        json!({"a": "job-test", "b": "job-b", "mode": "split"}),
    );
    assert_eq!(out["mode"], "split");
    let manifest: Value =
        serde_json::from_str(&std::fs::read_to_string(root.join("foamcraft.json")).unwrap())
            .unwrap();
    assert_eq!(manifest["view"]["compare"]["b"], "job-b");
    // Without a chosen field the surface pressure is shown.
    assert_eq!(manifest["view"]["surface_field"], "p");
    let path = root.join("split.png");
    match reg.execute(
        &mut s,
        "view.render",
        json!({"path": path, "width": 320, "height": 160}),
    ) {
        Err(e) if e.code == "view.no_gpu" => eprintln!("SKIPPED: no GPU adapter available"),
        Err(e) => panic!("view.render: {e}"),
        Ok(_) => assert_eq!(&std::fs::read(&path).unwrap()[1..4], b"PNG"),
    }
    // The difference b − a of the same geometry: 5 Pa everywhere.
    let out = run(
        &reg,
        &mut s,
        "view.set_compare",
        json!({"a": "job-test", "b": "job-b", "mode": "difference"}),
    );
    assert!((out["difference"]["min"].as_f64().unwrap() - 5.0).abs() < 1e-4);
    assert!((out["difference"]["max"].as_f64().unwrap() - 5.0).abs() < 1e-4);
    let err = reg
        .execute(
            &mut s,
            "view.set_compare",
            json!({"a": "job-test", "b": "job-c", "mode": "difference"}),
        )
        .unwrap_err();
    assert_eq!(err.code, "results.geometry_differs");
    run(&reg, &mut s, "view.set_compare", json!({"a": null}));
    let manifest: Value =
        serde_json::from_str(&std::fs::read_to_string(root.join("foamcraft.json")).unwrap())
            .unwrap();
    assert!(manifest["view"]["compare"].is_null());
}

#[test]
fn mesh_edges_are_hidden_while_a_result_colours_the_model() {
    use foamcraft_core::project::ViewSettings;
    let mut view = ViewSettings::default();
    assert!(foamcraft_api::scene::draws_mesh_edges(&view));
    view.result_job = Some("job-1".into());
    view.surface_field = Some("p".into());
    assert!(!foamcraft_api::scene::draws_mesh_edges(&view));
}
