// SPDX-License-Identifier: GPL-3.0-or-later
//! Bridge to the running app: the app listens on a loopback TCP port and runs commands that
//! another local process (the MCP server, the CLI) sends, in its own session, so the user sees
//! every step. Port and a random token are in `<data dir>/app-bridge.json`; a request without
//! the token is refused.
//!
//! Protocol: one JSON object per line. Request `{"token", "command", "params"}`, answer
//! `{"ok": value}` or `{"error": {"code", "detail"}}`.

use std::hash::{BuildHasher, Hasher};
use std::io::{BufRead, BufReader, Write};
use std::net::{Ipv4Addr, SocketAddr, TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use foamcraft_core::command::{Registry, Session};
use foamcraft_core::error::FoamError;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

/// File in the data folder that tells clients where the app listens.
pub const BRIDGE_FILE: &str = "app-bridge.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
struct BridgeInfo {
    port: u16,
    token: String,
    pid: u32,
}

/// 128 random bits as hex, from the randomly seeded hasher of the standard library.
fn random_token() -> String {
    let part = || {
        let mut h = std::collections::hash_map::RandomState::new().build_hasher();
        h.write_u128(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |d| d.as_nanos()),
        );
        h.write_u32(std::process::id());
        h.finish()
    };
    format!("{:016x}{:016x}", part(), part())
}

/// The listening bridge; dropping it removes the bridge file (the port closes with the process).
pub struct Bridge {
    file: PathBuf,
    pub port: u16,
}

impl Drop for Bridge {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.file);
    }
}

fn answer(registry: &Registry, session: &Mutex<Session>, token: &str, line: &str) -> Value {
    #[derive(Deserialize)]
    struct Request {
        token: String,
        command: String,
        #[serde(default)]
        params: Value,
    }
    let request: Request = match serde_json::from_str(line) {
        Ok(r) => r,
        Err(e) => return json!({"error": {"code": "bridge.failed", "detail": e.to_string()}}),
    };
    if request.token != token {
        return json!({"error": {"code": "bridge.refused", "detail": ""}});
    }
    let params = if request.params.is_null() {
        json!({})
    } else {
        request.params
    };
    let mut s = session.lock().unwrap_or_else(|e| e.into_inner());
    match registry.execute(&mut s, &request.command, params) {
        Ok(v) => json!({ "ok": v }),
        Err(e) => json!({"error": {"code": e.code, "detail": e.detail}}),
    }
}

fn handle(stream: TcpStream, registry: &Registry, session: &Mutex<Session>, token: &str) {
    let Ok(mut writer) = stream.try_clone() else {
        return;
    };
    for line in BufReader::new(stream).lines() {
        let Ok(line) = line else { return };
        if line.trim().is_empty() {
            continue;
        }
        let reply = answer(registry, session, token, &line);
        if writeln!(writer, "{reply}")
            .and_then(|_| writer.flush())
            .is_err()
        {
            return;
        }
    }
}

/// Listens on 127.0.0.1 (a free port) and writes the bridge file into `data_dir`.
pub fn serve(
    registry: Arc<Registry>,
    session: Arc<Mutex<Session>>,
    data_dir: &Path,
) -> std::io::Result<Bridge> {
    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0))?;
    let port = listener.local_addr()?.port();
    let token = random_token();
    std::fs::create_dir_all(data_dir)?;
    let file = data_dir.join(BRIDGE_FILE);
    let info = BridgeInfo {
        port,
        token: token.clone(),
        pid: std::process::id(),
    };
    std::fs::write(&file, serde_json::to_string(&info)?)?;
    std::thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            let (registry, session, token) = (registry.clone(), session.clone(), token.clone());
            std::thread::spawn(move || handle(stream, &registry, &session, &token));
        }
    });
    Ok(Bridge { file, port })
}

/// A client of the app's bridge.
#[derive(Debug, Clone)]
pub struct BridgeClient {
    addr: SocketAddr,
    token: String,
}

impl BridgeClient {
    /// Reads the bridge file of `data_dir` and checks that the app answers.
    pub fn connect(data_dir: &Path) -> Result<Self, FoamError> {
        let no_app = |detail: String| FoamError::new("bridge.no_app", detail);
        let text = std::fs::read_to_string(data_dir.join(BRIDGE_FILE))
            .map_err(|e| no_app(e.to_string()))?;
        let info: BridgeInfo = serde_json::from_str(&text).map_err(|e| no_app(e.to_string()))?;
        let addr = SocketAddr::from((Ipv4Addr::LOCALHOST, info.port));
        TcpStream::connect_timeout(&addr, Duration::from_secs(2))
            .map_err(|e| no_app(e.to_string()))?;
        Ok(Self {
            addr,
            token: info.token,
        })
    }

    /// The same address with another token (for tests of the refusal).
    pub fn with_token(&self, token: &str) -> Self {
        Self {
            addr: self.addr,
            token: token.into(),
        }
    }

    /// Runs a command in the app's session and waits for its answer.
    pub fn execute(&self, command: &str, params: Value) -> Result<Value, FoamError> {
        let failed = |e: std::io::Error| FoamError::new("bridge.failed", e.to_string());
        let mut stream =
            TcpStream::connect_timeout(&self.addr, Duration::from_secs(2)).map_err(failed)?;
        let request = json!({"token": self.token, "command": command, "params": params});
        writeln!(stream, "{request}").map_err(failed)?;
        stream.flush().map_err(failed)?;
        let mut line = String::new();
        BufReader::new(&stream)
            .read_line(&mut line)
            .map_err(failed)?;
        let reply: Value = serde_json::from_str(&line)
            .map_err(|e| FoamError::new("bridge.failed", e.to_string()))?;
        if let Some(e) = reply.get("error") {
            return Err(FoamError::new(
                e["code"].as_str().unwrap_or("bridge.failed"),
                e["detail"].as_str().unwrap_or_default(),
            ));
        }
        Ok(reply.get("ok").cloned().unwrap_or(Value::Null))
    }
}
