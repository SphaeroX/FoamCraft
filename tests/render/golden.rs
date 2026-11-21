// SPDX-License-Identifier: GPL-3.0-or-later
//! Offscreen render of the test scene compared with a golden image.
//!
//! Without a GPU adapter the tests print `SKIPPED` and return; that counts as skipped, not passed.
//! Set `FOAMCRAFT_REQUIRE_GPU=1` to turn a missing adapter into a failure.

use std::path::PathBuf;

use foamcraft_render::{Renderer, Scene, camera::OrbitCamera};

const SIZE: u32 = 256;
/// Allowed difference per colour channel (0..255); covers driver rasterisation differences.
const CHANNEL_TOLERANCE: u8 = 8;
/// Allowed share of pixels above the channel tolerance (edge anti-aliasing differences).
const PIXEL_TOLERANCE: f64 = 0.005;

fn renderer() -> Option<Renderer> {
    let renderer = Renderer::new_headless();
    if renderer.is_none() {
        assert!(
            std::env::var_os("FOAMCRAFT_REQUIRE_GPU").is_none(),
            "no GPU adapter but FOAMCRAFT_REQUIRE_GPU is set"
        );
        eprintln!("SKIPPED: no GPU adapter available");
    }
    renderer
}

fn render_cube(renderer: &Renderer) -> Vec<u8> {
    let scene = Scene::test_cube();
    let mut camera = OrbitCamera::default();
    let (center, radius) = scene.bounding_sphere();
    camera.fit(center, radius, 1.0);
    renderer.render_offscreen(&scene, &camera, SIZE, SIZE)
}

fn golden_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/golden/test_cube.png")
}

fn read_png(path: &PathBuf) -> Vec<u8> {
    let decoder = png::Decoder::new(std::io::BufReader::new(
        std::fs::File::open(path).expect("open golden image"),
    ));
    let mut reader = decoder.read_info().expect("png header");
    let mut buf = vec![0; reader.output_buffer_size().expect("buffer size")];
    let info = reader.next_frame(&mut buf).expect("png frame");
    assert_eq!((info.width, info.height), (SIZE, SIZE));
    assert_eq!(info.color_type, png::ColorType::Rgba);
    buf.truncate(info.buffer_size());
    buf
}

#[test]
fn offscreen_frame_shows_cube_in_centre_on_background() {
    let Some(renderer) = renderer() else { return };
    let pixels = render_cube(&renderer);
    assert_eq!(pixels.len(), (SIZE * SIZE * 4) as usize);
    let at = |x: u32, y: u32| {
        let i = ((y * SIZE + x) * 4) as usize;
        [pixels[i], pixels[i + 1], pixels[i + 2], pixels[i + 3]]
    };
    let background = Scene::test_cube().background_rgba8();
    assert_eq!(at(0, 0), background);
    assert_eq!(at(SIZE - 1, SIZE - 1), background);
    assert_ne!(at(SIZE / 2, SIZE / 2), background);
}

#[test]
fn offscreen_frame_matches_golden_image() {
    let Some(renderer) = renderer() else { return };
    let pixels = render_cube(&renderer);
    let path = golden_path();
    if std::env::var_os("FOAMCRAFT_UPDATE_GOLDEN").is_some() {
        foamcraft_render::write_png(&path, SIZE, SIZE, &pixels).expect("write golden");
    }
    let golden = read_png(&path);
    let off = pixels
        .as_chunks::<4>()
        .0
        .iter()
        .zip(golden.as_chunks::<4>().0)
        .filter(|(a, b)| {
            a.iter()
                .zip(b.iter())
                .any(|(x, y)| x.abs_diff(*y) > CHANNEL_TOLERANCE)
        })
        .count();
    let share = off as f64 / f64::from(SIZE * SIZE);
    assert!(
        share <= PIXEL_TOLERANCE,
        "{:.3}% of pixels differ",
        share * 100.0
    );
}

#[test]
fn imported_cube_with_axes_matches_golden_image() {
    let Some(renderer) = renderer() else { return };
    let path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/data/geometry/cube_binary.stl");
    let mesh = foamcraft_geometry::read_mesh(&path).expect("read cube");
    let placement = foamcraft_geometry::Placement {
        unit: "mm".into(),
        ..Default::default()
    };
    let world = placement.apply(&mesh);
    let scene = Scene::from_models(&[(world.positions.as_slice(), world.triangles.as_slice())]);
    assert_eq!(scene.meshes.len(), 4, "model plus three axes");
    let mut camera = OrbitCamera::default();
    let (center, radius) = scene.bounding_sphere();
    camera.fit(center, radius * 1.1, 1.0);
    let pixels = renderer.render_offscreen(&scene, &camera, SIZE, SIZE);
    let golden =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/golden/imported_cube.png");
    if std::env::var_os("FOAMCRAFT_UPDATE_GOLDEN").is_some() {
        foamcraft_render::write_png(&golden, SIZE, SIZE, &pixels).expect("write golden");
    }
    let reference = read_png(&golden);
    let off = pixels
        .as_chunks::<4>()
        .0
        .iter()
        .zip(reference.as_chunks::<4>().0)
        .filter(|(a, b)| {
            a.iter()
                .zip(b.iter())
                .any(|(x, y)| x.abs_diff(*y) > CHANNEL_TOLERANCE)
        })
        .count();
    let share = off as f64 / f64::from(SIZE * SIZE);
    assert!(
        share <= PIXEL_TOLERANCE,
        "{:.3}% of pixels differ",
        share * 100.0
    );
}

#[test]
fn uploaded_scene_renders_like_the_direct_path() {
    let Some(renderer) = renderer() else { return };
    let scene = Scene::test_cube();
    let mut camera = OrbitCamera::default();
    let (center, radius) = scene.bounding_sphere();
    camera.fit(center, radius, 1.0);
    let gpu = renderer.upload(&scene);
    let a = renderer.render_offscreen_gpu(&gpu, &camera, SIZE, SIZE);
    let b = renderer.render_offscreen(&scene, &camera, SIZE, SIZE);
    assert_eq!(a, b);
}

#[test]
fn cube_in_wind_tunnel_matches_golden_image() {
    let Some(renderer) = renderer() else { return };
    let path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/data/geometry/cube_binary.stl");
    let mesh = foamcraft_geometry::read_mesh(&path).expect("read cube");
    let world = foamcraft_geometry::Placement {
        unit: "mm".into(),
        ..Default::default()
    }
    .apply(&mesh);
    let mut scene = Scene::from_models(&[(world.positions.as_slice(), world.triangles.as_slice())]);
    let t = foamcraft_sim::tunnel_box(
        [0.0; 3],
        [0.02; 3],
        30.0,
        foamcraft_core::project::Ground::Fixed,
    );
    scene.meshes.extend(foamcraft_render::scene::tunnel(
        t.center,
        [t.length, t.width, t.height],
        t.yaw_deg,
        true,
    ));
    let mut camera = OrbitCamera::default();
    let (center, radius) = scene.bounding_sphere();
    camera.fit(center, radius, 1.0);
    let pixels = renderer.render_offscreen(&scene, &camera, SIZE, SIZE);
    let golden =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/golden/wind_tunnel.png");
    if std::env::var_os("FOAMCRAFT_UPDATE_GOLDEN").is_some() {
        foamcraft_render::write_png(&golden, SIZE, SIZE, &pixels).expect("write golden");
    }
    let reference = read_png(&golden);
    let off = pixels
        .as_chunks::<4>()
        .0
        .iter()
        .zip(reference.as_chunks::<4>().0)
        .filter(|(a, b)| {
            a.iter()
                .zip(b.iter())
                .any(|(x, y)| x.abs_diff(*y) > CHANNEL_TOLERANCE)
        })
        .count();
    let share = off as f64 / f64::from(SIZE * SIZE);
    assert!(
        share <= PIXEL_TOLERANCE,
        "{:.3}% of pixels differ",
        share * 100.0
    );
}

#[test]
fn mesh_edges_are_drawn_over_the_surface() {
    let Some(renderer) = renderer() else { return };
    let path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/data/geometry/cube_binary.stl");
    let mesh = foamcraft_geometry::read_mesh(&path).expect("read cube");
    let world = foamcraft_geometry::Placement {
        unit: "mm".into(),
        ..Default::default()
    }
    .apply(&mesh);
    let mut scene = Scene::from_models(&[(world.positions.as_slice(), world.triangles.as_slice())]);
    let plain = {
        let mut camera = OrbitCamera::default();
        let (c, r) = scene.bounding_sphere();
        camera.fit(c, r, 1.0);
        renderer.render_offscreen(&scene, &camera, SIZE, SIZE)
    };
    scene.meshes.push(foamcraft_render::scene::mesh_edges(
        &world.positions,
        &world.triangles,
        [1.0, 0.0, 1.0],
        0.0002,
    ));
    let mut camera = OrbitCamera::default();
    let (c, r) = scene.bounding_sphere();
    camera.fit(c, r, 1.0);
    let with_edges = renderer.render_offscreen(&scene, &camera, SIZE, SIZE);
    let magenta = with_edges
        .as_chunks::<4>()
        .0
        .iter()
        .filter(|p| p[0] > 120 && p[1] < 60 && p[2] > 120)
        .count();
    assert!(magenta > 50, "only {magenta} edge pixels");
    assert_ne!(plain, with_edges);
}

#[test]
fn polylines_become_line_pairs() {
    let m = foamcraft_render::scene::polylines(
        &[vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [1.0, 1.0, 0.0]]],
        [1.0, 0.0, 0.0],
        true,
    );
    assert!(m.lines);
    // Closed triangle: three segments.
    assert_eq!(m.indices.len(), 6);
    let open = foamcraft_render::scene::polylines(
        &[vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [1.0, 1.0, 0.0]]],
        [1.0, 0.0, 0.0],
        false,
    );
    assert_eq!(open.indices.len(), 4);
}

#[test]
fn a_translucent_mesh_lets_what_is_behind_show_through() {
    let Some(renderer) = renderer() else { return };
    use foamcraft_render::glam::Vec3;
    let back = foamcraft_render::scene::box_mesh(
        Vec3::new(0.0, 0.0, 0.0),
        Vec3::new(2.0, 2.0, 0.1),
        [1.0, 0.0, 0.0],
    );
    let mut front = foamcraft_render::scene::box_mesh(
        Vec3::new(0.0, 0.0, 0.5),
        Vec3::new(2.0, 2.0, 0.1),
        [0.0, 0.0, 1.0],
    );
    let render = |front: &foamcraft_render::scene::Mesh| {
        let scene = Scene {
            meshes: vec![back.clone(), front.clone()],
            background: Scene::empty().background,
        };
        let mut camera = OrbitCamera::default();
        camera.orbit(0.0, 1.5);
        camera.fit(Vec3::ZERO, 1.0, 1.0);
        renderer.render_offscreen(&scene, &camera, SIZE, SIZE)
    };
    let red = |img: &[u8]| {
        img.as_chunks::<4>()
            .0
            .iter()
            .filter(|p| p[0] > 60 && p[2] < 30)
            .count()
    };
    let opaque = render(&front);
    front.opacity = 0.5;
    let translucent = render(&front);
    assert!(
        red(&opaque) < 50,
        "red behind an opaque plate: {}",
        red(&opaque)
    );
    assert!(
        red(&translucent) > 1000,
        "red through a translucent plate: {}",
        red(&translucent)
    );
}

#[test]
fn vertex_colours_replace_the_mesh_colour() {
    let Some(renderer) = renderer() else { return };
    use foamcraft_render::glam::Vec3;
    let mut plate =
        foamcraft_render::scene::box_mesh(Vec3::ZERO, Vec3::new(2.0, 2.0, 0.1), [1.0, 0.0, 0.0]);
    // Green where x < 0, blue elsewhere.
    plate.vertex_colors = Some(
        plate
            .vertices
            .iter()
            .map(|v| {
                if v.position[0] < 0.0 {
                    [0.0, 1.0, 0.0]
                } else {
                    [0.0, 0.0, 1.0]
                }
            })
            .collect(),
    );
    let scene = Scene {
        meshes: vec![plate],
        background: Scene::empty().background,
    };
    let mut camera = OrbitCamera::default();
    camera.orbit(0.0, 1.5);
    camera.fit(Vec3::ZERO, 1.0, 1.0);
    let img = renderer.render_offscreen(&scene, &camera, SIZE, SIZE);
    let px = img.as_chunks::<4>().0;
    let red = px
        .iter()
        .filter(|p| p[0] > 60 && p[1] < 30 && p[2] < 30)
        .count();
    let green = px.iter().filter(|p| p[1] > 60 && p[0] < 30).count();
    let blue = px
        .iter()
        .filter(|p| p[2] > 60 && p[0] < 30 && p[1] < 30)
        .count();
    assert!(red < 50, "mesh colour still drawn: {red}");
    assert!(green > 1000 && blue > 1000, "green {green} blue {blue}");
}

#[test]
fn cube_coloured_by_a_field_matches_golden_image() {
    let Some(renderer) = renderer() else { return };
    let path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/data/geometry/cube_binary.stl");
    let mesh = foamcraft_geometry::read_mesh(&path).expect("read cube");
    let world = foamcraft_geometry::Placement {
        unit: "mm".into(),
        ..Default::default()
    }
    .apply(&mesh);
    // Field: height of the triangle centre, coloured with the FoamCraft scale.
    let ids = vec![1; world.triangles.len()];
    let mut m = foamcraft_render::scene::mesh_from_triangles_with_ids(
        &world.positions,
        &world.triangles,
        &ids,
        [0.5, 0.5, 0.5],
    );
    let colors: Vec<[f32; 3]> = world
        .triangles
        .iter()
        .flat_map(|t| {
            let z = t
                .iter()
                .map(|i| world.positions[*i as usize][2])
                .sum::<f32>()
                / 3.0;
            [foamcraft_sim::results::colormap(f64::from(z) / 0.02); 3]
        })
        .collect();
    m.vertex_colors = Some(colors);
    let scene = Scene {
        meshes: vec![m],
        background: Scene::empty().background,
    };
    let mut camera = OrbitCamera::default();
    let (center, radius) = scene.bounding_sphere();
    camera.fit(center, radius * 1.1, 1.0);
    let pixels = renderer.render_offscreen(&scene, &camera, SIZE, SIZE);
    let golden =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/golden/coloured_cube.png");
    if std::env::var_os("FOAMCRAFT_UPDATE_GOLDEN").is_some() || !golden.is_file() {
        foamcraft_render::write_png(&golden, SIZE, SIZE, &pixels).expect("write golden");
    }
    let reference = read_png(&golden);
    let off = pixels
        .as_chunks::<4>()
        .0
        .iter()
        .zip(reference.as_chunks::<4>().0)
        .filter(|(a, b)| {
            a.iter()
                .zip(b.iter())
                .any(|(x, y)| x.abs_diff(*y) > CHANNEL_TOLERANCE)
        })
        .count();
    let share = off as f64 / f64::from(SIZE * SIZE);
    assert!(
        share <= PIXEL_TOLERANCE,
        "{:.3}% of pixels differ",
        share * 100.0
    );
}

#[test]
fn slice_through_an_analytic_field_matches_golden_image() {
    use foamcraft_sim::results::{GridSpec, colormap, grid_from_rows};
    let Some(renderer) = renderer() else { return };
    let spec = GridSpec {
        min: [0.0; 3],
        max: [1.0; 3],
        dims: [24, 24, 24],
    };
    let mut rows = Vec::new();
    for k in 0..24 {
        for j in 0..24 {
            for i in 0..24 {
                let p = spec.point(i, j, k);
                let f = (std::f64::consts::TAU * p[0]).sin() * (std::f64::consts::TAU * p[1]).cos();
                rows.push(vec![p[0], p[1], p[2], f]);
            }
        }
    }
    let grid = grid_from_rows(&spec, &["f"], &rows);
    let m = foamcraft_sim::visual::slice(&[grid], "f", [0.5, 0.5, 0.5], [0.0, 0.0, 1.0], 64);
    let ids = vec![0; m.triangles.len()];
    let mut mesh = foamcraft_render::scene::mesh_from_triangles_with_ids(
        &m.positions,
        &m.triangles,
        &ids,
        [0.5; 3],
    );
    mesh.vertex_colors = Some(
        m.triangles
            .iter()
            .flat_map(|t| t.map(|i| colormap((f64::from(m.values[i as usize]) + 1.0) / 2.0)))
            .collect(),
    );
    let scene = Scene {
        meshes: vec![mesh],
        background: Scene::empty().background,
    };
    let mut camera = OrbitCamera::default();
    camera.orbit(0.0, 1.5);
    camera.fit(foamcraft_render::glam::Vec3::new(0.5, 0.5, 0.5), 0.75, 1.0);
    let pixels = renderer.render_offscreen(&scene, &camera, SIZE, SIZE);
    let golden =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/golden/analytic_slice.png");
    if std::env::var_os("FOAMCRAFT_UPDATE_GOLDEN").is_some() || !golden.is_file() {
        foamcraft_render::write_png(&golden, SIZE, SIZE, &pixels).expect("write golden");
    }
    let reference = read_png(&golden);
    let off = pixels
        .as_chunks::<4>()
        .0
        .iter()
        .zip(reference.as_chunks::<4>().0)
        .filter(|(a, b)| {
            a.iter()
                .zip(b.iter())
                .any(|(x, y)| x.abs_diff(*y) > CHANNEL_TOLERANCE)
        })
        .count();
    let share = off as f64 / f64::from(SIZE * SIZE);
    assert!(
        share <= PIXEL_TOLERANCE,
        "{:.3}% of pixels differ",
        share * 100.0
    );
}
