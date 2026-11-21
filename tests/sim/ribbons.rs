// SPDX-License-Identifier: GPL-3.0-or-later
//! Ribbons and tubes (Task_35): the frame along the line, the twist from the swirl, tube radius
//! and time marks.

use foamcraft_sim::ribbons::{Frame, frames, marks, ribbon, tube, tube_radius, vorticity};
use glam::DVec3;

/// Points along a line with flow times from a constant speed.
fn line(points: Vec<DVec3>, speed: f64) -> (Vec<DVec3>, Vec<f64>, Vec<f64>) {
    let mut taus = vec![0.0];
    for w in points.windows(2) {
        taus.push(taus.last().unwrap() + (w[1] - w[0]).length() / speed);
    }
    let speeds = vec![speed; points.len()];
    (points, taus, speeds)
}

fn check(f: &[Frame]) {
    for x in f {
        assert!((x.t.length() - 1.0).abs() < 1e-9);
        assert!((x.n.length() - 1.0).abs() < 1e-9);
        assert!(x.t.dot(x.n).abs() < 1e-9);
        assert!((x.t.cross(x.n).length() - 1.0).abs() < 1e-9);
    }
}

#[test]
fn uniform_flow_gives_a_flat_ribbon() {
    let (p, tau, sp) = line(
        (0..50)
            .map(|i| DVec3::new(f64::from(i) * 0.1, 0.0, 0.0))
            .collect(),
        2.0,
    );
    let f = frames(&p, &tau, &sp, &vec![DVec3::ZERO; p.len()]);
    check(&f);
    for x in &f {
        assert!((x.n - f[0].n).length() < 1e-12);
    }
    assert_eq!(ribbon(&f, 0.1).len(), 50);
}

#[test]
fn a_ribbon_on_the_axis_of_a_rotating_flow_twists_by_omega_l_over_u() {
    // u = U x + Omega × r with Omega along x: vorticity 2 Omega along x.
    let (u, omega) = (2.0, 3.0);
    let field = |r: DVec3| Some(DVec3::new(u, -omega * r.z, omega * r.y));
    let w = vorticity(&field, DVec3::new(0.3, 0.0, 0.0), 0.01);
    assert!(
        (w - DVec3::new(2.0 * omega, 0.0, 0.0)).length() < 1e-9,
        "{w}"
    );
    let len = 1.5;
    let n = 300;
    let (p, tau, sp) = line(
        (0..=n)
            .map(|i| DVec3::new(len * f64::from(i) / f64::from(n), 0.0, 0.0))
            .collect(),
        u,
    );
    let om: Vec<DVec3> = p.iter().map(|x| vorticity(&field, *x, 0.01)).collect();
    let f = frames(&p, &tau, &sp, &om);
    check(&f);
    let (a, b) = (f[0].n, f.last().unwrap().n);
    let twist = a.cross(b).dot(DVec3::X).atan2(a.dot(b));
    let want = omega * len / u;
    assert!((twist - want).abs() < 0.02 * want, "{twist} vs {want}");
}

#[test]
fn a_circle_without_swirl_keeps_the_normal_out_of_its_plane() {
    let n = 400;
    let (p, tau, sp) = line(
        (0..=n)
            .map(|i| {
                let a = std::f64::consts::TAU * f64::from(i) / f64::from(n);
                DVec3::new(a.cos(), a.sin(), 0.0)
            })
            .collect(),
        1.0,
    );
    let f = frames(&p, &tau, &sp, &vec![DVec3::ZERO; p.len()]);
    check(&f);
    for w in f.windows(2) {
        assert!(w[0].n.dot(w[1].n) > 0.99, "flipped");
    }
    for x in &f {
        assert!(x.n.z.abs() > 0.999, "{}", x.n);
    }
}

#[test]
fn tubes_thin_where_it_is_fast_and_marks_spread_with_speed() {
    assert_eq!(tube_radius(1.0, 1.0, 0.01), 0.01);
    assert!((tube_radius(4.0, 1.0, 0.01) - 0.005).abs() < 1e-12);
    assert_eq!(tube_radius(100.0, 1.0, 0.01), 0.005);
    assert_eq!(tube_radius(0.0, 1.0, 0.01), 0.01 * 5f64.sqrt().min(2.0));
    // Slow first half (1 m/s), fast second half (3 m/s): the marks are three times as far apart.
    let mut pts = Vec::new();
    let mut taus = Vec::new();
    let mut speeds = Vec::new();
    let mut tau = 0.0;
    for i in 0..=200 {
        let x = f64::from(i) * 0.01;
        let v = if x < 1.0 { 1.0 } else { 3.0 };
        if i > 0 {
            tau += 0.01 / v;
        }
        pts.push(DVec3::new(x, 0.0, 0.0));
        taus.push(tau);
        speeds.push(v);
    }
    let f = frames(&pts, &taus, &speeds, &vec![DVec3::ZERO; pts.len()]);
    let m = marks(&f, 0.1);
    let slow = m[2] - m[1];
    let fast = m[m.len() - 1] - m[m.len() - 2];
    assert!((fast / slow - 3.0).abs() < 0.05, "{slow} {fast}");
    let t = tube(&f, 1.0, 0.02, 8);
    assert_eq!(t[0].len(), 8);
    assert!(((t[0][0].0 - pts[0]).length() - 0.02).abs() < 1e-9);
    assert!(((t[150][0].0 - pts[150]).length() - 0.02 / 3f64.sqrt()).abs() < 1e-9);
}
