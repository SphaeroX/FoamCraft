// SPDX-License-Identifier: GPL-3.0-or-later
//! Bridge between the interface and the command API: `api_call` runs a command, state events are
//! forwarded as `foamcraft://event`.

use std::sync::{Arc, Mutex};

use foamcraft_api::Session;
use foamcraft_core::command::Registry;
use foamcraft_core::error::{FoamError, Language, Message};
use serde::Serialize;
use serde_json::Value;
use tauri::{AppHandle, Emitter, Manager};

pub struct ApiState {
    registry: Arc<Registry>,
    session: Arc<Mutex<Session>>,
    /// Lets the MCP server (`foamcraft mcp --app`) run commands in this session.
    _bridge: Option<foamcraft_api::bridge::Bridge>,
}

/// Error as the interface receives it: code plus both translations.
#[derive(Serialize)]
pub struct ApiError {
    code: String,
    detail: String,
    en: Message,
    de: Message,
}

impl From<FoamError> for ApiError {
    fn from(e: FoamError) -> Self {
        Self {
            en: e.localized(Language::En),
            de: e.localized(Language::De),
            code: e.code,
            detail: e.detail,
        }
    }
}

impl ApiState {
    pub fn new(app: &AppHandle) -> Self {
        let data_dir = foamcraft_core::paths::data_dir();
        let session = Session::new(data_dir.clone());
        let handle = app.clone();
        session.events().subscribe(move |event| {
            let _ = handle.emit("foamcraft://event", event);
            match event.name.as_str() {
                // Every solver iteration keeps the computer from sleeping during a run.
                "run.progress" | "run.coefficients" | "mesh.stage" | "run.stage" => keep_awake(),
                // The end of a run flashes the taskbar entry when the window is in the background.
                "run.result" | "run.failed" => {
                    if let Some(window) = handle.get_webview_window("main") {
                        let _ = window
                            .request_user_attention(Some(tauri::UserAttentionType::Informational));
                    }
                }
                _ => {}
            }
        });
        let registry = Arc::new(foamcraft_api::registry());
        let session = Arc::new(Mutex::new(session));
        let bridge = foamcraft_api::bridge::serve(registry.clone(), session.clone(), &data_dir)
            .map_err(|e| eprintln!("foamcraft: no bridge for MCP: {e}"))
            .ok();
        Self {
            registry,
            session,
            _bridge: bridge,
        }
    }
}

/// Runs a command of the command API (off the main thread).
#[tauri::command(async)]
pub fn api_call(
    name: String,
    params: Value,
    state: tauri::State<'_, ApiState>,
) -> Result<Value, Box<ApiError>> {
    let mut session = state.session.lock().unwrap_or_else(|e| e.into_inner());
    state
        .registry
        .execute(&mut session, &name, params)
        .map_err(|e| Box::new(ApiError::from(e)))
}

impl ApiState {
    /// Runs `f` with the open project (the session stays locked meanwhile).
    pub fn with_project<T>(
        &self,
        f: impl FnOnce(Option<&foamcraft_core::project::Project>) -> T,
    ) -> T {
        let mut session = self.session.lock().unwrap_or_else(|e| e.into_inner());
        // The view shows what a command would see: changes of background work included.
        session.apply_updates();
        f(session.project_ref())
    }
}

/// Resets the system idle timer so the computer does not go to sleep while a run reports
/// progress (no effect on the display).
#[cfg(windows)]
fn keep_awake() {
    const ES_SYSTEM_REQUIRED: u32 = 0x0000_0001;
    unsafe extern "system" {
        fn SetThreadExecutionState(flags: u32) -> u32;
    }
    // SAFETY: SetThreadExecutionState (kernel32) takes a flag value and has no other
    // preconditions; without ES_CONTINUOUS it only resets the idle timer once.
    unsafe {
        SetThreadExecutionState(ES_SYSTEM_REQUIRED);
    }
}

#[cfg(not(windows))]
fn keep_awake() {}
