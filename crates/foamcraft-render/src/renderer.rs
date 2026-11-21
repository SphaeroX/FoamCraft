// SPDX-License-Identifier: GPL-3.0-or-later
//! GPU renderer shared by the viewport and offscreen frames.
//!
//! Colours are treated as display (sRGB-encoded) values and written to non-sRGB targets, so an
//! offscreen frame stores exactly the colours the viewport shows. Every vertex carries a pick id
//! (0 = not pickable); a second pass writes these ids into an integer texture for picking.

use std::collections::HashMap;
use std::sync::Mutex;

use bytemuck::{Pod, Zeroable};
use wgpu::util::DeviceExt;

use crate::camera::OrbitCamera;
use crate::scene::{Scene, Vertex};

/// Format of offscreen frames: 8-bit RGBA.
pub const OFFSCREEN_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;
const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;
const PICK_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::R32Uint;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Globals {
    view_proj: [[f32; 4]; 4],
    eye: [f32; 4],
    /// Pick id under the mouse (0 = none).
    hover: u32,
    _pad: [u32; 3],
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct ObjectUniform {
    color: [f32; 4],
}

/// Scene content on the GPU, with the current highlight state.
pub struct GpuScene {
    /// Animated particles drawn after the meshes.
    pub particles: Option<crate::particles::GpuParticles>,
    /// Vertices, colours, indices, object bind group, index count, lines.
    meshes: Vec<(
        wgpu::Buffer,
        wgpu::Buffer,
        wgpu::Buffer,
        wgpu::BindGroup,
        u32,
        bool,
    )>,
    background: [f64; 4],
    hover: u32,
    /// One bit per pick id.
    selected: wgpu::Buffer,
    selected_words: usize,
}

/// Where and how one pass draws.
struct DrawTarget<'a> {
    view: &'a wgpu::TextureView,
    format: wgpu::TextureFormat,
    clear: wgpu::Color,
    width: u32,
    height: u32,
    /// Part of the target to draw into (x, y, width, height); `None` is all of it.
    region: Option<[u32; 4]>,
    /// Keep what is in the target (the second half of a split view) instead of clearing it.
    keep: bool,
}

pub struct Renderer {
    pub instance: wgpu::Instance,
    pub adapter: wgpu::Adapter,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    globals_layout: wgpu::BindGroupLayout,
    object_layout: wgpu::BindGroupLayout,
    pipeline_layout: wgpu::PipelineLayout,
    shader: wgpu::ShaderModule,
    pipelines: Mutex<HashMap<(wgpu::TextureFormat, bool), wgpu::RenderPipeline>>,
}

/// Creates the default wgpu instance with all native backends.
pub fn default_instance() -> wgpu::Instance {
    wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle_from_env())
}

fn bits(ids: &[u32], words: usize) -> Vec<u32> {
    let mut out = vec![0u32; words];
    for &id in ids {
        if let Some(w) = out.get_mut(id as usize / 32) {
            *w |= 1 << (id % 32);
        }
    }
    out
}

impl Renderer {
    /// Renderer without a window. `None` when the machine has no usable GPU adapter.
    pub fn new_headless() -> Option<Self> {
        Self::new(default_instance(), None)
    }

    /// Renderer on `instance`, optionally compatible with a window surface.
    pub fn new(instance: wgpu::Instance, surface: Option<&wgpu::Surface<'_>>) -> Option<Self> {
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            force_fallback_adapter: false,
            compatible_surface: surface,
            apply_limit_buckets: false,
        }))
        .ok()?;
        let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            label: Some("foamcraft"),
            ..Default::default()
        }))
        .ok()?;

        let uniform = |binding| wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        };
        let globals_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("globals"),
            entries: &[
                uniform(0),
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });
        let object_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("object"),
            entries: &[uniform(0)],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("mesh"),
            bind_group_layouts: &[Some(&globals_layout), Some(&object_layout)],
            immediate_size: 0,
        });
        let shader = device.create_shader_module(wgpu::include_wgsl!("shader.wgsl"));
        Some(Self {
            instance,
            adapter,
            device,
            queue,
            globals_layout,
            object_layout,
            pipeline_layout,
            shader,
            pipelines: Mutex::new(HashMap::new()),
        })
    }

    /// Colour pipeline for `format`, or the pick pipeline for [`PICK_FORMAT`].
    fn pipeline(&self, format: wgpu::TextureFormat, lines: bool) -> wgpu::RenderPipeline {
        // A poisoned lock only means another render panicked; the cache itself stays valid.
        let mut cache = self.pipelines.lock().unwrap_or_else(|e| e.into_inner());
        let entry_point = if format == PICK_FORMAT {
            "fs_pick"
        } else {
            "fs_main"
        };
        cache
            .entry((format, lines))
            .or_insert_with(|| {
                self.device
                    .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                        label: Some(entry_point),
                        layout: Some(&self.pipeline_layout),
                        vertex: wgpu::VertexState {
                            module: &self.shader,
                            entry_point: Some("vs_main"),
                            compilation_options: Default::default(),
                            buffers: &[
                                Some(wgpu::VertexBufferLayout {
                                    array_stride: std::mem::size_of::<Vertex>() as u64,
                                    step_mode: wgpu::VertexStepMode::Vertex,
                                    attributes: &wgpu::vertex_attr_array![
                                        0 => Float32x3, 1 => Float32x3, 2 => Uint32
                                    ],
                                }),
                                Some(wgpu::VertexBufferLayout {
                                    array_stride: 12,
                                    step_mode: wgpu::VertexStepMode::Vertex,
                                    attributes: &wgpu::vertex_attr_array![3 => Float32x3],
                                }),
                            ],
                        },
                        fragment: Some(wgpu::FragmentState {
                            module: &self.shader,
                            entry_point: Some(entry_point),
                            compilation_options: Default::default(),
                            targets: &[Some(wgpu::ColorTargetState {
                                format,
                                blend: None,
                                write_mask: wgpu::ColorWrites::ALL,
                            })],
                        }),
                        primitive: wgpu::PrimitiveState {
                            topology: if lines {
                                wgpu::PrimitiveTopology::LineList
                            } else {
                                wgpu::PrimitiveTopology::TriangleList
                            },
                            cull_mode: if lines { None } else { Some(wgpu::Face::Back) },
                            ..Default::default()
                        },
                        depth_stencil: Some(wgpu::DepthStencilState {
                            format: DEPTH_FORMAT,
                            depth_write_enabled: Some(true),
                            depth_compare: Some(wgpu::CompareFunction::Less),
                            stencil: Default::default(),
                            bias: Default::default(),
                        }),
                        multisample: Default::default(),
                        multiview_mask: None,
                        cache: None,
                    })
            })
            .clone()
    }

    /// Uploads the scene once; draw it many times with [`Renderer::render_gpu`].
    /// Bind group of the per-object uniform (colour and opacity).
    pub(crate) fn object_bind(&self, color: [f32; 3], opacity: f32) -> wgpu::BindGroup {
        let ub = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("object"),
                contents: bytemuck::bytes_of(&ObjectUniform {
                    color: [color[0], color[1], color[2], opacity],
                }),
                usage: wgpu::BufferUsages::UNIFORM,
            });
        self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("object"),
            layout: &self.object_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: ub.as_entire_binding(),
            }],
        })
    }

    pub fn upload(&self, scene: &Scene) -> GpuScene {
        let device = &self.device;
        let meshes: Vec<_> = scene
            .meshes
            .iter()
            .filter(|m| !m.indices.is_empty())
            .map(|mesh| {
                let vb = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("vertices"),
                    contents: bytemuck::cast_slice(&mesh.vertices),
                    usage: wgpu::BufferUsages::VERTEX,
                });
                let ib = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("indices"),
                    contents: bytemuck::cast_slice(&mesh.indices),
                    usage: wgpu::BufferUsages::INDEX,
                });
                // One colour per vertex: the field colours, or the mesh colour everywhere.
                let colors = mesh
                    .vertex_colors
                    .clone()
                    .filter(|c| c.len() == mesh.vertices.len())
                    .unwrap_or_else(|| vec![mesh.color; mesh.vertices.len()]);
                let cb = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("colors"),
                    contents: bytemuck::cast_slice(&colors),
                    usage: wgpu::BufferUsages::VERTEX,
                });
                let [r, g, b] = mesh.color;
                let ub = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("object"),
                    contents: bytemuck::bytes_of(&ObjectUniform {
                        color: [r, g, b, mesh.opacity],
                    }),
                    usage: wgpu::BufferUsages::UNIFORM,
                });
                let bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("object"),
                    layout: &self.object_layout,
                    entries: &[wgpu::BindGroupEntry {
                        binding: 0,
                        resource: ub.as_entire_binding(),
                    }],
                });
                (vb, cb, ib, bind, mesh.indices.len() as u32, mesh.lines)
            })
            .collect();
        let max_id = scene
            .meshes
            .iter()
            .flat_map(|m| m.vertices.iter().map(|v| v.pick))
            .max()
            .unwrap_or(0);
        let selected_words = max_id as usize / 32 + 1;
        let selected = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("selected"),
            contents: bytemuck::cast_slice(&vec![0u32; selected_words]),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });
        GpuScene {
            particles: None,
            meshes,
            background: scene.background,
            hover: 0,
            selected,
            selected_words,
        }
    }

    /// Sets the hovered pick id and the selected pick ids.
    pub fn set_highlight(&self, scene: &mut GpuScene, hover: Option<u32>, selected: &[u32]) {
        scene.hover = hover.unwrap_or(0);
        let words = bits(selected, scene.selected_words);
        self.queue
            .write_buffer(&scene.selected, 0, bytemuck::cast_slice(&words));
    }

    /// Draws `scene` into `target` (a view of a texture with `format` and size `width` x `height`).
    pub fn render_to_view(
        &self,
        scene: &Scene,
        camera: &OrbitCamera,
        target: &wgpu::TextureView,
        format: wgpu::TextureFormat,
        width: u32,
        height: u32,
    ) {
        self.render_gpu(&self.upload(scene), camera, target, format, width, height);
    }

    fn depth_view(&self, width: u32, height: u32) -> wgpu::TextureView {
        self.device
            .create_texture(&wgpu::TextureDescriptor {
                label: Some("depth"),
                size: wgpu::Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: DEPTH_FORMAT,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[],
            })
            .create_view(&Default::default())
    }

    /// One pass over all meshes into `target` (colour or pick ids, depending on `format`).
    fn draw(&self, scene: &GpuScene, camera: &OrbitCamera, pass: DrawTarget<'_>) {
        let DrawTarget {
            view: target,
            format,
            clear,
            width,
            height,
            region,
            keep,
        } = pass;
        let device = &self.device;
        let [rx, ry, rw, rh] = region.unwrap_or([0, 0, width, height]);
        let aspect = rw as f32 / rh.max(1) as f32;
        let globals = Globals {
            view_proj: camera.view_proj(aspect).to_cols_array_2d(),
            eye: camera.eye().extend(1.0).into(),
            hover: scene.hover,
            _pad: [0; 3],
        };
        let globals_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("globals"),
            contents: bytemuck::bytes_of(&globals),
            usage: wgpu::BufferUsages::UNIFORM,
        });
        let globals_bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("globals"),
            layout: &self.globals_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: globals_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: scene.selected.as_entire_binding(),
                },
            ],
        });
        let depth_view = self.depth_view(width, height);
        let pipeline = self.pipeline(format, false);
        let line_pipeline = self.pipeline(format, true);
        let mut encoder = device.create_command_encoder(&Default::default());
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("scene"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: target,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: if keep {
                            wgpu::LoadOp::Load
                        } else {
                            wgpu::LoadOp::Clear(clear)
                        },
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &depth_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Discard,
                    }),
                    stencil_ops: None,
                }),
                ..Default::default()
            });
            pass.set_viewport(rx as f32, ry as f32, rw as f32, rh as f32, 0.0, 1.0);
            pass.set_bind_group(0, &globals_bind, &[]);
            if let Some(p) = &scene.particles {
                pass.set_pipeline(&line_pipeline);
                pass.set_bind_group(1, &p.object, &[]);
                pass.set_vertex_buffer(0, p.verts.slice(..));
                pass.set_vertex_buffer(1, p.colors.slice(..));
                pass.set_index_buffer(p.indices.slice(..), wgpu::IndexFormat::Uint32);
                pass.draw_indexed(0..p.count * 2, 0, 0..1);
            }
            for (vb, cb, ib, bind, count, lines) in &scene.meshes {
                pass.set_pipeline(if *lines { &line_pipeline } else { &pipeline });
                pass.set_bind_group(1, bind, &[]);
                pass.set_vertex_buffer(0, vb.slice(..));
                pass.set_vertex_buffer(1, cb.slice(..));
                pass.set_index_buffer(ib.slice(..), wgpu::IndexFormat::Uint32);
                pass.draw_indexed(0..*count, 0, 0..1);
            }
        }
        self.queue.submit([encoder.finish()]);
    }

    /// Draws an uploaded scene.
    pub fn render_gpu(
        &self,
        scene: &GpuScene,
        camera: &OrbitCamera,
        target: &wgpu::TextureView,
        format: wgpu::TextureFormat,
        width: u32,
        height: u32,
    ) {
        let [r, g, b, a] = scene.background;
        let clear = wgpu::Color { r, g, b, a };
        self.draw(
            scene,
            camera,
            DrawTarget {
                view: target,
                format,
                clear,
                width,
                height,
                region: None,
                keep: false,
            },
        );
    }

    /// Two uploaded scenes side by side with the same camera (split view of two runs), in the
    /// columns `span` = (first column, number of columns) of the target; the rest is background.
    #[allow(clippy::too_many_arguments)]
    pub fn render_split_gpu(
        &self,
        left: &GpuScene,
        right: &GpuScene,
        camera: &OrbitCamera,
        target: &wgpu::TextureView,
        format: wgpu::TextureFormat,
        width: u32,
        height: u32,
        span: (u32, u32),
    ) {
        let x0 = span.0.min(width.saturating_sub(2));
        let used = span.1.clamp(2, width - x0);
        let half = used / 2;
        for (scene, region, keep) in [
            (left, [x0, 0, half, height], false),
            (right, [x0 + half, 0, used - half, height], true),
        ] {
            let [r, g, b, a] = scene.background;
            self.draw(
                scene,
                camera,
                DrawTarget {
                    view: target,
                    format,
                    clear: wgpu::Color { r, g, b, a },
                    width,
                    height,
                    region: Some(region),
                    keep,
                },
            );
        }
    }

    /// Offscreen frame of a split view.
    pub fn render_offscreen_split(
        &self,
        left: &GpuScene,
        right: &GpuScene,
        camera: &OrbitCamera,
        width: u32,
        height: u32,
    ) -> Vec<u8> {
        let texture = self.target(OFFSCREEN_FORMAT, width, height);
        let view = texture.create_view(&Default::default());
        self.render_split_gpu(
            left,
            right,
            camera,
            &view,
            OFFSCREEN_FORMAT,
            width,
            height,
            (0, width),
        );
        self.read_back(&texture, (0, 0), (width, height))
    }

    fn target(&self, format: wgpu::TextureFormat, width: u32, height: u32) -> wgpu::Texture {
        self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("offscreen"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        })
    }

    /// Copies a region of a 4-byte-per-texel texture into tightly packed rows.
    fn read_back(&self, texture: &wgpu::Texture, origin: (u32, u32), size: (u32, u32)) -> Vec<u8> {
        let (width, height) = size;
        let unpadded = width * 4;
        let padded = unpadded.div_ceil(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT)
            * wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
        let buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("readback"),
            size: u64::from(padded * height),
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let mut encoder = self.device.create_command_encoder(&Default::default());
        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture,
                mip_level: 0,
                origin: wgpu::Origin3d {
                    x: origin.0,
                    y: origin.1,
                    z: 0,
                },
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(padded),
                    rows_per_image: Some(height),
                },
            },
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );
        self.queue.submit([encoder.finish()]);
        let slice = buffer.slice(..);
        slice.map_async(wgpu::MapMode::Read, |_| {});
        // Waiting for the device is the only way to finish a synchronous readback.
        let _ = self.device.poll(wgpu::PollType::wait_indefinitely());
        let Ok(data) = slice.get_mapped_range() else {
            return Vec::new();
        };
        let mut out = Vec::with_capacity((unpadded * height) as usize);
        for row in data.chunks(padded as usize).take(height as usize) {
            out.extend_from_slice(&row[..unpadded as usize]);
        }
        drop(data);
        buffer.unmap();
        out
    }

    /// Renders into a new image and returns tightly packed RGBA8 rows, top row first.
    pub fn render_offscreen(
        &self,
        scene: &Scene,
        camera: &OrbitCamera,
        width: u32,
        height: u32,
    ) -> Vec<u8> {
        self.render_offscreen_gpu(&self.upload(scene), camera, width, height)
    }

    /// Offscreen frame of an uploaded scene.
    pub fn render_offscreen_gpu(
        &self,
        scene: &GpuScene,
        camera: &OrbitCamera,
        width: u32,
        height: u32,
    ) -> Vec<u8> {
        let texture = self.target(OFFSCREEN_FORMAT, width, height);
        let view = texture.create_view(&Default::default());
        self.render_gpu(scene, camera, &view, OFFSCREEN_FORMAT, width, height);
        self.read_back(&texture, (0, 0), (width, height))
    }

    /// Pick id at pixel (`x`, `y`) of a `width` x `height` view; `None` for the background.
    pub fn pick(
        &self,
        scene: &GpuScene,
        camera: &OrbitCamera,
        width: u32,
        height: u32,
        x: u32,
        y: u32,
    ) -> Option<u32> {
        if x >= width || y >= height {
            return None;
        }
        let texture = self.target(PICK_FORMAT, width, height);
        let view = texture.create_view(&Default::default());
        let clear = wgpu::Color {
            r: 0.0,
            g: 0.0,
            b: 0.0,
            a: 0.0,
        };
        self.draw(
            scene,
            camera,
            DrawTarget {
                view: &view,
                format: PICK_FORMAT,
                clear,
                width,
                height,
                region: None,
                keep: false,
            },
        );
        let bytes = self.read_back(&texture, (x, y), (1, 1));
        let id = u32::from_le_bytes(bytes.get(..4)?.try_into().ok()?);
        (id != 0).then_some(id)
    }
}
