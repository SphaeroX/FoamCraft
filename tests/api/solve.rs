// SPDX-License-Identifier: GPL-3.0-or-later
//! run.start on a wind tunnel project against the fake container engine: meshing, solving with
//! the convergence automation, the result object, and one restart after a divergence.

use std::fmt::Write as _;
use std::path::PathBuf;
use std::sync::Arc;

use foamcraft_api::registry_with;
use foamcraft_core::command::{Registry, Session};
use foamcraft_runtime::fake::FakeEngine;
use foamcraft_runtime::{CmdOutput, DockerRunner};
use serde_json::json;

const GIB: u64 = 1024 * 1024 * 1024;

fn tail(text: &str) -> CmdOutput {
    CmdOutput::ok(&json!({"text": text, "next_offset": text.len(), "exit": 0}).to_string())
}

/// Solver log with `n` iterations whose drag coefficient settles at 1.05.
fn settling_log(n: usize) -> String {
    let mut log = String::new();
    for i in 1..=n {
        let cd = 1.05 + 0.4 * (-(i as f64) / 10.0).exp();
        let _ = write!(
            log,
            "Time = {i}\n\nGAMG:  Solving for p, Initial residual = {r}, Final residual = 0.001, No Iterations 2\n\
             ExecutionTime = {i}.0 s  ClockTime = {i} s\n\nforceCoeffs forces write:\n    Coefficient\tTotal\tPressure\tViscous\tInternal\n\
             \x20   Cd:\t{cd}\t{cd}\t0\t0\n    Cl:\t0.2\t0.2\t0\t0\n    Cs:\t0\t0\t0\t0\n\n    writing force and moment coefficient files.\n\n",
            r = 0.5 / i as f64,
        );
    }
    log.push_str("yPlus yPlus write:\n    writing field yPlus\n    patch ground y+ : min = 1, max = 400, average = 80\n    patch model_body y+ : min = 3, max = 90, average = 45\n\nEnd\n");
    log
}

fn engine(solve_log: &str) -> FakeEngine {
    let info = format!(
        r#"{{"ServerVersion":"29","OperatingSystem":"Docker Desktop","KernelVersion":"6.6","NCPU":8,"MemTotal":{}}}"#,
        16 * GIB
    );
    let status = r#"{"agent_version":"0.1.0","openfoam":{"api":"2606","patch":"0","build":"x"},"cpus":1,"memory_bytes":1024,"gpu":{"available":false,"detail":""}}"#;
    let agent_tail = |stage: &'static str| -> Vec<&'static str> {
        vec![
            "exec",
            "cid",
            "foamcraft-agent",
            "tail",
            "/work/case",
            stage,
        ]
    };
    FakeEngine::new()
        .on(&["info"], CmdOutput::ok(&info))
        .on(&["volume"], CmdOutput::ok("v\n"))
        .on(&["run"], CmdOutput::ok("cid\n"))
        .on(&["cp"], CmdOutput::ok(""))
        .on(&["exec", "-d"], CmdOutput::ok(""))
        .on(
            &["exec", "cid", "foamcraft-agent", "status"],
            CmdOutput::ok(status),
        )
        .on(&agent_tail("background"), tail("End\n"))
        .on(&agent_tail("mesh"), tail("End\n"))
        .on(
            &agent_tail("check"),
            tail("    cells:            1000\n    Max skewness = 1.0 OK.\nMesh OK.\nEnd\n"),
        )
        .on(&agent_tail("solve"), tail(solve_log))
        .on(&["exec", "cid", "sh"], CmdOutput::ok(""))
        .on(&["exec", "cid", "touch"], CmdOutput::ok(""))
        .on(&["rm"], CmdOutput::ok(""))
}

fn project(name: &str, solve_log: &str) -> (Registry, Session, PathBuf) {
    let reg = registry_with(Arc::new(DockerRunner::with_host(engine(solve_log), true)));
    let base = std::env::temp_dir().join(format!("fc-api-solve-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&base);
    let mut s = Session::new(base.join("data"));
    reg.execute(
        &mut s,
        "project.new",
        json!({"path": base.join("p"), "name": "S"}),
    )
    .unwrap();
    let model =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/data/geometry/cube_binary.stl");
    reg.execute(
        &mut s,
        "geometry.import",
        json!({"path": model, "unit": "m"}),
    )
    .unwrap();
    reg.execute(
        &mut s,
        "scenario.set_wind_tunnel",
        json!({"speed_m_s": 20.0, "direction_deg": 0.0, "ground": "none",
               "medium": {"kind": "air", "temperature_c": 20.0}, "quality": "draft"}),
    )
    .unwrap();
    (reg, s, base.join("p"))
}

#[test]
fn run_start_meshes_solves_and_returns_the_result() {
    let (reg, mut s, root) = project("ok", &settling_log(400));
    let out = reg
        .execute(&mut s, "run.start", json!({"wait": true, "cores": 1}))
        .unwrap();
    let result = &out["result"];
    assert_eq!(result["reason"], "stable", "{result}");
    // Draft: 300 expected iterations, window 60; the curve settles within the first windows.
    let decided = result["decided_at_iteration"].as_u64().unwrap();
    assert!((120..400).contains(&decided), "{decided}");
    assert!((result["coefficients"]["cd"].as_f64().unwrap() - 1.05).abs() < 0.01);
    assert!((result["coefficients"]["cl"].as_f64().unwrap() - 0.2).abs() < 1e-9);
    // The cube is 20 m (unit m): area 400 m², q = 0.5 rho 20² with air at 20 °C.
    let q = result["reference"]["dynamic_pressure_pa"].as_f64().unwrap();
    let area = result["reference"]["area_m2"].as_f64().unwrap();
    assert!((area - 400.0).abs() < 4.0, "{area}");
    let drag = result["forces"]["drag_n"].as_f64().unwrap();
    let cd = result["coefficients"]["cd"].as_f64().unwrap();
    assert!((drag - cd * q * area).abs() < 1e-6 * drag.abs());
    assert_eq!(result["y_plus"].as_array().unwrap().len(), 1);
    assert_eq!(result["hints"], json!(["run.stable"]));
    assert_eq!(result["max_iterations"], 900);
    assert!(!result["history"]["cd"].as_array().unwrap().is_empty());
    assert!(result["mesh"]["cells"] == 1000);
    // The solver was asked to stop, and the result is stored.
    let job = result["job_id"].as_str().unwrap();
    assert!(root.join("results").join(format!("{job}.json")).is_file());
    let stored = reg.execute(&mut s, "run.result", json!({})).unwrap();
    assert_eq!(stored["job_id"], job);
    assert_eq!(stored["reason"], result["reason"]);
    let stored_cd = stored["coefficients"]["cd"].as_f64().unwrap();
    assert!((stored_cd - cd).abs() < 1e-12);
}

#[test]
fn a_diverged_run_restarts_once_with_cautious_numerics() {
    let diverging = "Time = 1\nsmoothSolver:  Solving for Ux, Initial residual = nan, Final residual = nan, No Iterations 1000\n";
    let (reg, mut s, root) = project("diverge", diverging);
    let err = reg
        .execute(&mut s, "run.start", json!({"wait": true, "cores": 1}))
        .unwrap_err();
    assert_eq!(err.code, "run.diverged");
    // The second attempt used first-order convection and stronger relaxation.
    let info = reg.execute(&mut s, "project.info", json!({})).unwrap();
    let job = info["runs"]
        .as_array()
        .unwrap()
        .last()
        .unwrap()
        .as_str()
        .unwrap()
        .to_string();
    let schemes =
        std::fs::read_to_string(root.join("runs").join(&job).join("case/system/fvSchemes"))
            .unwrap();
    assert!(schemes.contains("div(phi,U)      bounded Gauss upwind;"));
    let record: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(root.join("runs").join(&job).join("job.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(record["state"]["state"], "failed");
    assert!(reg.execute(&mut s, "run.result", json!({})).is_err());
}

#[test]
fn a_running_job_can_be_followed_again_after_a_restart() {
    let (reg, mut s, root) = project("attach", &settling_log(400));
    let out = reg
        .execute(&mut s, "run.start", json!({"wait": true, "cores": 1}))
        .unwrap();
    let job = out["job_id"].as_str().unwrap().to_string();
    assert_eq!(
        reg.execute(&mut s, "run.active", json!({})).unwrap(),
        json!([])
    );
    // As if the app had ended while the solver was running.
    let store = foamcraft_core::job::JobStore::new(root.join("runs"));
    let mut record = store.load(&job).unwrap();
    record.state = foamcraft_core::job::JobState::Running(foamcraft_core::job::Stage::Solve);
    store.save(&record).unwrap();
    std::fs::remove_file(root.join("results").join(format!("{job}.json"))).unwrap();
    let active = reg.execute(&mut s, "run.active", json!({})).unwrap();
    assert_eq!(active[0]["job_id"], json!(job));
    let attached = reg
        .execute(&mut s, "run.attach", json!({"job_id": job, "wait": true}))
        .unwrap();
    assert_eq!(attached["result"]["reason"], "stable");
    assert!(root.join("results").join(format!("{job}.json")).is_file());
    assert_eq!(
        reg.execute(&mut s, "run.active", json!({})).unwrap(),
        json!([])
    );
}

#[test]
fn the_log_tail_and_message_texts_are_available_to_the_interface() {
    let (reg, mut s, _) = project("log", &settling_log(200));
    let out = reg
        .execute(&mut s, "run.start", json!({"wait": true, "cores": 1}))
        .unwrap();
    let log = reg
        .execute(
            &mut s,
            "run.log",
            json!({"job_id": out["job_id"], "lines": 5}),
        )
        .unwrap();
    let lines = log["lines"].as_array().unwrap();
    assert_eq!(lines.len(), 5);
    assert!(
        lines
            .last()
            .unwrap()
            .as_str()
            .unwrap()
            .starts_with("[solve]")
    );
    let texts = reg
        .execute(
            &mut s,
            "error.describe",
            json!({"codes": ["run.diverged", "run.oscillating"]}),
        )
        .unwrap();
    assert!(
        !texts["run.diverged"]["de"]["text"]
            .as_str()
            .unwrap()
            .is_empty()
    );
    assert!(
        !texts["run.oscillating"]["en"]["fix"]
            .as_str()
            .unwrap()
            .is_empty()
    );
    assert!(texts.get("nope.nothing").is_none());
}

#[test]
fn a_run_counts_as_done_only_when_its_result_is_stored() {
    let (reg, mut s, root) = project("busy", &settling_log(400));
    let job = reg
        .execute(&mut s, "run.start", json!({"cores": 1}))
        .unwrap()["job_id"]
        .as_str()
        .unwrap()
        .to_string();
    let start = std::time::Instant::now();
    loop {
        let st = reg
            .execute(&mut s, "run.status", json!({"job_id": job}))
            .unwrap();
        // While the run is evaluated and its fields extracted, it is still busy.
        if st["busy"] == false && st["state"]["state"] == "succeeded" {
            break;
        }
        assert!(st.get("busy").is_some(), "{st}");
        assert!(start.elapsed().as_secs() < 60, "{st}");
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    assert!(root.join("results").join(format!("{job}.json")).is_file());
}
