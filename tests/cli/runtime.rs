// SPDX-License-Identifier: GPL-3.0-or-later
use std::process::Command;

fn foamcraft(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_foamcraft"))
        .args(args)
        .output()
        .expect("run foamcraft")
}

#[test]
fn runtime_help_lists_check_build_and_stop() {
    let out = foamcraft(&["runtime", "--help"]);
    assert!(out.status.success());
    let text = String::from_utf8_lossy(&out.stdout);
    for cmd in ["check", "build", "stop"] {
        assert!(text.contains(cmd), "{cmd} missing:\n{text}");
    }
}

#[test]
#[ignore]
fn runtime_check_command_reports_openfoam_cores_memory_and_gpu() {
    let out = foamcraft(&["runtime", "check"]);
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(
        out.status.success(),
        "{text}\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
    for part in ["OpenFOAM v2606", "Cores:", "Memory:", "GPU:"] {
        assert!(text.contains(part), "{part} missing:\n{text}");
    }
}

#[test]
#[ignore]
fn runtime_check_command_speaks_german() {
    let out = foamcraft(&["--lang", "de", "runtime", "check"]);
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(
        text.contains("Kerne:") && text.contains("Arbeitsspeicher:"),
        "{text}"
    );
}

#[test]
fn run_and_job_commands_exist() {
    let out = foamcraft(&["run", "--help"]);
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(out.status.success());
    for opt in ["--iterations", "--cores", "--jobs-dir"] {
        assert!(text.contains(opt), "{opt} missing:\n{text}");
    }
    let out = foamcraft(&["job", "--help"]);
    let text = String::from_utf8_lossy(&out.stdout);
    for cmd in ["list", "log", "resume", "clean"] {
        assert!(text.contains(cmd), "{cmd} missing:\n{text}");
    }
}

#[test]
fn log_of_unknown_job_is_a_coded_error() {
    let dir = std::env::temp_dir().join(format!("fc-cli-jobs-{}", std::process::id()));
    let out = foamcraft(&["job", "log", "nope", "--jobs-dir", dir.to_str().unwrap()]);
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("job.not_found"));
}

#[test]
#[ignore]
fn runtime_run_command_shows_progress_and_keeps_the_log() {
    let dir = std::env::temp_dir().join(format!("fc-cli-run-{}", std::process::id()));
    let dir = dir.to_str().unwrap();
    let out = foamcraft(&["run", "--iterations", "30", "--jobs-dir", dir]);
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(
        out.status.success(),
        "{text}\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
    for part in ["mesh", "solve", "Iteration 30", "p "] {
        assert!(text.contains(part), "{part} missing:\n{text}");
    }
    let id = text
        .lines()
        .find_map(|l| l.strip_prefix("Job "))
        .expect("job id line")
        .trim()
        .to_string();
    let log = foamcraft(&["job", "log", &id, "--jobs-dir", dir]);
    assert!(String::from_utf8_lossy(&log.stdout).contains("[solve] End"));
    let list = foamcraft(&["job", "list", "--jobs-dir", dir]);
    assert!(String::from_utf8_lossy(&list.stdout).contains(&id));
    let clean = foamcraft(&["job", "clean", &id, "--jobs-dir", dir]);
    assert!(
        clean.status.success(),
        "{}",
        String::from_utf8_lossy(&clean.stderr)
    );
}

#[test]
#[ignore]
fn runtime_step_import_works_from_the_command_line() {
    let base = std::env::temp_dir().join(format!("fc-cli-step-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&base);
    let project = base.join("P");
    let step = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/data/cad/pipe_flange.step");
    let run = |args: &[&str]| {
        Command::new(env!("CARGO_BIN_EXE_foamcraft"))
            .args(args)
            .env("FOAMCRAFT_DATA_DIR", base.join("data"))
            .output()
            .expect("run foamcraft")
    };
    assert!(
        run(&["project", "new", project.to_str().unwrap()])
            .status
            .success()
    );
    let params = serde_json::json!({"path": step}).to_string();
    let out = run(&[
        "command",
        "run",
        "geometry.import",
        &params,
        "--project",
        project.to_str().unwrap(),
    ]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let info: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(info["format"], "step");
    let faces = run(&[
        "command",
        "run",
        "surface.list",
        "--project",
        project.to_str().unwrap(),
    ]);
    let faces: serde_json::Value = serde_json::from_slice(&faces.stdout).unwrap();
    assert_eq!(faces.as_array().unwrap().len(), 6);
}
