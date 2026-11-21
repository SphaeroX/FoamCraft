// SPDX-License-Identifier: GPL-3.0-or-later
use foamcraft_core::error::{FoamError, Language, catalog, message};

#[test]
fn every_catalog_entry_has_english_and_german_text_and_fix() {
    let entries = catalog();
    assert!(!entries.is_empty());
    for (code, entry) in entries {
        for lang in [Language::En, Language::De] {
            let m = entry.get(lang);
            assert!(!m.text.trim().is_empty(), "{code} {lang:?} text");
            assert!(!m.fix.trim().is_empty(), "{code} {lang:?} fix");
        }
        assert_ne!(entry.en.text, entry.de.text, "{code} is not translated");
        let (domain, problem) = code.split_once('.').expect("code is domain.problem");
        assert!(!domain.is_empty() && !problem.is_empty());
    }
}

#[test]
fn runtime_failure_codes_exist() {
    for code in [
        "runtime.docker_missing",
        "runtime.daemon_not_running",
        "runtime.no_wsl2_backend",
        "runtime.insufficient_memory",
        "runtime.image_missing",
        "runtime.command_failed",
    ] {
        assert!(message(code, Language::De).is_some(), "{code}");
    }
}

#[test]
fn error_displays_code_and_detail() {
    let err = FoamError::new("runtime.docker_missing", "docker: not found");
    assert_eq!(err.code, "runtime.docker_missing");
    let text = err.to_string();
    assert!(text.contains("runtime.docker_missing") && text.contains("docker: not found"));
    assert!(err.localized(Language::De).text.contains("Docker"));
}

#[test]
fn unknown_codes_have_no_message() {
    assert!(message("nope.nothing", Language::En).is_none());
}
