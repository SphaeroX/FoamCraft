// SPDX-License-Identifier: GPL-3.0-or-later
//! Key figures of a run with units and a reliability light (the content of `results.summary`).
//! The schema is stable: the MCP server and the report read it.

use foamcraft_core::error::FoamError;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Version of the summary schema.
pub const SUMMARY_SCHEMA: u32 = 1;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Figure {
    /// Stable id, for example `drag_n`; its explanation is the catalog entry `figure.<id>`.
    pub id: String,
    pub value: f64,
    pub unit: String,
    /// Part (face group) or opening key; `None` for the whole model.
    pub part: Option<String>,
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum Level {
    Green,
    Yellow,
    Red,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Check {
    /// Catalog code with the reason, for example `reliability.converged`.
    pub code: String,
    pub level: Level,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Reliability {
    /// The worst check.
    pub level: Level,
    pub checks: Vec<Check>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Summary {
    pub schema: u32,
    pub job_id: String,
    /// `wind_tunnel` or `internal_flow`.
    pub kind: String,
    pub figures: Vec<Figure>,
    pub reliability: Reliability,
}

/// Drag area Cd · A, m².
pub fn drag_area(cd: f64, area: f64) -> f64 {
    cd * area
}

/// Power to overcome the drag, W.
pub fn power(drag_n: f64, speed: f64) -> f64 {
    drag_n * speed
}

/// Loss coefficient zeta = Δp_total / (½ ρ v²).
pub fn loss_coefficient(loss_pa: f64, density: f64, velocity: f64) -> f64 {
    loss_pa / (0.5 * density * velocity * velocity).max(1e-30)
}

fn check(code: &str, level: Level) -> Check {
    Check {
        code: code.into(),
        level,
    }
}

fn convergence(reason: &str, fluctuation: f64) -> Check {
    match reason {
        "stable" => check("reliability.converged", Level::Green),
        "oscillating" if fluctuation < 5.0 => check("reliability.oscillating", Level::Yellow),
        "oscillating" => check("reliability.oscillating_strongly", Level::Red),
        _ if fluctuation < 2.0 => check("reliability.not_settled", Level::Yellow),
        _ => check("reliability.not_converged", Level::Red),
    }
}

fn mesh(verdict: &str) -> Check {
    match verdict {
        "good" => check("reliability.mesh_good", Level::Green),
        "acceptable" => check("reliability.mesh_acceptable", Level::Yellow),
        _ => check("reliability.mesh_poor", Level::Red),
    }
}

fn finish(checks: Vec<Check>) -> Reliability {
    Reliability {
        level: checks.iter().map(|c| c.level).max().unwrap_or(Level::Green),
        checks,
    }
}

/// Wind tunnel: convergence, mesh quality and the average y+ of the model.
pub fn reliability_wind(
    reason: &str,
    fluctuation: f64,
    verdict: &str,
    y_plus: Option<f64>,
) -> Reliability {
    let mut checks = vec![convergence(reason, fluctuation), mesh(verdict)];
    if let Some(y) = y_plus {
        checks.push(if (30.0..=300.0).contains(&y) {
            check("reliability.y_plus_ok", Level::Green)
        } else if (5.0..=1000.0).contains(&y) {
            check("reliability.y_plus_off", Level::Yellow)
        } else {
            check("reliability.y_plus_far_off", Level::Red)
        });
    }
    finish(checks)
}

/// Internal flow: convergence, mesh quality and the mass balance.
pub fn reliability_internal(
    reason: &str,
    fluctuation: f64,
    verdict: &str,
    balance: f64,
) -> Reliability {
    let balance = if balance < 0.005 {
        check("reliability.balance_ok", Level::Green)
    } else if balance < 0.01 {
        check("reliability.balance_off", Level::Yellow)
    } else {
        check("reliability.balance_bad", Level::Red)
    };
    finish(vec![
        convergence(reason, fluctuation),
        mesh(verdict),
        balance,
    ])
}

fn f(v: &Value, path: &[&str]) -> Option<f64> {
    path.iter().try_fold(v, |v, k| v.get(*k))?.as_f64()
}

fn s<'a>(v: &'a Value, path: &[&str]) -> Option<&'a str> {
    path.iter().try_fold(v, |v, k| v.get(*k))?.as_str()
}

fn bad(what: &str) -> FoamError {
    FoamError::new("results.read_failed", format!("result without {what}"))
}

/// The summary of a stored result (wind tunnel or internal flow).
pub fn summarize(result: &Value) -> Result<Summary, FoamError> {
    let job_id = s(result, &["job_id"])
        .ok_or_else(|| bad("job_id"))?
        .to_string();
    let reason = s(result, &["reason"]).unwrap_or("limit_reached");
    let fluctuation = f(result, &["fluctuation_percent"]).unwrap_or(0.0);
    let verdict = s(result, &["mesh", "verdict"]).unwrap_or("acceptable");
    let mut figures = Vec::new();
    let mut push = |id: &str, value: Option<f64>, unit: &str, part: Option<&str>| {
        if let Some(value) = value {
            figures.push(Figure {
                id: id.into(),
                value,
                unit: unit.into(),
                part: part.map(String::from),
            });
        }
    };
    if s(result, &["kind"]) == Some("internal_flow") {
        let loss = f(result, &["pressure_loss_pa"]);
        push("pressure_loss_pa", loss, "Pa", None);
        push(
            "static_pressure_loss_pa",
            f(result, &["static_pressure_loss_pa"]),
            "Pa",
            None,
        );
        let openings = result["openings"].as_array().cloned().unwrap_or_default();
        let inlet_velocity = openings
            .iter()
            .find(|o| o["role"] == "inlet")
            .and_then(|o| o["velocity_m_s"].as_f64());
        let medium: foamcraft_core::project::MediumSpec =
            serde_json::from_value(result["scenario"]["medium"].clone())
                .map_err(|_| bad("medium"))?;
        let density = crate::medium_properties(&medium).density;
        push(
            "loss_coefficient",
            loss.zip(inlet_velocity)
                .map(|(l, v)| loss_coefficient(l, density, v)),
            "-",
            None,
        );
        for o in &openings {
            let key = o["key"].as_str();
            push("flow_m3_s", o["flow_m3_s"].as_f64(), "m³/s", key);
            push("mass_flow_kg_s", o["mass_flow_kg_s"].as_f64(), "kg/s", key);
            push("share", o["share"].as_f64().map(|v| v * 100.0), "%", key);
            push(
                "total_pressure_pa",
                o["total_pressure_pa"].as_f64(),
                "Pa",
                key,
            );
        }
        let balance = f(result, &["mass_balance_error"]).unwrap_or(1.0);
        push("mass_balance_error", Some(balance * 100.0), "%", None);
        return Ok(Summary {
            schema: SUMMARY_SCHEMA,
            job_id,
            kind: "internal_flow".into(),
            figures,
            reliability: reliability_internal(reason, fluctuation, verdict, balance),
        });
    }
    let area = f(result, &["reference", "area_m2"]);
    let speed = f(result, &["reference", "speed_m_s"]);
    let cd = f(result, &["coefficients", "cd"]);
    let drag = f(result, &["forces", "drag_n"]);
    for id in ["cd", "cl", "cs", "cm_pitch", "cm_roll", "cm_yaw"] {
        push(id, f(result, &["coefficients", id]), "-", None);
    }
    for id in ["drag_n", "lift_n", "side_n"] {
        push(id, f(result, &["forces", id]), "N", None);
    }
    push(
        "drag_pressure_n",
        f(result, &["drag_split", "pressure_n"]),
        "N",
        None,
    );
    push(
        "drag_viscous_n",
        f(result, &["drag_split", "viscous_n"]),
        "N",
        None,
    );
    push(
        "drag_area_m2",
        cd.zip(area).map(|(c, a)| drag_area(c, a)),
        "m²",
        None,
    );
    push(
        "power_w",
        drag.zip(speed).map(|(d, u)| power(d, u)),
        "W",
        None,
    );
    push("reference_area_m2", area, "m²", None);
    for part in result["parts"].as_array().cloned().unwrap_or_default() {
        let name = part["name"].as_str();
        for id in ["drag_n", "lift_n", "side_n"] {
            push(id, part["forces"][id].as_f64(), "N", name);
        }
        push("cd", part["coefficients"]["cd"].as_f64(), "-", name);
        push(
            "drag_pressure_n",
            part["drag_split"]["pressure_n"].as_f64(),
            "N",
            name,
        );
        push(
            "drag_viscous_n",
            part["drag_split"]["viscous_n"].as_f64(),
            "N",
            name,
        );
    }
    let y_plus = result["y_plus"].as_array().and_then(|ys| {
        let v: Vec<f64> = ys.iter().filter_map(|y| y["average"].as_f64()).collect();
        (!v.is_empty()).then(|| v.iter().sum::<f64>() / v.len() as f64)
    });
    push("y_plus_average", y_plus, "-", None);
    Ok(Summary {
        schema: SUMMARY_SCHEMA,
        job_id,
        kind: "wind_tunnel".into(),
        figures,
        reliability: reliability_wind(reason, fluctuation, verdict, y_plus),
    })
}
