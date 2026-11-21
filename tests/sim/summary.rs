// SPDX-License-Identifier: GPL-3.0-or-later
//! Key figures of a run (results.summary): derived values against hand calculations, the
//! reliability light, and the stable schema (snapshot in tests/data/snapshots).

use foamcraft_sim::summary::{
    Level, Summary, drag_area, loss_coefficient, power, reliability_internal, reliability_wind,
    summarize,
};
use serde_json::json;

#[test]
fn derived_values_match_hand_calculations() {
    // Cd 0.3 on 0.5 m²: drag area 0.15 m².
    assert!((drag_area(0.3, 0.5) - 0.15).abs() < 1e-12);
    // 40 N at 25 m/s: 1000 W.
    assert!((power(40.0, 25.0) - 1000.0).abs() < 1e-9);
    // 450 Pa loss at 3 m/s in water (1000 kg/m³): zeta = 450 / 4500 = 0.1.
    assert!((loss_coefficient(450.0, 1000.0, 3.0) - 0.1).abs() < 1e-12);
}

#[test]
fn the_reliability_light_follows_the_worst_check() {
    let good = reliability_wind("stable", 0.5, "good", Some(60.0));
    assert_eq!(good.level, Level::Green);
    let borderline = reliability_wind("oscillating", 3.0, "acceptable", Some(60.0));
    assert_eq!(borderline.level, Level::Yellow);
    assert!(
        borderline
            .checks
            .iter()
            .any(|c| c.code == "reliability.oscillating")
    );
    let bad = reliability_wind("limit_reached", 12.0, "good", Some(2.0));
    assert_eq!(bad.level, Level::Red);
    assert!(
        bad.checks
            .iter()
            .any(|c| c.code == "reliability.y_plus_far_off" && c.level == Level::Red)
    );
    assert_eq!(
        reliability_internal("stable", 0.2, "good", 0.001).level,
        Level::Green
    );
    assert_eq!(
        reliability_internal("stable", 0.2, "good", 0.008).level,
        Level::Yellow
    );
    assert_eq!(
        reliability_internal("stable", 0.2, "poor", 0.0).level,
        Level::Red
    );
}

fn wind_result() -> serde_json::Value {
    json!({
        "job_id": "job-1",
        "reason": "stable",
        "fluctuation_percent": 0.4,
        "coefficients": {"cd": 0.3, "cl": -0.1, "cs": 0.0, "cm_pitch": 0.01, "cm_roll": 0.0, "cm_yaw": 0.0},
        "forces": {"drag_n": 40.0, "lift_n": -13.3, "side_n": 0.0},
        "drag_split": {"pressure_n": 36.0, "viscous_n": 4.0},
        "parts": [{"name": "body", "coefficients": {"cd": 0.2, "cl": 0.0, "cs": 0.0, "cm_pitch": 0.0, "cm_roll": 0.0, "cm_yaw": 0.0},
                   "forces": {"drag_n": 26.7, "lift_n": 0.0, "side_n": 0.0}}],
        "reference": {"area_m2": 0.5, "length_m": 1.0, "speed_m_s": 25.0, "density_kg_m3": 1.2, "dynamic_pressure_pa": 375.0},
        "y_plus": [{"patch": "model", "min": 1.0, "max": 100.0, "average": 50.0}],
        "mesh": {"job_id": "m", "cells": 100, "verdict": "good", "layers_target": 3, "layers_achieved": 2.5, "fallbacks": []}
    })
}

#[test]
fn a_wind_tunnel_summary_has_all_figures_with_units() {
    let s: Summary = summarize(&wind_result()).unwrap();
    assert_eq!(s.kind, "wind_tunnel");
    let get = |id: &str| {
        s.figures
            .iter()
            .find(|f| f.id == id && f.part.is_none())
            .unwrap_or_else(|| panic!("{id}"))
    };
    assert_eq!(get("drag_n").unit, "N");
    assert!((get("drag_area_m2").value - 0.15).abs() < 1e-12);
    assert!((get("power_w").value - 1000.0).abs() < 1e-9);
    assert_eq!(get("drag_pressure_n").value, 36.0);
    assert_eq!(get("cd").unit, "-");
    assert!(
        s.figures
            .iter()
            .any(|f| f.id == "drag_n" && f.part.as_deref() == Some("body"))
    );
    assert_eq!(s.reliability.level, Level::Green);
}

#[test]
fn an_internal_flow_summary_has_loss_flows_and_balance() {
    let r = json!({
        "kind": "internal_flow",
        "job_id": "job-2",
        "reason": "oscillating",
        "fluctuation_percent": 2.0,
        "pressure_loss_pa": 450.0,
        "static_pressure_loss_pa": 400.0,
        "mass_balance_error": 0.002,
        "openings": [
            {"key": "m:cap-1", "role": "inlet", "flow_m3_s": 0.003, "mass_flow_kg_s": 3.0, "share": null, "velocity_m_s": 3.0, "static_pressure_pa": 500.0, "total_pressure_pa": 5000.0},
            {"key": "m:cap-2", "role": "outlet", "flow_m3_s": 0.003, "mass_flow_kg_s": 3.0, "share": 1.0, "velocity_m_s": 3.0, "static_pressure_pa": 0.0, "total_pressure_pa": 4500.0}
        ],
        "scenario": {"medium": {"kind": "water", "temperature_c": 20.0}},
        "mesh": {"job_id": "m", "cells": 100, "verdict": "acceptable", "layers_target": 0, "layers_achieved": 0.0, "fallbacks": []}
    });
    let s = summarize(&r).unwrap();
    assert_eq!(s.kind, "internal_flow");
    let zeta = s
        .figures
        .iter()
        .find(|f| f.id == "loss_coefficient")
        .unwrap();
    // 450 Pa / (0.5 · 998.2 · 3²).
    assert!(
        (zeta.value - 450.0 / (0.5 * 998.2 * 9.0)).abs() < 1e-3,
        "{}",
        zeta.value
    );
    assert!(
        s.figures
            .iter()
            .any(|f| f.id == "flow_m3_s" && f.part.as_deref() == Some("m:cap-2"))
    );
    assert!(s.figures.iter().any(|f| f.id == "share" && f.unit == "%"));
    assert_eq!(s.reliability.level, Level::Yellow);
}

#[test]
fn the_summary_schema_is_stable() {
    // Task_18 (MCP) and Task_20 (report) rely on it; change only with a stated reason.
    let schema = serde_json::to_string_pretty(&schemars::schema_for!(Summary)).unwrap();
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/data/snapshots/results_summary_schema.json");
    if std::env::var_os("FOAMCRAFT_UPDATE_SNAPSHOTS").is_some() || !path.is_file() {
        std::fs::write(&path, &schema).unwrap();
    }
    let want = std::fs::read_to_string(&path).unwrap();
    assert_eq!(schema.replace("\r\n", "\n"), want.replace("\r\n", "\n"));
}

#[test]
fn every_figure_and_reason_is_explained_in_both_languages() {
    use foamcraft_core::error::{Language, message};
    let mut codes: Vec<String> = summarize(&wind_result())
        .unwrap()
        .figures
        .iter()
        .map(|f| format!("figure.{}", f.id))
        .collect();
    // Internal flow figures (see `summarize`).
    for id in [
        "pressure_loss_pa",
        "static_pressure_loss_pa",
        "loss_coefficient",
        "flow_m3_s",
        "mass_flow_kg_s",
        "share",
        "total_pressure_pa",
        "mass_balance_error",
    ] {
        codes.push(format!("figure.{id}"));
    }
    for reason in ["stable", "oscillating", "limit_reached"] {
        for fluctuation in [0.5, 3.0, 10.0] {
            for verdict in ["good", "acceptable", "poor"] {
                for y in [1.0, 10.0, 100.0] {
                    let r = reliability_wind(reason, fluctuation, verdict, Some(y));
                    codes.extend(r.checks.into_iter().map(|c| c.code));
                }
                for balance in [0.001, 0.007, 0.05] {
                    let r = reliability_internal(reason, fluctuation, verdict, balance);
                    codes.extend(r.checks.into_iter().map(|c| c.code));
                }
            }
        }
    }
    codes.sort();
    codes.dedup();
    for code in &codes {
        for lang in [Language::En, Language::De] {
            let m = message(code, lang).unwrap_or_else(|| panic!("{code} has no {lang:?} text"));
            assert!(!m.text.is_empty() && !m.fix.is_empty(), "{code} {lang:?}");
        }
    }
    assert!(codes.len() >= 30, "{codes:?}");
}
