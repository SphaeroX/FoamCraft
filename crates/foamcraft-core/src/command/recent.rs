// SPDX-License-Identifier: GPL-3.0-or-later
//! Recently opened projects, stored in `<data dir>/recent.json`.

use std::path::Path;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

const MAX_RECENT: usize = 10;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct RecentProject {
    pub path: String,
    pub name: String,
}

fn file(data_dir: &Path) -> std::path::PathBuf {
    data_dir.join("recent.json")
}

pub fn load(data_dir: &Path) -> Vec<RecentProject> {
    std::fs::read_to_string(file(data_dir))
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default()
}

/// The same folder spelled with `/` or `\`, a trailing separator or other letter case (Windows
/// paths are not case sensitive) compares equal.
fn same_folder(a: &str, b: &str) -> bool {
    let norm = |p: &str| {
        let p = p.replace('\\', "/");
        let p = p.trim_end_matches("/").to_string();
        if cfg!(windows) { p.to_lowercase() } else { p }
    };
    norm(a) == norm(b)
}

/// Moves `entry` to the front. Failing to store the list is not an error for the user.
pub fn remember(data_dir: &Path, entry: RecentProject) {
    let mut list = load(data_dir);
    list.retain(|r| !same_folder(&r.path, &entry.path));
    list.insert(0, entry);
    list.truncate(MAX_RECENT);
    if std::fs::create_dir_all(data_dir).is_ok()
        && let Ok(json) = serde_json::to_string_pretty(&list)
    {
        let _ = std::fs::write(file(data_dir), json);
    }
}
