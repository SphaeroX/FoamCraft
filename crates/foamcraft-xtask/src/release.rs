// SPDX-License-Identifier: GPL-3.0-or-later
//! Release files: the license bundle shipped with every installer and the project version.

use std::collections::BTreeSet;
use std::fmt::Write as _;
use std::path::Path;

use crate::license::Package;

/// Notice on the trade marks of OpenCFD Limited (docs/licensing.md).
pub const TRADEMARK_NOTICE: &str = "This offering is not approved or endorsed by OpenCFD Limited, \
producer and distributor of the OpenFOAM software via www.openfoam.com, and owner of the \
OPENFOAM® and OpenCFD® trade marks.";

/// Where the source of FoamCraft and of the OpenFOAM release in the runtime image is.
pub fn notice(version: &str, openfoam: &str) -> String {
    format!(
        "FoamCraft {version}\n\
         Copyright the FoamCraft contributors. Licensed under the GNU General Public License,\n\
         version 3 or later (LICENSE.txt). Source code: https://github.com/SphaeroX/FoamCraft\n\
         (tag v{version}).\n\n\
         FoamCraft uses OpenFOAM {openfoam} (GPL-3.0-or-later), which runs inside the FoamCraft\n\
         runtime image ghcr.io/sphaerox/foamcraft-runtime:{version}. The image is built from\n\
         runtime/Dockerfile in the FoamCraft repository; the corresponding OpenFOAM source is at\n\
         https://develop.openfoam.com/Development/openfoam/-/tree/OpenFOAM-{openfoam}\n\n\
         {TRADEMARK_NOTICE}\n\n\
         The report font Libertinus Serif is licensed under the SIL Open Font License 1.1\n\
         (OFL.txt). The licenses of all other components are in THIRD-PARTY-NOTICES.txt.\n"
    )
}

fn is_license_file(name: &str) -> bool {
    let upper = name.to_ascii_uppercase();
    ["LICENSE", "LICENCE", "COPYING", "NOTICE", "UNLICENSE"]
        .iter()
        .any(|p| upper.starts_with(p))
}

/// License texts in the folder of a package, by file name.
fn license_texts(dir: &Path) -> Vec<(String, String)> {
    let mut found: Vec<(String, String)> = std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .filter(|e| e.path().is_file())
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().into_owned();
            if !is_license_file(&name) {
                return None;
            }
            std::fs::read_to_string(e.path()).ok().map(|t| (name, t))
        })
        .collect();
    found.sort();
    found
}

/// The text of THIRD-PARTY-NOTICES.txt: every package with its license and license files.
pub fn third_party_notices(packages: &[Package]) -> String {
    let mut seen = BTreeSet::new();
    let mut out = String::from(
        "Third-party components of FoamCraft\n\
         ===================================\n\n\
         Every Rust crate and npm package FoamCraft is built from, with its license and the\n\
         license files the package ships.\n",
    );
    let mut sorted: Vec<&Package> = packages.iter().collect();
    sorted.sort_by(|a, b| {
        (&a.name, &a.version, &a.ecosystem).cmp(&(&b.name, &b.version, &b.ecosystem))
    });
    for p in sorted {
        if !seen.insert((p.ecosystem.clone(), p.name.clone(), p.version.clone())) {
            continue;
        }
        let _ = write!(
            out,
            "\n------------------------------------------------------------------------\n\
             {} {} ({})\nLicense: {}\n",
            p.name,
            p.version,
            p.ecosystem,
            p.license.as_deref().unwrap_or("see license files")
        );
        for (name, text) in p.dir.as_deref().map(license_texts).unwrap_or_default() {
            let _ = write!(out, "\n--- {name} ---\n{}\n", text.trim_end());
        }
    }
    out
}

/// The version in the `[workspace.package]` table of the root Cargo.toml.
pub fn workspace_version(cargo_toml: &str) -> Option<String> {
    let mut in_table = false;
    for line in cargo_toml.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            in_table = line == "[workspace.package]";
            continue;
        }
        if in_table
            && let Some(rest) = line.strip_prefix("version")
            && let Some(value) = rest.trim_start().strip_prefix('=')
        {
            return Some(value.trim().trim_matches('"').to_string());
        }
    }
    None
}
