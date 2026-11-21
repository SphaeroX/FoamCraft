// SPDX-License-Identifier: GPL-3.0-or-later
//! `cargo xtask license-check`: checks Rust and npm dependency licenses.

use std::path::PathBuf;
use std::process::ExitCode;

use foamcraft_xtask::license::{Policy, cargo_packages, npm_packages, violations};

const USAGE: &str =
    "usage: cargo xtask license-check [--skip-cargo] [--policy FILE] [--npm-lock FILE]";

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn license_check(args: &[String]) -> Result<bool, String> {
    let root = repo_root();
    let mut policy_path = root.join("license-policy.json");
    let mut npm_lock = root.join("package-lock.json");
    let mut skip_cargo = false;
    let mut it = args.iter();
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "--skip-cargo" => skip_cargo = true,
            "--policy" => policy_path = it.next().ok_or("--policy needs a path")?.into(),
            "--npm-lock" => npm_lock = it.next().ok_or("--npm-lock needs a path")?.into(),
            other => return Err(format!("unknown argument {other}\n{USAGE}")),
        }
    }
    let policy = Policy::load(&policy_path)?;
    let mut packages = npm_packages(&npm_lock)?;
    if !skip_cargo {
        packages.extend(cargo_packages(&root)?);
    }
    let bad = violations(&packages, &policy);
    for p in &bad {
        eprintln!(
            "license not allowed: {} {} {} ({})",
            p.ecosystem,
            p.name,
            p.version,
            p.license.as_deref().unwrap_or("no license")
        );
    }
    println!(
        "checked {} packages, {} not allowed",
        packages.len(),
        bad.len()
    );
    Ok(bad.is_empty())
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let result = match args.first().map(String::as_str) {
        Some("license-check") => license_check(&args[1..]),
        _ => Err(USAGE.into()),
    };
    match result {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::FAILURE,
        Err(e) => {
            eprintln!("xtask: {e}");
            ExitCode::from(2)
        }
    }
}
