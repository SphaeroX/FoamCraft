// SPDX-License-Identifier: GPL-3.0-or-later
//! Light trails (Task_35): particles that keep the last K head positions in a ring on the GPU
//! (`particles_trails.wgsl`), drawn as wide, fading lines in screen space into a floating-point
//! light image that adds up (`trails.wgsl`), with an optional soft glow and a composition onto
//! the scene (`trails_post.wgsl`). Without trails nothing of this runs: every other picture
//! stays exactly as before.

use std::sync::Mutex;

use bytemuck::{Pod, Zeroable};
use wgpu::util::DeviceExt;

use crate::camera::ViewCamera;
use crate::particles::{FlowLevel, GpuParticles, ParticleSettings};
use crate::renderer::{DEPTH_FORMAT, Renderer};

/// Format of the light image.
pub const LIGHT_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba16Float;
/// Most trail points over all particles (about 38 MB).
pub const MAX_POINTS: u64 = 2_400_000;
/// Points per trail by default.
pub const DEFAULT_K: u32 = 24;

/// How long the trails are and how they look.
#[derive(Debug, Clone, PartialEq)]
pub struct TrailSettings {
    /// Points per particle (at least 2).
    pub k: u32,
    /// A point every `every` steps.
    pub every: u32,
    /// Width at the head, pixels.
    pub width_px: f32,
    /// Brightness of one trail.
    pub exposure: f32,
    pub glow: bool,
    /// Colour by speed (`range`) or the constant colour.
    pub by_speed: bool,
    pub range: [f32; 2],
    pub color: [f32; 3],
}

/// Points per trail for `count` particles within the memory limit: `wanted`, or fewer (then the
/// interface says "trails shortened").
pub fn points_per_trail(count: u32, wanted: u32) -> u32 {
    let most = (MAX_POINTS / u64::from(count.max(1))).min(u64::from(u32::MAX)) as u32;
    wanted.min(most).max(2)
}

/// Slot of the point `j` behind the head when the head is in `slot` (the same in the shader).
pub fn slot_behind(slot: u32, j: u32, k: u32) -> u32 {
    (slot + k - (j % k)) % k
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct TrailUniform {
    k: u32,
    slot: u32,
    write: u32,
    _pad: u32,
    window: f32,
    _a: f32,
    _b: f32,
    _c: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct ViewUniform {
    view_proj: [[f32; 4]; 4],
    rot: [f32; 4],
    ring: [u32; 4],
    style: [f32; 4],
    color: [f32; 4],
}

/// The ring of a particle set on the GPU.
pub struct GpuTrails {
    pub(crate) ring: wgpu::Buffer,
    uniform: wgpu::Buffer,
    pub settings: TrailSettings,
    /// Slot of the newest point and the number of points written (at most K).
    pub(crate) slot: u32,
    pub(crate) written: u32,
    window: f32,
    yaw: [f32; 2],
    /// Long exposure: draw the scene without trails (they are added step by step).
    pub(crate) suppress: bool,
    /// Long exposure in the view: half-life of the light, s (none: a fresh light image per frame).
    pub(crate) fade: Option<f32>,
    /// Points written so far (all steps).
    pub(crate) writes: u64,
    /// What the light image of the view holds: camera, size and points when last drawn.
    memory: Mutex<Option<FadeMemory>>,
}

#[derive(Clone, Copy)]
struct FadeMemory {
    view_proj: [[f32; 4]; 4],
    size: (u32, u32),
    region: [u32; 4],
    writes: u64,
    at: std::time::Instant,
}

impl GpuTrails {
    pub(crate) fn uniform(&self) -> &wgpu::Buffer {
        &self.uniform
    }

    /// Before a step: which slot it writes and the flow time since the last write.
    pub(crate) fn before_step(&mut self, queue: &wgpu::Queue, frame: u32, dt: f32) {
        self.window += dt;
        let write = frame.is_multiple_of(self.settings.every.max(1));
        if write {
            self.writes += 1;
            self.slot = (self.slot + 1) % self.settings.k;
            self.written = (self.written + 1).min(self.settings.k);
        }
        let u = TrailUniform {
            k: self.settings.k,
            slot: self.slot,
            write: u32::from(write),
            _pad: 0,
            window: self.window,
            _a: 0.0,
            _b: 0.0,
            _c: 0.0,
        };
        queue.write_buffer(&self.uniform, 0, bytemuck::bytes_of(&u));
        if write {
            self.window = 0.0;
        }
    }
}

/// Light image, its smaller copies for the glow, and their sampler, for one size.
pub(crate) struct TrailTargets {
    size: (u32, u32),
    light: wgpu::TextureView,
    small: [wgpu::TextureView; 2],
    sampler: wgpu::Sampler,
}

/// Pipelines and layouts of the trails, made on first use.
pub(crate) struct TrailKit {
    trail_layout: wgpu::BindGroupLayout,
    trail: wgpu::RenderPipeline,
    post_layout: wgpu::BindGroupLayout,
    down: wgpu::RenderPipeline,
    blur: wgpu::RenderPipeline,
    fade: wgpu::RenderPipeline,
    post_module: wgpu::ShaderModule,
    post_pipeline_layout: wgpu::PipelineLayout,
    compose: Mutex<std::collections::HashMap<wgpu::TextureFormat, wgpu::RenderPipeline>>,
    targets: Mutex<Option<TrailTargets>>,
}

impl Renderer {
    /// Particles with light trails: like `create_particles`, plus a ring of `trail.k` points per
    /// particle.
    pub fn create_trail_particles(
        &self,
        levels: &[FlowLevel],
        seeds: &[[f32; 3]],
        s: &ParticleSettings,
        trail: &TrailSettings,
    ) -> GpuParticles {
        let mut t = trail.clone();
        t.k = points_per_trail(s.count, t.k.max(2));
        let count = s.count.max(1);
        let ring = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("trail ring"),
            size: u64::from(count) * u64::from(t.k) * 16,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        let u = TrailUniform {
            k: t.k,
            slot: 0,
            write: 1,
            _pad: 0,
            window: 0.0,
            _a: 0.0,
            _b: 0.0,
            _c: 0.0,
        };
        let uniform = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("trail"),
                contents: bytemuck::bytes_of(&u),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            });
        let rad = s.yaw_deg.to_radians();
        let trails = GpuTrails {
            ring,
            uniform,
            settings: t,
            slot: 0,
            written: 1,
            window: 0.0,
            yaw: [rad.cos(), rad.sin()],
            suppress: false,
            fade: None,
            writes: 0,
            memory: Mutex::new(None),
        };
        self.create_particles_with_trails(levels, seeds, s, trails)
    }

    /// Long exposure in the view: the light fades with this half-life (s) instead of being
    /// cleared every frame; none switches it off.
    pub fn set_trail_fade(&self, p: &mut GpuParticles, half_life: Option<f32>) {
        if let Some(t) = p.trails.as_mut() {
            t.fade = half_life.map(|h| h.max(0.05));
            *t.memory.lock().unwrap_or_else(|e| e.into_inner()) = None;
        }
    }

    /// The light pass of a frame of the view: the whole trails into a cleared light image, or
    /// with the long exposure the pieces written since the last frame onto the faded light. A
    /// moved camera or another size starts the light again.
    pub(crate) fn trail_view_light(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        p: &GpuParticles,
        camera: &dyn ViewCamera,
        depth: &wgpu::TextureView,
        size: (u32, u32),
        region: [u32; 4],
    ) {
        let Some(t) = &p.trails else { return };
        let k = t.settings.k;
        let Some(half) = t.fade else {
            self.trail_light(encoder, p, camera, depth, size, region, false, k - 1, 1.0);
            return;
        };
        let aspect = region[2] as f32 / region[3].max(1) as f32;
        let now = FadeMemory {
            view_proj: camera.view_proj(aspect).to_cols_array_2d(),
            size,
            region,
            writes: t.writes,
            at: std::time::Instant::now(),
        };
        let mut memory = t.memory.lock().unwrap_or_else(|e| e.into_inner());
        let last =
            memory.filter(|m| m.view_proj == now.view_proj && m.size == size && m.region == region);
        // Points per second at 60 frames: the light of one piece lasts about half / ln 2.
        let per_second = 60.0 / t.settings.every.max(1) as f32;
        let scale = ((k - 1) as f32 / (per_second * half / std::f32::consts::LN_2)).min(1.0);
        match last {
            Some(m) => {
                let new = (now.writes - m.writes).min(u64::from(k - 1)) as u32;
                let elapsed = (now.at - m.at).as_secs_f32().min(1.0);
                self.trail_fade(encoder, size, region, 0.5f32.powf(elapsed / half));
                if new > 0 {
                    self.trail_light(encoder, p, camera, depth, size, region, true, new, scale);
                }
            }
            None => {
                self.trail_light(encoder, p, camera, depth, size, region, false, k - 1, scale);
            }
        }
        // Only a frame that drew new pieces moves the memory on (the fade keeps its clock).
        if last.is_none_or(|m| now.writes != m.writes) {
            *memory = Some(now);
        } else if let Some(m) = memory.as_mut() {
            m.at = now.at;
        }
    }

    /// Multiplies the light image by `factor` inside `region`.
    fn trail_fade(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        size: (u32, u32),
        region: [u32; 4],
        factor: f32,
    ) {
        let kit = self.kit();
        self.with_targets(size.0, size.1, |targets| {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("trail fade"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &targets.light,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                ..Default::default()
            });
            pass.set_viewport(
                region[0] as f32,
                region[1] as f32,
                region[2] as f32,
                region[3] as f32,
                0.0,
                1.0,
            );
            let f = f64::from(factor);
            pass.set_blend_constant(wgpu::Color {
                r: f,
                g: f,
                b: f,
                a: f,
            });
            pass.set_pipeline(&kit.fade);
            pass.draw(0..3, 0..1);
        });
    }

    /// Points of all trails `(x, y, z, w)` in ring order (for tests).
    pub fn read_trails(&self, p: &GpuParticles) -> Vec<[f32; 4]> {
        match &p.trails {
            Some(t) => self.read_buffer_len(&t.ring, p.count * t.settings.k),
            None => Vec::new(),
        }
    }

    fn kit(&self) -> &TrailKit {
        self.trail_kit.get_or_init(|| {
            let device = &self.device;
            let trail_module = device.create_shader_module(wgpu::include_wgsl!("trails.wgsl"));
            let post_module = device.create_shader_module(wgpu::include_wgsl!("trails_post.wgsl"));
            let trail_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("trail"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::VERTEX,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::VERTEX,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
            });
            let trail_pipeline_layout =
                device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("trail"),
                    bind_group_layouts: &[Some(&trail_layout)],
                    immediate_size: 0,
                });
            let add = wgpu::BlendState {
                color: wgpu::BlendComponent {
                    src_factor: wgpu::BlendFactor::One,
                    dst_factor: wgpu::BlendFactor::One,
                    operation: wgpu::BlendOperation::Add,
                },
                alpha: wgpu::BlendComponent {
                    src_factor: wgpu::BlendFactor::One,
                    dst_factor: wgpu::BlendFactor::One,
                    operation: wgpu::BlendOperation::Add,
                },
            };
            let trail = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("trail"),
                layout: Some(&trail_pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &trail_module,
                    entry_point: Some("vs_trail"),
                    compilation_options: Default::default(),
                    buffers: &[],
                },
                fragment: Some(wgpu::FragmentState {
                    module: &trail_module,
                    entry_point: Some("fs_trail"),
                    compilation_options: Default::default(),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: LIGHT_FORMAT,
                        blend: Some(add),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                primitive: wgpu::PrimitiveState {
                    cull_mode: None,
                    ..Default::default()
                },
                // Hidden behind the model, without writing depth: order does not matter.
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: DEPTH_FORMAT,
                    depth_write_enabled: Some(false),
                    depth_compare: Some(wgpu::CompareFunction::LessEqual),
                    stencil: Default::default(),
                    bias: Default::default(),
                }),
                multisample: Default::default(),
                multiview_mask: None,
                cache: None,
            });
            let tex = |binding| wgpu::BindGroupLayoutEntry {
                binding,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            };
            let post_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("trail post"),
                entries: &[
                    tex(0),
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                    tex(2),
                    wgpu::BindGroupLayoutEntry {
                        binding: 3,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
            });
            let post_pipeline_layout =
                device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("trail post"),
                    bind_group_layouts: &[Some(&post_layout)],
                    immediate_size: 0,
                });
            let full = |entry: &str, format, blend| {
                device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                    label: Some(entry),
                    layout: Some(&post_pipeline_layout),
                    vertex: wgpu::VertexState {
                        module: &post_module,
                        entry_point: Some("vs_full"),
                        compilation_options: Default::default(),
                        buffers: &[],
                    },
                    fragment: Some(wgpu::FragmentState {
                        module: &post_module,
                        entry_point: Some(entry),
                        compilation_options: Default::default(),
                        targets: &[Some(wgpu::ColorTargetState {
                            format,
                            blend,
                            write_mask: wgpu::ColorWrites::ALL,
                        })],
                    }),
                    primitive: Default::default(),
                    depth_stencil: None,
                    multisample: Default::default(),
                    multiview_mask: None,
                    cache: None,
                })
            };
            let down = full("fs_down", LIGHT_FORMAT, None);
            let blur = full("fs_blur", LIGHT_FORMAT, None);
            // Fading: the light times a constant (the fragment itself adds nothing).
            let empty = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("trail fade"),
                bind_group_layouts: &[],
                immediate_size: 0,
            });
            let scale = wgpu::BlendComponent {
                src_factor: wgpu::BlendFactor::Zero,
                dst_factor: wgpu::BlendFactor::Constant,
                operation: wgpu::BlendOperation::Add,
            };
            let fade = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("trail fade"),
                layout: Some(&empty),
                vertex: wgpu::VertexState {
                    module: &post_module,
                    entry_point: Some("vs_full"),
                    compilation_options: Default::default(),
                    buffers: &[],
                },
                fragment: Some(wgpu::FragmentState {
                    module: &post_module,
                    entry_point: Some("fs_fade"),
                    compilation_options: Default::default(),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: LIGHT_FORMAT,
                        blend: Some(wgpu::BlendState {
                            color: scale,
                            alpha: scale,
                        }),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                primitive: Default::default(),
                depth_stencil: None,
                multisample: Default::default(),
                multiview_mask: None,
                cache: None,
            });
            TrailKit {
                trail_layout,
                trail,
                post_layout,
                down,
                blur,
                fade,
                post_module,
                post_pipeline_layout,
                compose: Mutex::new(Default::default()),
                targets: Mutex::new(None),
            }
        })
    }

    fn compose_pipeline(&self, format: wgpu::TextureFormat) -> wgpu::RenderPipeline {
        let kit = self.kit();
        let mut cache = kit.compose.lock().unwrap_or_else(|e| e.into_inner());
        cache
            .entry(format)
            .or_insert_with(|| {
                // Screen blend: c · (1 - scene) + scene = 1 - (1 - scene)(1 - c); the alpha
                // keeps the larger of both (transparent backgrounds get cover where it glows).
                let blend = wgpu::BlendState {
                    color: wgpu::BlendComponent {
                        src_factor: wgpu::BlendFactor::OneMinusDst,
                        dst_factor: wgpu::BlendFactor::One,
                        operation: wgpu::BlendOperation::Add,
                    },
                    alpha: wgpu::BlendComponent {
                        src_factor: wgpu::BlendFactor::One,
                        dst_factor: wgpu::BlendFactor::One,
                        operation: wgpu::BlendOperation::Max,
                    },
                };
                self.device
                    .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                        label: Some("trail compose"),
                        layout: Some(&kit.post_pipeline_layout),
                        vertex: wgpu::VertexState {
                            module: &kit.post_module,
                            entry_point: Some("vs_full"),
                            compilation_options: Default::default(),
                            buffers: &[],
                        },
                        fragment: Some(wgpu::FragmentState {
                            module: &kit.post_module,
                            entry_point: Some("fs_compose"),
                            compilation_options: Default::default(),
                            targets: &[Some(wgpu::ColorTargetState {
                                format,
                                blend: Some(blend),
                                write_mask: wgpu::ColorWrites::ALL,
                            })],
                        }),
                        primitive: Default::default(),
                        depth_stencil: None,
                        multisample: Default::default(),
                        multiview_mask: None,
                        cache: None,
                    })
            })
            .clone()
    }

    fn with_targets<T>(&self, w: u32, h: u32, f: impl FnOnce(&TrailTargets) -> T) -> T {
        let kit = self.kit();
        let mut guard = kit.targets.lock().unwrap_or_else(|e| e.into_inner());
        if guard.as_ref().is_none_or(|t| t.size != (w, h)) {
            let make = |w: u32, h: u32| {
                self.device
                    .create_texture(&wgpu::TextureDescriptor {
                        label: Some("light"),
                        size: wgpu::Extent3d {
                            width: w.max(1),
                            height: h.max(1),
                            depth_or_array_layers: 1,
                        },
                        mip_level_count: 1,
                        sample_count: 1,
                        dimension: wgpu::TextureDimension::D2,
                        format: LIGHT_FORMAT,
                        usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                            | wgpu::TextureUsages::TEXTURE_BINDING,
                        view_formats: &[],
                    })
                    .create_view(&Default::default())
            };
            let (sw, sh) = ((w / 4).max(1), (h / 4).max(1));
            *guard = Some(TrailTargets {
                size: (w, h),
                light: make(w, h),
                small: [make(sw, sh), make(sw, sh)],
                sampler: self.device.create_sampler(&wgpu::SamplerDescriptor {
                    label: Some("light"),
                    mag_filter: wgpu::FilterMode::Linear,
                    min_filter: wgpu::FilterMode::Linear,
                    address_mode_u: wgpu::AddressMode::ClampToEdge,
                    address_mode_v: wgpu::AddressMode::ClampToEdge,
                    ..Default::default()
                }),
            });
        }
        f(guard.as_ref().unwrap_or_else(|| unreachable!()))
    }

    fn post_bind(
        &self,
        source: &wgpu::TextureView,
        glow: &wgpu::TextureView,
        sampler: &wgpu::Sampler,
        post: [f32; 4],
    ) -> wgpu::BindGroup {
        let kit = self.kit();
        let buf = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("trail post"),
                contents: bytemuck::cast_slice(&post),
                usage: wgpu::BufferUsages::UNIFORM,
            });
        self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("trail post"),
            layout: &kit.post_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(source),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(glow),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: buf.as_entire_binding(),
                },
            ],
        })
    }

    /// Records the light pass of the trails into the light image (cleared unless `load`).
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn trail_light(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        p: &GpuParticles,
        camera: &dyn ViewCamera,
        depth: &wgpu::TextureView,
        size: (u32, u32),
        region: [u32; 4],
        load: bool,
        segments: u32,
        scale: f32,
    ) {
        let Some(t) = &p.trails else { return };
        let kit = self.kit();
        let aspect = region[2] as f32 / region[3].max(1) as f32;
        let s = &t.settings;
        let u = ViewUniform {
            view_proj: camera.view_proj(aspect).to_cols_array_2d(),
            rot: [t.yaw[0], t.yaw[1], region[2] as f32, region[3] as f32],
            ring: [s.k, t.slot, t.written, segments.max(1)],
            style: [s.width_px, s.exposure * scale, s.range[0], s.range[1]],
            color: [
                s.color[0],
                s.color[1],
                s.color[2],
                if s.by_speed { 1.0 } else { 0.0 },
            ],
        };
        let ubuf = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("trail view"),
                contents: bytemuck::bytes_of(&u),
                usage: wgpu::BufferUsages::UNIFORM,
            });
        let bind = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("trail"),
            layout: &kit.trail_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: ubuf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: t.ring.as_entire_binding(),
                },
            ],
        });
        self.with_targets(size.0, size.1, |targets| {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("trail light"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &targets.light,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: if load {
                            wgpu::LoadOp::Load
                        } else {
                            wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT)
                        },
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: depth,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                ..Default::default()
            });
            pass.set_viewport(
                region[0] as f32,
                region[1] as f32,
                region[2] as f32,
                region[3] as f32,
                0.0,
                1.0,
            );
            pass.set_pipeline(&kit.trail);
            pass.set_bind_group(0, &bind, &[]);
            pass.draw(0..p.count * segments.max(1) * 6, 0..1);
        });
    }

    /// Records the glow and the composition of the light image onto `target`.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn trail_post(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        p: &GpuParticles,
        target: &wgpu::TextureView,
        format: wgpu::TextureFormat,
        size: (u32, u32),
        region: [u32; 4],
    ) {
        let Some(t) = &p.trails else { return };
        let kit = self.kit();
        let glow = t.settings.glow;
        let compose = self.compose_pipeline(format);
        self.with_targets(size.0, size.1, |targets| {
            let full = |encoder: &mut wgpu::CommandEncoder,
                        pipeline: &wgpu::RenderPipeline,
                        bind: &wgpu::BindGroup,
                        view: &wgpu::TextureView,
                        load: bool,
                        viewport: Option<[u32; 4]>| {
                let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("trail post"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view,
                        depth_slice: None,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: if load {
                                wgpu::LoadOp::Load
                            } else {
                                wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT)
                            },
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: None,
                    ..Default::default()
                });
                if let Some(r) = viewport {
                    pass.set_viewport(r[0] as f32, r[1] as f32, r[2] as f32, r[3] as f32, 0.0, 1.0);
                }
                pass.set_pipeline(pipeline);
                pass.set_bind_group(0, bind, &[]);
                pass.draw(0..3, 0..1);
            };
            if glow {
                let (sw, sh) = ((size.0 / 4).max(1) as f32, (size.1 / 4).max(1) as f32);
                let b = self.post_bind(&targets.light, &targets.light, &targets.sampler, [0.0; 4]);
                full(encoder, &kit.down, &b, &targets.small[0], false, None);
                let b = self.post_bind(
                    &targets.small[0],
                    &targets.small[0],
                    &targets.sampler,
                    [1.0 / sw, 0.0, 0.0, 0.0],
                );
                full(encoder, &kit.blur, &b, &targets.small[1], false, None);
                let b = self.post_bind(
                    &targets.small[1],
                    &targets.small[1],
                    &targets.sampler,
                    [0.0, 1.0 / sh, 0.0, 0.0],
                );
                full(encoder, &kit.blur, &b, &targets.small[0], false, None);
            }
            let b = self.post_bind(
                &targets.light,
                &targets.small[0],
                &targets.sampler,
                [0.0, 0.0, if glow { 1.0 } else { 0.0 }, 1.0],
            );
            full(encoder, &compose, &b, target, true, Some(region));
        });
    }

    /// A still with long exposure: the particles run `steps` steps of `dt`; each step adds the
    /// newest piece of every trail to the light image (brightness divided by the steps), then
    /// the light is put onto the scene once.
    pub fn render_offscreen_exposure(
        &self,
        scene: &mut crate::GpuScene,
        camera: &dyn ViewCamera,
        width: u32,
        height: u32,
        steps: u32,
        dt: f32,
    ) -> Vec<u8> {
        let texture = self.target_texture(width, height);
        let view = texture.create_view(&Default::default());
        let depth = self.stored_depth(width, height);
        if let Some(t) = scene.particles.as_mut().and_then(|p| p.trails.as_mut()) {
            t.suppress = true;
        }
        self.draw_with_depth(scene, camera, &view, width, height, &depth);
        let steps = steps.max(1);
        let region = [0, 0, width, height];
        if let Some(p) = scene.particles.as_mut() {
            let every = p.trails.as_ref().map_or(1, |t| t.settings.every);
            if let Some(t) = p.trails.as_mut() {
                t.settings.every = 1;
            }
            for i in 0..steps {
                self.step_particles(p, dt, 1);
                let mut encoder = self.device.create_command_encoder(&Default::default());
                self.trail_light(
                    &mut encoder,
                    p,
                    camera,
                    &depth,
                    (width, height),
                    region,
                    i > 0,
                    1,
                    1.0 / steps as f32 * 4.0,
                );
                self.queue.submit([encoder.finish()]);
            }
            let mut encoder = self.device.create_command_encoder(&Default::default());
            self.trail_post(
                &mut encoder,
                p,
                &view,
                crate::OFFSCREEN_FORMAT,
                (width, height),
                region,
            );
            self.queue.submit([encoder.finish()]);
            if let Some(t) = p.trails.as_mut() {
                t.settings.every = every;
                t.suppress = false;
            }
        }
        self.read_texture(&texture, width, height)
    }
}

/// Cuts a clip-space segment at the near plane (z >= 0, wgpu convention): the point `a` moved
/// along to `b` where it crosses the plane. The same as `cut` in `trails.wgsl`.
pub fn cut_near(a: glam::Vec4, b: glam::Vec4) -> glam::Vec4 {
    if a.z >= 0.0 {
        return a;
    }
    let t = (a.z / (a.z - b.z)).clamp(0.0, 1.0);
    a.lerp(b, t)
}
