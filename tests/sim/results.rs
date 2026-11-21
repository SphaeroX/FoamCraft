// SPDX-License-Identifier: GPL-3.0-or-later
//! Result data: colour scale, sampled volume grids with a solid mask, surface fields and the
//! compact binary cache format.

use foamcraft_sim::results::{
    Grid, GridSpec, Surface, colormap, grid_from_rows, level_dims, oklab_lightness,
};

#[test]
fn the_colour_scale_gets_lighter_evenly_from_blue_to_yellow() {
    let n = 64;
    let lightness: Vec<f64> = (0..=n)
        .map(|i| oklab_lightness(colormap(i as f64 / n as f64)))
        .collect();
    for w in lightness.windows(2) {
        assert!(w[1] > w[0], "{lightness:?}");
    }
    // Even steps: no step more than twice the mean step.
    let mean = (lightness[n] - lightness[0]) / n as f64;
    assert!(lightness.windows(2).all(|w| w[1] - w[0] < 2.0 * mean));
    let [r0, _, b0] = colormap(0.0);
    let [r1, g1, b1] = colormap(1.0);
    assert!(b0 > r0, "dark end is blue");
    assert!(r1 > b1 && g1 > b1, "light end is yellow");
    for t in [0.0, 0.3, 0.7, 1.0] {
        assert!(colormap(t).iter().all(|c| (0.0..=1.0).contains(c)));
    }
}

#[test]
fn level_dimensions_follow_the_box_and_the_point_budget() {
    let d = level_dims([2.0, 1.0, 0.5], 64_000);
    let total: u64 = d.iter().map(|x| u64::from(*x)).product();
    assert!(total <= 64_000 && total > 40_000, "{d:?}");
    assert!(d[0] > d[1] && d[1] > d[2]);
}

fn analytic(p: [f64; 3]) -> f64 {
    p[0].sin() + p[1] * p[1] + 2.0 * p[2]
}

/// Rows like a raw sample set of an analytic field; points inside `solid` are left out.
fn rows(spec: &GridSpec, solid: Option<([f64; 3], [f64; 3])>) -> Vec<Vec<f64>> {
    let mut out = Vec::new();
    for k in 0..spec.dims[2] {
        for j in 0..spec.dims[1] {
            for i in 0..spec.dims[0] {
                let p = spec.point(i, j, k);
                if let Some((lo, hi)) = solid
                    && (0..3).all(|a| p[a] > lo[a] && p[a] < hi[a])
                {
                    continue;
                }
                out.push(vec![p[0], p[1], p[2], analytic(p), 1.0, 2.0, 3.0]);
            }
        }
    }
    out
}

#[test]
fn an_analytic_field_is_sampled_with_a_small_error() {
    let spec = GridSpec {
        min: [0.0, 0.0, 0.0],
        max: [1.0, 1.0, 1.0],
        dims: [21, 21, 21],
    };
    let grid = grid_from_rows(&spec, &["f", "Ux", "Uy", "Uz"], &rows(&spec, None));
    // Trilinear interpolation: error O(h²) with h ≈ 0.045.
    let mut worst: f64 = 0.0;
    for n in 0..200 {
        let t = n as f64 / 200.0;
        let p = [
            0.1 + 0.8 * t,
            0.1 + 0.8 * (t * 7.3).fract(),
            0.1 + 0.8 * (t * 3.1).fract(),
        ];
        let v = grid.sample("f", p).unwrap();
        worst = worst.max((v - analytic(p)).abs());
    }
    assert!(worst < 5e-3, "largest error {worst}");
    assert_eq!(grid.sample("Uy", [0.5, 0.5, 0.5]), Some(2.0));
}

#[test]
fn points_inside_a_solid_cube_are_masked() {
    let spec = GridSpec {
        min: [0.0, 0.0, 0.0],
        max: [1.0, 1.0, 1.0],
        dims: [20, 20, 20],
    };
    let cube = ([0.3, 0.3, 0.3], [0.7, 0.7, 0.7]);
    let grid = grid_from_rows(&spec, &["f", "Ux", "Uy", "Uz"], &rows(&spec, Some(cube)));
    let masked = grid.mask.iter().filter(|m| **m).count();
    // 0.4 of the span on each axis: about 8 of 20 points per axis lie inside.
    assert_eq!(masked, 8 * 8 * 8);
    assert_eq!(grid.sample("f", [0.5, 0.5, 0.5]), None);
    assert!(grid.sample("f", [0.1, 0.1, 0.1]).is_some());
}

#[test]
fn surface_and_grid_survive_the_binary_cache_format() {
    let surface = Surface {
        points: vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
        triangles: vec![[0, 1, 2]],
        fields: vec![("p".into(), vec![12.5]), ("yPlus".into(), vec![3.0])],
    };
    let back = Surface::from_bytes(&surface.to_bytes()).unwrap();
    assert_eq!(back, surface);
    let spec = GridSpec {
        min: [0.0, 0.0, 0.0],
        max: [1.0, 1.0, 1.0],
        dims: [4, 3, 2],
    };
    let grid = grid_from_rows(&spec, &["f", "Ux", "Uy", "Uz"], &rows(&spec, None));
    let back = Grid::from_bytes(&grid.to_bytes()).unwrap();
    assert_eq!(back, grid);
    // 24 points × 4 fields × 4 bytes plus the mask and a short header.
    assert!(grid.to_bytes().len() < 24 * 4 * 4 + 24 + 200);
}

#[test]
fn the_extraction_dictionary_names_fields_and_levels() {
    use foamcraft_sim::results::{extraction_dict, set_columns};
    let level = GridSpec {
        min: [0.0, -1.0, 0.0],
        max: [2.0, 1.0, 1.0],
        dims: [40, 20, 10],
    };
    let d = extraction_dict("\"model.*\"", true, &[level.clone(), level]);
    assert!(d.contains("type wallShearStress;") && d.contains("type yPlus;"));
    assert!(d.contains("fields (p wallShearStress yPlus);"));
    assert!(d.contains("level0 { type array;") && d.contains("level1 { type array;"));
    assert!(d.contains("pointsDensity (40 20 10);"));
    assert!(d.contains("fields (U p k);"));
    let laminar = extraction_dict("walls", false, &[]);
    assert!(!laminar.contains("yPlus") && laminar.contains("fields (U p);"));
    assert_eq!(set_columns("level0_k_p_U.xy"), ["k", "p", "Ux", "Uy", "Uz"]);
}

#[test]
fn patches_are_written_in_one_pair_of_brackets() {
    let d = foamcraft_sim::results::extraction_dict("\"model.*\"", true, &[]);
    assert!(d.contains("patches (\"model.*\");"), "{d}");
    assert!(!d.contains("(("));
}

#[test]
fn the_difference_field_needs_the_same_geometry() {
    use foamcraft_sim::results::{Surface, difference_field};
    let surface = |p: Vec<f32>| Surface {
        points: vec![[0.0; 3], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
        triangles: vec![[0, 1, 2]],
        fields: vec![("p".into(), p)],
    };
    let a = surface(vec![10.0]);
    let b = surface(vec![12.5]);
    assert_eq!(difference_field(&a, &b, "p"), Some(vec![2.5]));
    assert_eq!(difference_field(&a, &b, "U"), None);
    let mut other = surface(vec![1.0]);
    other.points[2] = [0.0, 2.0, 0.0];
    assert_eq!(difference_field(&a, &other, "p"), None);
}
