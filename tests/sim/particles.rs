// SPDX-License-Identifier: GPL-3.0-or-later
//! Particle integrator (CPU reference of the GPU advection), seed points and streamlines.

use foamcraft_sim::particles::{
    Particles, SeedRegion, VelocityLevel, hash, seed_points, streamline,
};
use foamcraft_sim::results::{GridSpec, grid_from_rows};

/// A velocity level over [lo, hi]³ from `u(p)`; points inside `solid` are masked.
fn level(
    lo: f64,
    hi: f64,
    n: u32,
    u: impl Fn([f64; 3]) -> [f64; 3],
    solid: Option<([f64; 3], [f64; 3])>,
) -> VelocityLevel {
    let spec = GridSpec {
        min: [lo; 3],
        max: [hi; 3],
        dims: [n, n, n],
    };
    let mut rows = Vec::new();
    for k in 0..n {
        for j in 0..n {
            for i in 0..n {
                let p = spec.point(i, j, k);
                if let Some((a, b)) = solid
                    && (0..3).all(|x| p[x] > a[x] && p[x] < b[x])
                {
                    continue;
                }
                let v = u(p);
                rows.push(vec![p[0], p[1], p[2], v[0], v[1], v[2]]);
            }
        }
    }
    VelocityLevel::from_grid(&grid_from_rows(&spec, &["Ux", "Uy", "Uz"], &rows)).unwrap()
}

#[test]
fn a_particle_in_a_uniform_field_moves_velocity_times_time() {
    let levels = [level(0.0, 10.0, 12, |_| [2.0, 1.0, 0.0], None)];
    let mut p = Particles::new(&levels, vec![[3.0, 3.0, 3.0]], 1, 7, 1.0e9);
    for _ in 0..100 {
        p.step(0.01);
    }
    let x = p.positions[0];
    assert!(
        (x[0] - 5.0).abs() < 1e-4 && (x[1] - 4.0).abs() < 1e-4 && (x[2] - 3.0).abs() < 1e-4,
        "{x:?}"
    );
}

#[test]
fn a_rigid_vortex_keeps_the_radius_within_one_percent() {
    let c = 0.5;
    let levels = [level(0.0, 1.0, 40, |p| [-(p[1] - c), p[0] - c, 0.0], None)];
    let mut p = Particles::new(&levels, vec![[0.8, 0.5, 0.5]], 1, 3, 1.0e9);
    // One full turn: 2 pi seconds.
    let steps = 2000;
    for _ in 0..steps {
        p.step(std::f32::consts::TAU / steps as f32);
        let x = p.positions[0];
        let r = ((x[0] - 0.5).powi(2) + (x[1] - 0.5).powi(2)).sqrt();
        assert!((r - 0.3).abs() < 0.003, "radius {r}");
    }
}

#[test]
fn no_particle_enters_the_mask() {
    let solid = ([4.0, 3.0, 3.0], [6.0, 7.0, 7.0]);
    let levels = [level(0.0, 10.0, 40, |_| [1.0, 0.0, 0.0], Some(solid))];
    let seeds: Vec<[f32; 3]> = (0..200)
        .map(|i| [0.5, 2.0 + 6.0 * (i as f32) / 200.0, 5.0])
        .collect();
    let mut p = Particles::new(&levels, seeds, 500, 11, 20.0);
    for _ in 0..600 {
        p.step(0.02);
        for x in &p.positions {
            let inside = (0..3).all(|k| x[k] > solid.0[k] as f32 && x[k] < solid.1[k] as f32);
            assert!(!inside, "particle in the solid at {x:?}");
        }
    }
    assert!(p.respawned > 0, "particles that hit the solid start again");
}

#[test]
fn the_same_seed_gives_the_same_particles() {
    let levels = [level(0.0, 1.0, 16, |p| [1.0, p[0] - 0.5, 0.0], None)];
    let seeds = seed_points(
        &SeedRegion {
            origin: [0.1, 0.2, 0.2],
            u: [0.0, 0.6, 0.0],
            v: [0.0, 0.0, 0.6],
        },
        256,
        42,
    );
    let run = || {
        let mut p = Particles::new(&levels, seeds.clone(), 1000, 42, 0.5);
        for _ in 0..50 {
            p.step(0.01);
        }
        p.positions
    };
    assert_eq!(run(), run());
    let mut other = Particles::new(&levels, seeds.clone(), 1000, 43, 0.5);
    other.step(0.01);
    assert_ne!(other.positions, run());
}

#[test]
fn seed_points_lie_in_the_region_and_the_hash_is_fixed() {
    let r = SeedRegion {
        origin: [1.0, 2.0, 3.0],
        u: [2.0, 0.0, 0.0],
        v: [0.0, 0.0, 1.0],
    };
    let pts = seed_points(&r, 100, 5);
    assert_eq!(pts.len(), 100);
    assert!(
        pts.iter()
            .all(|p| (1.0..=3.0).contains(&p[0]) && p[1] == 2.0 && (3.0..=4.0).contains(&p[2]))
    );
    // Reference values of the PCG hash (shared with the GPU shader).
    assert_eq!(hash(0), 129708002);
    assert_eq!(hash(1), 2831084092);
}

#[test]
fn a_streamline_follows_the_flow_and_stops_at_a_solid() {
    let solid = ([4.0, 0.0, 0.0], [6.0, 10.0, 10.0]);
    let levels = [level(0.0, 10.0, 30, |_| [1.0, 0.0, 0.0], Some(solid))];
    let line = streamline(&levels, [1.0, 5.0, 5.0], 0.05, 1000);
    assert!(line.len() > 10);
    let last = line.last().unwrap();
    assert!(last[0] < 4.0 && last[0] > 3.0, "{last:?}");
    assert!(line.iter().all(|p| (p[1] - 5.0).abs() < 1e-4));
}
