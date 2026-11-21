// SPDX-License-Identifier: GPL-3.0-or-later
//! Camera paths for videos: key frames are met exactly, the motion is continuous, and the
//! prepared paths (turntable and others) start and end where they should.

use foamcraft_render::camera::OrbitCamera;
use foamcraft_render::camera_path::{CameraKey, interpolate, preset};

fn key(t: f32, yaw: f32, pitch: f32, distance: f32, target: [f32; 3]) -> CameraKey {
    CameraKey {
        t,
        yaw,
        pitch,
        distance,
        target,
    }
}

fn keys() -> Vec<CameraKey> {
    vec![
        key(0.0, 0.0, 0.2, 5.0, [0.0, 0.0, 0.0]),
        key(2.0, 1.5, 0.6, 3.0, [0.5, 0.0, 0.0]),
        key(3.0, 2.0, 0.1, 4.0, [0.5, 0.5, 0.2]),
        key(6.0, 4.0, 0.3, 6.0, [0.0, 0.0, 0.0]),
    ]
}

#[test]
fn the_key_frames_are_met_exactly() {
    let base = OrbitCamera::default();
    for k in keys() {
        let c = interpolate(&keys(), k.t, &base);
        assert!((c.yaw - k.yaw).abs() < 1e-6, "{} {}", c.yaw, k.yaw);
        assert!((c.pitch - k.pitch).abs() < 1e-6);
        assert!((c.distance - k.distance).abs() < 1e-5);
        assert!((c.target - foamcraft_render::glam::Vec3::from(k.target)).length() < 1e-6);
        assert_eq!(c.fov_y, base.fov_y);
    }
    // Before the first and after the last key the camera stands still.
    assert_eq!(interpolate(&keys(), -1.0, &base).yaw, 0.0);
    assert_eq!(interpolate(&keys(), 9.0, &base).yaw, 4.0);
}

#[test]
fn the_motion_is_continuous_and_smooth() {
    let base = OrbitCamera::default();
    let n = 6000;
    let dt = 6.0 / n as f32;
    let sample = |i: usize| interpolate(&keys(), i as f32 * dt, &base);
    let mut previous_speed: Option<f32> = None;
    for i in 1..=n {
        let (a, b) = (sample(i - 1), sample(i));
        // No jumps in position.
        let jump =
            (b.yaw - a.yaw).abs() + (b.pitch - a.pitch).abs() + (b.distance - a.distance).abs();
        assert!(jump < 0.01, "jump {jump} at {}", i as f32 * dt);
        // No kinks: the speed of the yaw changes gradually, also across key frames.
        let speed = (b.yaw - a.yaw) / dt;
        if let Some(p) = previous_speed {
            assert!(
                (speed - p).abs() < 0.05,
                "kink {p} → {speed} at {}",
                i as f32 * dt
            );
        }
        previous_speed = Some(speed);
    }
}

#[test]
fn prepared_paths_start_at_the_current_view() {
    let base = OrbitCamera {
        target: [1.0, 2.0, 0.5].into(),
        distance: 7.0,
        ..Default::default()
    };
    let turntable = preset("turntable", &base, 8.0).unwrap();
    let start = interpolate(&turntable, 0.0, &base);
    let half = interpolate(&turntable, 4.0, &base);
    let end = interpolate(&turntable, 8.0, &base);
    assert!((start.yaw - base.yaw).abs() < 1e-6);
    assert!((half.yaw - base.yaw - std::f32::consts::PI).abs() < 1e-4);
    assert!((end.yaw - base.yaw - std::f32::consts::TAU).abs() < 1e-4);
    assert!((end.pitch - base.pitch).abs() < 1e-6);
    assert!((end.target - base.target).length() < 1e-6);
    for name in ["swing", "rise", "approach"] {
        let path = preset(name, &base, 5.0).unwrap();
        let first = interpolate(&path, 0.0, &base);
        assert!((first.yaw - base.yaw).abs() < 1e-6, "{name}");
        assert!((first.distance - base.distance).abs() < 1e-4, "{name}");
        assert!(path.last().unwrap().t <= 5.0 + 1e-6, "{name}");
    }
    assert!(preset("unknown", &base, 5.0).is_none());
}
