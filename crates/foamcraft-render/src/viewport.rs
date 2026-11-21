// SPDX-License-Identifier: GPL-3.0-or-later
//! A window surface that shows the scene.

use crate::camera::OrbitCamera;
use crate::renderer::GpuScene;
use crate::renderer::Renderer;

pub struct Viewport {
    surface: wgpu::Surface<'static>,
    config: wgpu::SurfaceConfiguration,
}

impl Viewport {
    /// Configures `surface` for `renderer`. Prefers a non-sRGB format so colours match offscreen
    /// frames exactly.
    pub fn new(
        renderer: &Renderer,
        surface: wgpu::Surface<'static>,
        width: u32,
        height: u32,
    ) -> Option<Self> {
        let caps = surface.get_capabilities(&renderer.adapter);
        let format = caps
            .formats
            .iter()
            .copied()
            .find(|f| !f.is_srgb())
            .or_else(|| caps.formats.first().copied())?;
        let alpha_mode = if caps.alpha_modes.contains(&wgpu::CompositeAlphaMode::Opaque) {
            wgpu::CompositeAlphaMode::Opaque
        } else {
            *caps.alpha_modes.first()?
        };
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width: width.max(1),
            height: height.max(1),
            present_mode: wgpu::PresentMode::AutoVsync,
            desired_maximum_frame_latency: 2,
            alpha_mode,
            view_formats: vec![],
            color_space: wgpu::SurfaceColorSpace::Auto,
        };
        surface.configure(&renderer.device, &config);
        Some(Self { surface, config })
    }

    pub fn size(&self) -> (u32, u32) {
        (self.config.width, self.config.height)
    }

    pub fn resize(&mut self, renderer: &Renderer, width: u32, height: u32) {
        if width == 0 || height == 0 {
            return;
        }
        self.config.width = width;
        self.config.height = height;
        self.surface.configure(&renderer.device, &self.config);
    }

    /// Draws one frame. Lost or outdated surfaces are reconfigured and skipped.
    pub fn render(&mut self, renderer: &Renderer, scene: &GpuScene, camera: &OrbitCamera) {
        let frame = match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(frame)
            | wgpu::CurrentSurfaceTexture::Suboptimal(frame) => frame,
            _ => {
                self.surface.configure(&renderer.device, &self.config);
                return;
            }
        };
        let view = frame.texture.create_view(&Default::default());
        renderer.render_gpu(
            scene,
            camera,
            &view,
            self.config.format,
            self.config.width,
            self.config.height,
        );
        renderer.queue.present(frame);
    }

    /// Draws two scenes side by side with one camera, in the columns `span` (first, count).
    pub fn render_split(
        &mut self,
        renderer: &Renderer,
        left: &GpuScene,
        right: &GpuScene,
        camera: &OrbitCamera,
        span: (u32, u32),
    ) {
        let frame = match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(frame)
            | wgpu::CurrentSurfaceTexture::Suboptimal(frame) => frame,
            _ => {
                self.surface.configure(&renderer.device, &self.config);
                return;
            }
        };
        let view = frame.texture.create_view(&Default::default());
        renderer.render_split_gpu(
            left,
            right,
            camera,
            &view,
            self.config.format,
            self.config.width,
            self.config.height,
            span,
        );
        renderer.queue.present(frame);
    }
}
