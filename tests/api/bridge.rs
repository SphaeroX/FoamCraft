// SPDX-License-Identifier: GPL-3.0-or-later
//! The bridge to the running app: another process (CLI, MCP server) runs commands in the app's
//! session over a loopback connection with a token.

use std::sync::{Arc, Mutex};

use foamcraft_api::bridge::{BridgeClient, serve};
use foamcraft_core::command::Session;
use serde_json::json;

fn base(name: &str) -> std::path::PathBuf {
    let b = std::env::temp_dir().join(format!("fc-bridge-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&b);
    std::fs::create_dir_all(&b).unwrap();
    b
}

#[test]
fn a_command_over_the_bridge_changes_the_session_of_the_app() {
    let dir = base("run");
    let session = Arc::new(Mutex::new(Session::new(dir.join("data"))));
    let registry = Arc::new(foamcraft_api::registry());
    let events = Arc::new(Mutex::new(Vec::new()));
    {
        let events = events.clone();
        session
            .lock()
            .unwrap()
            .events()
            .subscribe(move |e| events.lock().unwrap().push(e.name.clone()));
    }
    let _bridge = serve(registry, session.clone(), &dir.join("data")).unwrap();
    let client = BridgeClient::connect(&dir.join("data")).unwrap();
    let info = client
        .execute(
            "project.new",
            json!({"path": dir.join("p"), "name": "Bridged"}),
        )
        .unwrap();
    assert_eq!(info["name"], "Bridged");
    // The app sees the project and the event that tells its interface to reload.
    let s = session.lock().unwrap();
    assert_eq!(s.project_ref().unwrap().manifest.name, "Bridged");
    assert!(
        events
            .lock()
            .unwrap()
            .iter()
            .any(|e| e == "project.changed")
    );
}

#[test]
fn errors_keep_their_code_and_a_wrong_token_is_refused() {
    let dir = base("errors");
    let session = Arc::new(Mutex::new(Session::new(dir.join("data"))));
    let _bridge = serve(
        Arc::new(foamcraft_api::registry()),
        session,
        &dir.join("data"),
    )
    .unwrap();
    let client = BridgeClient::connect(&dir.join("data")).unwrap();
    let err = client.execute("project.info", json!({})).unwrap_err();
    assert_eq!(err.code, "project.none_open");
    let forged = client.with_token("wrong");
    let err = forged.execute("project.recent", json!({})).unwrap_err();
    assert_eq!(err.code, "bridge.refused");
}

#[test]
fn without_a_running_app_connecting_fails_clearly() {
    let dir = base("none");
    let err = BridgeClient::connect(&dir.join("data")).unwrap_err();
    assert_eq!(err.code, "bridge.no_app");
}
