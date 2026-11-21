// SPDX-License-Identifier: GPL-3.0-or-later
//! Convergence automation on synthetic coefficient curves: stable, oscillating, drifting,
//! diverging.

use foamcraft_sim::solve::{Coefficients, ConvergenceSettings, Verdict, assess, assess_with_flow};

fn curve(n: usize, cd: impl Fn(f64) -> f64, cl: impl Fn(f64) -> f64) -> Vec<Coefficients> {
    (1..=n)
        .map(|i| {
            let i = i as f64;
            Coefficients {
                cd: cd(i),
                cl: cl(i),
                ..Default::default()
            }
        })
        .collect()
}

/// Feeds the curve iteration by iteration until the assessment stops the run.
fn run(history: &[Coefficients], s: &ConvergenceSettings) -> (usize, Verdict, Coefficients) {
    for n in 1..=history.len() {
        let a = assess(&history[..n], s);
        if a.verdict != Verdict::Continue {
            return (n, a.verdict, a.mean);
        }
    }
    let a = assess(history, s);
    (history.len(), a.verdict, a.mean)
}

fn settings() -> ConvergenceSettings {
    ConvergenceSettings::for_iterations(500, 1500)
}

#[test]
fn window_follows_the_expected_iterations() {
    assert_eq!(ConvergenceSettings::for_iterations(500, 1500).window, 100);
    assert_eq!(ConvergenceSettings::for_iterations(50, 100).window, 30);
    assert_eq!(ConvergenceSettings::for_iterations(5000, 9000).window, 200);
}

#[test]
fn a_settling_curve_stops_as_stable() {
    let h = curve(
        1500,
        |i| 0.45 + 0.3 * (-i / 40.0).exp(),
        |i| 0.1 - 0.2 * (-i / 30.0).exp(),
    );
    let (n, verdict, mean) = run(&h, &settings());
    assert_eq!(verdict, Verdict::Stable);
    assert!(n < 500, "stopped only at {n}");
    assert!((mean.cd - 0.45).abs() < 0.005, "{mean:?}");
    assert!((mean.cl - 0.1).abs() < 0.005, "{mean:?}");
}

#[test]
fn an_oscillating_curve_stops_with_the_mean() {
    let h = curve(
        1500,
        |i| 1.05 + 0.05 * (i * std::f64::consts::TAU / 23.0).sin(),
        |i| 0.2 * (i * std::f64::consts::TAU / 37.0).sin(),
    );
    let (n, verdict, mean) = run(&h, &settings());
    assert_eq!(verdict, Verdict::Oscillating);
    assert!(n < 1500);
    assert!((mean.cd - 1.05).abs() < 0.01, "{mean:?}");
    assert!(mean.cl.abs() < 0.02, "{mean:?}");
}

#[test]
fn a_drifting_curve_runs_to_the_limit() {
    let h = curve(1500, |i| 0.5 + 0.0004 * i, |_| 0.0);
    let (n, verdict, mean) = run(&h, &settings());
    assert_eq!(verdict, Verdict::LimitReached);
    assert_eq!(n, 1500);
    // The mean of the last window.
    assert!((mean.cd - (0.5 + 0.0004 * 1450.5)).abs() < 1e-6, "{mean:?}");
}

#[test]
fn a_diverging_curve_is_detected() {
    let h = curve(1500, |i| 1.0 + 0.01 * (0.05 * i).exp(), |_| 0.0);
    let (n, verdict, _) = run(&h, &settings());
    assert_eq!(verdict, Verdict::Diverged);
    assert!(n < 400, "{n}");
    // Not a number is divergence at once.
    let mut nan = curve(10, |_| 1.0, |_| 0.0);
    nan[9].cd = f64::NAN;
    assert_eq!(assess(&nan, &settings()).verdict, Verdict::Diverged);
}

#[test]
fn nothing_stops_before_two_windows() {
    let h = curve(150, |_| 1.0, |_| 0.0);
    assert_eq!(assess(&h, &settings()).verdict, Verdict::Continue);
    let h = curve(200, |_| 1.0, |_| 0.0);
    assert_eq!(assess(&h, &settings()).verdict, Verdict::Stable);
}

/// Mean speed in the domain of a jet that starts from rest: it overshoots while the front
/// crosses the domain and settles at 0.12 m/s (shape of the free jet validation run, where the
/// 1 % band was met after about 470 iterations).
fn developing_flow(n: usize) -> Vec<f64> {
    (1..=n)
        .map(|i| {
            let i = i as f64;
            0.12 * (1.0 - (-i / 60.0).exp()) + 0.05 * (-((i - 125.0) / 40.0).powi(2)).exp()
        })
        .collect()
}

/// Feeds forces and flow iteration by iteration until the assessment stops the run.
fn run_with_flow(
    history: &[Coefficients],
    flow: &[f64],
    s: &ConvergenceSettings,
) -> (usize, Verdict, Coefficients) {
    for n in 1..=history.len() {
        let a = assess_with_flow(&history[..n], &flow[..n], s);
        if a.verdict != Verdict::Continue {
            return (n, a.verdict, a.mean);
        }
    }
    let a = assess_with_flow(history, flow, s);
    (history.len(), a.verdict, a.mean)
}

#[test]
fn forces_near_zero_do_not_stop_a_run_whose_flow_still_develops() {
    // Draft quality: window 60. The model stands beside the jet: its coefficients are zero from
    // the first iteration, so the forces alone look settled after two windows.
    let s = ConvergenceSettings::for_iterations(300, 900);
    let h = curve(900, |_| -3e-7, |_| 1e-7);
    let flow = developing_flow(900);
    assert_eq!(run(&h, &s).0, 120);
    let (n, verdict, mean) = run_with_flow(&h, &flow, &s);
    assert_eq!(verdict, Verdict::Stable);
    assert!((250..500).contains(&n), "stopped at {n}");
    assert!(mean.cd.abs() < 1e-6, "{mean:?}");
    // The speed at the stop is the settled one.
    assert!((flow[n - 1] - 0.12).abs() < 0.0012, "{}", flow[n - 1]);
}

#[test]
fn settled_flow_leaves_the_verdict_to_the_forces() {
    let s = settings();
    let flow = vec![0.12; 1500];
    let h = curve(
        1500,
        |i| 1.05 + 0.05 * (i * std::f64::consts::TAU / 23.0).sin(),
        |i| 0.2 * (i * std::f64::consts::TAU / 37.0).sin(),
    );
    let (n, verdict, mean) = run_with_flow(&h, &flow, &s);
    assert_eq!((n, verdict), {
        let (n, v, _) = run(&h, &s);
        (n, v)
    });
    assert_eq!(verdict, Verdict::Oscillating);
    assert!((mean.cd - 1.05).abs() < 0.01, "{mean:?}");
    // Forces that still drift are not stopped by a settled flow.
    let drift = curve(300, |i| 0.5 + 0.0004 * i, |_| 0.0);
    assert_eq!(
        assess_with_flow(&drift, &flow[..300], &s).verdict,
        Verdict::Continue
    );
}

#[test]
fn a_flow_that_swings_or_drifts_runs_to_the_limit() {
    let s = ConvergenceSettings::for_iterations(300, 900);
    let h = curve(900, |_| 0.0, |_| 0.0);
    // Window means that agree are not enough for the flow: in the validation run they agreed
    // after 245 iterations, while the jet was still far from its final shape.
    let swing: Vec<f64> = (1..=900)
        .map(|i| 0.12 + 0.01 * (i as f64 * std::f64::consts::TAU / 23.0).sin())
        .collect();
    let (n, verdict, _) = run_with_flow(&h, &swing, &s);
    assert_eq!((n, verdict), (900, Verdict::LimitReached));
    let drift: Vec<f64> = (1..=900).map(|i| 0.05 + 0.0002 * i as f64).collect();
    let (n, verdict, _) = run_with_flow(&h, &drift, &s);
    assert_eq!((n, verdict), (900, Verdict::LimitReached));
}

#[test]
fn a_flow_that_is_not_a_number_counts_as_diverged() {
    let s = ConvergenceSettings::for_iterations(300, 900);
    let h = curve(200, |_| 0.0, |_| 0.0);
    let mut flow = vec![0.12; 200];
    flow[199] = f64::NAN;
    assert_eq!(assess_with_flow(&h, &flow, &s).verdict, Verdict::Diverged);
}
