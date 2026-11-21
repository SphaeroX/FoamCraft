// SPDX-License-Identifier: GPL-3.0-or-later
//! export.image and export.video on a project with a coloured result surface: size,
//! transparency, legend, frame count, repeatability, cancelling. Without a GPU the tests skip.

use std::path::{Path, PathBuf};

use foamcraft_core::command::{Registry, Session};
use foamcraft_sim::results::Surface;
use serde_json::{Value, json};

fn setup(name: &str) -> Option<(Registry, Session, PathBuf)> {
    if foamcraft_render::Renderer::new_headless().is_none() {
        assert!(
            std::env::var_os("FOAMCRAFT_REQUIRE_GPU").is_none(),
            "no GPU adapter"
        );
        eprintln!("SKIPPED: no GPU adapter available");
        return None;
    }
    let reg = foamcraft_api::registry();
    let base = std::env::temp_dir().join(format!("fc-export-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&base);
    let mut s = Session::new(base.join("data"));
    let root = base.join("p");
    reg.execute(&mut s, "project.new", json!({"path": root, "name": "E"}))
        .unwrap();
    let model =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/data/geometry/cube_binary.stl");
    reg.execute(
        &mut s,
        "geometry.import",
        json!({"path": model, "unit": "mm"}),
    )
    .unwrap();
    write_surface(&root);
    reg.execute(
        &mut s,
        "view.set_surface_field",
        json!({"field": "p", "job_id": "job-e"}),
    )
    .unwrap();
    Some((reg, s, root))
}

/// The cube surface (metres) with a pressure per triangle, as a result cache of `job-e`.
fn write_surface(root: &Path) {
    let dir = root.join("cache/results/job-e");
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
        fields: vec![("p".into(), (0..n).map(|i| i as f32 * 10.0 - 50.0).collect())],
    };
    std::fs::write(dir.join("surface.bin"), surface.to_bytes()).unwrap();
    let meta = json!({
        "job_id": "job-e",
        "fields": [{"name": "p", "unit": "Pa", "location": "surface", "min": -50.0, "max": (n as f64) * 10.0 - 60.0}],
        "levels": [], "yaw_deg": 0.0, "bytes": 1, "colormap": []
    });
    std::fs::write(dir.join("meta.json"), meta.to_string()).unwrap();
}

fn png(path: &Path) -> image::RgbaImage {
    image::open(path).unwrap().to_rgba8()
}

#[test]
fn a_4k_image_with_transparent_background() {
    let Some((reg, mut s, root)) = setup("image") else {
        return;
    };
    let path = root.join("shot.png");
    let out = reg
        .execute(
            &mut s,
            "export.image",
            json!({"path": path, "resolution": "4k", "transparent": true, "legend": false}),
        )
        .unwrap();
    assert_eq!(out["width"], 3840);
    let img = png(&path);
    assert_eq!(img.dimensions(), (3840, 2160));
    // Corners are background: fully transparent. The model in the middle is opaque.
    assert_eq!(img.get_pixel(0, 0)[3], 0);
    assert_eq!(img.get_pixel(3839, 2159)[3], 0);
    assert_eq!(img.get_pixel(1920, 1080)[3], 255);
}

#[test]
fn the_legend_can_be_switched_on_and_off() {
    let Some((reg, mut s, root)) = setup("legend") else {
        return;
    };
    let with = root.join("with.png");
    let without = root.join("without.png");
    for (path, legend) in [(&with, true), (&without, false)] {
        reg.execute(
            &mut s,
            "export.image",
            json!({"path": path, "width": 800, "height": 500, "legend": legend}),
        )
        .unwrap();
    }
    let (a, b) = (png(&with), png(&without));
    // The legend sits at the bottom right; the rest of the picture is the same.
    let differs = |x0: u32, y0: u32, x1: u32, y1: u32| {
        (y0..y1).any(|y| (x0..x1).any(|x| a.get_pixel(x, y) != b.get_pixel(x, y)))
    };
    assert!(differs(500, 380, 800, 500));
    assert!(!differs(0, 0, 400, 300));
}

fn export_video(reg: &Registry, s: &mut Session, path: &Path) -> Value {
    reg.execute(
        s,
        "export.video",
        json!({"path": path, "width": 160, "height": 120, "fps": 10, "duration_s": 2.0,
               "camera": {"preset": "turntable"}, "wait": true}),
    )
    .unwrap()
}

#[test]
fn a_video_has_the_right_frame_count_and_repeats_exactly() {
    let Some((reg, mut s, root)) = setup("video") else {
        return;
    };
    let a = root.join("a.avi");
    let b = root.join("b.avi");
    let out = export_video(&reg, &mut s, &a);
    assert_eq!(out["frames"], 20);
    assert_eq!(out["state"], "done");
    export_video(&reg, &mut s, &b);
    let bytes = std::fs::read(&a).unwrap();
    assert_eq!(&bytes[0..4], b"RIFF");
    let avih = bytes.windows(4).position(|w| w == b"avih").unwrap() + 8;
    let frames = u32::from_le_bytes(bytes[avih + 16..avih + 20].try_into().unwrap());
    let us = u32::from_le_bytes(bytes[avih..avih + 4].try_into().unwrap());
    assert_eq!(frames, 20);
    assert_eq!(us, 100_000);
    // Same settings, same frames.
    assert_eq!(bytes, std::fs::read(&b).unwrap());
    // The turntable really turns: the first and the middle frame differ.
    let frames = foamcraft_render::video::read_frames(&a).unwrap();
    assert_eq!(frames.len(), 20);
    assert_ne!(frames[0], frames[10]);
}

#[test]
fn cancelling_removes_the_incomplete_file() {
    let Some((reg, mut s, root)) = setup("cancel") else {
        return;
    };
    let path = root.join("long.avi");
    let started = reg
        .execute(
            &mut s,
            "export.video",
            json!({"path": path, "width": 640, "height": 360, "fps": 30, "duration_s": 60.0}),
        )
        .unwrap();
    let id = started["export_id"].as_str().unwrap().to_string();
    // Let it write a few frames, then cancel.
    let t = std::time::Instant::now();
    while reg
        .execute(&mut s, "export.status", json!({"export_id": id}))
        .unwrap()["frame"]
        .as_u64()
        .unwrap_or(0)
        < 3
    {
        assert!(t.elapsed().as_secs() < 60);
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    reg.execute(&mut s, "export.cancel", json!({"export_id": id}))
        .unwrap();
    let t = std::time::Instant::now();
    loop {
        let st = reg
            .execute(&mut s, "export.status", json!({"export_id": id}))
            .unwrap();
        if st["state"] == "cancelled" {
            break;
        }
        assert!(t.elapsed().as_secs() < 30, "{st}");
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    assert!(!path.exists());
}

#[test]
fn a_preview_frame_comes_back_as_png_data() {
    let Some((reg, mut s, _)) = setup("preview") else {
        return;
    };
    let out = reg
        .execute(
            &mut s,
            "export.preview",
            json!({"width": 320, "height": 180, "time_s": 1.0, "duration_s": 4.0, "camera": {"preset": "swing"}}),
        )
        .unwrap();
    let data = out["png_base64"].as_str().unwrap();
    assert!(data.len() > 1000);
}
