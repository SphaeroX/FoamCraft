// SPDX-License-Identifier: GPL-3.0-or-later
//! The ride (Task_32): path, readings, timing, camera track, events and suggestions on analytic
//! fields sampled on grids.

use foamcraft_sim::results::{Grid, GridSpec, Surface};
use foamcraft_sim::ride::{
    self, CamKey, End, Flow, Kind, Mode, Path, Probe, RidePoint, Timing, camera_at, camera_track,
    events, features, suggest, surface_candidates, timing, wind_candidates,
};
use glam::DVec3;

/// A grid of `field(p) -> (u, p, masked)` with names Ux Uy Uz p U_mag.
fn grid(spec: GridSpec, field: &dyn Fn(DVec3) -> Option<(DVec3, f64)>) -> Grid {
    let n = spec.len();
    let mut values = vec![vec![0.0f32; n]; 5];
    let mut mask = vec![false; n];
    for k in 0..spec.dims[2] {
        for j in 0..spec.dims[1] {
            for i in 0..spec.dims[0] {
                let id = spec.index(i, j, k);
                match field(DVec3::from(spec.point(i, j, k))) {
                    Some((u, p)) => {
                        values[0][id] = u.x as f32;
                        values[1][id] = u.y as f32;
                        values[2][id] = u.z as f32;
                        values[3][id] = p as f32;
                        values[4][id] = u.length() as f32;
                    }
                    None => mask[id] = true,
                }
            }
        }
    }
    Grid {
        spec,
        names: ["Ux", "Uy", "Uz", "p", "U_mag"].map(String::from).to_vec(),
        values,
        mask,
    }
}

/// Potential flow around a sphere of radius 1 at the origin, U = 1 along +X, rho = 1.
fn sphere_flow(p: DVec3) -> Option<(DVec3, f64)> {
    let r = p.length();
    if r < 1.0 {
        return None;
    }
    let r3 = r * r * r;
    let u = DVec3::X * (1.0 + 0.5 / r3) - p * (1.5 * p.x / (r3 * r * r));
    Some((u, 0.5 * (1.0 - u.length_squared())))
}

fn sphere_surface(n: u32) -> Surface {
    let mut points = Vec::new();
    for i in 0..=n {
        let th = std::f64::consts::PI * f64::from(i) / f64::from(n);
        for j in 0..n {
            let ph = std::f64::consts::TAU * f64::from(j) / f64::from(n);
            points.push([
                th.cos() as f32,
                (th.sin() * ph.cos()) as f32,
                (th.sin() * ph.sin()) as f32,
            ]);
        }
    }
    let mut triangles = Vec::new();
    for i in 0..n {
        for j in 0..n {
            let a = i * n + j;
            let b = i * n + (j + 1) % n;
            let c = (i + 1) * n + j;
            let d = (i + 1) * n + (j + 1) % n;
            triangles.push([a, c, d]);
            triangles.push([a, d, b]);
        }
    }
    Surface {
        points,
        triangles,
        fields: vec![],
    }
}

fn sphere() -> Flow {
    let coarse = grid(
        GridSpec {
            min: [-5.0, -3.0, -3.0],
            max: [7.0, 3.0, 3.0],
            dims: [48, 24, 24],
        },
        &sphere_flow,
    );
    let fine = grid(
        GridSpec {
            min: [-1.6, -1.6, -1.6],
            max: [1.6, 1.6, 1.6],
            dims: [64, 64, 64],
        },
        &sphere_flow,
    );
    Flow::new(
        vec![coarse, fine],
        Some(&sphere_surface(48)),
        Some(1.0),
        1.0,
    )
    .unwrap()
}

fn uniform() -> Flow {
    let g = grid(
        GridSpec {
            min: [0.0, 0.0, 0.0],
            max: [4.0, 1.0, 1.0],
            dims: [40, 10, 10],
        },
        &|_| Some((DVec3::new(2.0, 0.0, 0.0), 0.0)),
    );
    Flow::new(vec![g], None, Some(2.0), 1.0).unwrap()
}

fn vortex() -> Flow {
    let g = grid(
        GridSpec {
            min: [-2.0, -2.0, -0.5],
            max: [2.0, 2.0, 0.5],
            dims: [40, 40, 8],
        },
        &|p| Some((DVec3::new(-p.y, p.x, 0.0), 0.0)),
    );
    let mut f = Flow::new(vec![g], None, Some(1.0), 1.0).unwrap();
    // Half a turn in each direction: one full turn in all.
    f.max_length = std::f64::consts::PI;
    f
}

#[test]
fn a_uniform_field_gives_a_straight_path_with_the_right_flow_time() {
    let f = uniform();
    let path = f.trace([2.0, 0.5, 0.5]).unwrap();
    let first = &path.points[0];
    let last = path.points.last().unwrap();
    for p in &path.points {
        assert!((p.pos[1] - 0.5).abs() < 1e-9 && (p.pos[2] - 0.5).abs() < 1e-9);
    }
    let len = path.length();
    let tau = last.tau - first.tau;
    assert!(len > 3.0, "{len}");
    assert!(
        (tau - len / 2.0).abs() < 0.01 * len / 2.0,
        "{tau} vs {}",
        len / 2.0
    );
    assert_eq!(
        (path.back_end, path.forward_end),
        (End::Outside, End::Outside)
    );
}

#[test]
fn backward_and_forward_parts_meet_at_the_start_and_the_sampling_is_even() {
    let f = uniform();
    let path = f.trace([2.0, 0.5, 0.5]).unwrap();
    assert!((path.position_at(path.start_s) - DVec3::new(2.0, 0.5, 0.5)).length() < 1e-9);
    let n = path.points.len();
    for w in path.points[..n - 1].windows(2) {
        assert!((w[1].s - w[0].s - path.ds).abs() < 1e-9);
    }
    assert!(path.points[n - 1].s - path.points[n - 2].s <= path.ds + 1e-12);
    // Positions follow the arc length.
    for w in path.points.windows(2) {
        let d = (DVec3::from(w[1].pos) - DVec3::from(w[0].pos)).length();
        assert!((d - (w[1].s - w[0].s)).abs() < 1e-9);
    }
}

#[test]
fn a_solid_body_vortex_keeps_the_radius_and_turns_once() {
    let f = vortex();
    let path = f.trace([1.0, 0.0, 0.0]).unwrap();
    for p in &path.points {
        let r = (p.pos[0] * p.pos[0] + p.pos[1] * p.pos[1]).sqrt();
        assert!((r - 1.0).abs() < 0.01, "radius {r}");
    }
    let feat = features(&path, &f);
    let tau = std::f64::consts::TAU;
    assert!((feat.turn - tau).abs() < 0.02 * tau, "turn {}", feat.turn);
    assert_eq!(path.forward_end, End::Length);
}

#[test]
fn the_stagnation_streamline_of_a_sphere_follows_potential_flow_and_stalls() {
    let f = sphere();
    let path = f.trace([-4.0, 0.0, 0.0]).unwrap();
    assert_eq!(path.forward_end, End::Stalled);
    // On the axis |u| = 1 - R³/x³. Trilinear interpolation of this curve errs by h²/8 · |u''|
    // (below 0.002 for x <= -1.3 with h = 0.05 on the fine grid, 0.02 on the coarse grid at
    // x = -2.5); 0.03 leaves room for the off-grid sample points.
    let mut checked = 0;
    for p in &path.points {
        let x = p.pos[0];
        if (-2.5..=-1.3).contains(&x) {
            let want = 1.0 - 1.0 / (-x).powi(3);
            assert!(
                (p.speed - want).abs() < 0.03,
                "x {x}: {} vs {want}",
                p.speed
            );
            checked += 1;
        }
    }
    assert!(checked > 50);
    // It ends in front of the sphere: the grid wall lies between the last fluid and the first
    // solid grid point, so within one grid step (0.05) of the true surface.
    let last = path.points.last().unwrap();
    assert!(last.pos[0] > -1.1 && last.pos[0] < -0.95, "{:?}", last.pos);
    for p in &path.points {
        assert!(!matches!(
            f.velocity(DVec3::from(p.pos)),
            Probe::Blocked | Probe::Outside
        ));
    }
    // Readings: pressure rises to the stagnation value, the wall distance falls.
    assert!(last.cp > 0.8, "cp {}", last.cp);
    assert!(last.d < 0.1);
}

#[test]
fn paths_around_the_sphere_never_enter_the_solid() {
    let f = sphere();
    for y in [0.2, 0.5, 0.9] {
        let path = f.trace([-4.0, y, 0.1]).unwrap();
        for p in &path.points {
            assert!(DVec3::from(p.pos).length() > 0.99, "{:?}", p.pos);
            assert!(!matches!(f.velocity(DVec3::from(p.pos)), Probe::Blocked));
        }
        // It passes the sphere and leaves the domain downstream.
        assert!(path.points.last().unwrap().pos[0] > 5.0);
    }
}

#[test]
fn a_start_in_the_solid_has_no_path() {
    assert!(sphere().trace([0.0, 0.0, 0.0]).is_none());
    assert!(sphere().trace([50.0, 0.0, 0.0]).is_none());
}

fn sphere_path(y: f64) -> (Flow, Path) {
    let f = sphere();
    let p = f.trace([-4.0, y, 0.0]).unwrap();
    (f, p)
}

#[test]
fn screen_time_is_monotonic_and_the_calm_ride_keeps_its_floor() {
    let (f, path) = sphere_path(0.3);
    let t = timing(&path, &f, Mode::Calm, 1.0);
    assert!(t.t.windows(2).all(|w| w[1] > w[0]));
    assert!(t.v.iter().all(|v| *v >= 0.08 * f.length - 1e-12));
    // The time factor is flow time per screen time: slow motion below 1.
    for (i, p) in path.points.iter().enumerate() {
        let want = t.v[i] / p.speed.max(0.02 * f.u_ref);
        assert!(
            (t.factor[i] - want).abs() < 1e-9 * want,
            "{} vs {want}",
            t.factor[i]
        );
    }
    // The speed control scales the duration exactly.
    let fast = timing(&path, &f, Mode::Calm, 2.0);
    assert!((fast.duration() * 2.0 - t.duration()).abs() < 1e-9 * t.duration());
    let slow = timing(&path, &f, Mode::Calm, 0.5);
    assert!((slow.duration() - 2.0 * t.duration()).abs() < 1e-9 * t.duration());
}

#[test]
fn the_coaster_has_a_constant_time_factor_and_a_bounded_duration() {
    let (f, path) = sphere_path(0.3);
    let t = timing(&path, &f, Mode::Coaster, 1.0);
    let want = (2.5 * path.length() / f.length).clamp(10.0, 45.0);
    assert!(
        (t.duration() - want).abs() < 1e-6 * want,
        "{} vs {want}",
        t.duration()
    );
    let k = t.factor[0];
    assert!(t.factor.iter().all(|x| (x - k).abs() < 1e-9 * k));
}

#[test]
fn arc_length_and_screen_time_map_both_ways() {
    let (f, path) = sphere_path(0.3);
    let t = timing(&path, &f, Mode::Calm, 1.0);
    for s in [
        0.0,
        0.3 * path.length(),
        0.77 * path.length(),
        path.length(),
    ] {
        let back = t.s_at(&path, t.t_at(&path, s));
        assert!(
            (back - s).abs() < 1e-9 * path.length().max(1.0),
            "{s} {back}"
        );
    }
}

fn helix() -> (Flow, Path, Timing) {
    // Rising helix: rotation about Z plus a constant lift.
    let g = grid(
        GridSpec {
            min: [-2.0, -2.0, -1.0],
            max: [2.0, 2.0, 9.0],
            dims: [40, 40, 100],
        },
        &|p| Some((DVec3::new(-p.y, p.x, 0.3), 0.0)),
    );
    let mut f = Flow::new(vec![g], None, Some(1.0), 1.0).unwrap();
    f.length = 1.0;
    f.max_length = 12.0;
    let path = f.trace([1.0, 0.0, 0.0]).unwrap();
    let t = timing(&path, &f, Mode::Coaster, 1.0);
    (f, path, t)
}

fn check_frames(track: &[CamKey]) {
    for k in track {
        assert!((k.forward.length() - 1.0).abs() < 1e-9);
        assert!((k.up.length() - 1.0).abs() < 1e-9);
        assert!(k.up.dot(k.forward).abs() < 1e-9);
    }
}

#[test]
fn the_coaster_frame_never_flips_on_a_helix() {
    let (f, path, t) = helix();
    let track = camera_track(&path, &t, &f, Mode::Coaster);
    check_frames(&track);
    for w in track.windows(2) {
        assert!(w[0].up.dot(w[1].up) > 0.9);
    }
    // It banks into the turn, at most 35°.
    let mid = &track[track.len() / 2];
    let bank = mid.bank_deg;
    assert!(bank > 1.0 && bank <= 35.0 + 1e-6, "bank {bank}");
}

#[test]
fn the_calm_camera_keeps_the_horizon_level() {
    let (f, path, _) = helix();
    let t = timing(&path, &f, Mode::Calm, 1.0);
    let track = camera_track(&path, &t, &f, Mode::Calm);
    check_frames(&track);
    for k in &track {
        // Up lies in the plane of the view direction and world Z.
        let side = k.forward.cross(DVec3::Z);
        if side.length() > 0.5 {
            assert!(k.up.dot(side.normalize()).abs() < 1e-6);
            assert!(k.up.z > 0.0);
        }
        assert_eq!(k.fov_deg, 75.0);
    }
}

#[test]
fn the_same_time_gives_the_same_camera_at_30_and_60_frames_per_second() {
    let (f, path) = sphere_path(0.3);
    let t = timing(&path, &f, Mode::Calm, 1.0);
    let track = camera_track(&path, &t, &f, Mode::Calm);
    for k in 0..(t.duration() * 30.0) as u32 {
        let a = camera_at(&track, f64::from(k) / 30.0).unwrap();
        let b = camera_at(&track, f64::from(2 * k) / 60.0).unwrap();
        assert_eq!(a, b);
    }
    // The near plane never reaches the wall.
    for k in &track {
        assert!(k.near > 0.0 && k.near <= 0.01 * f.length + 1e-12);
    }
}

fn point(s: f64, speed: f64, cp: f64, d: f64, vx: f64) -> RidePoint {
    RidePoint {
        pos: [s, 0.0, 0.0],
        s,
        tau: 0.0,
        vel: [vx * speed, 0.0, 0.0],
        speed,
        p: cp * 0.5,
        cp,
        d,
        accel: 0.0,
        q: 0.0,
        slip: 0.0,
    }
}

#[test]
fn events_come_in_order_with_minimum_duration_and_cooldown() {
    let f = uniform();
    let (u, h, l) = (f.u_ref, f.h, f.length);
    // Hand-made ride: approach, stagnation at the wall, speed-up with suction, reverse flow, calm.
    let mut pts = Vec::new();
    let ds = 0.25 * h;
    let n = 2000;
    for i in 0..n {
        let s = f64::from(i) * ds;
        let x = f64::from(i) / f64::from(n);
        let (speed, cp, d, vx) = match x {
            x if x < 0.15 => (u, 0.0, 10.0 * h, 1.0),
            x if x < 0.3 => (u * (1.0 - (x - 0.15) * 3.0), (x - 0.15) * 6.0, 6.0 * h, 1.0),
            x if x < 0.4 => (0.1 * u, 0.95, 3.0 * h, 1.0),
            x if x < 0.55 => (1.4 * u, -0.8 + (x - 0.4), 1.5 * h, 1.0),
            x if x < 0.7 => (0.5 * u, 0.1, 6.0 * h, -1.0),
            _ => (u, 0.0, 10.0 * h, 1.0),
        };
        pts.push(point(s, speed, cp, d, vx));
    }
    let path = Path {
        points: pts,
        ds,
        start_s: 0.0,
        back_end: End::Outside,
        forward_end: End::Outside,
    };
    // One screen second per tenth of the ride.
    let t = Timing {
        t: (0..n).map(|i| f64::from(i) / f64::from(n) * 10.0).collect(),
        v: vec![1.0; n as usize],
        factor: vec![1.0; n as usize],
    };
    let ev = events(&path, &f, &t);
    let ids: Vec<&str> = ev.iter().map(|e| e.id).collect();
    assert_eq!(
        ids,
        [
            "approach",
            "stagnation",
            "acceleration",
            "near_wall",
            "suction",
            "reverse",
            "calm"
        ],
        "{ev:?}"
    );
    assert!(ev.windows(2).all(|w| w[0].s <= w[1].s));
    let _ = l;
    // A short blip (below 0.3 s) does not count; the same event twice within 2 s counts once.
    let mut pts = path.points.clone();
    for (i, p) in pts.iter_mut().enumerate() {
        let x = i as f64 / f64::from(n);
        p.speed = u;
        p.cp = 0.0;
        p.d = 10.0 * h;
        p.vel = [u, 0.0, 0.0];
        if (0.1..0.12).contains(&x) || (0.3..0.36).contains(&x) || (0.4..0.46).contains(&x) {
            p.speed = 1.3 * u;
        }
    }
    let path = Path {
        points: pts,
        ..path
    };
    let ev = events(&path, &f, &t);
    let acc: Vec<f64> = ev
        .iter()
        .filter(|e| e.id == "acceleration")
        .map(|e| e.t)
        .collect();
    assert_eq!(acc.len(), 1, "{ev:?}");
    assert!((acc[0] - 3.0).abs() < 0.01);
}

#[test]
fn suggestions_at_the_sphere() {
    let f = sphere();
    let mut starts = wind_candidates(&f, 7);
    assert_eq!(starts.len(), 384);
    starts.extend(surface_candidates(&f, 216, 7));
    assert!(starts.len() > 500);
    let list = suggest(&f, &starts, Kind::WindTunnel);
    let ids: Vec<&str> = list.iter().map(|s| s.id).collect();
    assert!(
        ids.contains(&"stagnation") && ids.contains(&"hugging"),
        "{ids:?}"
    );
    assert!(list.len() <= 5);
    assert!(!ids.contains(&"long") && !ids.contains(&"wake"), "{ids:?}");
    // Stagnation: the passing path that comes closest to standing still, near the axis.
    let stag = list.iter().find(|s| s.id == "stagnation").unwrap();
    assert!(stag.features.passes);
    let off = (stag.start[1].powi(2) + stag.start[2].powi(2)).sqrt();
    let best_off = starts[..384]
        .iter()
        .map(|s| (s[1].powi(2) + s[2].powi(2)).sqrt())
        .fold(f64::MAX, f64::min);
    let stall_best = starts
        .iter()
        .filter_map(|s| f.trace(*s))
        .map(|p| features(&p, &f))
        .filter(|x| x.through && x.passes && x.d_min < 0.1 * f.length && x.length > 1.0)
        .map(|x| x.stall)
        .fold(0.0, f64::max);
    assert!((stag.features.stall - stall_best).abs() < 1e-12);
    assert!(
        off < 0.2 || stag.start[0] > -2.0,
        "off axis {off}, best {best_off}"
    );
    // Hugging: the longest stretch close to the wall among the paths not taken by the category
    // chosen before it (stagnation).
    let hug = list.iter().find(|s| s.id == "hugging").unwrap();
    assert!(
        list.iter()
            .filter(|s| s.id != "stagnation")
            .all(|s| s.features.near <= hug.features.near)
    );
    // Never two picks too close to each other, and the same seed gives the same catalogue.
    for a in &list {
        for b in &list {
            if a.id != b.id {
                let d =
                    (DVec3::from(a.features.closest) - DVec3::from(b.features.closest)).length();
                assert!(d >= 0.05 * f.length, "{} {}", a.id, b.id);
            }
        }
    }
    let again = suggest(&f, &starts, Kind::WindTunnel);
    assert_eq!(
        list.iter().map(|s| s.start).collect::<Vec<_>>(),
        again.iter().map(|s| s.start).collect::<Vec<_>>()
    );
    let _ = ride::CATEGORIES;
}

#[test]
fn only_paths_that_run_through_count_and_circling_ones_do_not() {
    let f = uniform();
    let p = f.trace([2.0, 0.5, 0.5]).unwrap();
    assert!(features(&p, &f).through);
    let v = vortex();
    let p = v.trace([1.0, 0.0, 0.0]).unwrap();
    assert!(!features(&p, &v).through);
    // The stagnation streamline ends standing still in front of the model: it counts.
    let s = sphere();
    let p = s.trace([-4.0, 0.0, 0.0]).unwrap();
    assert!(features(&p, &s).through);
}

#[test]
fn the_ride_is_trimmed_to_the_stretch_around_the_model() {
    let f = sphere();
    let full = f.trace([-4.0, 0.5, 0.2]).unwrap();
    let start = full.position_at(full.start_s);
    let path = full.clone().trimmed(&f);
    let (l, h) = (f.length, f.h);
    let first = path.points[0].pos[0];
    let last = path.points.last().unwrap().pos[0];
    // From one model length in front of the model box to 1.5 lengths behind it.
    assert!((first - (-1.0 - l)).abs() < 0.3 * h + 1e-9, "{first}");
    assert!((last - (1.0 + 1.5 * l)).abs() < 0.3 * h + 1e-9, "{last}");
    assert!(path.points[0].s.abs() < 1e-12 && path.points[0].tau.abs() < 1e-12);
    assert!((path.length() - (path.points.last().unwrap().s)).abs() < 1e-12);
    // The start point lies before the stretch: the trimmed ride starts at its beginning.
    assert!(start.x < first);
    assert_eq!(path.start_s, 0.0);
    // Equal sampling stays.
    for w in path.points[..path.points.len() - 1].windows(2) {
        assert!((w[1].s - w[0].s - path.ds).abs() < 1e-9);
    }
}

#[test]
fn a_path_that_stops_at_an_opening_away_from_walls_runs_through() {
    // Internal flow: beyond inlet and outlet the grid holds no fluid, so the path stands still
    // there, but far from any wall. That is leaving, not a stagnation point.
    let g = grid(
        GridSpec {
            min: [0.0, 0.0, 0.0],
            max: [4.0, 1.0, 1.0],
            dims: [40, 10, 10],
        },
        &|p| (p.x > 0.5 && p.x < 3.5).then_some((DVec3::new(2.0, 0.0, 0.0), 0.0)),
    );
    // A wall far below the channel axis.
    let wall = Surface {
        points: vec![[0.0, -5.0, -5.0], [4.0, -5.0, -5.0], [0.0, 5.0, -5.0]],
        triangles: vec![[0, 1, 2]],
        fields: vec![],
    };
    let mut f = Flow::new(vec![g], Some(&wall), Some(2.0), 1.0).unwrap();
    f.main = None;
    let path = f.trace([2.0, 0.5, 0.5]).unwrap();
    assert_eq!(path.forward_end, End::Stalled);
    let x = features(&path, &f);
    assert!(x.through && x.passes);
}
