// SPDX-License-Identifier: GPL-3.0-or-later
//! The ride in the renderer (Task_32): the free camera, the camera trait, picture in picture and
//! golden images from the rider's view and the chase view.
//!
//! Without a GPU adapter the GPU tests print `SKIPPED` and return.

use std::path::PathBuf;

use foamcraft_render::camera::{FreeCamera, OrbitCamera, ViewCamera};
use foamcraft_render::glam::{Vec3, Vec4};
use foamcraft_render::scene::polylines;
use foamcraft_render::{Renderer, Scene};
use foamcraft_sim::results::{Grid, GridSpec, Surface};
use foamcraft_sim::ride::{self, Flow, Mode};

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

#[test]
fn the_free_camera_puts_its_view_point_in_the_middle_of_the_image() {
    let cam = FreeCamera {
        eye: Vec3::new(1.0, -2.0, 0.5),
        forward: Vec3::new(0.6, 0.8, 0.0),
        up: Vec3::Z,
        fov_y: 75f32.to_radians(),
        near: 0.01,
        far: 100.0,
    };
    let vp = ViewCamera::view_proj(&cam, 1.5);
    let p = vp * (cam.eye + cam.forward * 5.0).extend(1.0);
    let ndc = p / p.w;
    assert!(ndc.x.abs() < 1e-5 && ndc.y.abs() < 1e-5, "{ndc:?}");
    assert!(ndc.z > 0.0 && ndc.z < 1.0);
    // A point above the view direction lands in the upper half, one to the right on the right.
    let up = vp * (cam.eye + cam.forward * 5.0 + Vec3::Z).extend(1.0);
    assert!(up.y / up.w > 0.1);
    let right = cam.forward.cross(cam.up);
    let r = vp * (cam.eye + cam.forward * 5.0 + right).extend(1.0);
    assert!(r.x / r.w > 0.1);
    // Points nearer than the near plane are clipped (depth below 0).
    let close: Vec4 = vp * (cam.eye + cam.forward * 0.001).extend(1.0);
    assert!(close.z / close.w < 0.0);
    assert_eq!(ViewCamera::eye(&cam), cam.eye);
}

#[test]
fn the_orbit_camera_works_through_the_trait_unchanged() {
    let cam = OrbitCamera::default();
    assert_eq!(ViewCamera::view_proj(&cam, 1.2), cam.view_proj(1.2));
    assert_eq!(ViewCamera::eye(&cam), cam.eye());
}

#[test]
fn the_inset_shows_its_own_scene_on_its_own_background() {
    let Some(r) = renderer() else { return };
    let (w, h) = (200, 120);
    let main = Scene::test_cube();
    let mut cam = OrbitCamera::default();
    let (c, rad) = main.bounding_sphere();
    cam.fit(c, rad, w as f32 / h as f32);
    let mut inset = Scene::test_cube();
    inset.background = [0.9, 0.2, 0.1, 1.0];
    let rect = [150, 90, 50, 30];
    let px = r.render_offscreen_inset(&r.upload(&main), &cam, &r.upload(&inset), &cam, w, h, rect);
    let at = |x: u32, y: u32| {
        let i = ((y * w + x) * 4) as usize;
        [px[i], px[i + 1], px[i + 2], px[i + 3]]
    };
    assert_eq!(at(0, 0), main.background_rgba8());
    let bg = inset.background_rgba8();
    let near = |a: [u8; 4], b: [u8; 4]| a.iter().zip(b).all(|(x, y)| x.abs_diff(y) <= 2);
    // The inset's corner shows its background, the main image around it is unchanged.
    assert!(near(at(151, 91), bg), "{:?} vs {bg:?}", at(151, 91));
    assert!(near(at(199, 119), bg));
    assert_eq!(at(148, 88), main.background_rgba8());
    // The main image without the inset is the plain frame.
    let plain = r.render_offscreen(&main, &cam, w, h);
    let differ = (0..h)
        .flat_map(|y| (0..w).map(move |x| (x, y)))
        .filter(|(x, y)| !(*x >= 150 && *y >= 90))
        .filter(|(x, y)| {
            let i = ((y * w + x) * 4) as usize;
            (0..4).any(|k| px[i + k].abs_diff(plain[i + k]) > 2)
        })
        .count();
    assert_eq!(differ, 0);
}

/// Uniform flow along +X around the unit test cube (masked), with the cube as wall.
fn cube_flow() -> Flow {
    let spec = GridSpec {
        min: [-4.0, -2.0, -2.0],
        max: [4.0, 2.0, 2.0],
        dims: [80, 40, 40],
    };
    let n = spec.len();
    let mut values = vec![vec![0.0f32; n]; 4];
    let mut mask = vec![false; n];
    for k in 0..spec.dims[2] {
        for j in 0..spec.dims[1] {
            for i in 0..spec.dims[0] {
                let p = spec.point(i, j, k);
                let id = spec.index(i, j, k);
                if p.iter().all(|c| c.abs() < 0.5) {
                    mask[id] = true;
                    continue;
                }
                // Bend around the cube: a smooth sideways push near it.
                let r2 = p[1] * p[1] + p[2] * p[2];
                let bump = (-(p[0] * p[0]) * 2.0).exp() / (1.0 + 4.0 * r2);
                values[0][id] = 1.0;
                values[1][id] = (p[1] * bump * 1.5) as f32;
                values[2][id] = (p[2] * bump * 1.5) as f32;
            }
        }
    }
    let grid = Grid {
        spec,
        names: ["Ux", "Uy", "Uz", "p"].map(String::from).to_vec(),
        values,
        mask,
    };
    let cube = Scene::test_cube();
    let m = &cube.meshes[0];
    let surface = Surface {
        points: m.vertices.iter().map(|v| v.position).collect(),
        triangles: m.indices.chunks(3).map(|t| [t[0], t[1], t[2]]).collect(),
        fields: vec![],
    };
    Flow::new(vec![grid], Some(&surface), Some(1.0), 1.0).unwrap()
}

fn ride_frame(r: &Renderer, chase: bool) -> Vec<u8> {
    let flow = cube_flow();
    let path = flow.trace([-3.0, 0.25, 0.3]).unwrap();
    let t = ride::timing(&path, &flow, Mode::Calm, 1.0);
    let track = ride::camera_track(&path, &t, &flow, Mode::Calm);
    let mut key = ride::camera_at(&track, 1.5).unwrap();
    if chase {
        key = ride::chase(&flow, &key);
    }
    let v = |d: foamcraft_render::glam::DVec3| Vec3::new(d.x as f32, d.y as f32, d.z as f32);
    let cam = FreeCamera {
        eye: v(key.eye),
        forward: v(key.forward),
        up: v(key.up),
        fov_y: (key.fov_deg as f32).to_radians(),
        near: key.near as f32,
        far: key.far as f32,
    };
    let mut scene = Scene::test_cube();
    let line: Vec<[f32; 3]> = path
        .points
        .iter()
        .map(|p| p.pos.map(|c| c as f32))
        .collect();
    scene
        .meshes
        .push(polylines(&[line], [1.0, 0.85, 0.2], false));
    r.render_offscreen(&scene, &cam, SIZE, SIZE)
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
fn the_rider_view_matches_the_golden_image() {
    let Some(r) = renderer() else { return };
    let px = ride_frame(&r, false);
    // The model is ahead: the middle of the image is not background.
    compare("ride_first_person.png", &px);
}

#[test]
fn the_chase_view_matches_the_golden_image() {
    let Some(r) = renderer() else { return };
    let px = ride_frame(&r, true);
    compare("ride_chase.png", &px);
}
