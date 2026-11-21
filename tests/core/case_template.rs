// SPDX-License-Identifier: GPL-3.0-or-later
use foamcraft_core::case::{REFERENCE_CUBE, render_template};

#[test]
fn reference_case_contains_the_files_openfoam_needs() {
    let files: Vec<_> = REFERENCE_CUBE.files.iter().map(|(p, _)| *p).collect();
    for needed in [
        "system/controlDict",
        "system/blockMeshDict",
        "system/snappyHexMeshDict",
        "system/fvSchemes",
        "system/fvSolution",
        "system/decomposeParDict",
        "constant/transportProperties",
        "constant/turbulenceProperties",
        "0/U",
        "0/p",
        "0/k",
        "0/omega",
        "0/nut",
    ] {
        assert!(files.contains(&needed), "{needed} missing");
    }
}

#[test]
fn writing_replaces_every_placeholder() {
    let dir = std::env::temp_dir().join(format!("fc-case-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    REFERENCE_CUBE.write(&dir, &[("END_TIME", "150")]).unwrap();
    let control = std::fs::read_to_string(dir.join("system/controlDict")).unwrap();
    assert!(control.contains("endTime         150;"), "{control}");
    for (path, _) in REFERENCE_CUBE.files {
        let text = std::fs::read_to_string(dir.join(path)).unwrap();
        assert!(!text.contains("{{"), "placeholder left in {path}");
    }
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn missing_values_use_template_defaults() {
    let text = render_template("a {{X}} b {{Y}}", &[("X", "1")], &[("Y", "2")]);
    assert_eq!(text, "a 1 b 2");
}
