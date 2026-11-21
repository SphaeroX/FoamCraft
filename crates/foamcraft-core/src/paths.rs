// SPDX-License-Identifier: GPL-3.0-or-later
//! Default locations of FoamCraft data on this computer.

use std::path::{Path, PathBuf};

/// The app data folder: `%LOCALAPPDATA%\FoamCraft` on Windows, `$XDG_DATA_HOME/foamcraft` or
/// `~/.local/share/foamcraft` elsewhere. `FOAMCRAFT_DATA_DIR` overrides it.
pub fn data_dir() -> PathBuf {
    if let Some(dir) = std::env::var_os("FOAMCRAFT_DATA_DIR") {
        return PathBuf::from(dir);
    }
    if cfg!(windows)
        && let Some(local) = std::env::var_os("LOCALAPPDATA")
    {
        return PathBuf::from(local).join("FoamCraft");
    }
    if let Some(xdg) = std::env::var_os("XDG_DATA_HOME") {
        return PathBuf::from(xdg).join("foamcraft");
    }
    let home = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE"));
    home.map_or_else(
        || PathBuf::from(".foamcraft"),
        |h| PathBuf::from(h).join(".local/share/foamcraft"),
    )
}

/// Folder with job records and logs.
pub fn jobs_dir() -> PathBuf {
    data_dir().join("jobs")
}

/// Default folder for new projects: `FoamCraft` in the user's home folder (short, visible and not
/// inside a synchronised documents folder, where result caches would be uploaded).
/// `FOAMCRAFT_PROJECTS_DIR` overrides it.
pub fn projects_dir() -> PathBuf {
    if let Some(dir) = std::env::var_os("FOAMCRAFT_PROJECTS_DIR") {
        return PathBuf::from(dir);
    }
    let home = if cfg!(windows) {
        std::env::var_os("USERPROFILE").or_else(|| std::env::var_os("HOME"))
    } else {
        std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE"))
    };
    home.map_or_else(
        || PathBuf::from("FoamCraft"),
        |h| PathBuf::from(h).join("FoamCraft"),
    )
}

/// A folder name for a project name: characters a file system rejects become `_`, dots and
/// spaces at the ends are dropped (Windows cannot open such folders).
pub fn folder_name(name: &str) -> String {
    let cleaned: String = name
        .chars()
        .map(|c| match c {
            '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*' => '_',
            c if c.is_control() => '_',
            c => c,
        })
        .collect();
    let trimmed = cleaned.trim_matches(|c: char| c == '.' || c.is_whitespace());
    if trimmed.is_empty() {
        "Project".into()
    } else {
        trimmed.into()
    }
}

/// A folder for `name` inside `parent` that does not exist yet: `name`, then `name 2`, `name 3`
/// and so on. Nothing is created.
pub fn free_folder(parent: &Path, name: &str) -> PathBuf {
    let base = folder_name(name);
    let first = parent.join(&base);
    if !first.exists() {
        return first;
    }
    (2u32..)
        .map(|n| parent.join(format!("{base} {n}")))
        .find(|p| !p.exists())
        .unwrap_or(first)
}
