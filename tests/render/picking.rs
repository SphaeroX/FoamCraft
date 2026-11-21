// SPDX-License-Identifier: GPL-3.0-or-later
//! GPU picking with an id buffer, checked against a CPU ray through the same pixel.

use std::path::PathBuf;

use foamcraft_geometry::{Placement, raycast, read_mesh, segment};
use foamcraft_render::scene::{MODEL_COLOR, mesh_from_triangles_with_ids};
use foamcraft_render::{Renderer, Scene, camera::OrbitCamera};

const SIZE: u32 = 200;

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

/// The cube in metres with one pick id per face (face number) and the segmentation.
fn cube_scene() -> (
    Scene,
    foamcraft_geometry::Mesh,
    foamcraft_geometry::Segmentation,
) {
    let path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/data/geometry/cube_binary.stl");
    let mesh = read_mesh(&path).unwrap();
    let world = Placement {
        unit: "mm".into(),
        ..Default::default()
    }
    .apply(&mesh);
    let seg = segment(&mesh, 30.0);
    let scene = Scene {
        meshes: vec![mesh_from_triangles_with_ids(
            &world.positions,
            &world.triangles,
            &seg.face_of_triangle,
            MODEL_COLOR,
        )],
        background: Scene::empty().background,
    };
    (scene, world, seg)
}

fn camera(scene: &Scene) -> OrbitCamera {
    let mut cam = OrbitCamera::default();
    let (c, r) = scene.bounding_sphere();
    cam.fit(c, r * 1.1, 1.0);
    cam
}

#[test]
fn ray_through_the_centre_hits_the_expected_face() {
    let Some(renderer) = renderer() else { return };
    let (scene, world, seg) = cube_scene();
    let cam = camera(&scene);
    let gpu = renderer.upload(&scene);
    let picked = renderer.pick(&gpu, &cam, SIZE, SIZE, SIZE / 2, SIZE / 2);
    let eye = cam.eye();
    let dir = cam.target - eye;
    let hit = raycast(&world, eye.as_dvec3().into(), dir.as_dvec3().into()).expect("cpu hit");
    assert_eq!(picked, Some(seg.face_of_triangle[hit.triangle]));
}

#[test]
fn background_pixels_pick_nothing() {
    let Some(renderer) = renderer() else { return };
    let (scene, _, _) = cube_scene();
    let gpu = renderer.upload(&scene);
    assert_eq!(renderer.pick(&gpu, &camera(&scene), SIZE, SIZE, 0, 0), None);
}

#[test]
fn hovered_and_selected_faces_are_drawn_differently() {
    let Some(renderer) = renderer() else { return };
    let (scene, _, _) = cube_scene();
    let cam = camera(&scene);
    let mut gpu = renderer.upload(&scene);
    let id = renderer
        .pick(&gpu, &cam, SIZE, SIZE, SIZE / 2, SIZE / 2)
        .unwrap();
    let centre = |px: &[u8]| {
        let i = ((SIZE / 2 * SIZE + SIZE / 2) * 4) as usize;
        [px[i], px[i + 1], px[i + 2]]
    };
    let plain = centre(&renderer.render_offscreen_gpu(&gpu, &cam, SIZE, SIZE));
    renderer.set_highlight(&mut gpu, Some(id), &[]);
    let hovered = centre(&renderer.render_offscreen_gpu(&gpu, &cam, SIZE, SIZE));
    renderer.set_highlight(&mut gpu, None, &[id]);
    let selected = centre(&renderer.render_offscreen_gpu(&gpu, &cam, SIZE, SIZE));
    assert_ne!(plain, hovered);
    assert_ne!(plain, selected);
    assert_ne!(hovered, selected);
}
