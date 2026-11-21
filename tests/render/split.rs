// SPDX-License-Identifier: GPL-3.0-or-later
//! Split view: two scenes side by side with one camera; each half matches a frame of half width.

use foamcraft_render::scene::{MODEL_COLOR, mesh_from_triangles_with_ids};
use foamcraft_render::{Renderer, Scene, camera::OrbitCamera};

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

fn triangle(color: [f32; 3], shift: f32) -> Scene {
    let p = vec![
        [-0.5 + shift, -0.5, 0.0],
        [0.5 + shift, -0.5, 0.0],
        [shift, 0.5, 0.0],
    ];
    Scene {
        meshes: vec![mesh_from_triangles_with_ids(&p, &[[0, 1, 2]], &[1], color)],
        background: Scene::empty().background,
    }
}

fn half(pixels: &[u8], width: u32, height: u32, right: bool) -> Vec<u8> {
    let half = (width / 2) as usize;
    let mut out = Vec::new();
    for row in 0..height as usize {
        let start = row * width as usize * 4 + if right { half * 4 } else { 0 };
        out.extend_from_slice(&pixels[start..start + half * 4]);
    }
    out
}

#[test]
fn each_half_of_the_split_view_is_the_scene_alone_at_half_width() {
    let Some(r) = renderer() else { return };
    let (w, h) = (240, 160);
    let a = triangle(MODEL_COLOR, 0.0);
    let b = triangle([0.9, 0.3, 0.2], 0.2);
    let mut cam = OrbitCamera::default();
    cam.fit(glam_zero(), 1.0, (w / 2) as f32 / h as f32);
    let split = r.render_offscreen_split(&r.upload(&a), &r.upload(&b), &cam, w, h);
    assert_eq!(split.len(), (w * h * 4) as usize);
    let left = r.render_offscreen(&a, &cam, w / 2, h);
    let right = r.render_offscreen(&b, &cam, w / 2, h);
    let differ = |x: &[u8], y: &[u8]| x.iter().zip(y).filter(|(p, q)| p.abs_diff(**q) > 2).count();
    assert_eq!(differ(&half(&split, w, h, false), &left), 0);
    assert_eq!(differ(&half(&split, w, h, true), &right), 0);
    // The halves really differ (different colour and position).
    assert!(differ(&left, &right) > 100);
}

fn glam_zero() -> foamcraft_render::glam::Vec3 {
    foamcraft_render::glam::Vec3::ZERO
}
