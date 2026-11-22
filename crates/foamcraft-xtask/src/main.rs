// SPDX-License-Identifier: GPL-3.0-or-later
//! `cargo xtask license-check`: checks Rust and npm dependency licenses.
//! `cargo xtask notices <dir>`: writes the license bundle shipped with the installers.
//! `cargo xtask version`: prints the project version.

use std::path::PathBuf;
use std::process::ExitCode;

use foamcraft_xtask::license::{Policy, cargo_packages, npm_packages, violations};
use foamcraft_xtask::release::{notice, third_party_notices, workspace_version};

const USAGE: &str =
    "usage: cargo xtask license-check [--skip-cargo] [--policy FILE] [--npm-lock FILE]
       cargo xtask notices DIR
       cargo xtask version";

/// OpenFOAM release in the runtime image (`OPENFOAM_API` in runtime/Dockerfile).
fn openfoam_release(root: &std::path::Path) -> Result<String, String> {
    let text = std::fs::read_to_string(root.join("runtime/Dockerfile"))
        .map_err(|e| format!("runtime/Dockerfile: {e}"))?;
    text.lines()
        .find_map(|l| l.trim().strip_prefix("ARG OPENFOAM_API="))
        .map(|v| format!("v{}", v.trim()))
        .ok_or_else(|| "runtime/Dockerfile: no ARG OPENFOAM_API".into())
}

fn version(root: &std::path::Path) -> Result<String, String> {
    let text = std::fs::read_to_string(root.join("Cargo.toml")).map_err(|e| e.to_string())?;
    workspace_version(&text).ok_or_else(|| "Cargo.toml: no workspace version".into())
}

fn write_notices(args: &[String]) -> Result<bool, String> {
    let root = repo_root();
    let out = PathBuf::from(args.first().ok_or(USAGE)?);
    std::fs::create_dir_all(&out).map_err(|e| format!("{}: {e}", out.display()))?;
    let mut packages = npm_packages(&root.join("package-lock.json"))?;
    packages.extend(cargo_packages(&root)?);
    let write = |name: &str, text: &str| {
        std::fs::write(out.join(name), text).map_err(|e| format!("{name}: {e}"))
    };
    write("THIRD-PARTY-NOTICES.txt", &third_party_notices(&packages))?;
    write(
        "NOTICE.txt",
        &notice(&version(&root)?, &openfoam_release(&root)?),
    )?;
    let copy = |from: &str, to: &str| {
        std::fs::copy(root.join(from), out.join(to)).map_err(|e| format!("{from}: {e}"))
    };
    copy("LICENSE", "LICENSE.txt")?;
    copy("assets/fonts/OFL.txt", "OFL.txt")?;
    println!(
        "wrote the notices of {} packages to {}",
        packages.len(),
        out.display()
    );
    Ok(true)
}

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
        Some("notices") => write_notices(&args[1..]),
        Some("version") => version(&repo_root()).map(|v| {
            println!("{v}");
            true
        }),
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
