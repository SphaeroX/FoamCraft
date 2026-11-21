// SPDX-License-Identifier: GPL-3.0-or-later
//! Mesh planning: background cell size, refinement levels, layers, point in the flow domain.

use foamcraft_core::project::{Ground, MediumSpec, Quality};
use foamcraft_geometry::{Format, Mesh, MeshBuilder, Placement};
use foamcraft_sim::medium_properties;
use foamcraft_sim::mesh::{MeshPlan, first_layer_thickness, inside_any, plan_wind_tunnel};

fn cube(size: f32, offset: f32) -> Mesh {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/data/geometry/cube_binary.stl");
    let mesh = foamcraft_geometry::read_mesh(&path).unwrap();
    Placement {
        unit: "m".into(),
        scale: f64::from(size) / 20.0,
        translation: [f64::from(offset), 0.0, 0.0],
        ..Default::default()
    }
    .apply(&mesh)
}

fn sphere(r: f32, n: usize) -> Mesh {
    let point = |i: usize, j: usize| {
        if i == 0 || i == n {
            return [0.0, 0.0, r + if i == 0 { r } else { -r }];
        }
        let t = std::f32::consts::PI * i as f32 / n as f32;
        let f = 2.0 * std::f32::consts::PI * (j % n) as f32 / n as f32;
        [
            r * t.sin() * f.cos(),
            r * t.sin() * f.sin(),
            r + r * t.cos(),
        ]
    };
    let mut b = MeshBuilder::default();
    for i in 0..n {
        for j in 0..n {
            let (a, bb, c, d) = (
                point(i, j),
                point(i + 1, j),
                point(i + 1, j + 1),
                point(i, j + 1),
            );
            if i != 0 {
                b.triangle(a, bb, d);
            }
            if i != n - 1 {
                b.triangle(bb, c, d);
            }
        }
    }
    b.finish(Format::Stl, None)
}

fn plan(models: &[&Mesh], cells: u64) -> MeshPlan {
    let air = medium_properties(&MediumSpec::Air {
        temperature_c: 20.0,
    });
    plan_wind_tunnel(
        models,
        0.0,
        Ground::Fixed,
        20.0,
        &air,
        Quality::Standard,
        cells,
    )
}

#[test]
fn estimate_meets_the_budget_and_levels_are_nested() {
    for cells in [150_000u64, 500_000, 2_000_000] {
        let c = cube(1.0, 0.0);
        let p = plan(&[&c], cells);
        let ratio = p.estimated_cells as f64 / cells as f64;
        assert!(
            (0.8..=1.05).contains(&ratio),
            "{cells}: estimate {}",
            p.estimated_cells
        );
        assert!(p.surface_level >= p.near_level && p.near_level >= p.wake_level);
        assert!(p.wake_level >= 1);
        assert!(p.background_cell > 0.0);
        let n = p.background_cells;
        assert!(n.iter().all(|&k| k >= 4), "{n:?}");
        // The background cells fill the tunnel.
        let size = [
            p.tunnel_max[0] - p.tunnel_min[0],
            p.tunnel_max[1] - p.tunnel_min[1],
            p.tunnel_max[2] - p.tunnel_min[2],
        ];
        for k in 0..3 {
            assert!(
                (size[k] / f64::from(n[k]) - p.background_cell).abs() < 0.2 * p.background_cell
            );
        }
    }
}

#[test]
fn more_budget_gives_finer_surface_cells() {
    let c = cube(1.0, 0.0);
    assert!(plan(&[&c], 2_000_000).surface_cell < plan(&[&c], 200_000).surface_cell);
}

#[test]
fn first_layer_targets_wall_function_range() {
    // Flat plate estimate: Cf = 0.0576 Re^-0.2, u_tau = U sqrt(Cf / 2), y+ = y u_tau / nu.
    let air = medium_properties(&MediumSpec::Air {
        temperature_c: 20.0,
    });
    for (u, l) in [(20.0, 1.0), (5.0, 0.2), (40.0, 4.0)] {
        let y1 = first_layer_thickness(u, l, air.kinematic_viscosity);
        let re = u * l / air.kinematic_viscosity;
        let u_tau = u * (0.0576 * f64::powf(re, -0.2) / 2.0).sqrt();
        let y_plus = y1 * u_tau / air.kinematic_viscosity;
        assert!((30.0..=100.0).contains(&y_plus), "u {u} l {l}: y+ {y_plus}");
    }
}

#[test]
fn layer_count_follows_the_ratio_of_cell_size_and_first_layer() {
    let c = cube(1.0, 0.0);
    let p = plan(&[&c], 2_000_000);
    assert!((1..=10).contains(&p.layers), "{}", p.layers);
    assert!(p.expansion_ratio > 1.0 && p.expansion_ratio < 1.5);
}

#[test]
fn point_in_flow_lies_in_the_tunnel_and_outside_every_body() {
    let cases: Vec<Vec<Mesh>> = vec![
        vec![cube(1.0, 0.0)],
        vec![sphere(0.5, 40)],
        vec![cube(0.3, 0.0), cube(0.3, 1.0)],
    ];
    for models in &cases {
        let refs: Vec<&Mesh> = models.iter().collect();
        let p = plan(&refs, 300_000);
        let q = p.location_in_mesh;
        for k in 0..3 {
            assert!(q[k] > p.tunnel_min[k] && q[k] < p.tunnel_max[k], "{q:?}");
        }
        // In the flow frame the models are rotated by -yaw; here yaw is 0.
        assert!(!inside_any(&refs, q), "point {q:?} inside a body");
    }
    // A point in the middle of the cube is inside.
    let c = cube(1.0, 0.0);
    assert!(inside_any(&[&c], [0.5, 0.5, 0.5]));
}
