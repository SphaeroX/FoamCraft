// SPDX-License-Identifier: GPL-3.0-or-later
//! Openings of internal flow models: boundary loops, caps, kind of geometry, flood test.
//! Test bodies: `tests/data/geometry/internal/*.geo` (Gmsh scripts) and the STL made from them.

use std::f64::consts::PI;

use foamcraft_geometry::openings::{
    FluidModel, GeometryKind, Opening, cap_triangles, classify, fluid_check, opening_near,
    openings, triangles_area,
};
use foamcraft_geometry::{Mesh, read_mesh};

fn load(name: &str) -> Mesh {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/data/geometry")
        .join(name);
    read_mesh(&path).unwrap()
}

fn internal(name: &str) -> Mesh {
    load(&format!("internal/{name}.stl"))
}

fn sorted_areas(ops: &[Opening]) -> Vec<f64> {
    let mut a: Vec<f64> = ops.iter().map(|o| o.area).collect();
    a.sort_by(f64::total_cmp);
    a
}

fn close(value: f64, want: f64, share: f64) -> bool {
    (value - want).abs() <= share * want
}

#[test]
fn open_pipe_has_two_round_openings() {
    let mesh = internal("pipe_open");
    assert_eq!(classify(&mesh), GeometryKind::OpenShell);
    let ops = openings(&mesh);
    assert_eq!(ops.len(), 2);
    for o in &ops {
        assert!(close(o.area, PI * 100.0, 0.01), "area {}", o.area);
        assert!(o.planar, "deviation {}", o.deviation);
        let caps = cap_triangles(o);
        assert!(close(triangles_area(&caps), o.area, 1e-6));
    }
}

#[test]
fn tee_has_three_openings() {
    let ops = openings(&internal("tee_open"));
    let a = sorted_areas(&ops);
    assert_eq!(a.len(), 3, "{a:?}");
    assert!(close(a[0], PI * 36.0, 0.01), "{a:?}");
    assert!(
        close(a[1], PI * 64.0, 0.01) && close(a[2], PI * 64.0, 0.01),
        "{a:?}"
    );
}

#[test]
fn housing_with_bores_has_two_mouths() {
    let mesh = internal("housing_bores");
    assert_eq!(classify(&mesh), GeometryKind::ThickWalled { passages: 1 });
    let ops = openings(&mesh);
    assert_eq!(ops.len(), 2);
    for o in &ops {
        assert!(close(o.area, PI * 16.0, 0.01), "area {}", o.area);
        // The mouths are on the outer walls x = 0 and x = 60.
        assert!(
            o.centroid[0] < 0.1 || o.centroid[0] > 59.9,
            "{:?}",
            o.centroid
        );
    }
}

#[test]
fn thick_tube_has_two_mouths() {
    let mesh = internal("tube_thick");
    assert_eq!(classify(&mesh), GeometryKind::ThickWalled { passages: 1 });
    let a = sorted_areas(&openings(&mesh));
    assert_eq!(a.len(), 2);
    assert!(a.iter().all(|x| close(*x, PI * 49.0, 0.01)), "{a:?}");
}

#[test]
fn annular_gap_is_capped_with_a_ring() {
    let ops = openings(&internal("annulus_open"));
    assert_eq!(ops.len(), 2);
    for o in &ops {
        assert_eq!(o.loops.len(), 2, "outer loop and hole");
        assert!(close(o.area, PI * (144.0 - 36.0), 0.01), "area {}", o.area);
        assert!(close(triangles_area(&cap_triangles(o)), o.area, 1e-6));
    }
}

#[test]
fn a_curved_opening_is_flagged_as_not_flat() {
    let ops = openings(&internal("pipe_saddle"));
    assert_eq!(ops.len(), 2);
    assert_eq!(ops.iter().filter(|o| !o.planar).count(), 1);
    // It is still capped.
    let saddle = ops.iter().find(|o| !o.planar).unwrap();
    assert!(!cap_triangles(saddle).is_empty());
}

#[test]
fn a_closed_body_is_a_fluid_volume() {
    let mesh = load("cube_binary.stl");
    assert_eq!(classify(&mesh), GeometryKind::ClosedVolume);
    assert!(openings(&mesh).is_empty());
}

#[test]
fn the_opening_nearest_to_a_point_is_found() {
    let mesh = internal("pipe_open");
    let near_end = opening_near(&mesh, [49.0, 3.0, 0.0], None).unwrap();
    assert!(near_end.centroid[0] > 49.0);
    let near_start = opening_near(&mesh, [1.0, 0.0, 9.0], None).unwrap();
    assert!(near_start.centroid[0] < 1.0);
}

fn check(mesh: &Mesh, caps: &[&Opening]) -> foamcraft_geometry::openings::FluidCheck {
    let caps: Vec<_> = caps.iter().map(|o| cap_triangles(o)).collect();
    fluid_check(&[FluidModel { mesh, caps }])
}

#[test]
fn flood_test_finds_the_fluid_of_a_capped_pipe_and_the_leak_of_an_open_one() {
    let mesh = internal("pipe_open");
    let ops = openings(&mesh);
    let sealed = check(&mesh, &[&ops[0], &ops[1]]);
    assert!(sealed.sealed);
    let p = sealed.fluid_point.unwrap();
    assert!(
        p[0] > 0.0 && p[0] < 50.0 && (p[1] * p[1] + p[2] * p[2]).sqrt() < 10.0,
        "{p:?}"
    );
    let leak = check(&mesh, &[&ops[0]]);
    assert!(!leak.sealed);
    assert!(leak.fluid_point.is_none());
}

#[test]
fn flood_test_keeps_the_fluid_out_of_thick_walls() {
    let mesh = internal("housing_bores");
    let ops = openings(&mesh);
    let sealed = check(&mesh, &[&ops[0], &ops[1]]);
    assert!(sealed.sealed);
    let p = sealed.fluid_point.unwrap();
    // In the cavity (5..55, 5..35, 5..25) or a bore, never in the 5 mm walls.
    let in_cavity =
        (5.0..55.0).contains(&p[0]) && (5.0..35.0).contains(&p[1]) && (5.0..25.0).contains(&p[2]);
    assert!(in_cavity, "{p:?}");
    assert!(!check(&mesh, &[&ops[0]]).sealed);
}

#[test]
fn flood_test_on_ring_and_tee() {
    let ring = internal("annulus_open");
    let ops = openings(&ring);
    let c = check(&ring, &[&ops[0], &ops[1]]);
    assert!(c.sealed);
    let p = c.fluid_point.unwrap();
    let r = (p[1] * p[1] + p[2] * p[2]).sqrt();
    assert!(r > 6.0 && r < 12.0, "{p:?}");
    let tee = internal("tee_open");
    let ops = openings(&tee);
    let all: Vec<&Opening> = ops.iter().collect();
    assert!(check(&tee, &all).sealed);
    assert!(!check(&tee, &all[..2]).sealed);
}

#[test]
fn the_fluid_volume_of_a_long_thin_pipe_is_close_to_the_truth() {
    // D = 10 mm, 300 mm long: pi/4 · 100 · 300 = 23 562 mm³.
    let mesh = internal("pipe_d10_l300");
    let ops = openings(&mesh);
    let smallest = ops.iter().map(|o| o.area.sqrt()).fold(f64::MAX, f64::min);
    let caps: Vec<_> = ops.iter().map(cap_triangles).collect();
    let c = foamcraft_geometry::openings::fluid_check_with(
        &[FluidModel { mesh: &mesh, caps }],
        Some(smallest / 8.0),
    );
    assert!(c.sealed);
    let want = std::f64::consts::PI / 4.0 * 100.0 * 300.0;
    assert!(
        (c.fluid_volume - want).abs() < 0.15 * want,
        "{} vs {want}",
        c.fluid_volume
    );
}
