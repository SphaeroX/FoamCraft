// SPDX-License-Identifier: GPL-3.0-or-later
//! Drag maths of the move and rotate handles (Task_25).

use foamcraft_render::camera::OrbitCamera;
use foamcraft_render::handles::{AxisDrag, RingDrag, move_step, snap, wrap_pi};
use glam::Vec3;

fn ray_at(cam: &OrbitCamera, x: f32, y: f32) -> (Vec3, Vec3) {
    cam.ray(1.0, x, y)
}

/// Screen point (NDC) of a world point.
fn ndc(cam: &OrbitCamera, p: Vec3) -> (f32, f32) {
    let q = cam.view_proj(1.0).project_point3(p);
    (q.x, q.y)
}

#[test]
fn axis_drag_follows_the_mouse_along_the_axis_from_several_views() {
    for (yaw, pitch) in [(-1.0f32, 0.4f32), (0.3, 0.2), (2.5, -0.6), (-2.0, 1.2)] {
        let cam = OrbitCamera {
            yaw,
            pitch,
            ..Default::default()
        };
        for axis in [Vec3::X, Vec3::Y, Vec3::Z] {
            // Skip axes that point almost straight at the camera.
            let view = (cam.target - cam.eye()).normalize();
            if axis.dot(view).abs() > 0.95 {
                continue;
            }
            let origin = Vec3::ZERO;
            let (sx, sy) = ndc(&cam, origin);
            let mut drag = AxisDrag::new(origin, axis, ray_at(&cam, sx, sy));
            let goal = origin + axis * 0.7;
            let (gx, gy) = ndc(&cam, goal);
            let moved = drag.update(ray_at(&cam, gx, gy));
            assert!((moved - 0.7).abs() < 1e-3, "yaw {yaw} axis {axis}: {moved}");
        }
    }
}

#[test]
fn axis_drag_does_not_jump_when_looking_along_the_axis() {
    // Look straight down: the Z axis points at the camera.
    let cam = OrbitCamera {
        pitch: std::f32::consts::FRAC_PI_2 - 1e-3,
        ..Default::default()
    };
    let mut drag = AxisDrag::new(Vec3::ZERO, Vec3::Z, ray_at(&cam, 0.0, 0.0));
    let first = drag.update(ray_at(&cam, 0.0, 0.0));
    let second = drag.update(ray_at(&cam, 0.3, 0.2));
    assert!(first.abs() < 1e-3);
    assert!((second - first).abs() < 1e-3, "jumped to {second}");
}

#[test]
fn ring_drag_measures_the_turn_around_the_axis() {
    let cam = OrbitCamera {
        yaw: -0.8,
        pitch: 1.0,
        ..Default::default()
    };
    let pivot = Vec3::ZERO;
    let start = Vec3::new(1.0, 0.0, 0.0);
    let (sx, sy) = ndc(&cam, start);
    let mut drag = RingDrag::new(pivot, Vec3::Z, ray_at(&cam, sx, sy));
    let angle = 40f32.to_radians();
    let end = Vec3::new(angle.cos(), angle.sin(), 0.0);
    let (ex, ey) = ndc(&cam, end);
    let turned = drag.update(ray_at(&cam, ex, ey));
    assert!((turned - angle).abs() < 1e-3, "{}", turned.to_degrees());
    // Past 180° the angle keeps counting instead of wrapping to -180°.
    let mut total = 0.0;
    for deg in [90.0f32, 170.0, 200.0] {
        let a = deg.to_radians();
        let (x, y) = ndc(&cam, Vec3::new(a.cos(), a.sin(), 0.0));
        total = drag.update(ray_at(&cam, x, y));
    }
    assert!(
        (total.to_degrees() - 200.0).abs() < 0.5,
        "{}",
        total.to_degrees()
    );
}

#[test]
fn ring_drag_keeps_its_value_when_the_ring_is_seen_edge_on() {
    // Horizontal view: the plane of the Z ring contains the view direction at the centre.
    let cam = OrbitCamera {
        yaw: 0.0,
        pitch: 0.0,
        ..Default::default()
    };
    let mut drag = RingDrag::new(Vec3::ZERO, Vec3::Z, ray_at(&cam, 0.0, 0.0));
    let a = drag.update(ray_at(&cam, 0.0, 0.0));
    let b = drag.update(ray_at(&cam, 0.4, 0.0));
    assert!(a.is_finite() && b.is_finite());
    assert!((b - a).abs() < 1e-3);
}

#[test]
fn angles_wrap_and_values_snap() {
    assert!((wrap_pi(3.5 * std::f32::consts::PI) - std::f32::consts::PI * -0.5).abs() < 1e-5);
    assert_eq!(snap(0.123, 0.05), 0.1);
    assert_eq!(snap(-0.126, 0.05), -0.15);
    // Round steps of 1, 2 or 5 times a power of ten, about a twentieth of the size.
    assert_eq!(move_step(0.02), 0.001);
    assert_eq!(move_step(1.0), 0.05);
    assert_eq!(move_step(4.0), 0.2);
}

// --- Drawing and picking the handles ---

use foamcraft_render::handles::{HANDLE_KEYS, handle_meshes};
use foamcraft_render::{Renderer, Scene};
use glam::Mat4;

const SIZE: u32 = 256;

fn renderer() -> Option<Renderer> {
    let r = Renderer::new_headless();
    if r.is_none() {
        assert!(
            std::env::var_os("FOAMCRAFT_REQUIRE_GPU").is_none(),
            "no GPU adapter but FOAMCRAFT_REQUIRE_GPU is set"
        );
        eprintln!("SKIPPED: no GPU adapter available");
    }
    r
}

fn pixel(p: &[u8], x: u32, y: u32) -> [u8; 4] {
    let i = ((y * SIZE + x) * 4) as usize;
    [p[i], p[i + 1], p[i + 2], p[i + 3]]
}

fn to_pixel(cam: &OrbitCamera, p: Vec3) -> (u32, u32) {
    let (x, y) = ndc(cam, p);
    (
        ((x + 1.0) / 2.0 * SIZE as f32) as u32,
        ((1.0 - y) / 2.0 * SIZE as f32) as u32,
    )
}

#[test]
fn a_model_matrix_moves_a_mesh_without_a_new_upload() {
    let Some(r) = renderer() else { return };
    let scene = Scene::test_cube();
    let gpu = r.upload(&scene);
    let mut cam = OrbitCamera::default();
    cam.fit(Vec3::ZERO, 1.0, 1.0);
    let before = r.render_offscreen_gpu(&gpu, &cam, SIZE, SIZE);
    assert_ne!(pixel(&before, SIZE / 2, SIZE / 2), scene.background_rgba8());
    r.set_model_transform(&gpu, 0, Mat4::from_translation(Vec3::new(0.0, 0.0, 50.0)));
    let moved = r.render_offscreen_gpu(&gpu, &cam, SIZE, SIZE);
    assert_eq!(pixel(&moved, SIZE / 2, SIZE / 2), scene.background_rgba8());
    r.set_model_transform(&gpu, 0, Mat4::IDENTITY);
    assert_eq!(r.render_offscreen_gpu(&gpu, &cam, SIZE, SIZE), before);
}

#[test]
fn handles_are_drawn_in_front_and_picked_by_their_ids() {
    let Some(r) = renderer() else { return };
    let scene = Scene::test_cube();
    let mut gpu = r.upload(&scene);
    // Pivot inside the cube: the handles must still be visible and pickable.
    let meshes = handle_meshes(Vec3::ZERO, 1.0, 100);
    assert_eq!(meshes.len(), HANDLE_KEYS.len());
    r.set_handles(&mut gpu, &meshes);
    let mut cam = OrbitCamera::default();
    cam.fit(Vec3::ZERO, 1.4, 1.0);
    // Tip region of the X arrow (length 0.9 × size) and a point on the Z ring (radius 0.7).
    let (ax, ay) = to_pixel(&cam, Vec3::new(0.82, 0.0, 0.0));
    assert_eq!(r.pick(&gpu, &cam, SIZE, SIZE, ax, ay), Some(100));
    let s = std::f32::consts::FRAC_1_SQRT_2;
    let ring = Vec3::new(0.7 * s, -0.7 * s, 0.0);
    let (rx, ry) = to_pixel(&cam, ring);
    assert_eq!(r.pick(&gpu, &cam, SIZE, SIZE, rx, ry), Some(105));
    // The cube centre without handles in front is still the cube (pick id 0 there).
    r.set_handles(&mut gpu, &[]);
    assert_eq!(r.pick(&gpu, &cam, SIZE, SIZE, ax, ay), None);
}

#[test]
fn handles_at_a_cube_match_golden_image() {
    let Some(r) = renderer() else { return };
    let scene = Scene::test_cube();
    let mut gpu = r.upload(&scene);
    r.set_handles(&mut gpu, &handle_meshes(Vec3::ZERO, 1.0, 1));
    let mut cam = OrbitCamera::default();
    cam.fit(Vec3::ZERO, 1.4, 1.0);
    let pixels = r.render_offscreen_gpu(&gpu, &cam, SIZE, SIZE);
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/golden/handles_cube.png");
    if std::env::var_os("FOAMCRAFT_UPDATE_GOLDEN").is_some() {
        foamcraft_render::write_png(&path, SIZE, SIZE, &pixels).expect("write golden");
    }
    let decoder = png::Decoder::new(std::io::BufReader::new(
        std::fs::File::open(&path).expect("open golden image"),
    ));
    let mut reader = decoder.read_info().expect("png header");
    let mut golden = vec![0; reader.output_buffer_size().expect("size")];
    let info = reader.next_frame(&mut golden).expect("frame");
    golden.truncate(info.buffer_size());
    let off = pixels
        .chunks(4)
        .zip(golden.chunks(4))
        .filter(|(a, b)| a.iter().zip(b.iter()).any(|(x, y)| x.abs_diff(*y) > 8))
        .count();
    let share = off as f64 / f64::from(SIZE * SIZE);
    assert!(share <= 0.005, "{:.3}% of pixels differ", share * 100.0);
}

// --- Task_26: wind arrow ---

use foamcraft_render::handles::{wind_arrow_mesh, wind_direction_deg};

#[test]
fn dragging_the_wind_arrow_gives_the_flow_direction() {
    let cam = OrbitCamera {
        yaw: -0.7,
        pitch: 0.9,
        ..Default::default()
    };
    let centre = Vec3::new(0.2, -0.1, 0.05);
    // The arrow sits upstream: grabbing it at -X of the centre means flow towards +X (0°).
    for (spot, expected) in [
        (Vec3::new(-1.0, 0.0, 0.0), 0.0f32),
        (Vec3::new(0.0, -1.0, 0.0), 90.0),
        (Vec3::new(1.0, 0.0, 0.0), 180.0),
        (Vec3::new(-0.866, -0.5, 0.0), 30.0),
    ] {
        let (x, y) = ndc(&cam, centre + spot * 0.3);
        let deg = wind_direction_deg(centre, ray_at(&cam, x, y)).expect("hits the plane");
        let diff = wrap_pi((deg - expected).to_radians()).to_degrees();
        assert!(diff.abs() < 0.05, "{spot}: {deg} vs {expected}");
    }
    // Snapping to 5° is done with `snap`.
    assert_eq!(snap(32.4, 5.0), 30.0);
    // Looking exactly horizontally the plane is edge-on: no value.
    let flat = OrbitCamera {
        pitch: 0.0,
        target: centre,
        ..Default::default()
    };
    assert!(wind_direction_deg(centre, ray_at(&flat, 0.0, 0.0)).is_none());
}

#[test]
fn the_wind_arrow_points_downstream_from_the_upstream_side() {
    for deg in [0.0f32, 30.0, -120.0] {
        let m = wind_arrow_mesh(Vec3::ZERO, 1.0, deg, 7);
        assert!(m.vertices.iter().all(|v| v.pick == 7));
        let flow = Vec3::new(deg.to_radians().cos(), deg.to_radians().sin(), 0.0);
        // Tip: the point furthest downstream; tail: furthest upstream.
        let along = |v: &foamcraft_render::scene::Vertex| Vec3::from(v.position).dot(flow);
        let tip = m.vertices.iter().map(along).fold(f32::MIN, f32::max);
        let tail = m.vertices.iter().map(along).fold(f32::MAX, f32::min);
        assert!(tip < -0.9 && tip > -1.6, "{deg}: tip {tip}");
        assert!(tail < -2.0, "{deg}: tail {tail}");
    }
}

#[test]
fn wind_arrow_at_a_cube_matches_golden_images() {
    let Some(r) = renderer() else { return };
    for (deg, file) in [(0.0f32, "wind_arrow_0.png"), (30.0, "wind_arrow_30.png")] {
        let scene = Scene::test_cube();
        let mut gpu = r.upload(&scene);
        r.set_handles(&mut gpu, &[wind_arrow_mesh(Vec3::ZERO, 0.87, deg, 1)]);
        let mut cam = OrbitCamera::default();
        cam.fit(Vec3::ZERO, 2.6, 1.0);
        let pixels = r.render_offscreen_gpu(&gpu, &cam, SIZE, SIZE);
        let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/golden")
            .join(file);
        if std::env::var_os("FOAMCRAFT_UPDATE_GOLDEN").is_some() {
            foamcraft_render::write_png(&path, SIZE, SIZE, &pixels).expect("write golden");
        }
        let decoder = png::Decoder::new(std::io::BufReader::new(
            std::fs::File::open(&path).expect("open golden image"),
        ));
        let mut reader = decoder.read_info().expect("png header");
        let mut golden = vec![0; reader.output_buffer_size().expect("size")];
        let info = reader.next_frame(&mut golden).expect("frame");
        golden.truncate(info.buffer_size());
        let off = pixels
            .chunks(4)
            .zip(golden.chunks(4))
            .filter(|(a, b)| a.iter().zip(b.iter()).any(|(x, y)| x.abs_diff(*y) > 8))
            .count();
        let share = off as f64 / f64::from(SIZE * SIZE);
        assert!(
            share <= 0.005,
            "{file}: {:.3}% of pixels differ",
            share * 100.0
        );
    }
}

// --- Task_27: how far a slice can move ---

use foamcraft_render::handles::slice_range;

#[test]
fn a_slice_moves_only_within_the_grid_box() {
    let (lo, hi) = ([0.0, 0.0, 0.0], [2.0, 1.0, 1.0]);
    let r = slice_range([0.5, 0.5, 0.5], [1.0, 0.0, 0.0], lo, hi);
    assert!(
        (r[0] + 0.5).abs() < 1e-6 && (r[1] - 1.5).abs() < 1e-6,
        "{r:?}"
    );
    // Tilted normal: the plane still touches the box between the extreme corners.
    let n = [1.0 / 2f32.sqrt(), 1.0 / 2f32.sqrt(), 0.0];
    let r = slice_range([1.0, 0.5, 0.5], n, lo, hi);
    let expected_lo = -(1.0 + 0.5) / 2f32.sqrt();
    let expected_hi = (1.0 + 0.5) / 2f32.sqrt();
    assert!(
        (r[0] - expected_lo).abs() < 1e-5 && (r[1] - expected_hi).abs() < 1e-5,
        "{r:?}"
    );
}
