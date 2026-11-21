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
