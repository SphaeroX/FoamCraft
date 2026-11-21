// SPDX-License-Identifier: GPL-3.0-or-later
//! Surface flow in the renderer (Task_33): lines with running lights and fluttering tufts depend
//! only on the time; golden images for both. Without a GPU adapter the tests print `SKIPPED`.

use std::path::PathBuf;

use foamcraft_render::camera::OrbitCamera;
use foamcraft_render::flow::{FlowGeometry, FlowVertex};
use foamcraft_render::scene::mesh_from_triangles;
use foamcraft_render::{Renderer, Scene};
use foamcraft_sim::surface_flow::{SurfaceMesh, TangentField, even_lines, tuft_feet, tuft_rest};
use glam::DVec3;

const SIZE: u32 = 256;
const CHANNEL_TOLERANCE: u8 = 8;
const PIXEL_TOLERANCE: f64 = 0.005;

fn renderer() -> Option<Renderer> {
    let r = Renderer::new_headless();
    if r.is_none() {
        assert!(
            std::env::var_os("FOAMCRAFT_REQUIRE_GPU").is_none(),
            "no GPU adapter"
        );
        eprintln!("SKIPPED: no GPU adapter available");
    }
    r
}

fn sphere(n: u32) -> (Vec<[f32; 3]>, Vec<[u32; 3]>) {
    let mut pts = Vec::new();
    for i in 0..=n {
        let th = std::f64::consts::PI * f64::from(i) / f64::from(n);
        for j in 0..2 * n {
            let ph = std::f64::consts::PI * f64::from(j) / f64::from(n);
            pts.push([
                th.cos() as f32,
                (th.sin() * ph.cos()) as f32,
                (th.sin() * ph.sin()) as f32,
            ]);
        }
    }
    let m = 2 * n;
    let mut tris = Vec::new();
    for i in 0..n {
        for j in 0..m {
            let a = i * m + j;
            let b = i * m + (j + 1) % m;
            let c = (i + 1) * m + j;
            let d = (i + 1) * m + (j + 1) % m;
            tris.push([a, c, d]);
            tris.push([a, d, b]);
        }
    }
    (pts, tris)
}

fn f3(v: DVec3) -> [f32; 3] {
    [v.x as f32, v.y as f32, v.z as f32]
}

/// Sphere with pole-to-pole lines and tufts.
fn scene() -> (Scene, FlowGeometry) {
    let (pts, tris) = sphere(24);
    let mesh = SurfaceMesh::new(&pts, &tris, 1e-6);
    let dir: Vec<DVec3> = (0..mesh.tris.len())
        .map(|t| {
            let r = mesh.centroid(t).normalize();
            DVec3::X - r * r.x
        })
        .collect();
    let field = TangentField::new(&mesh, &dir);
    let mut geo = FlowGeometry::default();
    for line in even_lines(&mesh, &field, 0.15, 1, 6.0) {
        let times = line.runtimes(1.0);
        for k in 1..line.points.len() {
            for i in [k - 1, k] {
                let p = line.points[i].pos;
                geo.lines.push(FlowVertex {
                    pos: f3(p * 1.004),
                    color: [0.9, 0.92, 0.95],
                    lateral: [0.0; 3],
                    p: [times[i] as f32, line.phase as f32, 1.5, 0.0],
                    q: 0.0,
                });
            }
        }
    }
    for (t, foot) in tuft_feet(&mesh, 0.35, 2, 200) {
        let tuft = tuft_rest(&mesh, &field, t, foot, 0.2, 0.5, -DVec3::Z);
        let base = geo.tufts.len() as u32;
        for (k, p) in tuft.points.iter().enumerate() {
            for side in [-1.0, 1.0] {
                geo.tufts.push(FlowVertex {
                    pos: f3(*p + tuft.lateral * (side * 0.008)),
                    color: [1.0, 0.85, 0.55],
                    lateral: f3(tuft.lateral),
                    p: [(k as f64 * 0.05) as f32, 0.6, 4.0, (t % 7) as f32],
                    q: 1.3,
                });
            }
        }
        for k in 0..4u32 {
            let (a, b, c, d) = (
                base + 2 * k,
                base + 2 * k + 1,
                base + 2 * k + 2,
                base + 2 * k + 3,
            );
            geo.tuft_indices.extend([a, b, d, a, d, c]);
        }
    }
    let scene = Scene {
        meshes: vec![mesh_from_triangles(&pts, &tris, [0.12, 0.14, 0.18])],
        background: Scene::empty().background,
    };
    (scene, geo)
}

fn render(r: &Renderer, lines: bool, tufts: bool, time: f32) -> Vec<u8> {
    let (scene, mut geo) = scene();
    if !lines {
        geo.lines.clear();
    }
    if !tufts {
        geo.tufts.clear();
        geo.tuft_indices.clear();
    }
    let mut gpu = r.upload(&scene);
    r.set_surface_flow(&mut gpu, Some(&geo));
    gpu.time = time;
    let mut cam = OrbitCamera::default();
    cam.fit(glam::Vec3::ZERO, 1.05, 1.0);
    r.render_offscreen_gpu(&gpu, &cam, SIZE, SIZE)
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
        .as_chunks::<4>()
        .0
        .iter()
        .zip(golden.as_chunks::<4>().0)
        .filter(|(a, b)| {
            a.iter()
                .zip(*b)
                .any(|(x, y)| x.abs_diff(*y) > CHANNEL_TOLERANCE)
        })
        .count();
    let share = off as f64 / f64::from(SIZE * SIZE);
    assert!(
        share <= PIXEL_TOLERANCE,
        "{name}: {share} of the pixels differ"
    );
}

#[test]
fn surface_lines_match_the_golden_image() {
    let Some(r) = renderer() else { return };
    compare("surface_lines.png", &render(&r, true, false, 0.7));
}

#[test]
fn tufts_match_the_golden_image() {
    let Some(r) = renderer() else { return };
    compare("tufts.png", &render(&r, false, true, 0.7));
}

#[test]
fn the_picture_depends_on_the_time_only() {
    let Some(r) = renderer() else { return };
    let a = render(&r, true, true, 0.7);
    let b = render(&r, true, true, 0.7);
    let c = render(&r, true, true, 1.1);
    assert_eq!(a, b);
    let differ = a.iter().zip(&c).filter(|(x, y)| x != y).count();
    assert!(differ > 100, "{differ}");
}

#[test]
fn without_surface_flow_the_scene_is_unchanged() {
    let Some(r) = renderer() else { return };
    let (scene, _) = scene();
    let plain = r.render_offscreen(
        &scene,
        &{
            let mut c = OrbitCamera::default();
            c.fit(glam::Vec3::ZERO, 1.05, 1.0);
            c
        },
        SIZE,
        SIZE,
    );
    assert_eq!(render(&r, false, false, 0.7), plain);
}

/// A ribbon and a tube along a helix around the z axis with time marks.
fn ribbon_scene(tube: bool, time: f32) -> Vec<u8> {
    use foamcraft_sim::ribbons::{frames, ribbon, tube as tube_rings};
    let Some(r) = renderer() else { return vec![] };
    let n = 160;
    let pts: Vec<DVec3> = (0..=n)
        .map(|i| {
            let a = f64::from(i) / f64::from(n) * 2.5 * std::f64::consts::TAU;
            DVec3::new(
                0.6 * a.cos(),
                0.6 * a.sin(),
                -0.8 + 1.6 * f64::from(i) / f64::from(n),
            )
        })
        .collect();
    let taus: Vec<f64> = (0..=n).map(|i| f64::from(i) * 0.01).collect();
    let speeds: Vec<f64> = (0..=n).map(|i| 0.5 + f64::from(i) / f64::from(n)).collect();
    let omega = vec![DVec3::new(0.0, 0.0, 20.0); pts.len()];
    let fr = frames(&pts, &taus, &speeds, &omega);
    let mut geo = FlowGeometry::default();
    let p = |tau: f64| [tau as f32, 0.0, 0.1, 0.0];
    if tube {
        let rings = tube_rings(&fr, 1.0, 0.06, 8);
        for (f, ring) in fr.iter().zip(&rings) {
            for (j, (q, nrm)) in ring.iter().enumerate() {
                let c = if j == 0 {
                    [0.1, 0.1, 0.1]
                } else {
                    [0.3, 0.7, 0.9]
                };
                geo.ribbons.push(FlowVertex {
                    pos: f3(*q),
                    color: c,
                    lateral: f3(*nrm),
                    p: p(f.tau),
                    q: 0.0,
                });
            }
        }
        for i in 0..n {
            for j in 0..8u32 {
                let a = i * 8 + j;
                let b = i * 8 + (j + 1) % 8;
                geo.ribbon_indices.extend([a, b, b + 8, a, b + 8, a + 8]);
            }
        }
    } else {
        for (f, [a, b]) in fr.iter().zip(ribbon(&fr, 0.15)) {
            for q in [a, b] {
                geo.ribbons.push(FlowVertex {
                    pos: f3(q),
                    color: [0.95, 0.6, 0.15],
                    lateral: f3(f.n),
                    p: p(f.tau),
                    q: 0.0,
                });
            }
        }
        for i in 0..n {
            let (a, b, c, d) = (2 * i, 2 * i + 1, 2 * i + 2, 2 * i + 3);
            geo.ribbon_indices.extend([a, b, d, a, d, c]);
        }
    }
    let mut gpu = r.upload(&Scene::empty());
    r.set_surface_flow(&mut gpu, Some(&geo));
    gpu.time = time;
    let mut cam = OrbitCamera::default();
    cam.fit(glam::Vec3::ZERO, 1.1, 1.0);
    r.render_offscreen_gpu(&gpu, &cam, SIZE, SIZE)
}

#[test]
fn ribbons_and_tubes_match_their_golden_images() {
    let Some(_) = renderer() else { return };
    let a = ribbon_scene(false, 0.3);
    assert_eq!(a, ribbon_scene(false, 0.3));
    compare("ribbons.png", &a);
    compare("tubes.png", &ribbon_scene(true, 0.3));
    // The marks run with the time.
    assert_ne!(a, ribbon_scene(false, 0.35));
}
