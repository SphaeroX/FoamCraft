// SPDX-License-Identifier: GPL-3.0-or-later
//! Internal flow scenario: hydraulic diameter, laminar or turbulent, plausibility of the boundary
//! conditions, scenario.set_internal_flow.

use std::path::PathBuf;

use foamcraft_core::command::{Registry, Session, core_registry};
use foamcraft_sim::internal::{FlowRegime, hydraulic_diameter, regime};
use serde_json::{Value, json};

#[test]
fn hydraulic_diameter_of_circle_square_and_ring() {
    let r = 0.01;
    let circle = hydraulic_diameter(std::f64::consts::PI * r * r, 2.0 * std::f64::consts::PI * r);
    assert!((circle - 2.0 * r).abs() < 1e-12);
    assert!((hydraulic_diameter(0.04, 0.8) - 0.2).abs() < 1e-12);
    // Ring between R = 12 and r = 6: D_h = 2 (R - r).
    let (big, small) = (12.0f64, 6.0f64);
    let ring = hydraulic_diameter(
        std::f64::consts::PI * (big * big - small * small),
        2.0 * std::f64::consts::PI * (big + small),
    );
    assert!((ring - 12.0).abs() < 1e-9);
}

#[test]
fn laminar_below_2300_turbulent_above_4000() {
    assert_eq!(regime(1000.0), FlowRegime::Laminar);
    assert_eq!(regime(2299.0), FlowRegime::Laminar);
    assert_eq!(regime(3000.0), FlowRegime::Transitional);
    assert_eq!(regime(10_000.0), FlowRegime::Turbulent);
}

fn setup(name: &str) -> (Registry, Session) {
    let mut reg = core_registry();
    foamcraft_geometry::commands::register(&mut reg);
    foamcraft_sim::commands::register(&mut reg);
    let base = std::env::temp_dir().join(format!("fc-internal-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&base);
    let mut s = Session::new(base.join("data"));
    reg.execute(
        &mut s,
        "project.new",
        json!({"path": base.join("p"), "name": "I"}),
    )
    .unwrap();
    let file = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/data/geometry/internal/pipe_open.stl");
    reg.execute(
        &mut s,
        "geometry.import",
        json!({"path": file, "unit": "mm"}),
    )
    .unwrap();
    for point in [[0.0, 0.0, 0.0], [0.05, 0.0, 0.0]] {
        reg.execute(
            &mut s,
            "geometry.cap_opening",
            json!({"model": "model-1", "point": point}),
        )
        .unwrap();
    }
    reg.execute(
        &mut s,
        "scenario.set_boundary_role",
        json!({"keys": ["model-1:cap-1"], "role": "inlet"}),
    )
    .unwrap();
    reg.execute(
        &mut s,
        "scenario.set_boundary_role",
        json!({"keys": ["model-1:cap-2"], "role": "outlet"}),
    )
    .unwrap();
    (reg, s)
}

fn params(inlet: Value, outlet_key: &str, outlet_pa: f64) -> Value {
    json!({
        "medium": {"kind": "water", "temperature_c": 20.0},
        "quality": "draft",
        "inlets": [{"key": "model-1:cap-1", "condition": inlet}],
        "outlets": [{"key": outlet_key, "pressure_pa": outlet_pa}]
    })
}

#[test]
fn a_pipe_with_velocity_inlet_is_laminar_at_low_speed() {
    let (reg, mut s) = setup("ok");
    let info = reg
        .execute(
            &mut s,
            "scenario.set_internal_flow",
            params(
                json!({"type": "velocity", "m_s": 0.05}),
                "model-1:cap-2",
                0.0,
            ),
        )
        .unwrap();
    assert_eq!(info["scenario"]["kind"], "internal_flow");
    let inlet = &info["openings"][0];
    assert_eq!(inlet["key"], "model-1:cap-1");
    let dh = inlet["hydraulic_diameter_m"].as_f64().unwrap();
    assert!((dh - 0.02).abs() < 0.0002, "{dh}");
    let re = info["reynolds"].as_f64().unwrap();
    // Water at 20 °C: nu ≈ 1.0e-6 m²/s → Re ≈ 0.05 · 0.02 / 1e-6 ≈ 1000.
    assert!((900.0..1100.0).contains(&re), "{re}");
    assert_eq!(info["regime"], "laminar");
    let again = reg.execute(&mut s, "scenario.info", json!({})).unwrap();
    assert_eq!(again["regime"], "laminar");
    // Volume flow gives the same velocity.
    let q = 0.05 * std::f64::consts::PI * 1e-4;
    let info = reg
        .execute(
            &mut s,
            "scenario.set_internal_flow",
            params(
                json!({"type": "volume_flow", "m3_s": q}),
                "model-1:cap-2",
                0.0,
            ),
        )
        .unwrap();
    let u = info["openings"][0]["velocity_m_s"].as_f64().unwrap();
    assert!((u - 0.05).abs() < 0.001, "{u}");
    // A fast flow is turbulent.
    let info = reg
        .execute(
            &mut s,
            "scenario.set_internal_flow",
            params(
                json!({"type": "velocity", "m_s": 2.0}),
                "model-1:cap-2",
                0.0,
            ),
        )
        .unwrap();
    assert_eq!(info["regime"], "turbulent");
}

#[test]
fn implausible_combinations_are_rejected() {
    let (reg, mut s) = setup("bad");
    let mut err = |p: Value| reg_err(&reg, &mut s, p);
    assert_eq!(
        err(params(
            json!({"type": "velocity", "m_s": -1.0}),
            "model-1:cap-2",
            0.0
        )),
        "scenario.invalid_input"
    );
    assert_eq!(
        err(params(
            json!({"type": "velocity", "m_s": 1.0}),
            "model-1:face-1",
            0.0
        )),
        "scenario.role_mismatch"
    );
    assert_eq!(
        err(params(
            json!({"type": "velocity", "m_s": 1.0}),
            "model-1:cap-1",
            0.0
        )),
        "scenario.duplicate_boundary"
    );
    assert_eq!(
        err(params(
            json!({"type": "velocity", "m_s": 1.0}),
            "model-1:cap-9",
            0.0
        )),
        "surface.not_found"
    );
    assert_eq!(
        err(params(
            json!({"type": "total_pressure", "pa": 5.0}),
            "model-1:cap-2",
            10.0
        )),
        "scenario.no_driving_pressure"
    );
    let mut no_outlet = params(
        json!({"type": "velocity", "m_s": 1.0}),
        "model-1:cap-2",
        0.0,
    );
    no_outlet["outlets"] = json!([]);
    assert_eq!(err(no_outlet), "scenario.no_outlet");
    let mut no_inlet = params(
        json!({"type": "velocity", "m_s": 1.0}),
        "model-1:cap-2",
        0.0,
    );
    no_inlet["inlets"] = json!([]);
    assert_eq!(err(no_inlet), "scenario.no_inlet");
}

fn reg_err(reg: &Registry, s: &mut Session, p: Value) -> String {
    reg.execute(s, "scenario.set_internal_flow", p)
        .unwrap_err()
        .code
}

mod evaluation {
    use foamcraft_sim::internal::{BoundaryValues, FieldValueLog, OpeningRole, evaluate_openings};
    use foamcraft_sim::solve::{ConvergenceSettings, Verdict, assess_series};

    #[test]
    fn surface_field_values_are_read_per_iteration() {
        let text = "Time = 1\n\nsurfaceFieldValue flow_in_1 write:\n    sum(model_in_1) of phi = -0.002\n\n\
                    surfaceFieldValue pressure_in_1 write:\n    areaAverage(model_in_1) of p = 0.5\n\n\
                    Time = 2\n\nsurfaceFieldValue flow_in_1 write:\n    sum(model_in_1) of phi = -0.003\n\n\
                    surfaceFieldValue pressure_in_1 write:\n    areaAverage(model_in_1) of p = 0.6\n\nEnd\n";
        let mut log = FieldValueLog::new(2);
        for line in text.lines() {
            log.feed(line);
        }
        assert_eq!(log.samples.len(), 2);
        assert_eq!(log.samples[1]["flow_in_1"], -0.003);
        assert_eq!(log.samples[0]["pressure_in_1"], 0.5);
    }

    #[test]
    fn several_series_settle_together() {
        let s = ConvergenceSettings::for_iterations(300, 900);
        let dp: Vec<f64> = (1..=400)
            .map(|i| 100.0 + 50.0 * (-(i as f64) / 20.0).exp())
            .collect();
        let q: Vec<f64> = (1..=400).map(|_| 0.002).collect();
        let mut stop = None;
        for n in 1..=400 {
            let a = assess_series(&[&dp[..n], &q[..n]], &s);
            if a.verdict != Verdict::Continue {
                stop = Some((n, a));
                break;
            }
        }
        let (n, a) = stop.unwrap();
        assert_eq!(a.verdict, Verdict::Stable);
        assert!(n < 300, "{n}");
        assert!((a.means[0] - 100.0).abs() < 1.0);
        // A drifting flow keeps it running.
        let drift: Vec<f64> = (1..=400).map(|i| 0.002 + 1e-5 * i as f64).collect();
        assert_eq!(
            assess_series(&[&dp[..200], &drift[..200]], &s).verdict,
            Verdict::Continue
        );
    }

    #[test]
    fn openings_give_flow_shares_pressure_loss_and_mass_balance() {
        // One inlet (0.003 m³/s in), two outlets (0.002 and 0.000985 out), water.
        let rho = 1000.0;
        let b = |region: &str, role, area| BoundaryValues {
            key: format!("model-1:{region}"),
            region: region.into(),
            role,
            area_m2: area,
        };
        let boundaries = [
            b("in_1", OpeningRole::Inlet, 1e-3),
            b("out_1", OpeningRole::Outlet, 1e-3),
            b("out_2", OpeningRole::Outlet, 5e-4),
        ];
        let mut sample = std::collections::BTreeMap::new();
        sample.insert("flow_in_1".to_string(), -0.003);
        sample.insert("pressure_in_1".to_string(), 0.5); // kinematic: 500 Pa
        sample.insert("flow_out_1".to_string(), 0.002);
        sample.insert("pressure_out_1".to_string(), 0.0);
        sample.insert("flow_out_2".to_string(), 0.000985);
        sample.insert("pressure_out_2".to_string(), 0.0);
        let r = evaluate_openings(&[sample], &boundaries, rho, 10);
        let inlet = &r.openings[0];
        assert!((inlet.flow_m3_s - 0.003).abs() < 1e-12);
        assert!((inlet.mass_flow_kg_s - 3.0).abs() < 1e-9);
        assert!((inlet.static_pressure_pa - 500.0).abs() < 1e-9);
        // Velocity 3 m/s → dynamic pressure 4500 Pa.
        assert!((inlet.total_pressure_pa - 5000.0).abs() < 1e-6);
        let out1 = &r.openings[1];
        assert!((out1.share.unwrap() - 0.002 / 0.002985).abs() < 1e-9);
        assert!((r.static_pressure_loss_pa - 500.0).abs() < 1e-9);
        // Total: inlet 5000 Pa; outlets flow-weighted: (0.002·2000 + 0.000985·1940.45)/0.002985.
        let out2_total = 0.5 * rho * (0.000985f64 / 5e-4).powi(2);
        let outlets = (0.002 * 2000.0 + 0.000985 * out2_total) / 0.002985;
        assert!(
            (r.pressure_loss_pa - (5000.0 - outlets)).abs() < 1e-6,
            "{}",
            r.pressure_loss_pa
        );
        assert!((r.mass_balance_error - 0.005).abs() < 1e-9);
    }
}
