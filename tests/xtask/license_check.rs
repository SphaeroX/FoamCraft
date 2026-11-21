// SPDX-License-Identifier: GPL-3.0-or-later
use std::path::PathBuf;
use std::process::Command;

use foamcraft_xtask::license::{Package, Policy, expression_allowed, violations};

fn policy() -> Policy {
    Policy::new(["MIT", "Apache-2.0", "GPL-3.0-or-later", "GPL-2.0-or-later"])
}

#[test]
fn simple_allowed_and_forbidden_licenses() {
    let p = policy();
    assert!(expression_allowed("MIT", &p));
    assert!(expression_allowed("mit", &p));
    assert!(!expression_allowed("GPL-2.0-only", &p));
    assert!(!expression_allowed("SSPL-1.0", &p));
    assert!(!expression_allowed("", &p));
}

#[test]
fn or_needs_one_allowed_and_needs_all() {
    let p = policy();
    assert!(expression_allowed("MIT OR SSPL-1.0", &p));
    assert!(expression_allowed("MIT/Apache-2.0", &p));
    assert!(!expression_allowed("MIT AND SSPL-1.0", &p));
    assert!(expression_allowed("(MIT OR SSPL-1.0) AND Apache-2.0", &p));
    assert!(!expression_allowed("(MIT OR SSPL-1.0) AND BUSL-1.1", &p));
}

#[test]
fn with_exception_and_plus_suffix() {
    let p = policy();
    assert!(expression_allowed("Apache-2.0 WITH LLVM-exception", &p));
    assert!(expression_allowed("GPL-2.0+", &p));
    assert!(expression_allowed("GPL-3.0+", &p));
}

#[test]
fn reports_packages_without_allowed_license() {
    let p = policy();
    let packages = vec![
        Package::new("cargo", "good", "1.0.0", Some("MIT")),
        Package::new("npm", "bad", "2.0.0", Some("SSPL-1.0")),
        Package::new("cargo", "none", "0.1.0", None),
    ];
    let found: Vec<_> = violations(&packages, &p)
        .into_iter()
        .map(|v| v.name.clone())
        .collect();
    assert_eq!(found, ["bad", "none"]);
}

#[test]
fn exceptions_name_single_packages() {
    let mut p = policy();
    p.exceptions
        .insert("none".into(), "license file checked by hand".into());
    let packages = vec![Package::new("cargo", "none", "0.1.0", None)];
    assert!(violations(&packages, &p).is_empty());
}

fn data(name: &str) -> String {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/data/license")
        .join(name)
        .to_string_lossy()
        .into_owned()
}

fn run(lock: &str) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_foamcraft-xtask"))
        .args(["license-check", "--skip-cargo", "--npm-lock", &data(lock)])
        .output()
        .expect("run xtask")
}

#[test]
fn command_passes_with_allowed_licenses() {
    let out = run("allowed-lock.json");
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn command_fails_with_a_forbidden_license() {
    let out = run("forbidden-lock.json");
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("evil-db"));
}
