// SPDX-License-Identifier: GPL-3.0-or-later
//! Light trails (Task_35): the ring follows the CPU reference, new starts break the trail,
//! the memory limit, cutting at the near plane, hiding behind the model, golden images and the
//! long exposure. Without a GPU adapter the GPU tests print `SKIPPED`.

use std::path::PathBuf;

use foamcraft_render::camera::OrbitCamera;
use foamcraft_render::glam::{Vec3, Vec4};
use foamcraft_render::particles::{FlowLevel, ParticleColor, ParticleSettings};
use foamcraft_render::scene::cube_mesh;
use foamcraft_render::trails::{TrailSettings, cut_near, points_per_trail, slot_behind};
use foamcraft_render::{Renderer, Scene};
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

fn vortex() -> VelocityLevel {
    let spec = GridSpec {
        min: [0.0; 3],
        max: [1.0; 3],
        dims: [32, 32, 32],
    };
    let mut rows = Vec::new();
    for k in 0..32 {
        for j in 0..32 {
            for i in 0..32 {
                let p = spec.point(i, j, k);
                rows.push(vec![p[0], p[1], p[2], -(p[1] - 0.5), p[0] - 0.5, 0.05]);
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

fn settings(count: u32, max_age: f32) -> ParticleSettings {
    ParticleSettings {
        count,
        seed: 9,
        max_age,
        trail: 0.15,
        color: ParticleColor::Speed,
        range: [0.0, 0.7],
        constant: [1.0; 3],
        opacity: 1.0,
        yaw_deg: 0.0,
    }
}

fn trail(k: u32, every: u32) -> TrailSettings {
    TrailSettings {
        k,
        every,
        width_px: 3.0,
        exposure: 1.5,
        glow: false,
        by_speed: true,
        range: [0.0, 0.7],
        color: [1.0, 0.8, 0.4],
    }
}

fn seeds() -> Vec<[f32; 3]> {
    seed_points(
        &SeedRegion {
            origin: [0.2, 0.2, 0.1],
            u: [0.6, 0.0, 0.0],
            v: [0.0, 0.6, 0.0],
        },
        512,
        9,
    )
}

fn camera() -> OrbitCamera {
    let mut c = OrbitCamera::default();
    c.orbit(0.0, 1.5);
    c.fit(Vec3::new(0.5, 0.5, 0.2), 0.6, 1.0);
    c
}

#[test]
fn the_memory_limit_shortens_the_trails_and_the_order_runs_backwards() {
    assert_eq!(points_per_trail(50_000, 24), 24);
    assert_eq!(points_per_trail(200_000, 24), 12);
    assert_eq!(points_per_trail(10_000_000, 24), 2);
    assert_eq!(slot_behind(5, 0, 24), 5);
    assert_eq!(slot_behind(5, 1, 24), 4);
    assert_eq!(slot_behind(0, 1, 24), 23);
    assert_eq!(slot_behind(3, 24, 24), 3);
}

#[test]
fn a_piece_behind_the_camera_is_cut_to_finite_screen_coordinates() {
    let a = Vec4::new(0.5, 0.2, -0.3, 0.1);
    let b = Vec4::new(-0.2, 0.4, 0.6, 2.0);
    let c = cut_near(a, b);
    assert!(c.z.abs() < 1e-6);
    assert!(c.w > 0.0 && (c.x / c.w).is_finite() && (c.y / c.w).is_finite());
    assert_eq!(cut_near(b, a), b);
}

#[test]
fn the_ring_holds_the_last_positions_of_the_cpu_reference() {
    let Some(r) = renderer() else { return };
    let l = vortex();
    let k = 8;
    let mut gpu = r.create_trail_particles(
        &[gpu_level(&l)],
        &seeds(),
        &settings(300, 50.0),
        &trail(k, 1),
    );
    let levels = [l];
    let mut cpu = Particles::new(&levels, seeds(), 300, 9, 50.0);
    let mut history: Vec<Vec<[f32; 3]>> = Vec::new();
    let steps = 20u32;
    for _ in 0..steps {
        cpu.step(0.02);
        history.push(cpu.positions.clone());
    }
    r.step_particles(&mut gpu, 0.02, steps);
    let ring = r.read_trails(&gpu);
    // Every step writes: the newest slot is steps mod k.
    let slot = steps % k;
    let mut same = 0;
    let mut total = 0;
    for i in 0..300usize {
        for j in 0..(k - 1) {
            let g = ring[i * k as usize + slot_behind(slot, j, k) as usize];
            let c = history[(steps - 1 - j) as usize][i];
            total += 1;
            if (0..3).all(|a| (g[a] - c[a]).abs() < 1e-3) {
                same += 1;
            }
        }
    }
    assert!(same >= total * 98 / 100, "{same} of {total}");
}

#[test]
fn a_new_start_breaks_the_trail() {
    let Some(r) = renderer() else { return };
    let l = vortex();
    // Short lives: many particles start again.
    let mut gpu = r.create_trail_particles(
        &[gpu_level(&l)],
        &seeds(),
        &settings(300, 0.1),
        &trail(8, 1),
    );
    r.step_particles(&mut gpu, 0.02, 30);
    let ring = r.read_trails(&gpu);
    let breaks = ring.iter().filter(|p| p[3] < 0.0).count();
    assert!(breaks > 100, "{breaks}");
    assert!(ring.iter().any(|p| p[3] > 0.0));
}

fn scene_with_cube() -> Scene {
    Scene {
        meshes: vec![cube_mesh(Vec3::new(0.5, 0.5, 0.6), 0.25, [0.2, 0.3, 0.5])],
        background: Scene::empty().background,
    }
}

fn render(r: &Renderer, glow: bool, transparent: bool) -> Vec<u8> {
    let l = vortex();
    let mut t = trail(24, 2);
    t.glow = glow;
    let mut scene = scene_with_cube();
    if transparent {
        scene.background[3] = 0.0;
    }
    let mut gpu = r.upload(&scene);
    let mut p = r.create_trail_particles(&[gpu_level(&l)], &seeds(), &settings(3000, 3.0), &t);
    r.step_particles(&mut p, 0.02, 80);
    gpu.particles = Some(p);
    r.render_offscreen_gpu(&gpu, &camera(), SIZE, SIZE)
}

fn compare(name: &str, pixels: &[u8]) {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/golden")
        .join(name);
    if std::env::var_os("FOAMCRAFT_UPDATE_GOLDEN").is_some() {
        foamcraft_render::write_png(&path, SIZE, SIZE, pixels).expect("write golden");
    }
    let decoder = png::Decoder::new(std::io::BufReader::new(
        std::fs::File::open(&path).expect("open golden image"),
    ));
    let mut reader = decoder.read_info().expect("png header");
    let mut golden = vec![0; reader.output_buffer_size().expect("buffer size")];
    let info = reader.next_frame(&mut golden).expect("png frame");
    golden.truncate(info.buffer_size());
    let off = pixels
        .chunks(4)
        .zip(golden.chunks(4))
        .filter(|(a, b)| a.iter().zip(*b).any(|(x, y)| x.abs_diff(*y) > 8))
        .count();
    let share = off as f64 / f64::from(SIZE * SIZE);
    assert!(share <= 0.005, "{name}: {share} of the pixels differ");
}

#[test]
fn trails_match_the_golden_image_repeat_and_hide_behind_the_model() {
    let Some(r) = renderer() else { return };
    let a = render(&r, true, false);
    assert_eq!(a, render(&r, true, false));
    compare("particles_trails.png", &a);
    // The cube is in front of the trails: its middle keeps the cube's own colour.
    let plain = r.render_offscreen(&scene_with_cube(), &camera(), SIZE, SIZE);
    let at = |px: &[u8], x: u32, y: u32| {
        let i = ((y * SIZE + x) * 4) as usize;
        [px[i], px[i + 1], px[i + 2]]
    };
    let bg = Scene::empty().background_rgba8();
    let cube: Vec<(u32, u32)> = (0..SIZE * SIZE)
        .map(|i| (i % SIZE, i / SIZE))
        .filter(|(x, y)| at(&plain, *x, *y) != [bg[0], bg[1], bg[2]])
        .collect();
    assert!(!cube.is_empty());
    let n = cube.len() as u32;
    let cx = cube.iter().map(|p| p.0).sum::<u32>() / n;
    let cy = cube.iter().map(|p| p.1).sum::<u32>() / n;
    let (p, t) = (at(&plain, cx, cy), at(&a, cx, cy));
    assert!(
        p.iter().zip(t).all(|(x, y)| x.abs_diff(y) <= 2),
        "{p:?} {t:?}"
    );
    // Around the cube the trails do light the picture.
    let lit = (0..SIZE * SIZE)
        .map(|i| (i % SIZE, i / SIZE))
        .filter(|(x, y)| at(&a, *x, *y) != at(&plain, *x, *y))
        .count();
    assert!(lit > 2000, "{lit}");
}

#[test]
fn a_transparent_background_gets_cover_where_trails_glow() {
    let Some(r) = renderer() else { return };
    let px = render(&r, false, true);
    let alphas: Vec<u8> = px.chunks(4).map(|c| c[3]).collect();
    assert!(alphas.contains(&0), "no transparent pixel");
    assert!(
        alphas.iter().any(|a| *a > 30 && *a < 255),
        "no glowing cover"
    );
}

#[test]
fn the_long_exposure_repeats_and_matches_the_golden_image() {
    let Some(r) = renderer() else { return };
    let shot = || {
        let l = vortex();
        let mut gpu = r.upload(&Scene::empty());
        gpu.particles = Some(r.create_trail_particles(
            &[gpu_level(&l)],
            &seeds(),
            &settings(2000, 3.0),
            &trail(24, 2),
        ));
        r.render_offscreen_exposure(&mut gpu, &camera(), SIZE, SIZE, 120, 0.02)
    };
    let a = shot();
    assert_eq!(a, shot());
    compare("long_exposure.png", &a);
}

#[test]
fn the_long_exposure_in_the_view_keeps_the_light_until_the_camera_moves() {
    let Some(r) = renderer() else { return };
    let l = vortex();
    let make = |steps: u32| {
        let mut gpu = r.upload(&Scene::empty());
        let mut p = r.create_trail_particles(
            &[gpu_level(&l)],
            &seeds(),
            &settings(2000, 3.0),
            &trail(24, 2),
        );
        r.set_trail_fade(&mut p, Some(4.0));
        r.step_particles(&mut p, 0.02, steps);
        gpu.particles = Some(p);
        gpu
    };
    let mut kept = make(40);
    r.render_offscreen_gpu(&kept, &camera(), SIZE, SIZE);
    r.step_particles(kept.particles.as_mut().unwrap(), 0.02, 10);
    let second = r.render_offscreen_gpu(&kept, &camera(), SIZE, SIZE);
    let fresh = r.render_offscreen_gpu(&make(50), &camera(), SIZE, SIZE);
    assert_ne!(second, fresh, "the light of the first frame is kept");
    // A moved camera starts the light again.
    let mut moved = camera();
    moved.orbit(0.3, 0.0);
    let after_move = r.render_offscreen_gpu(&kept, &moved, SIZE, SIZE);
    let fresh_moved = r.render_offscreen_gpu(&make(50), &moved, SIZE, SIZE);
    assert_eq!(after_move, fresh_moved);
}

/// Frame time of 100 000 particles with K = 24 and glow at 1280 × 800 and at 4K (results in the
/// notes of Task_35): `cargo test -p foamcraft-render --test trails -- --ignored --nocapture`.
#[test]
#[ignore]
fn performance_of_100000_particles_with_trails() {
    let Some(r) = renderer() else { return };
    let l = vortex();
    let mut t = trail(24, 2);
    t.glow = true;
    let mut gpu = r.upload(&Scene::empty());
    gpu.particles =
        Some(r.create_trail_particles(&[gpu_level(&l)], &seeds(), &settings(100_000, 3.0), &t));
    for (w, h) in [(1280, 800), (3840, 2160)] {
        // Warm up (pipelines, light images), then measure.
        r.step_particles(gpu.particles.as_mut().unwrap(), 0.02, 50);
        r.render_offscreen_gpu(&gpu, &camera(), w, h);
        let frames = 20;
        let start = std::time::Instant::now();
        for _ in 0..frames {
            r.step_particles(gpu.particles.as_mut().unwrap(), 0.02, 1);
            r.render_offscreen_gpu(&gpu, &camera(), w, h);
        }
        let ms = start.elapsed().as_secs_f64() * 1000.0 / f64::from(frames);
        eprintln!("{w}x{h}: {ms:.1} ms per frame (with read-back)");
    }
}
