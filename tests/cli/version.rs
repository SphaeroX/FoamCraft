// SPDX-License-Identifier: GPL-3.0-or-later
use std::process::Command;

#[test]
fn version_flag_prints_workspace_version() {
    let output = Command::new(env!("CARGO_BIN_EXE_foamcraft"))
        .arg("--version")
        .output()
        .expect("run foamcraft");
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("utf-8");
    assert_eq!(
        stdout.trim(),
        format!("foamcraft {}", env!("CARGO_PKG_VERSION"))
    );
}
