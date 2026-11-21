// SPDX-License-Identifier: GPL-3.0-or-later
//! Surface flow in the renderer (Task_33): lines with running lights and fluttering tufts, drawn
//! with their own small pipelines (`flow.wgsl`) so the mesh shader and all golden images stay
//! as they are. The motion is a function of the time in the globals only.

use bytemuck::{Pod, Zeroable};
use wgpu::util::DeviceExt;

use crate::renderer::Renderer;

/// One vertex of a surface line or a tuft (see `flow.wgsl` for the meaning of `p` and `q`).
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Pod, Zeroable)]
pub struct FlowVertex {
    pub pos: [f32; 3],
    pub color: [f32; 3],
    pub lateral: [f32; 3],
    pub p: [f32; 4],
    pub q: f32,
}

/// Lines (pairs of vertices) and tufts (triangles) of the surface flow.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct FlowGeometry {
    pub lines: Vec<FlowVertex>,
    pub tufts: Vec<FlowVertex>,
    pub tuft_indices: Vec<u32>,
    /// Ribbons and tubes of streamlines (Task_35): triangles, `lateral` is the face normal.
    pub ribbons: Vec<FlowVertex>,
    pub ribbon_indices: Vec<u32>,
}

/// The surface flow on the GPU.
pub struct GpuFlow {
    pub(crate) lines: Option<(wgpu::Buffer, u32)>,
    pub(crate) tufts: Option<(wgpu::Buffer, wgpu::Buffer, u32)>,
    pub(crate) ribbons: Option<(wgpu::Buffer, wgpu::Buffer, u32)>,
}

impl Renderer {
    /// Uploads the surface flow of a scene; `None` removes it.
    pub fn set_surface_flow(&self, scene: &mut crate::GpuScene, geometry: Option<&FlowGeometry>) {
        scene.flow = geometry.map(|g| {
            let lines = (!g.lines.is_empty()).then(|| {
                (
                    self.device
                        .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                            label: Some("flow lines"),
                            contents: bytemuck::cast_slice(&g.lines),
                            usage: wgpu::BufferUsages::VERTEX,
                        }),
                    g.lines.len() as u32,
                )
            });
            let tufts = (!g.tufts.is_empty() && !g.tuft_indices.is_empty()).then(|| {
                (
                    self.device
                        .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                            label: Some("tufts"),
                            contents: bytemuck::cast_slice(&g.tufts),
                            usage: wgpu::BufferUsages::VERTEX,
                        }),
                    self.device
                        .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                            label: Some("tuft indices"),
                            contents: bytemuck::cast_slice(&g.tuft_indices),
                            usage: wgpu::BufferUsages::INDEX,
                        }),
                    g.tuft_indices.len() as u32,
                )
            });
            let ribbons = (!g.ribbons.is_empty() && !g.ribbon_indices.is_empty()).then(|| {
                (
                    self.device
                        .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                            label: Some("ribbons"),
                            contents: bytemuck::cast_slice(&g.ribbons),
                            usage: wgpu::BufferUsages::VERTEX,
                        }),
                    self.device
                        .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                            label: Some("ribbon indices"),
                            contents: bytemuck::cast_slice(&g.ribbon_indices),
                            usage: wgpu::BufferUsages::INDEX,
                        }),
                    g.ribbon_indices.len() as u32,
                )
            });
            GpuFlow {
                lines,
                tufts,
                ribbons,
            }
        });
    }

    /// Pipeline for lines (0), tufts (1) or ribbons (2), cached per format.
    pub(crate) fn flow_pipeline(
        &self,
        format: wgpu::TextureFormat,
        kind: u8,
    ) -> wgpu::RenderPipeline {
        let tufts = kind > 0;
        let mut cache = self
            .flow_pipelines
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        cache
            .entry((format, kind))
            .or_insert_with(|| {
                let (vs, fs) = match kind {
                    0 => ("vs_lines", "fs_lines"),
                    1 => ("vs_tufts", "fs_tufts"),
                    _ => ("vs_ribbons", "fs_ribbons"),
                };
                self.device
                    .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                        label: Some(vs),
                        layout: Some(&self.flow_layout),
                        vertex: wgpu::VertexState {
                            module: &self.flow_shader,
                            entry_point: Some(vs),
                            compilation_options: Default::default(),
                            buffers: &[Some(wgpu::VertexBufferLayout {
                                array_stride: std::mem::size_of::<FlowVertex>() as u64,
                                step_mode: wgpu::VertexStepMode::Vertex,
                                attributes: &wgpu::vertex_attr_array![
                                    0 => Float32x3, 1 => Float32x3, 2 => Float32x3,
                                    3 => Float32x4, 4 => Float32
                                ],
                            })],
                        },
                        fragment: Some(wgpu::FragmentState {
                            module: &self.flow_shader,
                            entry_point: Some(fs),
                            compilation_options: Default::default(),
                            targets: &[Some(wgpu::ColorTargetState {
                                format,
                                blend: None,
                                write_mask: wgpu::ColorWrites::ALL,
                            })],
                        }),
                        primitive: wgpu::PrimitiveState {
                            topology: if tufts {
                                wgpu::PrimitiveTopology::TriangleList
                            } else {
                                wgpu::PrimitiveTopology::LineList
                            },
                            // Tufts are seen from both sides.
                            cull_mode: None,
                            ..Default::default()
                        },
                        depth_stencil: Some(wgpu::DepthStencilState {
                            format: crate::renderer::DEPTH_FORMAT,
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

    /// Draws the surface flow of `scene` into an open colour pass.
    pub(crate) fn draw_flow(
        &self,
        pass: &mut wgpu::RenderPass<'_>,
        scene: &crate::GpuScene,
        format: wgpu::TextureFormat,
    ) {
        let Some(flow) = &scene.flow else {
            return;
        };
        if let Some((buf, count)) = &flow.lines {
            pass.set_pipeline(&self.flow_pipeline(format, 0));
            pass.set_vertex_buffer(0, buf.slice(..));
            pass.draw(0..*count, 0..1);
        }
        if let Some((verts, indices, count)) = &flow.tufts {
            pass.set_pipeline(&self.flow_pipeline(format, 1));
            pass.set_vertex_buffer(0, verts.slice(..));
            pass.set_index_buffer(indices.slice(..), wgpu::IndexFormat::Uint32);
            pass.draw_indexed(0..*count, 0, 0..1);
        }
        if let Some((verts, indices, count)) = &flow.ribbons {
            pass.set_pipeline(&self.flow_pipeline(format, 2));
            pass.set_vertex_buffer(0, verts.slice(..));
            pass.set_index_buffer(indices.slice(..), wgpu::IndexFormat::Uint32);
            pass.draw_indexed(0..*count, 0, 0..1);
        }
    }
}
