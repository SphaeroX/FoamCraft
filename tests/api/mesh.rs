// SPDX-License-Identifier: GPL-3.0-or-later
//! mesh.generate against the fake container engine: leak detection and the fallback ladder.

use std::path::PathBuf;
use std::sync::Arc;

use foamcraft_api::registry_with;
use foamcraft_core::command::Session;
use foamcraft_runtime::fake::FakeEngine;
use foamcraft_runtime::{CmdOutput, DockerRunner};
use serde_json::json;

const GIB: u64 = 1024 * 1024 * 1024;

fn tail(text: &str) -> CmdOutput {
    CmdOutput::ok(&json!({"text": text, "next_offset": text.len(), "exit": 0}).to_string())
}

/// Layers are always only 20 % built; everything else succeeds.
fn engine() -> FakeEngine {
    let info = format!(
        r#"{{"ServerVersion":"29","OperatingSystem":"Docker Desktop","KernelVersion":"6.6-microsoft-standard-WSL2","NCPU":8,"MemTotal":{}}}"#,
        16 * GIB
    );
    let status = r#"{"agent_version":"0.1.0","openfoam":{"api":"2606","patch":"0","build":"x"},"cpus":4,"memory_bytes":1024,"gpu":{"available":false,"detail":""}}"#;
    let layers = "patch faces layers overall thickness\n target mesh [m] [%]\n----- ----- ----- ---- --- ---\nmodel_body 100 4 0.8 0.001 20.0\n\nEnd\n";
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
        .on(
            &[
                "exec",
                "cid",
                "foamcraft-agent",
                "tail",
                "/work/case",
                "background",
            ],
            tail("End\n"),
        )
        .on(
            &[
                "exec",
                "cid",
                "foamcraft-agent",
                "tail",
                "/work/case",
                "mesh",
            ],
            tail(layers),
        )
        .on(
            &[
                "exec",
                "cid",
                "foamcraft-agent",
                "tail",
                "/work/case",
                "check",
            ],
            tail("    cells:            1000\n    Max skewness = 1.0 OK.\nMesh OK.\nEnd\n"),
        )
        .on(&["rm"], CmdOutput::ok(""))
}

fn project(file: &str) -> (foamcraft_core::command::Registry, Session) {
    let reg = registry_with(Arc::new(DockerRunner::with_host(engine(), true)));
    let base = std::env::temp_dir().join(format!("fc-api-mesh-{file}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&base);
    let mut s = Session::new(base.join("data"));
    reg.execute(
        &mut s,
        "project.new",
        json!({"path": base.join("p"), "name": "M"}),
    )
    .unwrap();
    let model = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/data/geometry")
        .join(file);
    reg.execute(
        &mut s,
        "geometry.import",
        json!({"path": model, "unit": "m"}),
    )
    .unwrap();
    reg.execute(
        &mut s,
        "scenario.set_wind_tunnel",
        json!({"speed_m_s": 40.0, "direction_deg": 0.0, "ground": "fixed",
               "medium": {"kind": "air", "temperature_c": 20.0}, "quality": "standard"}),
    )
    .unwrap();
    (reg, s)
}

#[test]
fn open_model_is_a_leak() {
    let (reg, mut s) = project("cube_open.stl");
    let err = reg
        .execute(&mut s, "mesh.generate", json!({"max_cells": 100000}))
        .unwrap_err();
    assert_eq!(err.code, "mesh.leak");
}

#[test]
fn failing_layers_trigger_the_fallback_ladder() {
    let (reg, mut s) = project("cube_binary.stl");
    let out = reg
        .execute(&mut s, "mesh.generate", json!({"max_cells": 300000}))
        .unwrap();
    let codes: Vec<_> = out["fallbacks"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| f["code"].clone())
        .collect();
    assert_eq!(
        codes.last(),
        Some(&json!("mesh.fallback_no_layers")),
        "{codes:?}"
    );
    assert!(out["plan"]["layers"] == json!(0));
    assert_eq!(out["cells"], 1000);
    assert!(
        !out["fallbacks"][0]["de"]["text"]
            .as_str()
            .unwrap()
            .is_empty()
    );
    let info = reg.execute(&mut s, "mesh.info", json!({})).unwrap();
    assert_eq!(info["fallbacks"], json!(codes));
}
