// SPDX-License-Identifier: GPL-3.0-or-later
//! GPU particles: agreement with the CPU reference, repeatable images, golden images of the
//! display styles and a performance measurement (ignored test).

use std::path::PathBuf;

use foamcraft_render::particles::{FlowLevel, ParticleColor, ParticleSettings};
use foamcraft_render::{Renderer, Scene, camera::OrbitCamera};
use foamcraft_sim::particles::{Particles, SeedRegion, VelocityLevel, seed_points};
use foamcraft_sim::results::{GridSpec, grid_from_rows};

const SIZE: u32 = 256;

fn renderer() -> Option<Renderer> {
    let r = Renderer::new_headless();
    if r.is_none() {
        eprintln!("SKIPPED: no GPU adapter available");
    }
    r
}

fn level(n: u32, u: impl Fn([f64; 3]) -> [f64; 3]) -> VelocityLevel {
    let spec = GridSpec {
        min: [0.0; 3],
        max: [1.0; 3],
        dims: [n, n, n],
    };
    let mut rows = Vec::new();
    for k in 0..n {
        for j in 0..n {
            for i in 0..n {
                let p = spec.point(i, j, k);
                let v = u(p);
                rows.push(vec![p[0], p[1], p[2], v[0], v[1], v[2]]);
            }
        }
    }
    VelocityLevel::from_grid(&grid_from_rows(&spec, &["Ux", "Uy", "Uz"], &rows)).unwrap()
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

fn vortex() -> VelocityLevel {
    level(32, |p| [-(p[1] - 0.5), p[0] - 0.5, 0.05])
}

fn settings(count: u32, seed: u32) -> ParticleSettings {
    ParticleSettings {
        count,
        seed,
        max_age: 3.0,
        trail: 0.15,
        color: ParticleColor::Speed,
        range: [0.0, 0.7],
        constant: [1.0; 3],
        opacity: 1.0,
        yaw_deg: 0.0,
    }
}

fn seeds(seed: u32) -> Vec<[f32; 3]> {
    seed_points(
        &SeedRegion {
            origin: [0.2, 0.2, 0.1],
            u: [0.6, 0.0, 0.0],
            v: [0.0, 0.6, 0.0],
        },
        512,
        seed,
    )
}

#[test]
fn gpu_particles_follow_the_cpu_reference() {
    let Some(r) = renderer() else { return };
    let l = vortex();
    let s = seeds(9);
    let mut gpu = r.create_particles(&[gpu_level(&l)], &s, &settings(500, 9));
    let levels = [l];
    let mut cpu = Particles::new(&levels, s, 500, 9, 3.0);
    for _ in 0..40 {
        cpu.step(0.02);
    }
    r.step_particles(&mut gpu, 0.02, 40);
    let g = r.read_particles(&gpu);
    let mut same = 0;
    for (a, b) in g.iter().zip(&cpu.positions) {
        if (0..3).all(|k| (a[k] - b[k]).abs() < 1e-3) {
            same += 1;
        }
    }
    // Rounding may flip a rare respawn decision at a boundary; nearly all must agree.
    assert!(same >= 495, "{same} of 500 agree");
}

fn render_particles(
    r: &Renderer,
    seed: u32,
    steps: u32,
    color: ParticleColor,
    opacity: f32,
) -> Vec<u8> {
    let l = vortex();
    let mut s = settings(4000, seed);
    s.color = color;
    s.opacity = opacity;
    s.constant = [0.9, 0.9, 0.9];
    let mut gpu = r.upload(&Scene::empty());
    let mut p = r.create_particles(&[gpu_level(&l)], &seeds(seed), &s);
    r.step_particles(&mut p, 0.02, steps);
    gpu.particles = Some(p);
    let mut camera = OrbitCamera::default();
    camera.orbit(0.0, 1.5);
    camera.fit(foamcraft_render::glam::Vec3::new(0.5, 0.5, 0.2), 0.6, 1.0);
    r.render_offscreen_gpu(&gpu, &camera, SIZE, SIZE)
}

#[test]
fn the_same_seed_gives_the_same_image() {
    let Some(r) = renderer() else { return };
    let a = render_particles(&r, 5, 30, ParticleColor::Speed, 1.0);
    let b = render_particles(&r, 5, 30, ParticleColor::Speed, 1.0);
    assert_eq!(a, b);
    let c = render_particles(&r, 6, 30, ParticleColor::Speed, 1.0);
    assert_ne!(a, c);
}

fn golden(name: &str, pixels: &[u8]) {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/golden")
        .join(name);
    if std::env::var_os("FOAMCRAFT_UPDATE_GOLDEN").is_some() || !path.is_file() {
        foamcraft_render::write_png(&path, SIZE, SIZE, pixels).expect("write golden");
    }
    let decoder = png::Decoder::new(std::io::BufReader::new(std::fs::File::open(&path).unwrap()));
    let mut reader = decoder.read_info().unwrap();
    let mut buf = vec![0; reader.output_buffer_size().unwrap()];
    reader.next_frame(&mut buf).unwrap();
    let off = pixels
        .chunks(4)
        .zip(buf.chunks(4))
        .filter(|(a, b)| a.iter().zip(b.iter()).any(|(x, y)| x.abs_diff(*y) > 8))
        .count();
    let share = off as f64 / f64::from(SIZE * SIZE);
    assert!(
        share <= 0.005,
        "{name}: {:.3}% of pixels differ",
        share * 100.0
    );
}

#[test]
fn particle_styles_match_golden_images() {
    let Some(r) = renderer() else { return };
    golden(
        "particles_speed.png",
        &render_particles(&r, 1, 40, ParticleColor::Speed, 1.0),
    );
    golden(
        "particles_age.png",
        &render_particles(&r, 1, 40, ParticleColor::Age, 1.0),
    );
    golden(
        "particles_smoke.png",
        &render_particles(&r, 1, 40, ParticleColor::Constant, 0.5),
    );
}

#[test]
fn streamlines_match_golden_image() {
    let Some(r) = renderer() else { return };
    let l = vortex();
    let levels = [l];
    let lines: Vec<Vec<[f32; 3]>> = seeds(3)
        .iter()
        .take(40)
        .map(|s| foamcraft_sim::particles::streamline(&levels, *s, 0.02, 300))
        .collect();
    let mesh = foamcraft_render::scene::polylines(&lines, [0.3, 0.8, 0.9], false);
    let scene = Scene {
        meshes: vec![mesh],
        background: Scene::empty().background,
    };
    let mut camera = OrbitCamera::default();
    camera.orbit(0.0, 1.5);
    camera.fit(foamcraft_render::glam::Vec3::new(0.5, 0.5, 0.2), 0.6, 1.0);
    golden(
        "streamlines.png",
        &r.render_offscreen(&scene, &camera, SIZE, SIZE),
    );
}

/// Performance on this computer: 200 000 particles in the three grid levels of a wind tunnel
/// result size (3 × 64³ points). Run with `-- --ignored --nocapture`.
#[test]
#[ignore = "performance measurement"]
fn performance_of_200000_particles() {
    let Some(r) = renderer() else { return };
    let levels: Vec<FlowLevel> = (0..3)
        .map(|_| gpu_level(&level(64, |p| [1.0, 0.1 * p[2], 0.0])))
        .collect();
    let mut s = settings(200_000, 1);
    s.max_age = 2.0;
    let seed = seed_points(
        &SeedRegion {
            origin: [0.05, 0.1, 0.1],
            u: [0.0, 0.8, 0.0],
            v: [0.0, 0.0, 0.8],
        },
        4096,
        1,
    );
    let mut gpu = r.upload(&Scene::empty());
    let mut p = r.create_particles(&levels, &seed, &s);
    r.step_particles(&mut p, 0.01, 10);
    let _ = r.read_particles(&p);
    gpu.particles = Some(p);
    let camera = OrbitCamera::default();
    let frames = 120;
    let start = std::time::Instant::now();
    for _ in 0..frames {
        if let Some(p) = gpu.particles.as_mut() {
            r.step_particles(p, 0.01, 1);
        }
        let _ = r.render_offscreen_gpu(&gpu, &camera, 1280, 800);
    }
    let per_frame = start.elapsed().as_secs_f64() / f64::from(frames);
    eprintln!(
        "200 000 particles, 1280 x 800 with read-back: {:.2} ms per frame ({:.0} frames/s), adapter {:?}",
        per_frame * 1000.0,
        1.0 / per_frame,
        r.adapter.get_info().name
    );
}

// --- Task_27: cutting the smoke open ---

#[test]
fn cut_particles_lie_on_the_visible_side() {
    let l = vortex();
    let mut p = Particles::new(std::slice::from_ref(&l), seeds(3), 2000, 3, 3.0);
    for _ in 0..20 {
        p.step(0.02);
    }
    let origin = [0.5, 0.5, 0.0];
    let normal = [1.0, 0.0, 0.0];
    let shown = foamcraft_sim::particles::visible_positions(&p.positions, origin, normal);
    assert!(!shown.is_empty() && shown.len() < p.positions.len());
    assert!(shown.iter().all(|q| q[0] <= 0.5));
}

#[test]
fn cut_smoke_matches_golden_image_and_no_cut_is_unchanged() {
    let Some(r) = renderer() else { return };
    let l = vortex();
    let mut s = settings(4000, 1);
    s.color = ParticleColor::Constant;
    s.opacity = 0.5;
    s.constant = [0.9, 0.9, 0.9];
    let draw = |clip: Option<([f32; 3], [f32; 3])>| {
        let mut gpu = r.upload(&Scene::empty());
        let mut p = r.create_particles(&[gpu_level(&l)], &seeds(1), &s);
        r.set_particle_clip(&mut p, clip);
        r.step_particles(&mut p, 0.02, 40);
        gpu.particles = Some(p);
        let mut camera = OrbitCamera::default();
        camera.orbit(0.0, 1.5);
        camera.fit(foamcraft_render::glam::Vec3::new(0.5, 0.5, 0.2), 0.6, 1.0);
        r.render_offscreen_gpu(&gpu, &camera, SIZE, SIZE)
    };
    // No plane gives exactly the picture without the feature.
    assert_eq!(
        draw(None),
        render_particles(&r, 1, 40, ParticleColor::Constant, 0.5)
    );
    let cut = draw(Some(([0.5, 0.5, 0.0], [1.0, 0.0, 0.0])));
    assert_ne!(cut, draw(None));
    golden("particles_smoke_cut.png", &cut);
}

// --- Task_28: smoke lines from a rake of nozzles ---

#[test]
fn rake_points_lie_close_to_their_nozzles() {
    let pts =
        foamcraft_sim::particles::rake_points([0.0, 0.5, 0.0], [0.0, 0.0, 1.0], 0.01, 11, 2200, 3);
    assert_eq!(pts.len(), 2200);
    for p in &pts {
        // On the line x = 0, y = 0.5 within the scatter radius, z within 0..1.
        let r = (p[0] * p[0] + (p[1] - 0.5) * (p[1] - 0.5)).sqrt();
        assert!(r <= 0.0101, "{p:?}");
        assert!(p[2] >= -0.011 && p[2] <= 1.011, "{p:?}");
        // Close to one of the eleven nozzles at z = k / 10.
        let k = (p[2] * 10.0).round();
        assert!((p[2] - k / 10.0).abs() <= 0.0101, "{p:?}");
    }
}

#[test]
fn smoke_lines_match_golden_image() {
    let Some(r) = renderer() else { return };
    let l = vortex();
    let mut s = settings(6000, 2);
    s.color = ParticleColor::Speed;
    s.opacity = 0.5;
    let seeds =
        foamcraft_sim::particles::rake_points([0.15, 0.5, 0.2], [0.0, 0.0, 0.0], 0.004, 9, 4096, 2)
            .into_iter()
            .enumerate()
            .map(|(i, mut p)| {
                // Nozzles along y across the vortex.
                p[1] = 0.1 + 0.8 * ((i % 9) as f32) / 8.0 + (p[1] - 0.5);
                p
            })
            .collect::<Vec<_>>();
    let mut gpu = r.upload(&Scene::empty());
    let mut p = r.create_particles(&[gpu_level(&l)], &seeds, &s);
    r.step_particles(&mut p, 0.02, 60);
    gpu.particles = Some(p);
    let mut camera = OrbitCamera::default();
    camera.orbit(0.0, 1.5);
    camera.fit(foamcraft_render::glam::Vec3::new(0.5, 0.5, 0.2), 0.6, 1.0);
    golden(
        "smoke_lines.png",
        &r.render_offscreen_gpu(&gpu, &camera, SIZE, SIZE),
    );
}
