// SPDX-License-Identifier: GPL-3.0-or-later
//! The MCP tools without a window: list and schemas, a whole wind tunnel run against the fake
//! container engine, the path sandbox, rendering, the guide, and the bridge to a running app.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use foamcraft_mcp::{Output, Server};
use foamcraft_runtime::fake::FakeEngine;
use foamcraft_runtime::{CmdOutput, DockerRunner};
use serde_json::{Value, json};

const GIB: u64 = 1024 * 1024 * 1024;

fn tail(text: &str) -> CmdOutput {
    CmdOutput::ok(&json!({"text": text, "next_offset": text.len(), "exit": 0}).to_string())
}

fn settling_log(n: usize) -> String {
    use std::fmt::Write as _;
    let mut log = String::new();
    for i in 1..=n {
        let cd = 1.05 + 0.4 * (-(i as f64) / 10.0).exp();
        let _ = write!(
            log,
            "Time = {i}\n\nGAMG:  Solving for p, Initial residual = {r}, Final residual = 0.001, No Iterations 2\n\
             ExecutionTime = {i}.0 s  ClockTime = {i} s\n\nforceCoeffs forces write:\n    Coefficient\tTotal\tPressure\tViscous\tInternal\n\
             \x20   Cd:\t{cd}\t{p}\t{v}\t0\n    Cl:\t0.2\t0.2\t0\t0\n    Cs:\t0\t0\t0\t0\n\n    writing force and moment coefficient files.\n\n",
            r = 0.5 / i as f64,
            p = cd * 0.9,
            v = cd * 0.1,
        );
    }
    log.push_str("yPlus yPlus write:\n    writing field yPlus\n    patch model_body y+ : min = 3, max = 90, average = 45\n\nEnd\n");
    log
}

fn engine() -> FakeEngine {
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
        .on(&agent_tail("solve"), tail(&settling_log(400)))
        .on(&["exec", "cid", "sh"], CmdOutput::ok(""))
        .on(&["exec", "cid", "touch"], CmdOutput::ok(""))
        .on(&["rm"], CmdOutput::ok(""))
}

fn base(name: &str) -> PathBuf {
    let b = std::env::temp_dir().join(format!("fc-mcp-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&b);
    std::fs::create_dir_all(b.join("work")).unwrap();
    b
}

fn server(dir: &Path) -> Server {
    let registry = foamcraft_api::registry_with(Arc::new(DockerRunner::with_host(engine(), true)));
    Server::local(registry, dir.join("data"), vec![dir.join("work")])
}

fn json_of(out: &[Output]) -> Value {
    match &out[0] {
        Output::Json(v) => v.clone(),
        Output::Png(_) => panic!("expected JSON"),
    }
}

fn call(s: &Server, tool: &str, args: Value) -> Value {
    json_of(
        &s.call(tool, args.clone())
            .unwrap_or_else(|e| panic!("{tool} {args}: {e}")),
    )
}

#[test]
fn every_tool_has_a_description_and_an_object_schema() {
    let dir = base("list");
    let s = server(&dir);
    let tools = s.tools();
    let names: Vec<&str> = tools.iter().map(|t| t.name.as_str()).collect();
    for want in [
        "project_new",
        "project_open",
        "geometry_import",
        "geometry_info",
        "surface_select_at",
        "geometry_cap_opening",
        "scenario_set_wind_tunnel",
        "scenario_set_internal_flow",
        "run_start",
        "run_status",
        "run_wait",
        "run_cancel",
        "results_summary",
        "results_probe",
        "results_compare",
        "view_render",
        "commands_list",
        "command_describe",
        "command_run",
    ] {
        assert!(names.contains(&want), "{want} missing in {names:?}");
    }
    let mut unique = names.clone();
    unique.sort();
    unique.dedup();
    assert_eq!(unique.len(), names.len());
    for t in &tools {
        assert!(t.description.len() > 20, "{}", t.name);
        assert_eq!(t.input_schema["type"], "object", "{}", t.name);
        assert!(
            t.name
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '_'),
            "{}",
            t.name
        );
    }
    // Long runs return at once: run_start has no `wait`.
    let start = tools.iter().find(|t| t.name == "run_start").unwrap();
    assert!(start.input_schema["properties"].get("wait").is_none());
    assert!(!s.instructions().is_empty());
    assert!(s.guide().contains("plausib"));
}

#[test]
fn a_whole_wind_tunnel_run_without_a_window() {
    let dir = base("e2e");
    let s = server(&dir);
    let project = dir.join("work/helmet");
    call(
        &s,
        "project_new",
        json!({"path": project, "name": "Helmet"}),
    );
    let model = dir.join("work/cube.stl");
    std::fs::copy(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/data/geometry/cube_binary.stl"),
        &model,
    )
    .unwrap();
    call(&s, "geometry_import", json!({"path": model, "unit": "m"}));
    call(
        &s,
        "scenario_set_wind_tunnel",
        json!({"speed_m_s": 20.0, "direction_deg": 0.0, "ground": "none",
               "medium": {"kind": "air", "temperature_c": 20.0}, "quality": "draft"}),
    );
    let started = call(&s, "run_start", json!({"cores": 1}));
    let job = started["job_id"].as_str().unwrap().to_string();
    let done = call(&s, "run_wait", json!({"job_id": job, "timeout_s": 120}));
    assert_eq!(done["finished"], true, "{done}");
    assert_eq!(done["state"]["state"], "succeeded", "{done}");
    let summary = call(&s, "results_summary", json!({}));
    assert_eq!(summary["schema"], 1);
    let cd = summary["figures"]
        .as_array()
        .unwrap()
        .iter()
        .find(|f| f["id"] == "cd")
        .unwrap();
    assert!((cd["value"].as_f64().unwrap() - 1.05).abs() < 0.01);
    // Compact answers: the summary carries the English text only.
    assert!(cd.get("de").is_none());
    assert!(!cd["text"].as_str().unwrap().is_empty());
}

#[test]
fn waiting_ends_at_the_time_limit() {
    let dir = base("wait");
    let s = server(&dir);
    call(
        &s,
        "project_new",
        json!({"path": dir.join("work/p"), "name": "P"}),
    );
    let err = s
        .call("run_wait", json!({"job_id": "job-unknown", "timeout_s": 1}))
        .unwrap_err();
    assert_eq!(err.code, "job.not_found");
}

#[test]
fn paths_outside_the_roots_are_refused() {
    let dir = base("sandbox");
    let s = server(&dir);
    let outside = std::env::temp_dir().join("fc-mcp-outside");
    for (tool, args) in [
        ("project_new", json!({"path": outside, "name": "X"})),
        (
            "command_run",
            json!({"name": "project.open", "params": {"path": outside}}),
        ),
        (
            "project_new",
            json!({"path": dir.join("work/../escape"), "name": "X"}),
        ),
        (
            "project_new",
            json!({"path": "relative/folder", "name": "X"}),
        ),
    ] {
        let err = s.call(tool, args.clone()).unwrap_err();
        assert_eq!(err.code, "mcp.path_outside_roots", "{tool} {args}");
    }
    assert!(!outside.exists());
    // Inside the roots it works.
    call(
        &s,
        "project_new",
        json!({"path": dir.join("work/ok"), "name": "Ok"}),
    );
}

#[test]
fn the_render_tool_answers_with_a_png_of_the_requested_size() {
    let dir = base("render");
    let s = server(&dir);
    call(
        &s,
        "project_new",
        json!({"path": dir.join("work/r"), "name": "R"}),
    );
    match s.call("view_render", json!({"width": 320, "height": 200})) {
        Err(e) if e.code == "view.no_gpu" => eprintln!("SKIPPED: no GPU adapter available"),
        Err(e) => panic!("{e}"),
        Ok(out) => {
            let Output::Png(bytes) = &out[0] else {
                panic!("expected an image")
            };
            assert_eq!(&bytes[1..4], b"PNG");
            let w = u32::from_be_bytes(bytes[16..20].try_into().unwrap());
            let h = u32::from_be_bytes(bytes[20..24].try_into().unwrap());
            assert_eq!((w, h), (320, 200));
        }
    }
}

#[test]
fn connected_to_the_app_a_tool_changes_the_app_state() {
    let dir = base("app");
    let session = Arc::new(Mutex::new(foamcraft_core::command::Session::new(
        dir.join("data"),
    )));
    let _bridge = foamcraft_api::bridge::serve(
        Arc::new(foamcraft_api::registry()),
        session.clone(),
        &dir.join("data"),
    )
    .unwrap();
    let s = Server::app(&dir.join("data"), vec![dir.join("work")]).unwrap();
    call(
        &s,
        "project_new",
        json!({"path": dir.join("work/a"), "name": "Shown"}),
    );
    assert_eq!(
        session.lock().unwrap().project_ref().unwrap().manifest.name,
        "Shown"
    );
    let info = call(&s, "project_info", json!({}));
    assert_eq!(info["name"], "Shown");
}

#[test]
fn export_tools_answer_at_once_and_respect_the_roots() {
    let dir = base("export");
    let s = server(&dir);
    let tools = s.tools();
    for name in [
        "export_image",
        "export_video",
        "export_status",
        "export_cancel",
    ] {
        assert!(tools.iter().any(|t| t.name == name), "{name}");
    }
    let video = tools.iter().find(|t| t.name == "export_video").unwrap();
    assert!(video.input_schema["properties"].get("wait").is_none());
    call(
        &s,
        "project_new",
        json!({"path": dir.join("work/x"), "name": "X"}),
    );
    let outside = std::env::temp_dir().join("fc-mcp-outside.png");
    let err = s
        .call("export_image", json!({"path": outside}))
        .unwrap_err();
    assert_eq!(err.code, "mcp.path_outside_roots");
}

#[test]
fn a_report_can_be_requested_and_its_paths_are_checked() {
    let dir = base("report");
    let s = server(&dir);
    assert!(s.tools().iter().any(|t| t.name == "export_report"));
    call(
        &s,
        "project_new",
        json!({"path": dir.join("work/r"), "name": "R"}),
    );
    let err = s
        .call(
            "export_report",
            json!({"path": dir.join("work/r/report"), "logo_path": std::env::temp_dir().join("logo.png")}),
        )
        .unwrap_err();
    assert_eq!(err.code, "mcp.path_outside_roots");
    // Inside the roots the command runs (and fails only because there is no result yet).
    let err = s
        .call("export_report", json!({"path": dir.join("work/r/report")}))
        .unwrap_err();
    assert_eq!(err.code, "run.no_result");
}

#[test]
fn projects_without_a_path_are_created_inside_the_first_root() {
    let dir = base("default-path");
    let s = server(&dir);
    let work = dir.join("work");
    let a = call(&s, "project_new", json!({"name": "Wing"}));
    assert_eq!(a["path"], json!(work.join("Wing")));
    // A sample through the generic command tool: also inside the root, never in the user's
    // default projects folder.
    let b = call(
        &s,
        "command_run",
        json!({"name": "samples.create", "params": {"sample": "wind_tunnel"}}),
    );
    let path = PathBuf::from(b["path"].as_str().unwrap());
    assert!(path.starts_with(&work), "{}", path.display());
    assert!(path.join("foamcraft.json").is_file());
}

#[test]
fn the_render_tool_takes_a_named_view() {
    let dir = base("render-view");
    let s = server(&dir);
    call(
        &s,
        "project_new",
        json!({"path": dir.join("work/v"), "name": "V"}),
    );
    let tools = s.tools();
    let render = tools.iter().find(|t| t.name == "view_render").unwrap();
    assert!(
        render.input_schema["properties"]["view"]["enum"]
            .as_array()
            .unwrap()
            .contains(&json!("front"))
    );
    let err = s
        .call(
            "view_render",
            json!({"width": 64, "height": 64, "view": "nowhere"}),
        )
        .unwrap_err();
    assert_eq!(err.code, "view.unknown_view");
    match s.call(
        "view_render",
        json!({"width": 64, "height": 64, "view": "top"}),
    ) {
        Err(e) if e.code == "view.no_gpu" => eprintln!("SKIPPED: no GPU adapter available"),
        Err(e) => panic!("{e}"),
        Ok(out) => assert!(matches!(&out[0], Output::Png(_))),
    }
}

#[test]
fn models_are_moved_and_turned_with_the_handle_commands() {
    let dir = base("move");
    let s = server(&dir);
    call(
        &s,
        "project_new",
        json!({"path": dir.join("work/m"), "name": "M"}),
    );
    let model = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/data/geometry/cube_binary.stl");
    let work = dir.join("work");
    std::fs::copy(&model, work.join("cube.stl")).unwrap();
    call(
        &s,
        "geometry_import",
        json!({"path": work.join("cube.stl"), "unit": "mm"}),
    );
    let moved = call(
        &s,
        "geometry_move_by",
        json!({"id": "model-1", "delta": [0.0, 0.0, 0.05]}),
    );
    assert_eq!(moved["translation"], json!([0.0, 0.0, 0.05]));
    let turned = call(
        &s,
        "geometry_rotate_about",
        json!({"id": "model-1", "axis": "z", "angle_deg": 90.0}),
    );
    assert!((turned["rotation_deg"][0].as_f64().unwrap() - 90.0).abs() < 1e-9);
    let pivots = call(&s, "geometry_pivot", json!({"id": "model-1"}));
    assert!(pivots["centroid"].is_array());
}

#[test]
fn the_ride_tools_are_listed_and_call_the_ride_commands() {
    let dir = base("ride");
    let s = server(&dir);
    let tools = s.tools();
    for want in ["ride_suggest", "ride_start", "ride_telemetry", "ride_stop"] {
        assert!(tools.iter().any(|t| t.name == want), "{want} missing");
    }
    let render = tools.iter().find(|t| t.name == "view_render").unwrap();
    assert_eq!(
        render.input_schema["properties"]["ride_t"]["type"],
        "number"
    );
    call(
        &s,
        "project_new",
        json!({"path": dir.join("work/r"), "name": "R"}),
    );
    // Without a result there is nothing to ride; without a ride nothing to read.
    assert_eq!(
        s.call("ride_suggest", json!({})).unwrap_err().code,
        "results.none"
    );
    assert_eq!(
        s.call("ride_telemetry", json!({})).unwrap_err().code,
        "ride.none"
    );
    assert_eq!(call(&s, "ride_stop", json!({}))["stopped"], true);
}

#[test]
fn the_surface_flow_tool_is_listed_and_needs_a_result() {
    let dir = base("surface-flow");
    let s = server(&dir);
    let tools = s.tools();
    assert!(tools.iter().any(|t| t.name == "view_set_surface_flow"));
    call(
        &s,
        "project_new",
        json!({"path": dir.join("work/f"), "name": "F"}),
    );
    assert_eq!(
        s.call("view_set_surface_flow", json!({"lines": {"on": true}}))
            .unwrap_err()
            .code,
        "results.none"
    );
}

#[test]
fn the_heavy_particle_tools_are_listed_and_answer() {
    let dir = base("heavy");
    let s = server(&dir);
    let tools = s.tools();
    for want in [
        "view_set_particles",
        "particles_materials",
        "results_impact",
        "results_impact_status",
    ] {
        assert!(tools.iter().any(|t| t.name == want), "{want} missing");
    }
    let list = call(&s, "particles_materials", json!({}));
    assert!(list.as_array().unwrap().iter().any(|m| m["id"] == "rain"));
    assert_eq!(
        s.call("results_impact_status", json!({"impact_id": "nothing"}))
            .unwrap_err()
            .code,
        "impact.not_found"
    );
}
