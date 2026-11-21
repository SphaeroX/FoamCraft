// SPDX-License-Identifier: GPL-3.0-or-later
//! Orbit camera. World space is Z-up and in metres.

use glam::{Mat4, Vec3};

/// Smallest allowed distance between eye and target.
const MIN_DISTANCE: f32 = 1e-4;
/// Pitch stays just short of straight up or down so the view basis is never degenerate.
const MAX_PITCH: f32 = std::f32::consts::FRAC_PI_2 - 1e-3;

/// A camera that orbits around a target point.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OrbitCamera {
    pub target: Vec3,
    pub distance: f32,
    /// Rotation around the Z axis in radians.
    pub yaw: f32,
    /// Elevation above the XY plane in radians.
    pub pitch: f32,
    /// Vertical field of view in radians.
    pub fov_y: f32,
}

impl Default for OrbitCamera {
    fn default() -> Self {
        Self {
            target: Vec3::ZERO,
            distance: 5.0,
            yaw: -60f32.to_radians(),
            pitch: 25f32.to_radians(),
            fov_y: 45f32.to_radians(),
        }
    }
}

impl OrbitCamera {
    /// Unit vector from the target to the eye.
    fn offset_dir(&self) -> Vec3 {
        let (sy, cy) = self.yaw.sin_cos();
        let (sp, cp) = self.pitch.sin_cos();
        Vec3::new(cp * cy, cp * sy, sp)
    }

    pub fn eye(&self) -> Vec3 {
        self.target + self.offset_dir() * self.distance
    }

    /// Rotates around the target by the given angles in radians.
    pub fn orbit(&mut self, delta_yaw: f32, delta_pitch: f32) {
        self.yaw += delta_yaw;
        self.pitch = (self.pitch + delta_pitch).clamp(-MAX_PITCH, MAX_PITCH);
    }

    /// Multiplies the distance by `factor` (< 1 moves closer). Non-positive factors are ignored.
    pub fn zoom(&mut self, factor: f32) {
        if factor > 0.0 && factor.is_finite() {
            self.distance = (self.distance * factor).max(MIN_DISTANCE);
        }
    }

    /// Moves the target in the view plane. `dx` and `dy` are fractions of the view height.
    pub fn pan(&mut self, dx: f32, dy: f32) {
        let forward = -self.offset_dir();
        let right = forward.cross(Vec3::Z).normalize();
        let up = right.cross(forward);
        let view_height = 2.0 * self.distance * (self.fov_y / 2.0).tan();
        self.target += (-right * dx + up * dy) * view_height;
    }

    /// Centres on a bounding sphere so it fills the narrower side of the view.
    pub fn fit(&mut self, center: Vec3, radius: f32, aspect: f32) {
        let half_y = self.fov_y / 2.0;
        let half_x = (half_y.tan() * aspect).atan();
        let half = half_y.min(half_x);
        self.target = center;
        self.distance = (radius / half.sin()).max(MIN_DISTANCE);
    }

    pub fn view(&self) -> Mat4 {
        glam::camera::rh::view::look_at_mat4(self.eye(), self.target, Vec3::Z)
    }

    /// Projection with depth in 0..1 (wgpu convention).
    pub fn projection(&self, aspect: f32) -> Mat4 {
        let near = (self.distance * 0.01).max(MIN_DISTANCE);
        let far = self.distance * 100.0;
        glam::camera::rh::proj::directx::perspective(self.fov_y, aspect, near, far)
    }

    pub fn view_proj(&self, aspect: f32) -> Mat4 {
        self.projection(aspect) * self.view()
    }

    /// Ray through a point of the view in normalised device coordinates (x right, y up, -1..1):
    /// origin on the near plane and unit direction.
    pub fn ray(&self, aspect: f32, ndc_x: f32, ndc_y: f32) -> (Vec3, Vec3) {
        let inverse = self.view_proj(aspect).inverse();
        let near = inverse.project_point3(Vec3::new(ndc_x, ndc_y, 0.0));
        let far = inverse.project_point3(Vec3::new(ndc_x, ndc_y, 1.0));
        (near, (far - near).normalize_or_zero())
    }
}

/// What the renderer needs from a camera; the orbit camera and the free camera of the ride
/// both provide it.
pub trait ViewCamera {
    fn view_proj(&self, aspect: f32) -> Mat4;
    fn eye(&self) -> Vec3;
}

impl ViewCamera for OrbitCamera {
    fn view_proj(&self, aspect: f32) -> Mat4 {
        OrbitCamera::view_proj(self, aspect)
    }

    fn eye(&self) -> Vec3 {
        OrbitCamera::eye(self)
    }
}

/// A camera placed freely: eye, view direction and up, with its own clip planes (the ride sets
/// the near plane from the wall distance so the view never cuts into the model).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FreeCamera {
    pub eye: Vec3,
    /// Unit view direction.
    pub forward: Vec3,
    /// Unit up direction, perpendicular to `forward`.
    pub up: Vec3,
    /// Vertical field of view in radians.
    pub fov_y: f32,
    pub near: f32,
    pub far: f32,
}

impl FreeCamera {
    pub fn view(&self) -> Mat4 {
        glam::camera::rh::view::look_at_mat4(self.eye, self.eye + self.forward, self.up)
    }

    /// Projection with depth in 0..1 (wgpu convention).
    pub fn projection(&self, aspect: f32) -> Mat4 {
        let near = self.near.max(1e-6);
        glam::camera::rh::proj::directx::perspective(
            self.fov_y,
            aspect,
            near,
            self.far.max(near * 2.0),
        )
    }

    /// The orbit camera that looks from the same eye in the same direction (for transitions).
    pub fn as_orbit(&self, distance: f32) -> OrbitCamera {
        let d = -self.forward.normalize_or(Vec3::X);
        OrbitCamera {
            target: self.eye - d * distance,
            distance,
            yaw: d.y.atan2(d.x),
            pitch: d.z.clamp(-1.0, 1.0).asin().clamp(-MAX_PITCH, MAX_PITCH),
            fov_y: self.fov_y,
        }
    }
}

impl ViewCamera for FreeCamera {
    fn view_proj(&self, aspect: f32) -> Mat4 {
        self.projection(aspect) * self.view()
    }

    fn eye(&self) -> Vec3 {
        self.eye
    }
}

/// Names of the standard views: six faces, twelve edges, eight corners and `default`.
/// Faces are seen from that side: `front` looks from -X towards +X (against the default flow).
pub const VIEW_NAMES: [&str; 27] = [
    "front",
    "back",
    "left",
    "right",
    "top",
    "bottom",
    "front_left",
    "front_right",
    "back_left",
    "back_right",
    "top_front",
    "top_back",
    "top_left",
    "top_right",
    "bottom_front",
    "bottom_back",
    "bottom_left",
    "bottom_right",
    "top_front_left",
    "top_front_right",
    "top_back_left",
    "top_back_right",
    "bottom_front_left",
    "bottom_front_right",
    "bottom_back_left",
    "bottom_back_right",
    "default",
];

/// Yaw and pitch in radians of a standard view (see [`VIEW_NAMES`]); `None` for an unknown name.
/// Top and bottom stop just short of the pole so the view never flips.
pub fn named_view(name: &str) -> Option<(f32, f32)> {
    if name == "default" {
        let d = OrbitCamera::default();
        return Some((d.yaw, d.pitch));
    }
    // Eye direction from the parts of the name, then yaw and pitch from it.
    let mut dir = Vec3::ZERO;
    let mut parts = 0;
    for part in name.split('_') {
        dir += match part {
            "front" => Vec3::NEG_X,
            "back" => Vec3::X,
            "left" => Vec3::Y,
            "right" => Vec3::NEG_Y,
            "top" => Vec3::Z,
            "bottom" => Vec3::NEG_Z,
            _ => return None,
        };
        parts += 1;
    }
    if !VIEW_NAMES.contains(&name) || parts == 0 {
        return None;
    }
    let horizontal = Vec3::new(dir.x, dir.y, 0.0);
    // Straight up or down: keep yaw at 180° so the front is at the bottom of the image.
    let yaw = if horizontal.length_squared() < 1e-9 {
        std::f32::consts::PI
    } else {
        dir.y.atan2(dir.x)
    };
    let pitch = dir
        .z
        .atan2(horizontal.length())
        .clamp(-MAX_PITCH, MAX_PITCH);
    Some((yaw, pitch))
}

/// Short camera move from one view to another. Yaw takes the shorter way round; the move eases
/// in and out and ends exactly on the target.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CameraTransition {
    from: OrbitCamera,
    to: OrbitCamera,
    duration: f32,
}

impl CameraTransition {
    pub fn new(from: OrbitCamera, to: OrbitCamera, duration: f32) -> Self {
        use std::f32::consts::{PI, TAU};
        let mut to = to;
        // Bring the target yaw within ±180° of the start so the camera turns the short way.
        let delta = (to.yaw - from.yaw + PI).rem_euclid(TAU) - PI;
        to.yaw = from.yaw + delta;
        Self {
            from,
            to,
            duration: duration.max(0.0),
        }
    }

    /// Duration in seconds.
    pub fn duration(&self) -> f32 {
        self.duration
    }

    /// True once `elapsed` seconds have passed the end.
    pub fn finished(&self, elapsed: f32) -> bool {
        elapsed >= self.duration
    }

    /// The camera `elapsed` seconds after the start.
    pub fn at(&self, elapsed: f32) -> OrbitCamera {
        if self.duration <= 0.0 || elapsed >= self.duration {
            return self.to;
        }
        if elapsed <= 0.0 {
            return self.from;
        }
        let s = elapsed / self.duration;
        let e = s * s * (3.0 - 2.0 * s);
        let lerp = |a: f32, b: f32| a + (b - a) * e;
        OrbitCamera {
            target: self.from.target.lerp(self.to.target, e),
            distance: lerp(self.from.distance, self.to.distance),
            yaw: lerp(self.from.yaw, self.to.yaw),
            pitch: lerp(self.from.pitch, self.to.pitch),
            fov_y: lerp(self.from.fov_y, self.to.fov_y),
        }
    }
}
