// SPDX-License-Identifier: GPL-3.0-or-later
//! Camera paths for videos: key frames of the orbit camera, interpolated with cubic Hermite
//! curves whose tangents follow the neighbouring keys (Catmull-Rom for uneven spacing). The path
//! meets every key exactly and its speed changes without jumps. Yaw is not wrapped, so a full
//! turn is a yaw change of 2π.

use glam::Vec3;
use serde::{Deserialize, Serialize};

use crate::camera::OrbitCamera;

/// One key frame: time in seconds and the orbit camera there.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct CameraKey {
    pub t: f32,
    /// Radians around the vertical axis.
    pub yaw: f32,
    /// Radians above the horizontal plane.
    pub pitch: f32,
    /// Metres from eye to target.
    pub distance: f32,
    /// Point looked at, metres.
    pub target: [f32; 3],
}

impl CameraKey {
    pub fn from_camera(t: f32, c: &OrbitCamera) -> Self {
        Self {
            t,
            yaw: c.yaw,
            pitch: c.pitch,
            distance: c.distance,
            target: c.target.into(),
        }
    }

    fn values(&self) -> [f32; 6] {
        [
            self.yaw,
            self.pitch,
            self.distance,
            self.target[0],
            self.target[1],
            self.target[2],
        ]
    }
}

/// The camera at time `t`; field of view from `base`. Keys must be sorted by time.
pub fn interpolate(keys: &[CameraKey], t: f32, base: &OrbitCamera) -> OrbitCamera {
    let to_camera = |v: [f32; 6]| OrbitCamera {
        yaw: v[0],
        pitch: v[1],
        distance: v[2],
        target: Vec3::new(v[3], v[4], v[5]),
        fov_y: base.fov_y,
    };
    let (Some(first), Some(last)) = (keys.first(), keys.last()) else {
        return *base;
    };
    if keys.len() == 1 || t <= first.t {
        return to_camera(first.values());
    }
    if t >= last.t {
        return to_camera(last.values());
    }
    let i = keys
        .windows(2)
        .position(|w| t < w[1].t)
        .unwrap_or(keys.len() - 2);
    let (k0, k1) = (&keys[i], &keys[i + 1]);
    let h = (k1.t - k0.t).max(1e-9);
    // Slope at key j: from the neighbours, one-sided at the ends.
    let slope = |j: usize, c: usize| -> f32 {
        let (a, b) = (j.saturating_sub(1), (j + 1).min(keys.len() - 1));
        let dt = (keys[b].t - keys[a].t).max(1e-9);
        (keys[b].values()[c] - keys[a].values()[c]) / dt
    };
    let s = (t - k0.t) / h;
    let (s2, s3) = (s * s, s * s * s);
    let (h00, h10, h01, h11) = (
        2.0 * s3 - 3.0 * s2 + 1.0,
        s3 - 2.0 * s2 + s,
        -2.0 * s3 + 3.0 * s2,
        s3 - s2,
    );
    let (v0, v1) = (k0.values(), k1.values());
    let mut out = [0.0; 6];
    for (c, o) in out.iter_mut().enumerate() {
        *o = h00 * v0[c] + h10 * h * slope(i, c) + h01 * v1[c] + h11 * h * slope(i + 1, c);
    }
    to_camera(out)
}

/// Prepared paths starting at the current view: `turntable` (one full turn), `swing` (45° to each
/// side and back), `rise` (half a turn while rising to a view from above), `approach` (half a
/// turn while moving closer). `None` for an unknown name.
pub fn preset(name: &str, base: &OrbitCamera, duration: f32) -> Option<Vec<CameraKey>> {
    use std::f32::consts::{FRAC_PI_4, PI, TAU};
    let d = duration.max(0.1);
    let at = |t: f32, yaw: f32, pitch: f32, distance: f32| CameraKey {
        t,
        yaw: base.yaw + yaw,
        pitch: base.pitch + pitch,
        distance: base.distance * distance,
        target: base.target.into(),
    };
    Some(match name {
        "turntable" => vec![at(0.0, 0.0, 0.0, 1.0), at(d, TAU, 0.0, 1.0)],
        "swing" => vec![
            at(0.0, 0.0, 0.0, 1.0),
            at(d * 0.25, FRAC_PI_4, 0.0, 1.0),
            at(d * 0.75, -FRAC_PI_4, 0.0, 1.0),
            at(d, 0.0, 0.0, 1.0),
        ],
        "rise" => {
            let up = (1.2 - base.pitch).max(0.0);
            vec![at(0.0, 0.0, 0.0, 1.0), at(d, PI, up, 1.0)]
        }
        "approach" => vec![at(0.0, 0.0, 0.0, 1.0), at(d, PI, 0.0, 0.6)],
        _ => return None,
    })
}
