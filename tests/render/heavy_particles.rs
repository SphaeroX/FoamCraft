// SPDX-License-Identifier: GPL-3.0-or-later
//! Heavy particles on the GPU (Task_34): the shader follows its CPU reference, no particle ends up
//! inside the solid, and a golden image. Without a GPU adapter the tests print `SKIPPED`.

use std::path::PathBuf;

use foamcraft_render::camera::OrbitCamera;
use foamcraft_render::particles::{FlowLevel, HeavySettings, ParticleColor, ParticleSettings};
use foamcraft_render::{Renderer, Scene};
use foamcraft_sim::heavy_gpu::{HeavyProps, HeavyReference, distance_at};
use foamcraft_sim::particles::{SeedRegion, VelocityLevel, seed_points};
use foamcraft_sim::results::{GridSpec, grid_from_rows};

const SIZE: u32 = 256;

fn renderer() -> Option<Renderer> {
    let r = Renderer::new_headless();
    if r.is_none() {
        eprintln!("SKIPPED: no GPU adapter available");
    }
    r
}

/// The solid: a block 0.6..0.75 in x, 0.25..0.75 in y and z.
const LO: [f64; 3] = [0.6, 0.25, 0.25];
const HI: [f64; 3] = [0.75, 0.75, 0.75];

fn signed_distance(p: [f64; 3]) -> f64 {
    let mut outside = 0.0f64;
    let mut inside = f64::MAX;
    for a in 0..3 {
        let d = (LO[a] - p[a]).max(p[a] - HI[a]);
        if d > 0.0 {
            outside += d * d;
        }
        inside = inside.min((p[a] - LO[a]).min(HI[a] - p[a]));
    }
    if outside > 0.0 {
        outside.sqrt()
    } else {
        -inside
    }
}

/// Uniform flow along +x around the block (32³ points); distance per point.
fn level() -> (VelocityLevel, Vec<f32>) {
    let spec = GridSpec {
        min: [0.0; 3],
        max: [1.0; 3],
        dims: [32, 32, 32],
    };
    let mut rows = Vec::new();
    let mut dist = Vec::new();
    for k in 0..32 {
        for j in 0..32 {
            for i in 0..32 {
                let p = spec.point(i, j, k);
                dist.push(signed_distance(p) as f32);
                if signed_distance(p) < 0.0 {
                    continue;
                }
                rows.push(vec![p[0], p[1], p[2], 0.5, 0.0, 0.0]);
            }
        }
    }
    let l = VelocityLevel::from_grid(&grid_from_rows(&spec, &["Ux", "Uy", "Uz"], &rows)).unwrap();
    (l, dist)
}

fn gpu_level(l: &VelocityLevel) -> FlowLevel {
    FlowLevel {
        min: l.min,
        max: l.max,
        dims: l.dims,
        data: l.data.clone(),
        pressure: vec![],
    }
}

fn settings(count: u32) -> ParticleSettings {
    ParticleSettings {
        count,
        seed: 4,
        max_age: 4.0,
        trail: 0.05,
        color: ParticleColor::Speed,
        range: [0.0, 0.6],
        constant: [1.0; 3],
        opacity: 1.0,
        yaw_deg: 0.0,
    }
}

fn heavy() -> HeavySettings {
    HeavySettings {
        tau_p: 0.5,
        diameter: 1e-3,
        nu: 1.5e-5,
        stick: true,
        gravity: [0.0, 0.0, -0.2],
        restitution: 0.3,
        friction: 0.2,
        stick_time: 0.6,
        mark: 0.02,
        settle: [0.0, 0.0, -0.1],
    }
}

fn props(h: &HeavySettings) -> HeavyProps {
    HeavyProps {
        tau_p: h.tau_p,
        diameter: h.diameter,
        nu: h.nu,
        stick: h.stick,
        gravity: h.gravity,
        restitution: h.restitution,
        friction: h.friction,
        stick_time: h.stick_time,
        mark: h.mark,
        settle: h.settle,
    }
}

fn seeds() -> Vec<[f32; 3]> {
    seed_points(
        &SeedRegion {
            origin: [0.1, 0.2, 0.2],
            u: [0.0, 0.6, 0.0],
            v: [0.0, 0.0, 0.6],
        },
        512,
        4,
    )
}

#[test]
fn gpu_heavy_particles_follow_the_cpu_reference_and_stay_out_of_the_solid() {
    let Some(r) = renderer() else { return };
    let (l, dist) = level();
    let h = heavy();
    let mut gpu = r.create_heavy_particles(&[gpu_level(&l)], &seeds(), &settings(500), &h, &dist);
    let levels = [l];
    let mut cpu = HeavyReference::new(&levels, &dist, seeds(), 500, 4, 4.0, props(&h));
    for _ in 0..200 {
        cpu.step(0.01);
    }
    r.step_particles(&mut gpu, 0.01, 200);
    let g = r.read_particles(&gpu);
    let m = r.read_motion(&gpu);
    let mut same = 0;
    for (a, b) in g.iter().zip(&cpu.state) {
        if (0..3).all(|k| (a[k] - b[k]).abs() < 1e-3) {
            same += 1;
        }
    }
    assert!(same >= 490, "{same} of 500 agree");
    // Some stick on the block's front face; none is inside the solid.
    let stuck = m.iter().filter(|x| x[3] >= 0.0).count();
    assert!(stuck > 0, "none stuck");
    for p in &g {
        let d = distance_at(&levels, &dist, [p[0], p[1], p[2]]);
        assert!(d > -0.02, "inside the solid: {p:?} {d}");
    }
}

#[test]
fn heavy_particles_match_the_golden_image() {
    let Some(r) = renderer() else { return };
    let (l, dist) = level();
    let mut gpu = r.upload(&Scene::empty());
    let mut p =
        r.create_heavy_particles(&[gpu_level(&l)], &seeds(), &settings(3000), &heavy(), &dist);
    r.step_particles(&mut p, 0.01, 150);
    gpu.particles = Some(p);
    let mut camera = OrbitCamera::default();
    camera.fit(foamcraft_render::glam::Vec3::new(0.5, 0.5, 0.5), 0.7, 1.0);
    let px = r.render_offscreen_gpu(&gpu, &camera, SIZE, SIZE);
    let path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/golden/particles_heavy.png");
    if std::env::var_os("FOAMCRAFT_UPDATE_GOLDEN").is_some() {
        foamcraft_render::write_png(&path, SIZE, SIZE, &px).expect("write golden");
    }
    let decoder = png::Decoder::new(std::io::BufReader::new(
        std::fs::File::open(&path).expect("open golden image"),
    ));
    let mut reader = decoder.read_info().expect("png header");
    let mut golden = vec![0; reader.output_buffer_size().expect("buffer size")];
    let info = reader.next_frame(&mut golden).expect("png frame");
    golden.truncate(info.buffer_size());
    let off = px
        .as_chunks::<4>()
        .0
        .iter()
        .zip(golden.as_chunks::<4>().0)
        .filter(|(a, b)| a.iter().zip(*b).any(|(x, y)| x.abs_diff(*y) > 8))
        .count();
    let share = off as f64 / f64::from(SIZE * SIZE);
    assert!(share <= 0.005, "{share} of the pixels differ");
}
