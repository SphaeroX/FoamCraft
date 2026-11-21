// SPDX-License-Identifier: GPL-3.0-or-later
//! wgpu renderer of FoamCraft: one renderer for the viewport and offscreen frames.

pub mod camera;
pub mod camera_path;
mod image;
pub mod jpeg;
pub mod overlay;
pub mod particles;
mod renderer;
pub mod scene;
pub mod video;
mod viewport;

pub use glam;
pub use image::write_png;
pub use renderer::{GpuScene, OFFSCREEN_FORMAT, Renderer, default_instance};
pub use scene::Scene;
pub use viewport::Viewport;
pub use wgpu;
