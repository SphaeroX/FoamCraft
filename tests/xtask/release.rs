// SPDX-License-Identifier: GPL-3.0-or-later
//! Release scripts: the license bundle lists every dependency of both lockfiles, and the version
//! of the workspace, the app package and the Tauri configuration agree (the runtime image tag is
//! derived from the workspace version, see tests/runtime/runner.rs).

use std::path::PathBuf;
use std::process::Command;

use foamcraft_xtask::license::{cargo_packages, npm_packages};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

#[test]
fn the_license_bundle_holds_every_dependency_of_both_lockfiles() {
    let out = std::env::temp_dir().join("foamcraft-notices-test");
    let _ = std::fs::remove_dir_all(&out);
    let status = Command::new(env!("CARGO_BIN_EXE_foamcraft-xtask"))
        .args(["notices", &out.to_string_lossy()])
        .status()
        .expect("run xtask");
    assert!(status.success());
    let text = std::fs::read_to_string(out.join("THIRD-PARTY-NOTICES.txt")).unwrap();
    let mut packages = npm_packages(&root().join("package-lock.json")).unwrap();
    packages.extend(cargo_packages(&root()).unwrap());
    assert!(packages.len() > 100);
    for p in &packages {
        let line = format!("{} {} ({})", p.name, p.version, p.ecosystem);
        assert!(text.contains(&line), "{line} missing in the notices");
    }
    // License texts are included, not only the names.
    assert!(text.contains("Permission is hereby granted"));
    // Source reference, trademark notice, the project licence and the font licence ship along.
    let notice = std::fs::read_to_string(out.join("NOTICE.txt")).unwrap();
    assert!(notice.contains("https://develop.openfoam.com/Development/openfoam/-/tree/OpenFOAM-v"));
    assert!(notice.contains("https://github.com/SphaeroX/FoamCraft"));
    assert!(notice.contains("not approved or endorsed by OpenCFD Limited"));
    assert!(out.join("LICENSE.txt").is_file());
    assert!(out.join("OFL.txt").is_file());
}

#[test]
fn versions_of_workspace_app_and_tauri_agree() {
    let out = Command::new(env!("CARGO_BIN_EXE_foamcraft-xtask"))
        .arg("version")
        .output()
        .expect("run xtask");
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let version = String::from_utf8(out.stdout).unwrap().trim().to_string();
    assert_eq!(version, env!("CARGO_PKG_VERSION"));
    let app: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(root().join("app/package.json")).unwrap())
            .unwrap();
    assert_eq!(app["version"], version);
    let tauri: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(root().join("app/src-tauri/tauri.conf.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(tauri["version"], version);
}
