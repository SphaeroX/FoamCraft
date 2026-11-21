// SPDX-License-Identifier: GPL-3.0-or-later
//! Run commands against the fake container engine.

use std::sync::Arc;
use std::sync::mpsc;
use std::time::Duration;

use foamcraft_api::registry_with;
use foamcraft_core::command::Session;
use foamcraft_runtime::fake::FakeEngine;
use foamcraft_runtime::{CmdOutput, DockerRunner};
use serde_json::json;

fn temp(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("fc-api-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

fn engine() -> FakeEngine {
    let status = r#"{"agent_version":"0.1.0","openfoam":{"api":"2606","patch":"0","build":"x"},"cpus":4,"memory_bytes":1024,"gpu":{"available":false,"detail":""}}"#;
    FakeEngine::new()
        .on(&["volume"], CmdOutput::ok("v\n"))
        .on(&["run"], CmdOutput::ok("cid\n"))
        .on(&["cp"], CmdOutput::ok(""))
        .on(&["exec", "-d"], CmdOutput::ok(""))
        .on(
            &["exec", "cid", "foamcraft-agent", "status"],
            CmdOutput::ok(status),
        )
        .on(
            &["exec", "cid", "foamcraft-agent", "tail"],
            CmdOutput::ok(r#"{"text":"End\n","next_offset":4,"exit":0}"#),
        )
        .on(&["rm"], CmdOutput::ok(""))
}

#[test]
fn run_commands_are_registered_with_schemas() {
    let reg = registry_with(Arc::new(DockerRunner::with_host(engine(), true)));
    for name in [
        "run.start",
        "run.cancel",
        "run.status",
        "project.new",
        "history.undo",
        "geometry.import",
        "geometry.set_transform",
    ] {
        let spec = reg
            .describe(name)
            .unwrap_or_else(|| panic!("{name} missing"));
        assert!(spec.params.is_object());
    }
}

#[test]
fn run_start_needs_a_project() {
    let reg = registry_with(Arc::new(DockerRunner::with_host(engine(), true)));
    let mut s = Session::new(temp("noproj").join("data"));
    let err = reg.execute(&mut s, "run.start", json!({})).unwrap_err();
    assert_eq!(err.code, "project.none_open");
}

#[test]
fn run_start_runs_in_the_background_and_reports_events_and_status() {
    let reg = registry_with(Arc::new(DockerRunner::with_host(engine(), true)));
    let base = temp("start");
    let mut s = Session::new(base.join("data"));
    reg.execute(
        &mut s,
        "project.new",
        json!({"path": base.join("p"), "name": "P"}),
    )
    .unwrap();
    let (tx, rx) = mpsc::channel();
    s.events().subscribe(move |e| {
        if e.name == "run.finished" {
            let _ = tx.send(e.payload.clone());
        }
    });
    let started = reg
        .execute(&mut s, "run.start", json!({"iterations": 10}))
        .unwrap();
    let job_id = started["job_id"].as_str().unwrap().to_string();
    let finished = rx
        .recv_timeout(Duration::from_secs(20))
        .expect("run.finished event");
    assert_eq!(finished["job_id"], json!(job_id));
    assert_eq!(finished["state"]["state"], json!("succeeded"));

    let info = reg.execute(&mut s, "project.info", json!({})).unwrap();
    assert_eq!(info["runs"], json!([job_id]));
    let status = reg
        .execute(&mut s, "run.status", json!({"job_id": job_id}))
        .unwrap();
    assert_eq!(status["state"]["state"], json!("succeeded"));
    assert!(base.join("p/runs").join(&job_id).join("job.json").is_file());
}

#[test]
fn status_and_cancel_of_unknown_jobs_report_not_found() {
    let reg = registry_with(Arc::new(DockerRunner::with_host(engine(), true)));
    let base = temp("unknown");
    let mut s = Session::new(base.join("data"));
    reg.execute(
        &mut s,
        "project.new",
        json!({"path": base.join("p"), "name": "P"}),
    )
    .unwrap();
    for name in ["run.status", "run.cancel"] {
        let err = reg
            .execute(&mut s, name, json!({"job_id": "nope"}))
            .unwrap_err();
        assert_eq!(err.code, "job.not_found", "{name}");
    }
}
