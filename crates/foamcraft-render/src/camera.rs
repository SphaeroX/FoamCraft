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
