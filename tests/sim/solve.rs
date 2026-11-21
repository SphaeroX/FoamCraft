// SPDX-License-Identifier: GPL-3.0-or-later
//! Solver setup and output: inlet turbulence, force log, y+, cautious numerics, result values.

use foamcraft_sim::solve::{
    Coefficients, SolverLog, YPlusHint, cautious_numerics, forces_from, inlet_turbulence,
    y_plus_hint,
};

fn log_text() -> String {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/data/logs/simpleFoam_forces.log");
    std::fs::read_to_string(path).unwrap()
}

#[test]
fn inlet_turbulence_matches_hand_calculation() {
    // k = 3/2 (I U)^2 = 1.5 (0.01 * 20)^2 = 0.06
    // omega = sqrt(k) / (Cmu^0.25 l) = 0.244949 / (0.547723 * 0.1) = 4.47214
    let t = inlet_turbulence(20.0, 0.01, 0.1);
    assert!((t.k - 0.06).abs() < 1e-12, "{t:?}");
    assert!((t.omega - 4.47214).abs() < 1e-4, "{t:?}");
    // 5 % at 2 m/s, l = 0.5 m: k = 1.5 * 0.01 = 0.015, omega = 0.122474 / 0.273861 = 0.447214
    let t = inlet_turbulence(2.0, 0.05, 0.5);
    assert!((t.k - 0.015).abs() < 1e-12);
    assert!((t.omega - 0.447214).abs() < 1e-5);
}

#[test]
fn force_coefficients_are_read_per_iteration() {
    let mut log = SolverLog::default();
    for line in log_text().lines() {
        log.feed(line);
    }
    let total: Vec<_> = log
        .samples
        .iter()
        .filter(|s| s.object == "forces")
        .collect();
    assert_eq!(total.len(), 2);
    assert_eq!(total[0].iteration, 1.0);
    assert_eq!(total[1].iteration, 2.0);
    let c = &total[1].coefficients;
    assert!((c.cd - 1.4529386).abs() < 1e-9);
    assert!((c.cl - 0.653671).abs() < 1e-9);
    assert!((c.cs - 0.0091735067).abs() < 1e-12);
    assert!((c.cm_pitch - 0.56202161).abs() < 1e-9);
    assert!((c.cm_roll - 0.32296223).abs() < 1e-9);
    assert!((c.cm_yaw + 0.72228532).abs() < 1e-9);
    // Residual history from the same lines.
    assert_eq!(log.iterations, vec![1.0, 2.0]);
    assert!((log.residuals["p"][1] - 0.045704789).abs() < 1e-12);
    assert!((log.residuals["Ux"][0] - 0.18571718).abs() < 1e-12);
}

#[test]
fn y_plus_is_read_per_patch() {
    let mut log = SolverLog::default();
    for line in log_text().lines() {
        log.feed(line);
    }
    assert_eq!(log.y_plus.len(), 3);
    let body = log.y_plus.iter().find(|y| y.patch == "model_body").unwrap();
    assert!((body.min - 0.40951827).abs() < 1e-9);
    assert!((body.max - 36.577488).abs() < 1e-9);
    assert!((body.average - 7.1988503).abs() < 1e-9);
}

#[test]
fn y_plus_hints_follow_the_wall_function_range() {
    assert_eq!(y_plus_hint(7.2), Some(YPlusHint::Low));
    assert_eq!(y_plus_hint(60.0), None);
    assert_eq!(y_plus_hint(450.0), Some(YPlusHint::High));
}

#[test]
fn forces_follow_from_coefficients() {
    // F = C q A, q = rho U^2 / 2 = 1.2 * 400 / 2 = 240 Pa, A = 0.5 m^2.
    let c = Coefficients {
        cd: 0.3,
        cl: -0.1,
        cs: 0.02,
        ..Default::default()
    };
    let f = forces_from(&c, 1.2, 20.0, 0.5);
    assert!((f.drag_n - 36.0).abs() < 1e-9);
    assert!((f.lift_n + 12.0).abs() < 1e-9);
    assert!((f.side_n - 2.4).abs() < 1e-9);
}

#[test]
fn cautious_numerics_damp_the_solver() {
    let schemes = "    div(phi,U)      bounded Gauss linearUpwindV grad(U);\n";
    let solution =
        "    consistent      true;\n    equations { U 0.9; \".*\" 0.9; }\n    fields { p 1; }\n";
    let (s, f) = cautious_numerics(schemes, solution);
    assert!(s.contains("div(phi,U)      bounded Gauss upwind;"), "{s}");
    assert!(f.contains("consistent      false;"), "{f}");
    assert!(f.contains("equations { U 0.5; \".*\" 0.5; }"), "{f}");
    assert!(f.contains("fields { p 0.3; }"), "{f}");
}

#[test]
fn solver_functions_cover_total_parts_y_plus_and_stop() {
    use foamcraft_sim::solve::solver_functions;
    let f = solver_functions(
        &["Inlet_face".into(), "body".into()],
        0.0004,
        0.02,
        20.0,
        1.2,
    );
    assert!(f.contains("    forces\n") && f.contains("patches (model);"));
    assert!(f.contains("part_Inlet_face") && f.contains("patches (model_Inlet_face);"));
    assert!(f.contains("part_body") && f.contains("patches (model_body);"));
    assert!(f.contains("Aref 4.000000e-4;") && f.contains("lRef 2.000000e-2;"));
    assert!(f.contains("magUInf 2.000000e1;") && f.contains("rhoInf 1.200000e0;"));
    assert!(f.contains("type yPlus;") && f.contains("type abort;"));
    // One region: the total is the only force object.
    let single = solver_functions(&["body".into()], 1.0, 1.0, 1.0, 1.0);
    assert_eq!(single.matches("type forceCoeffs;").count(), 1);
}

mod remaining {
    use foamcraft_sim::solve::{ConvergenceSettings, remaining_seconds};

    fn times(n: usize, step: f64) -> Vec<f64> {
        (1..=n).map(|i| i as f64 * step).collect()
    }

    #[test]
    fn before_the_expected_iterations_it_counts_to_them() {
        let s = ConvergenceSettings::for_iterations(300, 900);
        let r = remaining_seconds(&times(100, 0.5), &s, 300).unwrap();
        assert!((r - 100.0).abs() < 1e-9, "{r}");
    }

    #[test]
    fn after_the_expected_iterations_it_counts_one_window() {
        let s = ConvergenceSettings::for_iterations(300, 900);
        let r = remaining_seconds(&times(350, 0.5), &s, 300).unwrap();
        assert!((r - 30.0).abs() < 1e-9, "{r}");
        // Never beyond the limit.
        let r = remaining_seconds(&times(880, 0.5), &s, 300).unwrap();
        assert!((r - 10.0).abs() < 1e-9, "{r}");
    }

    #[test]
    fn single_slow_iterations_do_not_move_the_estimate() {
        let s = ConvergenceSettings::for_iterations(300, 900);
        let mut t = times(100, 0.5);
        for v in t.iter_mut().skip(95) {
            *v += 10.0; // one slow iteration (writing) at 96
        }
        let r = remaining_seconds(&t, &s, 300).unwrap();
        assert!((r - 100.0).abs() < 1e-9, "{r}");
    }

    #[test]
    fn no_estimate_from_too_few_iterations() {
        let s = ConvergenceSettings::for_iterations(300, 900);
        assert_eq!(remaining_seconds(&times(2, 0.5), &s, 300), None);
    }
}

#[test]
fn execution_times_are_recorded_per_iteration() {
    let mut log = SolverLog::default();
    for line in log_text().lines() {
        log.feed(line);
    }
    assert_eq!(log.execution_times, vec![0.72, 1.18]);
}

#[test]
fn pressure_and_viscous_parts_are_read() {
    let mut log = SolverLog::default();
    for line in log_text().lines() {
        log.feed(line);
    }
    let s = &log.samples[1];
    // "Cd: 1.4529386 1.4088993 0.044039265 0"
    assert!((s.pressure.cd - 1.4088993).abs() < 1e-9);
    assert!((s.viscous.cd - 0.044039265).abs() < 1e-9);
    assert!((s.pressure.cl - 0.6483358).abs() < 1e-9);
}
