// SPDX-License-Identifier: GPL-3.0-or-later
//! Particles advected on the GPU (compute shader `particles.wgsl`); each particle is drawn as a
//! short line from its tail to its head with the scene's line pipeline.

use wgpu::util::DeviceExt;

use crate::renderer::Renderer;

/// One velocity grid: `(ux, uy, uz, valid)` per point, as `foamcraft_sim::particles::VelocityLevel`.
#[derive(Debug, Clone, PartialEq)]
pub struct FlowLevel {
    pub min: [f32; 3],
    pub max: [f32; 3],
    pub dims: [u32; 3],
    pub data: Vec<[f32; 4]>,
    /// Pressure per point (Pa), empty when not needed.
    pub pressure: Vec<f32>,
}

/// How particles are coloured.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParticleColor {
    Speed,
    Age,
    Constant,
    Pressure,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ParticleSettings {
    pub count: u32,
    pub seed: u32,
    /// Seconds a particle lives before it starts again.
    pub max_age: f32,
    /// Length of the tail in seconds of flow.
    pub trail: f32,
    pub color: ParticleColor,
    /// Speed range of the colour scale (m/s).
    pub range: [f32; 2],
    pub constant: [f32; 3],
    /// Below 1 the particles are drawn translucent (smoke).
    pub opacity: f32,
    /// The grids are in a frame turned by this angle about z (wind tunnel flow frame).
    pub yaw_deg: f32,
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Params {
    count: u32,
    levels: u32,
    seeds: u32,
    seed: u32,
    frame: u32,
    color_by: u32,
    dt: f32,
    max_age: f32,
    trail: f32,
    range_lo: f32,
    range_hi: f32,
    _pad: f32,
    constant: [f32; 4],
    rot: [f32; 4],
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Level {
    lo: [f32; 4],
    hi: [f32; 4],
    dims: [u32; 4],
}

/// Particle buffers on the GPU.
pub struct GpuParticles {
    params: Params,
    params_buf: wgpu::Buffer,
    bind: wgpu::BindGroup,
    init: wgpu::ComputePipeline,
    advect: wgpu::ComputePipeline,
    state: wgpu::Buffer,
    pub(crate) verts: wgpu::Buffer,
    pub(crate) colors: wgpu::Buffer,
    pub(crate) indices: wgpu::Buffer,
    pub(crate) object: wgpu::BindGroup,
    pub(crate) count: u32,
}

impl GpuParticles {
    pub fn frame(&self) -> u32 {
        self.params.frame
    }
}

fn storage(
    device: &wgpu::Device,
    label: &str,
    bytes: &[u8],
    extra: wgpu::BufferUsages,
) -> wgpu::Buffer {
    // Empty buffers are not allowed; keep at least 16 bytes.
    let mut data = bytes.to_vec();
    if data.len() < 16 {
        data.resize(16, 0);
    }
    device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some(label),
        contents: &data,
        usage: wgpu::BufferUsages::STORAGE | extra,
    })
}

impl Renderer {
    /// Uploads the grids (coarse to fine) and seed points and places the particles.
    pub fn create_particles(
        &self,
        levels: &[FlowLevel],
        seeds: &[[f32; 3]],
        s: &ParticleSettings,
    ) -> GpuParticles {
        let device = &self.device;
        let mut field: Vec<[f32; 4]> = Vec::new();
        let mut pressure: Vec<f32> = Vec::new();
        let gpu_levels: Vec<Level> = levels
            .iter()
            .map(|l| {
                let offset = field.len() as u32;
                field.extend_from_slice(&l.data);
                if l.pressure.len() == l.data.len() {
                    pressure.extend_from_slice(&l.pressure);
                } else {
                    pressure.extend(std::iter::repeat_n(0.0, l.data.len()));
                }
                Level {
                    lo: [l.min[0], l.min[1], l.min[2], 0.0],
                    hi: [l.max[0], l.max[1], l.max[2], 0.0],
                    dims: [l.dims[0], l.dims[1], l.dims[2], offset],
                }
            })
            .collect();
        let seeds4: Vec<[f32; 4]> = seeds.iter().map(|p| [p[0], p[1], p[2], 0.0]).collect();
        let count = s.count.max(1);
        let params = Params {
            count,
            levels: gpu_levels.len() as u32,
            seeds: seeds.len() as u32,
            seed: s.seed,
            frame: 0,
            color_by: match s.color {
                ParticleColor::Speed => 0,
                ParticleColor::Age => 1,
                ParticleColor::Constant => 2,
                ParticleColor::Pressure => 3,
            },
            dt: 0.0,
            max_age: s.max_age,
            trail: s.trail,
            range_lo: s.range[0],
            range_hi: s.range[1],
            _pad: 0.0,
            constant: [s.constant[0], s.constant[1], s.constant[2], 1.0],
            rot: [
                s.yaw_deg.to_radians().cos(),
                s.yaw_deg.to_radians().sin(),
                0.0,
                0.0,
            ],
        };
        let params_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("particle params"),
            contents: bytemuck::bytes_of(&params),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let levels_buf = storage(
            device,
            "levels",
            bytemuck::cast_slice(&gpu_levels),
            wgpu::BufferUsages::empty(),
        );
        let field_buf = storage(
            device,
            "field",
            bytemuck::cast_slice(&field),
            wgpu::BufferUsages::empty(),
        );
        let pressure_buf = storage(
            device,
            "pressure",
            bytemuck::cast_slice(&pressure),
            wgpu::BufferUsages::empty(),
        );
        let seeds_buf = storage(
            device,
            "seeds",
            bytemuck::cast_slice(&seeds4),
            wgpu::BufferUsages::empty(),
        );
        let state = storage(
            device,
            "particle state",
            &vec![0u8; count as usize * 16],
            wgpu::BufferUsages::COPY_SRC,
        );
        let verts = storage(
            device,
            "particle vertices",
            &vec![0u8; count as usize * 2 * 28],
            wgpu::BufferUsages::VERTEX,
        );
        let colors = storage(
            device,
            "particle colours",
            &vec![0u8; count as usize * 2 * 12],
            wgpu::BufferUsages::VERTEX,
        );
        let index: Vec<u32> = (0..count * 2).collect();
        let indices = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("particle indices"),
            contents: bytemuck::cast_slice(&index),
            usage: wgpu::BufferUsages::INDEX,
        });
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("particles"),
            source: wgpu::ShaderSource::Wgsl(include_str!("particles.wgsl").into()),
        });
        let entry = |binding: u32, ty: wgpu::BufferBindingType| wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::COMPUTE,
            ty: wgpu::BindingType::Buffer {
                ty,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        };
        let read = wgpu::BufferBindingType::Storage { read_only: true };
        let write = wgpu::BufferBindingType::Storage { read_only: false };
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("particles"),
            entries: &[
                entry(0, wgpu::BufferBindingType::Uniform),
                entry(1, read),
                entry(2, read),
                entry(3, read),
                entry(4, write),
                entry(5, write),
                entry(6, write),
                entry(7, read),
            ],
        });
        let buffers = [
            &params_buf,
            &levels_buf,
            &field_buf,
            &seeds_buf,
            &state,
            &verts,
            &colors,
            &pressure_buf,
        ];
        let entries: Vec<wgpu::BindGroupEntry<'_>> = buffers
            .iter()
            .enumerate()
            .map(|(i, b)| wgpu::BindGroupEntry {
                binding: i as u32,
                resource: b.as_entire_binding(),
            })
            .collect();
        let bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("particles"),
            layout: &layout,
            entries: &entries,
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("particles"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let pipeline = |entry_point: &str| {
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some(entry_point),
                layout: Some(&pipeline_layout),
                module: &module,
                entry_point: Some(entry_point),
                compilation_options: Default::default(),
                cache: None,
            })
        };
        let object = self.object_bind([1.0, 1.0, 1.0], s.opacity);
        let mut gp = GpuParticles {
            params,
            params_buf,
            bind,
            init: pipeline("init"),
            advect: pipeline("advect"),
            state,
            verts,
            colors,
            indices,
            object,
            count,
        };
        self.dispatch(&mut gp, true);
        gp
    }

    fn dispatch(&self, p: &mut GpuParticles, init: bool) {
        self.queue
            .write_buffer(&p.params_buf, 0, bytemuck::bytes_of(&p.params));
        let mut encoder = self.device.create_command_encoder(&Default::default());
        {
            let mut pass = encoder.begin_compute_pass(&Default::default());
            pass.set_pipeline(if init { &p.init } else { &p.advect });
            pass.set_bind_group(0, &p.bind, &[]);
            pass.dispatch_workgroups(p.count.div_ceil(64), 1, 1);
        }
        self.queue.submit([encoder.finish()]);
    }

    /// Advances the particles by `steps` time steps of `dt` seconds.
    pub fn step_particles(&self, p: &mut GpuParticles, dt: f32, steps: u32) {
        for _ in 0..steps {
            p.params.frame = p.params.frame.wrapping_add(1);
            p.params.dt = dt;
            self.dispatch(p, false);
        }
    }

    /// Positions and ages `(x, y, z, age)` of all particles (for tests and checks).
    pub fn read_particles(&self, p: &GpuParticles) -> Vec<[f32; 4]> {
        let size = u64::from(p.count) * 16;
        let staging = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("particle readback"),
            size,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let mut encoder = self.device.create_command_encoder(&Default::default());
        encoder.copy_buffer_to_buffer(&p.state, 0, &staging, 0, size);
        self.queue.submit([encoder.finish()]);
        let slice = staging.slice(..);
        slice.map_async(wgpu::MapMode::Read, |_| {});
        let _ = self.device.poll(wgpu::PollType::wait_indefinitely());
        let Ok(data) = slice.get_mapped_range() else {
            return Vec::new();
        };
        let out = bytemuck::cast_slice::<u8, [f32; 4]>(&data).to_vec();
        drop(data);
        staging.unmap();
        out
    }
}
