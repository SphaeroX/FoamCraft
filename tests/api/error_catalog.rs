// SPDX-License-Identifier: GPL-3.0-or-later
//! Every error and hint code FoamCraft can report has an English and a German text and a
//! suggestion in the catalog (`crates/foamcraft-core/data/errors.json`).

use std::path::{Path, PathBuf};

use foamcraft_core::error::{Language, message};

fn sources(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).unwrap().flatten() {
        let path = entry.path();
        if path.is_dir() {
            sources(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    }
}

/// String literals that look like codes (`domain.problem`) after the given prefixes.
fn codes_in(text: &str, prefixes: &[&str]) -> Vec<String> {
    let mut codes = Vec::new();
    for prefix in prefixes {
        for part in text.split(prefix).skip(1) {
            // The code is the first argument, possibly on the next line.
            let Some(rest) = part.trim_start().strip_prefix('"') else {
                continue;
            };
            let Some(code) = rest.split('"').next() else {
                continue;
            };
            let valid = code.split_once('.').is_some_and(|(d, p)| {
                !d.is_empty()
                    && !p.is_empty()
                    && code
                        .chars()
                        .all(|c| c.is_ascii_lowercase() || c == '_' || c == '.')
            });
            if valid {
                codes.push(code.to_string());
            }
        }
    }
    codes
}

fn assert_complete(code: &str, origin: &str) {
    for lang in [Language::En, Language::De] {
        let m = message(code, lang).unwrap_or_else(|| panic!("{code} ({origin}) has no text"));
        assert!(!m.text.trim().is_empty(), "{code} {lang:?} text");
        assert!(!m.fix.trim().is_empty(), "{code} {lang:?} suggestion");
    }
}

#[test]
fn every_code_raised_in_the_sources_is_in_the_catalog() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut files = Vec::new();
    for dir in ["crates", "app/src-tauri/src"] {
        sources(&root.join(dir), &mut files);
    }
    let mut count = 0;
    for file in files {
        let text = std::fs::read_to_string(&file).unwrap();
        for code in codes_in(&text, &["FoamError::new("]) {
            assert_complete(&code, &file.display().to_string());
            count += 1;
        }
    }
    assert!(count > 50, "only {count} codes found");
}

#[test]
fn every_code_a_command_declares_is_in_the_catalog() {
    let reg = foamcraft_api::registry();
    for spec in reg.specs() {
        for code in spec.errors {
            assert_complete(code, spec.name);
        }
    }
}

#[test]
fn every_hint_of_a_result_is_in_the_catalog() {
    for code in [
        "run.stable",
        "run.oscillating",
        "run.limit_reached",
        "run.restarted",
        "run.y_plus_low",
        "run.y_plus_high",
        "mesh.fallback_fewer_layers",
        "mesh.fallback_no_layers",
        "mesh.fallback_coarser",
    ] {
        assert_complete(code, "hint");
    }
}
