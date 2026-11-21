// SPDX-License-Identifier: GPL-3.0-or-later
//! The ride in the window (Task_32): playing, pausing, jumping, looking around, the entry move
//! from the orbit view and the overview inset. The ride itself (path, timing, camera track) comes
//! from `foamcraft_api::ride`, so the window shows exactly what the export renders.

use std::time::Instant;

use foamcraft_api::ride::Ride;
use foamcraft_render::GpuScene;
use foamcraft_render::camera::{FreeCamera, OrbitCamera};
use foamcraft_render::glam::Vec3;
use serde::{Deserialize, Serialize};

/// Seconds of the move from the orbit view onto the ride.
pub const ENTRY_SECONDS: f32 = 0.8;
/// Seconds the look-around springs back in (to about 5 %).
const LOOK_BACK_SECONDS: f64 = 0.5;

pub struct Flight {
    pub ride: Ride,
    /// Screen time, s.
    pub t: f64,
    pub playing: bool,
    /// Look-around against the direction of travel (sideways, up), radians.
    pub look: (f64, f64),
    /// Looking around is held while the mouse drags (until this moment).
    pub look_until: Option<Instant>,
    /// The overview inset: the project scene uploaded once more.
    pub inset: GpuScene,
    /// The orbit camera before the ride: the overview, and where the ride returns to.
    pub before: OrbitCamera,
    /// Start of the entry move.
    pub entry: Option<Instant>,
    pub last: Instant,
}

/// The free camera that looks like an orbit camera.
pub fn free_of(c: &OrbitCamera) -> FreeCamera {
    let eye = c.eye();
    let forward = (c.target - eye).normalize_or(Vec3::X);
    let up = (Vec3::Z - forward * Vec3::Z.dot(forward)).normalize_or(Vec3::Y);
    FreeCamera {
        eye,
        forward,
        up,
        fov_y: c.fov_y,
        near: (c.distance * 0.01).max(1e-4),
        far: c.distance * 100.0,
    }
}

fn smooth(x: f32) -> f32 {
    let x = x.clamp(0.0, 1.0);
    x * x * (3.0 - 2.0 * x)
}

fn blend(a: &FreeCamera, b: &FreeCamera, k: f32) -> FreeCamera {
    let forward = a.forward.lerp(b.forward, k).normalize_or(b.forward);
    let up = a.up.lerp(b.up, k);
    let up = (up - forward * up.dot(forward)).normalize_or(b.up);
    FreeCamera {
        eye: a.eye.lerp(b.eye, k),
        forward,
        up,
        fov_y: a.fov_y + (b.fov_y - a.fov_y) * k,
        // Never cut into the model on the way in.
        near: a.near.min(b.near),
        far: a.far.max(b.far),
    }
}

impl Flight {
    pub fn new(ride: Ride, inset: GpuScene, before: OrbitCamera) -> Self {
        Self {
            ride,
            t: 0.0,
            playing: true,
            look: (0.0, 0.0),
            look_until: None,
            inset,
            before,
            entry: Some(Instant::now()),
            last: Instant::now(),
        }
    }

    pub fn view(&self) -> &str {
        &self.ride.spec.view
    }

    /// The camera of the main view; `None` in the overview (the orbit camera draws then).
    pub fn camera(&self) -> Option<FreeCamera> {
        let cam = self.ride.camera(self.t, self.view(), self.look)?;
        match self.entry {
            Some(start) => {
                let k = smooth(start.elapsed().as_secs_f32() / ENTRY_SECONDS);
                Some(blend(&free_of(&self.before), &cam, k))
            }
            None => Some(cam),
        }
    }

    /// Advances the clock; answers the flow time that passed (for the particles) and whether
    /// the picture changed.
    pub fn tick(&mut self) -> (f64, bool) {
        let now = Instant::now();
        let dt = now.duration_since(self.last).as_secs_f64().min(0.1);
        self.last = now;
        let mut changed = false;
        if let Some(start) = self.entry {
            changed = true;
            if start.elapsed().as_secs_f32() >= ENTRY_SECONDS {
                self.entry = None;
            }
        }
        let mut flow = 0.0;
        if self.playing {
            let end = self.ride.duration();
            let next = (self.t + dt).min(end);
            flow = self.ride.flow_dt(self.t, next - self.t);
            self.t = next;
            if next >= end {
                self.playing = false;
            }
            changed = true;
        }
        let held = self.look_until.is_some_and(|u| now < u);
        if !held && (self.look.0 != 0.0 || self.look.1 != 0.0) {
            // Exponential spring: 5 % left after LOOK_BACK_SECONDS.
            let k = (-3.0 * dt / LOOK_BACK_SECONDS).exp();
            self.look = (self.look.0 * k, self.look.1 * k);
            if self.look.0.abs() < 1e-3 && self.look.1.abs() < 1e-3 {
                self.look = (0.0, 0.0);
            }
            changed = true;
        }
        (flow, changed)
    }

    pub fn clock(&self) -> RideClock {
        RideClock {
            t: self.t,
            duration: self.ride.duration(),
            playing: self.playing,
        }
    }

    pub fn apply(&mut self, action: RideAction) {
        match action {
            RideAction::Play => {
                if self.t >= self.ride.duration() {
                    self.t = 0.0;
                }
                self.playing = true;
                self.last = Instant::now();
            }
            RideAction::Pause => self.playing = false,
            RideAction::Seek { t } => self.t = t.clamp(0.0, self.ride.duration()),
            RideAction::Look { dx, dy } => {
                self.look_until = Some(Instant::now() + std::time::Duration::from_millis(150));
                self.look = (
                    (self.look.0 + dx).clamp(-150f64.to_radians(), 150f64.to_radians()),
                    (self.look.1 + dy).clamp(-80f64.to_radians(), 80f64.to_radians()),
                );
            }
            RideAction::LookEnd => self.look_until = None,
            RideAction::State => {}
        }
    }
}

/// Playback actions of the window (not commands of the API: they steer the picture only).
#[derive(Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum RideAction {
    Play,
    Pause,
    /// Jump to screen time `t` (s).
    Seek {
        t: f64,
    },
    /// Look around while the mouse is dragged (radians, added up).
    Look {
        dx: f64,
        dy: f64,
    },
    LookEnd,
    State,
}

/// Clock of the ride, sent with `viewport://ride` every frame.
#[derive(Debug, Clone, Copy, Serialize)]
pub struct RideClock {
    pub t: f64,
    pub duration: f64,
    pub playing: bool,
}

/// The particle nearest to pixel (`x`, `y`) within `radius` pixels, in front of `blocker` (the
/// distance from the eye to the model under the pixel, if any).
pub fn nearest_particle(
    positions: &[[f32; 4]],
    yaw_deg: f32,
    camera: &OrbitCamera,
    size: (u32, u32),
    (x, y): (f32, f32),
    radius: f32,
    blocker: Option<f32>,
) -> Option<Vec3> {
    let (w, h) = (size.0.max(1) as f32, size.1.max(1) as f32);
    let vp = camera.view_proj(w / h);
    let rot = foamcraft_render::glam::Mat3::from_rotation_z(yaw_deg.to_radians());
    let eye = camera.eye();
    let mut best: Option<(f32, f32, Vec3)> = None;
    for p in positions {
        let world = rot * Vec3::new(p[0], p[1], p[2]);
        let c = vp * world.extend(1.0);
        if c.w <= 0.0 {
            continue;
        }
        let (px, py) = ((c.x / c.w + 1.0) * 0.5 * w, (1.0 - c.y / c.w) * 0.5 * h);
        let d = ((px - x).powi(2) + (py - y).powi(2)).sqrt();
        if d > radius {
            continue;
        }
        let depth = (world - eye).length();
        if blocker.is_some_and(|b| depth > b) {
            continue;
        }
        // Among those under the mouse the front one wins; then the nearest to the click.
        if best.is_none_or(|(bd, bdepth, _)| d < bd - 1.0 || (d <= bd + 1.0 && depth < bdepth)) {
            best = Some((d, depth, world));
        }
    }
    best.map(|b| b.2)
}
