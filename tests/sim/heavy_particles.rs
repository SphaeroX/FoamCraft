// SPDX-License-Identifier: GPL-3.0-or-later
//! Heavy particles (Task_34): the integrator (settling, response time, large steps, the massless
//! limit, the drag correction), impact on triangles, bouncing, and the critical Stokes number at
//! a cylinder and a sphere.

use foamcraft_sim::distance::WallDistance;
use foamcraft_sim::heavy::{
    Fate, Impact, Medium, Particle, critical_hits, drag_factor, segment_hit, step, tau_p, track,
};
use glam::DVec3;

const AIR: Medium = Medium {
    density: 1.2,
    kinematic_viscosity: 1.5e-5,
};

fn dust(d: f64) -> Particle {
    Particle {
        diameter: d,
        density: 1000.0,
        gravity: true,
        impact: Impact::Stick,
    }
}

#[test]
fn a_small_particle_settles_with_the_stokes_velocity() {
    let p = dust(20e-6);
    let tau = tau_p(&p, &AIR);
    let g = 9.81 * (1.0 - AIR.density / p.density);
    let (mut x, mut v) = (DVec3::ZERO, DVec3::ZERO);
    for _ in 0..2000 {
        (x, v) = step(x, v, &|_| Some(DVec3::ZERO), tau * 0.5, &p, &AIR);
    }
    let want = tau * g;
    // Re_p is tiny here, so the correction is below 1 %.
    assert!((-v.z - want).abs() < 0.01 * want, "{} vs {want}", -v.z);
    assert!(x.z < 0.0);
}

#[test]
fn after_one_response_time_it_has_63_percent_of_the_flow_speed() {
    let mut p = dust(5e-6);
    p.gravity = false;
    let tau = tau_p(&p, &AIR);
    let u = DVec3::new(1e-3, 0.0, 0.0); // slow: Re_p ≈ 0, f = 1
    let mut v = DVec3::ZERO;
    let mut x = DVec3::ZERO;
    let n = 100;
    for _ in 0..n {
        (x, v) = step(x, v, &|_| Some(u), tau / f64::from(n), &p, &AIR);
    }
    let share = v.x / u.x;
    assert!((share - 0.632).abs() < 0.01 * 0.632, "{share}");
}

#[test]
fn a_step_of_ten_response_times_stays_stable() {
    let mut p = dust(5e-6);
    p.gravity = false;
    let tau = tau_p(&p, &AIR);
    let u = DVec3::new(1e-3, 0.0, 0.0);
    let (_, v) = step(DVec3::ZERO, DVec3::ZERO, &|_| Some(u), 10.0 * tau, &p, &AIR);
    assert!((v.x - u.x).abs() < 1e-4 * u.x, "{}", v.x);
}

#[test]
fn a_vanishing_particle_follows_the_flow() {
    let mut p = dust(1e-9);
    p.gravity = false;
    // Solid body rotation: the massless path is a circle.
    let field = |x: DVec3| Some(DVec3::new(-x.y, x.x, 0.0));
    let (mut x, mut v) = (DVec3::new(1.0, 0.0, 0.0), DVec3::new(0.0, 1.0, 0.0));
    for _ in 0..1000 {
        (x, v) = step(x, v, &field, 1e-3, &p, &AIR);
    }
    assert!((x.length() - 1.0).abs() < 1e-3, "{}", x.length());
    assert!((x.y - 1f64.sin()).abs() < 1e-2, "{x}");
}

#[test]
fn above_re_p_one_the_particle_settles_slower_than_stokes() {
    assert_eq!(drag_factor(0.0), 1.0);
    assert!(drag_factor(10.0) > 1.0);
    assert!((drag_factor(2000.0) - 0.44 * 2000.0 / 24.0).abs() < 1e-9);
    let p = dust(1e-3);
    let tau = tau_p(&p, &AIR);
    let g = 9.81 * (1.0 - AIR.density / p.density);
    let (mut x, mut v) = (DVec3::ZERO, DVec3::ZERO);
    for _ in 0..4000 {
        (x, v) = step(x, v, &|_| Some(DVec3::ZERO), 1e-3, &p, &AIR);
    }
    let _ = x;
    assert!(-v.z < tau * g, "{} vs stokes {}", -v.z, tau * g);
    assert!(-v.z > 1.0);
}

/// A square plate x = 0 over -1..1 in y and z (two triangles).
fn plate() -> WallDistance {
    let pts = [
        [0.0, -1.0, -1.0],
        [0.0, 1.0, -1.0],
        [0.0, 1.0, 1.0],
        [0.0, -1.0, 1.0],
    ];
    WallDistance::new(&pts, &[[0, 1, 2], [0, 2, 3]], 0.1, 0.4)
}

#[test]
fn segments_hit_faces_edges_and_miss_beside() {
    let w = plate();
    let hit = segment_hit(&w, DVec3::new(-0.5, 0.2, 0.3), DVec3::new(0.5, 0.2, 0.3)).unwrap();
    assert!((hit.point - DVec3::new(0.0, 0.2, 0.3)).length() < 1e-12);
    assert!((hit.t - 0.5).abs() < 1e-12);
    assert!(hit.normal.x.abs() > 0.999);
    // On the shared edge (the diagonal) it still hits.
    assert!(segment_hit(&w, DVec3::new(-0.5, 0.5, 0.5), DVec3::new(0.5, 0.5, 0.5)).is_some());
    // Just beside the plate it misses.
    assert!(segment_hit(&w, DVec3::new(-0.5, 1.01, 0.0), DVec3::new(0.5, 1.01, 0.0)).is_none());
    // Not reaching the plate.
    assert!(segment_hit(&w, DVec3::new(-0.5, 0.0, 0.0), DVec3::new(-0.1, 0.0, 0.0)).is_none());
}

fn uniform(x: DVec3) -> Option<DVec3> {
    (x.x.abs() < 5.0 && x.y.abs() < 5.0 && x.z.abs() < 5.0).then_some(DVec3::new(2.0, 0.0, 0.0))
}

#[test]
fn a_particle_flying_at_a_plate_hits_where_expected_and_sticks() {
    let w = plate();
    let mut p = dust(1e-3);
    p.gravity = false;
    let t = track(
        DVec3::new(-2.0, 0.1, -0.2),
        DVec3::new(2.0, 0.0, 0.0),
        &uniform,
        &w,
        &p,
        &AIR,
        0.1,
        10.0,
    );
    match t.fate {
        Fate::Stuck(hit) => {
            assert!((hit.point - DVec3::new(0.0, 0.1, -0.2)).length() < 1e-9);
            assert!((hit.angle_deg - 90.0).abs() < 1e-6);
            assert!(hit.normal_speed > 1.5);
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn bouncing_halves_the_normal_speed_and_five_bounces_stick() {
    let w = plate();
    let mut p = dust(1e-3);
    p.gravity = false;
    p.impact = Impact::Bounce {
        restitution: 0.5,
        friction: 0.2,
    };
    let still = |_x: DVec3| Some(DVec3::ZERO);
    let t = track(
        DVec3::new(-0.5, 0.0, 0.0),
        DVec3::new(30.0, 0.0, 0.0),
        &still,
        &w,
        &p,
        &AIR,
        0.1,
        10.0,
    );
    assert!(!t.bounces.is_empty());
    let first = &t.bounces[0];
    assert!(
        (first.after.x + 0.5 * first.before.x).abs() < 1e-9 * first.before.x.abs(),
        "{first:?}"
    );
    // Between two plates the particle bounces back and forth until it sticks.
    let pts = [
        [0.0, -1.0, -1.0],
        [0.0, 1.0, -1.0],
        [0.0, 1.0, 1.0],
        [0.0, -1.0, 1.0],
        [-0.2, -1.0, -1.0],
        [-0.2, 1.0, -1.0],
        [-0.2, 1.0, 1.0],
        [-0.2, -1.0, 1.0],
    ];
    let two = WallDistance::new(
        &pts,
        &[[0, 1, 2], [0, 2, 3], [4, 5, 6], [4, 6, 7]],
        0.1,
        0.4,
    );
    p.impact = Impact::Bounce {
        restitution: 0.9,
        friction: 0.0,
    };
    let t = track(
        DVec3::new(-0.1, 0.0, 0.0),
        DVec3::new(30.0, 0.0, 0.0),
        &still,
        &two,
        &p,
        &AIR,
        0.1,
        10.0,
    );
    assert_eq!(t.bounces.len(), 5);
    assert!(matches!(t.fate, Fate::Stuck(_)), "{:?}", t.fate);
}

#[test]
fn the_critical_stokes_number_at_cylinder_and_sphere() {
    // Hits only above K = 1/8 (cylinder) and 1/12 (sphere); see the derivation in Task_34.
    for (sphere, critical) in [(false, 1.0 / 8.0), (true, 1.0 / 12.0)] {
        assert_eq!(
            critical_hits(sphere, 0.8 * critical, 200),
            0,
            "below, sphere {sphere}"
        );
        let at2 = critical_hits(sphere, 2.0 * critical, 200);
        assert!(at2 > 0, "above, sphere {sphere}");
        let at10 = critical_hits(sphere, 10.0, 200);
        assert!(at10 >= at2);
        let at100 = critical_hits(sphere, 100.0, 200);
        assert!(at100 as f64 >= 0.95 * 200.0, "K = 100: {at100}");
    }
}

#[test]
fn smoothing_keeps_the_number_of_hits() {
    use foamcraft_sim::heavy::smooth;
    // A strip of four triangles with all hits on the first.
    let tris = [[0, 1, 2], [1, 3, 2], [2, 3, 4], [3, 5, 4]];
    let areas = [1.0, 2.0, 1.0, 0.5];
    let values = [10.0, 0.0, 0.0, 0.0];
    let s = smooth(&tris, &areas, 6, &values);
    let total: f64 = s.iter().sum();
    assert!((total - 10.0).abs() < 1e-9, "{total}");
    assert!(s[0] > s[3]);
}

#[test]
fn the_impact_counts_do_not_depend_on_the_number_of_cores() {
    use foamcraft_sim::heavy::impact_counts;
    use std::sync::atomic::{AtomicBool, AtomicU64};
    let w = plate();
    let mut p = dust(1e-4);
    p.gravity = false;
    let starts: Vec<DVec3> = (0..3000)
        .map(|i| {
            let a = f64::from(i % 60) / 60.0 * 2.4 - 1.2;
            let b = f64::from(i / 60) / 50.0 * 2.4 - 1.2;
            DVec3::new(-2.0, a, b)
        })
        .collect();
    let v0 = |_: DVec3| DVec3::new(2.0, 0.0, 0.0);
    let run = |threads: usize| {
        impact_counts(
            &starts,
            &v0,
            &uniform,
            &w,
            &p,
            &AIR,
            0.1,
            10.0,
            threads,
            &AtomicBool::new(false),
            &AtomicU64::new(0),
        )
        .unwrap()
    };
    let one = run(1);
    let four = run(4);
    assert_eq!(one, four);
    // Very heavy particles fly straight: those aimed at the plate (2 × 2 of 2.4 × 2.4) hit.
    let share = one.total as f64 / 3000.0;
    assert!((share - 4.0 / 5.76).abs() < 0.03, "{share}");
}

#[test]
fn the_terminal_velocity_includes_the_drag_correction() {
    use foamcraft_sim::heavy::terminal_velocity;
    // A 1 mm raindrop falls at about 4 m/s (Stokes alone would say 30 m/s).
    let rain = dust(1e-3);
    let v = terminal_velocity(&rain, &AIR);
    assert!(v.z < -3.0 && v.z > -5.0, "{v}");
    // It is where the integrator ends up in still air.
    let (mut x, mut w) = (DVec3::ZERO, DVec3::ZERO);
    for _ in 0..4000 {
        (x, w) = step(x, w, &|_| Some(DVec3::ZERO), 1e-3, &rain, &AIR);
    }
    let _ = x;
    assert!((w.z - v.z).abs() < 0.01 * v.z.abs(), "{} vs {}", w.z, v.z);
    // Small particles: the Stokes value.
    let fine = dust(5e-6);
    let tau = tau_p(&fine, &AIR);
    let g = 9.81 * (1.0 - AIR.density / fine.density);
    assert!((terminal_velocity(&fine, &AIR).z + tau * g).abs() < 1e-3 * tau * g);
}
