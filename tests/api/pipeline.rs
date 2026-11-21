// SPDX-License-Identifier: GPL-3.0-or-later
//! `run.start` as one background pipeline (mesh, then solve) against a scripted engine whose
//! meshing stage can be held: the command answers at once, other commands work meanwhile, the
//! meshing reports progress, can be cancelled and stores the mesh in the project.

use std::fmt::Write as _;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use foamcraft_api::registry_with;
use foamcraft_core::command::{Registry, Session};
use foamcraft_runtime::fake::FakeEngine;
use foamcraft_runtime::{CmdOutput, DockerRunner, Engine, LogLine};
use serde_json::{Value, json};

const GIB: u64 = 1024 * 1024 * 1024;

fn tail(text: &str) -> CmdOutput {
    CmdOutput::ok(&json!({"text": text, "next_offset": text.len(), "exit": 0}).to_string())
}

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
    log.push_str("yPlus yPlus write:\n    writing field yPlus\n    patch model_body y+ : min = 3, max = 90, average = 45\n\nEnd\n");
    log
}

/// Snappy output with the three phases, as far as the progress estimate needs it.
const SNAPPY: &str = "Refinement phase\n----------------\n\nSurface refinement iteration 0\n\
Shell refinement iteration 0\n\nMorphing phase\n--------------\n\nSnapping to features in 10 iterations ...\n\
Morph iteration 0\nMorph iteration 1\n\nShrinking and layer addition phase\n\nLayer addition iteration 0\n\n\
patch            faces        layers        overall thickness\n\
\x20                         target   mesh     [m]       [%]\n\
-----            -----    -----    ----     ---       ---\n\
model_body       100      3        3.0      0.001     100.0\n\nEnd\n";

/// A fake engine whose snappyHexMesh stage keeps running until the gate opens.
struct GatedEngine {
    inner: FakeEngine,
    open: Arc<AtomicBool>,
    calls: Arc<Mutex<Vec<Vec<String>>>>,
}

impl Engine for GatedEngine {
    fn run(&self, args: &[String], on_line: &mut dyn FnMut(LogLine)) -> std::io::Result<CmdOutput> {
        self.calls.lock().unwrap().push(args.to_vec());
        let is_mesh_tail = args.len() > 5 && args[3] == "tail" && args[5] == "mesh";
        if is_mesh_tail && !self.open.load(Ordering::SeqCst) {
            // Still running: the first poll shows the start of the log, later ones nothing new.
            let first = args.get(6).is_some_and(|o| o == "0");
            let text = if first { "Refinement phase\n" } else { "" };
            let offset: usize = args.get(6).and_then(|o| o.parse().ok()).unwrap_or(0);
            return Ok(CmdOutput::ok(
                &json!({"text": text, "next_offset": offset + text.len(), "exit": null})
                    .to_string(),
            ));
        }
        if is_mesh_tail {
            let offset: usize = args.get(6).and_then(|o| o.parse().ok()).unwrap_or(0);
            let rest = &SNAPPY[offset.min(SNAPPY.len())..];
            return Ok(CmdOutput::ok(
                &json!({"text": rest, "next_offset": SNAPPY.len(), "exit": 0}).to_string(),
            ));
        }
        self.inner.run(args, on_line)
    }
}

struct Fixture {
    reg: Registry,
    s: Session,
    root: PathBuf,
    open: Arc<AtomicBool>,
    calls: Arc<Mutex<Vec<Vec<String>>>>,
    events: Arc<Mutex<Vec<(String, Value)>>>,
}

fn fixture(name: &str, open: bool) -> Fixture {
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
    let inner = FakeEngine::new()
        .on(&["info"], CmdOutput::ok(&info))
        .on(&["volume"], CmdOutput::ok("v\n"))
        .on(&["run"], CmdOutput::ok("cid\n"))
        .on(&["cp"], CmdOutput::ok(""))
        .on(&["exec", "-d"], CmdOutput::ok(""))
        .on(
            &["exec", "cid", "foamcraft-agent", "status"],
            CmdOutput::ok(status),
        )
        .on(
            &agent_tail("background"),
            tail("Creating block mesh\nEnd\n"),
        )
        .on(
            &agent_tail("check"),
            tail("    cells:            1000\n    Max skewness = 1.0 OK.\nMesh OK.\nEnd\n"),
        )
        .on(&agent_tail("solve"), tail(&settling_log(400)))
        .on(&["exec", "cid", "sh"], CmdOutput::ok(""))
        .on(&["exec", "cid", "touch"], CmdOutput::ok(""))
        .on(&["rm"], CmdOutput::ok(""));
    let gate = Arc::new(AtomicBool::new(open));
    let calls = Arc::new(Mutex::new(Vec::new()));
    let engine = GatedEngine {
        inner,
        open: gate.clone(),
        calls: calls.clone(),
    };
    let reg = registry_with(Arc::new(DockerRunner::with_host(engine, true)));
    let base = std::env::temp_dir().join(format!("fc-api-pipe-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&base);
    let mut s = Session::new(base.join("data"));
    let events = Arc::new(Mutex::new(Vec::new()));
    let sink = events.clone();
    s.events().subscribe(move |e| {
        sink.lock()
            .unwrap()
            .push((e.name.clone(), e.payload.clone()));
    });
    reg.execute(
        &mut s,
        "project.new",
        json!({"path": base.join("p"), "name": "P"}),
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
    // No ground and draft quality: no boundary layers, so one meshing attempt is accepted.
    reg.execute(
        &mut s,
        "scenario.set_wind_tunnel",
        json!({"speed_m_s": 20.0, "direction_deg": 0.0, "ground": "none",
               "medium": {"kind": "air", "temperature_c": 20.0}, "quality": "draft"}),
    )
    .unwrap();
    Fixture {
        reg,
        s,
        root: base.join("p"),
        open: gate,
        calls,
        events,
    }
}

impl Fixture {
    fn run(&mut self, name: &str, params: Value) -> Value {
        self.reg
            .execute(&mut self.s, name, params)
            .unwrap_or_else(|e| panic!("{name}: {e}"))
    }

    fn events_named(&self, name: &str) -> Vec<Value> {
        self.events
            .lock()
            .unwrap()
            .iter()
            .filter(|(n, _)| n == name)
            .map(|(_, p)| p.clone())
            .collect()
    }

    /// Waits until the run is neither busy nor unfinished; returns its last status.
    fn wait_idle(&mut self, job: &str) -> Value {
        let start = Instant::now();
        loop {
            let st = self.run("run.status", json!({"job_id": job}));
            let state = st["state"]["state"].as_str().unwrap_or("").to_string();
            if st["busy"] == false && state != "pending" && state != "running" {
                return st;
            }
            assert!(start.elapsed() < Duration::from_secs(60), "{st}");
            std::thread::sleep(Duration::from_millis(20));
        }
    }

    fn wait_for_event(&self, name: &str) {
        let start = Instant::now();
        while self.events_named(name).is_empty() {
            assert!(start.elapsed() < Duration::from_secs(30), "no {name} event");
            std::thread::sleep(Duration::from_millis(10));
        }
    }
}

#[test]
fn run_start_answers_at_once_and_reports_the_meshing() {
    let mut f = fixture("answers", false);
    let begun = Instant::now();
    let started = f.run("run.start", json!({"cores": 1}));
    assert!(
        begun.elapsed() < Duration::from_secs(5),
        "blocked for {:?}",
        begun.elapsed()
    );
    let job = started["job_id"].as_str().unwrap().to_string();
    // The run is listed at once and busy; other commands are answered while the mesh is built.
    let info = f.run("project.info", json!({}));
    assert_eq!(info["runs"], json!([job]));
    let st = f.run("run.status", json!({"job_id": job}));
    assert_eq!(st["busy"], true, "{st}");
    f.wait_for_event("mesh.progress");
    let stages = f.events_named("mesh.stage");
    assert_eq!(stages[0]["run_id"], json!(job));
    assert_eq!(stages[0]["stage"], "background");
    assert_ne!(stages[0]["job_id"], json!(job), "the mesh has its own job");
    // The log of the run shows the meshing while it runs.
    let log = f.run("run.log", json!({"job_id": job, "lines": 50}));
    let text = log["lines"].to_string();
    assert!(text.contains("[background] Creating block mesh"), "{text}");
    assert_eq!(
        f.reg
            .execute(&mut f.s, "mesh.info", json!({}))
            .unwrap_err()
            .code,
        "mesh.none"
    );
    // Let the mesh finish: the run goes on to the solver and ends with a result.
    f.open.store(true, Ordering::SeqCst);
    let st = f.wait_idle(&job);
    assert_eq!(st["state"]["state"], "succeeded", "{st}");
    let mesh = f.run("mesh.info", json!({}));
    assert_eq!(mesh["cells"], 1000);
    let result = f.run("run.result", json!({"job_id": job}));
    assert_eq!(result["reason"], "stable");
    assert!(f.root.join("foamcraft.json").is_file());
    let saved = std::fs::read_to_string(f.root.join("foamcraft.json")).unwrap();
    assert!(
        saved.contains(mesh["job_id"].as_str().unwrap()),
        "mesh record saved"
    );
    // Progress went through the phases and never went backwards.
    let progress = f.events_named("mesh.progress");
    let fractions: Vec<f64> = progress
        .iter()
        .map(|p| p["fraction"].as_f64().unwrap())
        .collect();
    assert!(fractions.windows(2).all(|w| w[1] >= w[0]), "{fractions:?}");
    assert!(*fractions.last().unwrap() > 0.9, "{fractions:?}");
    let phases: Vec<&str> = progress
        .iter()
        .filter_map(|p| p["phase"].as_str())
        .collect();
    for phase in ["refine", "snap", "layers"] {
        assert!(phases.contains(&phase), "{phase} missing in {phases:?}");
    }
    assert!(progress.iter().all(|p| p["run_id"] == json!(job)));
    // The whole log: meshing first, then the solver.
    let log = f.run("run.log", json!({"job_id": job, "lines": 100000}));
    let lines: Vec<&str> = log["lines"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(Value::as_str)
        .collect();
    let first_mesh = lines.iter().position(|l| l.starts_with("[mesh]")).unwrap();
    let first_solve = lines.iter().position(|l| l.starts_with("[solve]")).unwrap();
    assert!(first_mesh < first_solve);
}

#[test]
fn meshing_can_be_cancelled() {
    let mut f = fixture("cancel", false);
    let job = f.run("run.start", json!({"cores": 1}))["job_id"]
        .as_str()
        .unwrap()
        .to_string();
    f.wait_for_event("mesh.progress");
    f.run("run.cancel", json!({"job_id": job}));
    let st = f.wait_idle(&job);
    assert_eq!(st["state"]["state"], "cancelled", "{st}");
    let failed = f.events_named("run.failed");
    assert_eq!(failed.len(), 1, "{failed:?}");
    assert_eq!(failed[0]["job_id"], json!(job));
    assert_eq!(failed[0]["code"], "run.cancelled");
    // No mesh was stored, nothing is left running and the container and volume are gone.
    assert_eq!(
        f.reg
            .execute(&mut f.s, "mesh.info", json!({}))
            .unwrap_err()
            .code,
        "mesh.none"
    );
    assert_eq!(f.run("run.active", json!({})), json!([]));
    let calls = f.calls.lock().unwrap().clone();
    assert!(
        calls
            .iter()
            .any(|c| c.starts_with(&["rm".into(), "-f".into()])),
        "container not removed"
    );
    assert!(
        calls
            .iter()
            .any(|c| c.starts_with(&["volume".into(), "rm".into()])),
        "volume not removed"
    );
}

#[test]
fn mesh_only_stops_after_the_mesh() {
    let mut f = fixture("mesh-only", true);
    let out = f.run("run.start", json!({"mesh_only": true, "wait": true}));
    assert!(out["result"].is_null(), "{out}");
    let mesh = f.run("mesh.info", json!({}));
    assert_eq!(mesh["cells"], 1000);
    // It is no run of the project and has no result.
    assert_eq!(f.run("project.info", json!({}))["runs"], json!([]));
    assert!(f.reg.execute(&mut f.s, "run.result", json!({})).is_err());
    assert_eq!(f.run("run.active", json!({})), json!([]));
    assert_eq!(f.events_named("mesh.ready").len(), 1);
}

#[test]
fn a_second_start_reuses_the_mesh() {
    let mut f = fixture("reuse", true);
    f.run("run.start", json!({"wait": true, "cores": 1}));
    let first_mesh = f.run("mesh.info", json!({}))["job_id"].clone();
    let stages_before = f.events_named("mesh.stage").len();
    f.run("run.start", json!({"wait": true, "cores": 1}));
    assert_eq!(f.run("mesh.info", json!({}))["job_id"], first_mesh);
    assert_eq!(f.events_named("mesh.stage").len(), stages_before);
    assert_eq!(
        f.run("project.info", json!({}))["runs"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
}

#[test]
fn a_changed_scenario_or_model_gets_a_new_mesh() {
    let mut f = fixture("remesh", true);
    f.run("run.start", json!({"wait": true, "cores": 1}));
    let first = f.run("mesh.info", json!({}))["job_id"].clone();
    // Another flow direction turns the tunnel: the old mesh no longer fits.
    f.run(
        "scenario.set_wind_tunnel",
        json!({"speed_m_s": 20.0, "direction_deg": 30.0, "ground": "none",
               "medium": {"kind": "air", "temperature_c": 20.0}, "quality": "draft"}),
    );
    f.run("run.start", json!({"wait": true, "cores": 1}));
    let second = f.run("mesh.info", json!({}))["job_id"].clone();
    assert_ne!(second, first);
    // Moving the model also asks for a new mesh; the volume of the replaced one is dropped.
    f.calls.lock().unwrap().clear();
    f.run(
        "geometry.set_transform",
        json!({"id": "model-1", "translation": [0.0, 0.0, 1.0]}),
    );
    f.run("run.start", json!({"wait": true, "cores": 1}));
    let third = f.run("mesh.info", json!({}))["job_id"].clone();
    assert_ne!(third, second);
    let calls = f.calls.lock().unwrap().clone();
    assert!(
        calls
            .iter()
            .any(|c| c.starts_with(&["volume".into(), "rm".into()])),
        "old mesh volume kept"
    );
    // A view setting does not touch the mesh.
    f.run("project.rename", json!({"name": "Other name"}));
    f.run("run.start", json!({"wait": true, "cores": 1}));
    assert_eq!(f.run("mesh.info", json!({}))["job_id"], third);
}

#[test]
fn a_run_that_lost_its_meshing_is_reported_as_lost() {
    let mut f = fixture("lost", true);
    // As if the app had ended while the mesh of a run was built: the run exists, pending, without
    // a container, and nobody follows it.
    let store = foamcraft_core::job::JobStore::new(f.root.join("runs"));
    let record = store.create("wind_tunnel_solve").unwrap();
    let active = f.run("run.active", json!({}));
    assert_eq!(active[0]["job_id"], json!(record.id));
    let err = f
        .reg
        .execute(&mut f.s, "run.attach", json!({"job_id": record.id}))
        .unwrap_err();
    assert_eq!(err.code, "run.lost");
    assert_eq!(f.run("run.active", json!({})), json!([]));
}

/// Output of a docker command line (the real engine; runtime tests only).
fn docker(args: &[&str]) -> String {
    let out = std::process::Command::new("docker")
        .args(args)
        .output()
        .expect("docker");
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

#[test]
#[ignore]
fn runtime_cancelling_the_meshing_leaves_no_container_or_volume() {
    let reg = foamcraft_api::registry();
    let base = std::env::temp_dir().join(format!("fc-api-pipe-rt-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&base);
    let mut s = Session::new(base.join("data"));
    let events = Arc::new(Mutex::new(Vec::<(String, Value)>::new()));
    let sink = events.clone();
    s.events().subscribe(move |e| {
        sink.lock()
            .unwrap()
            .push((e.name.clone(), e.payload.clone()));
    });
    reg.execute(
        &mut s,
        "samples.create",
        json!({"sample": "wind_tunnel", "path": base.join("p")}),
    )
    .unwrap();
    let begun = Instant::now();
    let job = reg.execute(&mut s, "run.start", json!({})).unwrap()["job_id"]
        .as_str()
        .unwrap()
        .to_string();
    assert!(
        begun.elapsed() < Duration::from_secs(20),
        "run.start blocked"
    );
    // Wait until snappyHexMesh works, then cancel.
    let start = Instant::now();
    let mesh_job = loop {
        let found = events.lock().unwrap().iter().find_map(|(n, p)| {
            (n == "mesh.progress" && p["phase"] == "refine")
                .then(|| p["job_id"].as_str().unwrap().to_string())
        });
        if let Some(id) = found {
            break id;
        }
        assert!(
            start.elapsed() < Duration::from_secs(300),
            "no meshing phase"
        );
        std::thread::sleep(Duration::from_millis(200));
    };
    // Other commands are answered while the mesh is built, and the log is readable.
    let log = reg
        .execute(&mut s, "run.log", json!({"job_id": job, "lines": 20}))
        .unwrap();
    assert!(!log["lines"].as_array().unwrap().is_empty());
    let store = foamcraft_core::job::JobStore::new(base.join("p").join("runs"));
    let container = store.load(&mesh_job).unwrap().container.unwrap();
    assert!(!docker(&["ps", "-q", "--filter", &format!("id={container}")]).is_empty());
    reg.execute(&mut s, "run.cancel", json!({"job_id": job}))
        .unwrap();
    let start = Instant::now();
    loop {
        let st = reg
            .execute(&mut s, "run.status", json!({"job_id": job}))
            .unwrap();
        if st["busy"] == false {
            assert_eq!(st["state"]["state"], "cancelled", "{st}");
            break;
        }
        assert!(start.elapsed() < Duration::from_secs(120), "{st}");
        std::thread::sleep(Duration::from_millis(200));
    }
    assert!(
        docker(&["ps", "-aq", "--filter", &format!("id={container}")]).is_empty(),
        "container left behind"
    );
    let volume = format!("foamcraft-{mesh_job}");
    assert!(
        docker(&["volume", "ls", "-q", "--filter", &format!("name={volume}")]).is_empty(),
        "volume left behind"
    );
    assert_eq!(
        reg.execute(&mut s, "mesh.info", json!({}))
            .unwrap_err()
            .code,
        "mesh.none"
    );
}

// --- Task_30: several scenarios, shared meshes, the queue ---

fn tunnel(speed: f64, direction: f64) -> Value {
    json!({"speed_m_s": speed, "direction_deg": direction, "ground": "none",
           "medium": {"kind": "air", "temperature_c": 20.0}, "quality": "draft"})
}

#[test]
fn scenarios_are_listed_added_renamed_activated_and_removed() {
    let mut f = fixture("scenarios", true);
    let list = f.run("scenario.list", json!({}));
    assert_eq!(list["active"], "scenario-1");
    assert_eq!(list["scenarios"].as_array().unwrap().len(), 1);
    let added = f.run("scenario.add", json!({"name": "Fast", "copy": true}));
    assert_eq!(added["id"], "scenario-2");
    f.run("scenario.activate", json!({"id": "scenario-2"}));
    f.run("scenario.set_wind_tunnel", tunnel(40.0, 0.0));
    f.run(
        "scenario.rename",
        json!({"id": "scenario-2", "name": "Very fast"}),
    );
    let list = f.run("scenario.list", json!({}));
    assert_eq!(list["active"], "scenario-2");
    let s2 = &list["scenarios"][1];
    assert_eq!(s2["name"], "Very fast");
    assert_eq!(s2["scenario"]["speed_m_s"], 40.0);
    assert_eq!(list["scenarios"][0]["scenario"]["speed_m_s"], 20.0);
    // Undo of the rename, then of the change, then of the activation.
    f.run("history.undo", json!({}));
    assert_eq!(
        f.run("scenario.list", json!({}))["scenarios"][1]["name"],
        "Fast"
    );
    let err = f
        .reg
        .execute(&mut f.s, "scenario.remove", json!({"id": "scenario-2"}))
        .unwrap_err();
    assert_eq!(err.code, "scenario.remove_active");
    f.run("scenario.activate", json!({"id": "scenario-1"}));
    f.run("scenario.remove", json!({"id": "scenario-2"}));
    assert_eq!(
        f.run("scenario.list", json!({}))["scenarios"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    let err = f
        .reg
        .execute(&mut f.s, "scenario.activate", json!({"id": "scenario-7"}))
        .unwrap_err();
    assert_eq!(err.code, "scenario.not_found");
}

#[test]
fn the_queue_runs_all_scenarios_in_order_and_shares_fitting_meshes() {
    let mut f = fixture("queue", true);
    // Scenario 2: a little faster, same direction and quality: same mesh.
    f.run("scenario.add", json!({"name": "Faster", "copy": true}));
    f.run("scenario.activate", json!({"id": "scenario-2"}));
    f.run("scenario.set_wind_tunnel", tunnel(21.0, 0.0));
    // Scenario 3: another direction: its own mesh.
    f.run("scenario.add", json!({"name": "Turned", "copy": true}));
    f.run("scenario.activate", json!({"id": "scenario-3"}));
    f.run("scenario.set_wind_tunnel", tunnel(20.0, 30.0));
    f.run("scenario.activate", json!({"id": "scenario-1"}));
    let out = f.run("run.start_all", json!({"wait": true, "cores": 1}));
    let runs = out["runs"].as_array().unwrap();
    assert_eq!(runs.len(), 3);
    assert_eq!(
        runs.iter()
            .map(|r| r["scenario"].clone())
            .collect::<Vec<_>>(),
        [
            json!("scenario-1"),
            json!("scenario-2"),
            json!("scenario-3")
        ]
    );
    assert!(runs.iter().all(|r| r["error"].is_null()), "{out}");
    let list = f.run("scenario.list", json!({}));
    let mesh = |i: usize| list["scenarios"][i]["mesh"]["job_id"].clone();
    assert!(mesh(0).is_string());
    assert_eq!(mesh(0), mesh(1), "{list}");
    assert_ne!(mesh(0), mesh(2), "{list}");
    // Each run knows its scenario; the active scenario did not change.
    assert_eq!(list["active"], "scenario-1");
    assert_eq!(list["scenarios"][1]["runs"].as_array().unwrap().len(), 1);
    assert_eq!(f.events_named("run.queue").len(), 3);
    assert_eq!(f.events_named("run.queue_finished").len(), 1);
}

#[test]
fn the_queue_can_be_cancelled() {
    let mut f = fixture("queue-cancel", false);
    f.run("scenario.add", json!({"name": "Second", "copy": true}));
    let out = f.run("run.start_all", json!({"cores": 1}));
    let first = out["runs"][0]["job_id"].as_str().unwrap().to_string();
    f.wait_for_event("mesh.stage");
    f.run("run.cancel_queue", json!({}));
    let st = f.wait_idle(&first);
    assert_eq!(st["state"]["state"], "cancelled", "{st}");
    let start = Instant::now();
    while f.events_named("run.queue_finished").is_empty() {
        assert!(
            start.elapsed() < Duration::from_secs(30),
            "queue did not end"
        );
        std::thread::sleep(Duration::from_millis(20));
    }
    // The second scenario was not started.
    assert_eq!(f.events_named("run.queue").len(), 1);
    let fin = &f.events_named("run.queue_finished")[0];
    assert_eq!(fin["cancelled"], true);
}
