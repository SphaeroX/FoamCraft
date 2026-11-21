// SPDX-License-Identifier: GPL-3.0-or-later
use foamcraft_render::camera::OrbitCamera;
use glam::Vec3;

const EPS: f32 = 1e-4;

#[test]
fn eye_lies_at_distance_from_target() {
    let cam = OrbitCamera {
        target: Vec3::new(1.0, 2.0, 3.0),
        distance: 7.0,
        ..Default::default()
    };
    assert!((cam.eye().distance(cam.target) - 7.0).abs() < EPS);
}

#[test]
fn z_is_up() {
    let cam = OrbitCamera {
        pitch: 0.5,
        ..Default::default()
    };
    assert!(cam.eye().z > cam.target.z);
}

#[test]
fn orbit_keeps_distance_and_clamps_pitch() {
    let mut cam = OrbitCamera::default();
    let d = cam.distance;
    cam.orbit(0.7, 10.0);
    assert!((cam.eye().distance(cam.target) - d).abs() < EPS);
    assert!(cam.pitch < std::f32::consts::FRAC_PI_2);
    cam.orbit(0.0, -20.0);
    assert!(cam.pitch > -std::f32::consts::FRAC_PI_2);
}

#[test]
fn orbit_changes_yaw() {
    let mut cam = OrbitCamera::default();
    let yaw = cam.yaw;
    cam.orbit(0.3, 0.0);
    assert!((cam.yaw - yaw - 0.3).abs() < EPS);
}

#[test]
fn zoom_scales_distance_and_stays_positive() {
    let mut cam = OrbitCamera {
        distance: 10.0,
        ..Default::default()
    };
    cam.zoom(0.5);
    assert!((cam.distance - 5.0).abs() < EPS);
    for _ in 0..200 {
        cam.zoom(0.1);
    }
    assert!(cam.distance > 0.0);
    cam.zoom(-3.0);
    assert!(cam.distance > 0.0);
}

#[test]
fn pan_moves_target_perpendicular_to_view() {
    let mut cam = OrbitCamera::default();
    let before = cam.target;
    let view_dir = (cam.target - cam.eye()).normalize();
    let d = cam.distance;
    cam.pan(0.1, 0.2);
    let moved = cam.target - before;
    assert!(moved.length() > 0.0);
    assert!(moved.dot(view_dir).abs() < EPS);
    assert!((cam.distance - d).abs() < EPS);
}

#[test]
fn fit_puts_sphere_inside_view() {
    let mut cam = OrbitCamera::default();
    let center = Vec3::new(5.0, -2.0, 1.0);
    let radius = 3.0;
    for aspect in [0.5_f32, 1.0, 2.0] {
        cam.fit(center, radius, aspect);
        assert!((cam.target - center).length() < EPS);
        let half_fov_y = cam.fov_y / 2.0;
        let half_fov_x = (half_fov_y.tan() * aspect).atan();
        let half = half_fov_y.min(half_fov_x);
        // The sphere touches the narrower frustum side.
        assert!((cam.distance * half.sin() - radius).abs() < 1e-3);
    }
}

#[test]
fn view_projection_maps_target_to_screen_centre() {
    let cam = OrbitCamera::default();
    let clip = cam.view_proj(1.5) * cam.target.extend(1.0);
    let ndc = clip.truncate() / clip.w;
    assert!(ndc.x.abs() < EPS && ndc.y.abs() < EPS);
    assert!(ndc.z > 0.0 && ndc.z < 1.0);
}

#[test]
fn the_ray_through_the_screen_centre_hits_the_target() {
    use foamcraft_render::glam::Vec3;
    let mut camera = foamcraft_render::camera::OrbitCamera::default();
    camera.fit(Vec3::new(1.0, 2.0, 3.0), 0.5, 1.5);
    let (origin, dir) = camera.ray(1.5, 0.0, 0.0);
    assert!((dir.length() - 1.0).abs() < 1e-5);
    // Closest point of the ray to the target.
    let t = (Vec3::new(1.0, 2.0, 3.0) - origin).dot(dir);
    let closest = origin + dir * t;
    assert!(
        closest.distance(Vec3::new(1.0, 2.0, 3.0)) < 1e-3,
        "{closest}"
    );
    // A ray to the right of the centre points to the right of the view direction.
    let (_, right) = camera.ray(1.5, 0.5, 0.0);
    assert!(right.angle_between(dir) > 0.05);
}

// --- Task_24: named views and transitions ---

use foamcraft_render::camera::{CameraTransition, VIEW_NAMES, named_view};

fn view_dir(name: &str) -> Vec3 {
    let (yaw, pitch) = named_view(name).expect("known view");
    let cam = OrbitCamera {
        yaw,
        pitch,
        ..Default::default()
    };
    (cam.eye() - cam.target).normalize()
}

#[test]
fn faces_look_from_the_expected_side() {
    let cases = [
        ("front", Vec3::NEG_X),
        ("back", Vec3::X),
        ("left", Vec3::Y),
        ("right", Vec3::NEG_Y),
        ("top", Vec3::Z),
        ("bottom", Vec3::NEG_Z),
    ];
    for (name, eye_side) in cases {
        let d = view_dir(name);
        assert!(d.dot(eye_side) > 0.999, "{name}: eye direction {d}");
    }
}

#[test]
fn corners_and_edges_look_from_their_diagonal() {
    let c = view_dir("top_front_right");
    let expected = Vec3::new(-1.0, -1.0, 1.0).normalize();
    assert!(c.dot(expected) > 0.9999, "corner {c}");
    let c = view_dir("bottom_back_left");
    assert!(c.dot(Vec3::new(1.0, 1.0, -1.0).normalize()) > 0.9999);
    let e = view_dir("front_right");
    assert!(e.dot(Vec3::new(-1.0, -1.0, 0.0).normalize()) > 0.9999);
    let e = view_dir("top_back");
    assert!(e.dot(Vec3::new(1.0, 0.0, 1.0).normalize()) > 0.9999);
    let e = view_dir("bottom_left");
    assert!(e.dot(Vec3::new(0.0, 1.0, -1.0).normalize()) > 0.9999);
}

#[test]
fn all_named_views_exist_and_unknown_is_none() {
    // 6 faces, 12 edges, 8 corners and the default view.
    assert_eq!(VIEW_NAMES.len(), 27);
    for name in VIEW_NAMES {
        assert!(named_view(name).is_some(), "{name}");
    }
    assert!(named_view("sideways").is_none());
    let d = OrbitCamera::default();
    assert_eq!(named_view("default"), Some((d.yaw, d.pitch)));
}

#[test]
fn top_and_bottom_do_not_flip() {
    for name in ["top", "bottom"] {
        let (yaw, pitch) = named_view(name).unwrap();
        assert!(pitch.abs() < std::f32::consts::FRAC_PI_2);
        let cam = OrbitCamera {
            yaw,
            pitch,
            ..Default::default()
        };
        let m = cam.view();
        assert!(m.is_finite(), "{name}");
        // From above the front (-X) is at the bottom of the image, from below at the top, as when
        // tilting from the front view: screen up is +X for top and -X for bottom.
        let up_world = m.inverse().transform_vector3(Vec3::Y);
        let sign = if name == "top" { 1.0 } else { -1.0 };
        assert!(up_world.x * sign > 0.9, "{name}: {up_world}");
    }
}

#[test]
fn transition_takes_the_shorter_way_and_ends_on_target() {
    let start = OrbitCamera {
        yaw: 170f32.to_radians(),
        ..Default::default()
    };
    let mut target = start;
    target.yaw = -170f32.to_radians();
    target.pitch = 0.0;
    let tr = CameraTransition::new(start, target, 0.3);
    // Halfway the yaw passes 180°, not 0°.
    let mid = tr.at(0.15);
    let yaw = mid.yaw.rem_euclid(std::f32::consts::TAU).to_degrees();
    assert!((yaw - 180.0).abs() < 5.0, "mid yaw {yaw}");
    let end = tr.at(0.3);
    let dir = |c: &OrbitCamera| (c.eye() - c.target).normalize();
    assert!(dir(&end).dot(dir(&target)) > 0.99999);
    assert!((end.pitch - target.pitch).abs() < EPS);
    assert!(tr.finished(0.3));
    assert!(!tr.finished(0.1));
    assert_eq!(tr.at(0.0), start);
}

#[test]
fn transition_keeps_target_and_distance_moving_smoothly() {
    let start = OrbitCamera::default();
    let target = OrbitCamera {
        target: Vec3::new(1.0, 2.0, 3.0),
        distance: 9.0,
        ..start
    };
    let tr = CameraTransition::new(start, target, 1.0);
    let end = tr.at(2.0);
    assert!((end.distance - 9.0).abs() < EPS);
    assert!(end.target.distance(target.target) < EPS);
    let mid = tr.at(0.5);
    assert!(mid.distance > 5.0 && mid.distance < 9.0);
}
