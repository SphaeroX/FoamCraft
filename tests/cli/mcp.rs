// SPDX-License-Identifier: GPL-3.0-or-later
//! `foamcraft mcp --root <dir>` as an MCP client starts it: JSON-RPC lines over stdio.

use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};

use serde_json::{Value, json};

#[test]
fn the_binary_speaks_mcp_over_stdio() {
    let dir = std::env::temp_dir().join(format!("fc-cli-mcp-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("work")).unwrap();
    let mut child = Command::new(env!("CARGO_BIN_EXE_foamcraft"))
        .args(["mcp", "--root"])
        .arg(dir.join("work"))
        .env("FOAMCRAFT_DATA_DIR", dir.join("data"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let mut stdin = child.stdin.take().unwrap();
    let mut lines = BufReader::new(child.stdout.take().unwrap()).lines();
    let mut send = |v: Value| {
        writeln!(stdin, "{v}").unwrap();
        stdin.flush().unwrap();
    };
    let mut receive = |id: u64| -> Value {
        loop {
            let line = lines.next().expect("server closed stdout").unwrap();
            let v: Value = serde_json::from_str(&line).unwrap();
            if v["id"] == id {
                return v;
            }
        }
    };
    send(
        json!({"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {
        "protocolVersion": "2025-06-18", "capabilities": {},
        "clientInfo": {"name": "test", "version": "0"}}}),
    );
    let init = receive(1);
    assert_eq!(init["result"]["serverInfo"]["name"], "foamcraft", "{init}");
    assert!(
        init["result"]["instructions"]
            .as_str()
            .unwrap()
            .contains("run_wait")
    );
    send(json!({"jsonrpc": "2.0", "method": "notifications/initialized"}));
    send(json!({"jsonrpc": "2.0", "id": 2, "method": "tools/list"}));
    let tools = receive(2);
    let list = tools["result"]["tools"].as_array().unwrap();
    assert!(list.iter().any(|t| t["name"] == "view_render"));
    send(
        json!({"jsonrpc": "2.0", "id": 3, "method": "tools/call", "params": {
        "name": "project_new", "arguments": {"path": dir.join("work/p"), "name": "Stdio"}}}),
    );
    let out = receive(3);
    assert_eq!(out["result"]["isError"], false, "{out}");
    drop(send);
    child.kill().ok();
    let _ = child.wait();
}

/// A minimal MCP client over the stdio of `foamcraft mcp`.
struct Client {
    child: std::process::Child,
    stdin: std::process::ChildStdin,
    lines: std::io::Lines<BufReader<std::process::ChildStdout>>,
    next: u64,
}

impl Client {
    fn start(root: &std::path::Path, data: &std::path::Path) -> Self {
        let mut child = Command::new(env!("CARGO_BIN_EXE_foamcraft"))
            .args(["mcp", "--root"])
            .arg(root)
            .env("FOAMCRAFT_DATA_DIR", data)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        let stdin = child.stdin.take().unwrap();
        let lines = BufReader::new(child.stdout.take().unwrap()).lines();
        let mut c = Self {
            child,
            stdin,
            lines,
            next: 1,
        };
        c.request(
            "initialize",
            json!({"protocolVersion": "2025-06-18", "capabilities": {},
            "clientInfo": {"name": "runtime-test", "version": "0"}}),
        );
        writeln!(
            c.stdin,
            "{}",
            json!({"jsonrpc": "2.0", "method": "notifications/initialized"})
        )
        .unwrap();
        c
    }

    fn request(&mut self, method: &str, params: Value) -> Value {
        let id = self.next;
        self.next += 1;
        writeln!(
            self.stdin,
            "{}",
            json!({"jsonrpc": "2.0", "id": id, "method": method, "params": params})
        )
        .unwrap();
        self.stdin.flush().unwrap();
        loop {
            let line = self.lines.next().expect("server closed stdout").unwrap();
            let v: Value = serde_json::from_str(&line).unwrap();
            if v["id"] == id {
                return v["result"].clone();
            }
        }
    }

    /// Calls a tool; returns the parsed JSON text and the images.
    fn tool(&mut self, name: &str, args: Value) -> (Value, Vec<String>) {
        let out = self.request("tools/call", json!({"name": name, "arguments": args}));
        assert_eq!(out["isError"], false, "{name}: {out}");
        let mut value = Value::Null;
        let mut images = Vec::new();
        for c in out["content"].as_array().unwrap() {
            match c["type"].as_str() {
                Some("text") if value.is_null() => {
                    value = serde_json::from_str(c["text"].as_str().unwrap()).unwrap()
                }
                Some("image") => images.push(c["data"].as_str().unwrap().to_string()),
                _ => {}
            }
        }
        (value, images)
    }
}

impl Drop for Client {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// Removes the containers and volumes of the jobs of a project, also when the test fails.
struct Cleanup(std::path::PathBuf);

impl Drop for Cleanup {
    fn drop(&mut self) {
        let store = foamcraft_core::job::JobStore::new(self.0.join("runs"));
        let runner = foamcraft_runtime::DockerRunner::local();
        for record in store.list().unwrap_or_default() {
            let _ = foamcraft_runtime::jobs::remove_job_resources(&runner, &record);
        }
    }
}

#[test]
#[ignore]
fn runtime_a_wind_tunnel_run_over_mcp() {
    let dir = std::env::temp_dir().join(format!("fc-cli-mcp-rt-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("work")).unwrap();
    let model = dir.join("work/cube.stl");
    std::fs::copy(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/data/geometry/cube_binary.stl"),
        &model,
    )
    .unwrap();
    let project = dir.join("work/cube");
    let _cleanup = Cleanup(project.clone());
    let mut c = Client::start(&dir.join("work"), &dir.join("data"));
    c.tool("project_new", json!({"path": project, "name": "MCP cube"}));
    c.tool("geometry_import", json!({"path": model, "unit": "mm"}));
    c.tool(
        "scenario_set_wind_tunnel",
        json!({"speed_m_s": 20.0, "direction_deg": 0.0, "ground": "none",
        "medium": {"kind": "air", "temperature_c": 20.0}, "quality": "draft"}),
    );
    let (started, _) = c.tool("run_start", json!({}));
    let job = started["job_id"].as_str().unwrap().to_string();
    let mut done = Value::Null;
    for _ in 0..6 {
        done = c
            .tool("run_wait", json!({"job_id": job, "timeout_s": 300}))
            .0;
        if done["finished"] == true {
            break;
        }
    }
    assert_eq!(done["state"]["state"], "succeeded", "{done}");
    let (summary, _) = c.tool("results_summary", json!({}));
    let cd = summary["figures"]
        .as_array()
        .unwrap()
        .iter()
        .find(|f| f["id"] == "cd")
        .unwrap()["value"]
        .as_f64()
        .unwrap();
    assert!((0.5..2.0).contains(&cd), "{summary}");
    assert_ne!(summary["reliability"]["level"], Value::Null);
    c.tool("view_set_surface_field", json!({"field": "p"}));
    let (_, images) = c.tool("view_render", json!({"width": 640, "height": 400}));
    match images.first() {
        Some(png) => assert!(png.len() > 1000),
        None => panic!("no image"),
    }
}
