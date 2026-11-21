// SPDX-License-Identifier: GPL-3.0-or-later
//! The system check of the setup assistant against simulated environments, the hardware
//! profile for sample computers, and the diagnostic report.

use std::sync::Arc;

use foamcraft_api::system::{Probe, profile, steps};
use foamcraft_runtime::fake::FakeEngine;
use foamcraft_runtime::{CmdOutput, DockerRunner};
use serde_json::Value;

const GIB: u64 = 1024 * 1024 * 1024;

fn info(kernel: &str, memory: u64) -> CmdOutput {
    CmdOutput::ok(&format!(
        r#"{{"ServerVersion":"29.1","OperatingSystem":"Docker Desktop","KernelVersion":"{kernel}","NCPU":8,"MemTotal":{memory}}}"#
    ))
}

fn probe(gpu: Option<&str>, free: u64) -> Probe {
    Probe {
        gpu: gpu.map(String::from),
        free_disk_bytes: Some(free),
    }
}

fn state(list: &[Value], id: &str) -> String {
    list.iter()
        .find(|s| s["id"] == id)
        .unwrap_or_else(|| panic!("{id} missing"))["state"]
        .as_str()
        .unwrap()
        .to_string()
}

fn texts_complete(list: &[Value]) {
    for s in list {
        for lang in ["en", "de"] {
            assert!(!s[lang]["text"].as_str().unwrap().is_empty(), "{s}");
            if s["state"] == "action" {
                assert!(!s[lang]["fix"].as_str().unwrap().is_empty(), "{s}");
                assert!(
                    s[lang]["help"].as_array().is_some_and(|h| h.len() >= 2),
                    "{s}"
                );
            }
        }
    }
}

#[test]
fn without_docker_the_first_step_is_to_install_it() {
    let runner = Arc::new(DockerRunner::with_host(FakeEngine::missing_binary(), true));
    let list = steps(runner.as_ref(), &probe(None, 100 * GIB));
    assert_eq!(state(&list, "docker"), "action");
    assert_eq!(list[0]["code"], "runtime.docker_missing");
    assert_eq!(state(&list, "service"), "skipped");
    assert_eq!(state(&list, "image"), "skipped");
    texts_complete(&list);
}

#[test]
fn a_stopped_service_is_explained_and_recognised_once_fixed() {
    let stopped = FakeEngine::new().on(
        &["info"],
        CmdOutput::failed(1, "error during connect: open //./pipe/dockerDesktopLinuxEngine: The system cannot find the file specified."),
    );
    let list = steps(
        &DockerRunner::with_host(stopped, true),
        &probe(None, 100 * GIB),
    );
    assert_eq!(state(&list, "docker"), "ok");
    assert_eq!(state(&list, "service"), "action");
    let service = list.iter().find(|s| s["id"] == "service").unwrap();
    assert_eq!(service["code"], "runtime.daemon_not_running");
    // The help explains a missing WSL distribution, the state of the reference computer.
    let help = service["de"]["help"].to_string();
    assert!(help.contains("WSL"), "{help}");
    texts_complete(&list);

    // After the user started Docker the same check passes.
    let running = FakeEngine::new()
        .on(
            &["info"],
            info("6.6.87.2-microsoft-standard-WSL2", 16 * GIB),
        )
        .on(&["image", "inspect"], CmdOutput::ok("[]"));
    let list = steps(
        &DockerRunner::with_host(running, true),
        &probe(Some("NVIDIA RTX 4080"), 100 * GIB),
    );
    for id in ["docker", "service", "backend", "memory", "image", "disk"] {
        assert_eq!(state(&list, id), "ok", "{id}");
    }
    assert_eq!(state(&list, "gpu"), "ok");
}

#[test]
fn little_memory_no_gpu_and_no_image_are_reported() {
    let engine = FakeEngine::new()
        .on(&["info"], info("5.15.0-hyperv", 2 * GIB))
        .on(
            &["image", "inspect"],
            CmdOutput::failed(1, "Error: No such image: foamcraft-runtime:0.1.0"),
        );
    let list = steps(
        &DockerRunner::with_host(engine, true),
        &probe(None, 5 * GIB),
    );
    assert_eq!(state(&list, "backend"), "warning");
    assert_eq!(state(&list, "memory"), "warning");
    assert_eq!(state(&list, "image"), "action");
    assert_eq!(state(&list, "disk"), "warning");
    assert_eq!(state(&list, "gpu"), "info");
    texts_complete(&list);
}

#[test]
fn the_hardware_profile_gives_defaults_for_sample_computers() {
    // Laptop: 8 physical cores, Docker with 16 GiB.
    let p = profile(8, 32 * GIB, 8, 16 * GIB);
    assert_eq!(p.default_cores, 8);
    assert_eq!(p.max_cells, 6_400_000);
    assert_eq!(p.quality, "standard");
    // Small machine: Docker with 4 GiB and 4 cores.
    let p = profile(4, 8 * GIB, 4, 4 * GIB);
    assert_eq!(
        (p.default_cores, p.max_cells, p.quality.as_str()),
        (4, 1_600_000, "draft")
    );
    // Workstation, but Docker may use only 6 of 16 cores.
    let p = profile(16, 64 * GIB, 6, 32 * GIB);
    assert_eq!((p.default_cores, p.quality.as_str()), (6, "fine"));
    assert_eq!(p.max_cells, 12_800_000);
}

#[test]
fn the_diagnostic_report_is_plain_text_without_the_user_folder() {
    let reg = foamcraft_api::registry_with(Arc::new(DockerRunner::with_host(
        FakeEngine::missing_binary(),
        true,
    )));
    let base = std::env::temp_dir().join(format!("fc-system-{}", std::process::id()));
    let mut s = foamcraft_core::command::Session::new(base.join("data"));
    let out = reg
        .execute(&mut s, "system.diagnostics", serde_json::json!({}))
        .unwrap();
    let text = out["text"].as_str().unwrap();
    assert!(text.contains("FoamCraft"));
    assert!(text.contains("runtime.docker_missing"));
    assert!(text.contains("physical cores"));
    if let Some(home) = std::env::var_os("USERPROFILE").or_else(|| std::env::var_os("HOME")) {
        assert!(
            !text.contains(&*home.to_string_lossy()),
            "home folder leaked"
        );
    }
    // The check as a command: not ready without Docker.
    let check = reg
        .execute(&mut s, "system.check", serde_json::json!({}))
        .unwrap();
    assert_eq!(check["ready"], false);
    assert!(check["profile"]["physical_cores"].as_u64().unwrap() >= 1);
}

#[test]
fn the_wording_of_current_docker_versions_is_recognised_as_a_stopped_service() {
    let stopped = FakeEngine::new().on(
        &["info"],
        CmdOutput::failed(
            1,
            "failed to connect to the docker API at npipe:////./pipe/dockerDesktopLinuxEngine; check if the path is correct and if the daemon is running: open //./pipe/dockerDesktopLinuxEngine: The system cannot find the file specified.",
        ),
    );
    let list = steps(
        &DockerRunner::with_host(stopped, true),
        &probe(None, 100 * GIB),
    );
    let service = list.iter().find(|s| s["id"] == "service").unwrap();
    assert_eq!(service["code"], "runtime.daemon_not_running");
    // Also with a different endpoint name.
    let other = FakeEngine::new().on(
        &["info"],
        CmdOutput::failed(1, "failed to connect to the docker API at npipe:////./pipe/other; check if the path is correct and if the daemon is running"),
    );
    let list = steps(
        &DockerRunner::with_host(other, true),
        &probe(None, 100 * GIB),
    );
    assert_eq!(
        list.iter().find(|s| s["id"] == "service").unwrap()["code"],
        "runtime.daemon_not_running"
    );
}
