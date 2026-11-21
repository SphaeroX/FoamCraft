// SPDX-License-Identifier: GPL-3.0-or-later
//! Tests against the real Docker runtime. They need Docker and the runtime image
//! (`foamcraft runtime build`), so they are ignored by default:
//!     cargo test --workspace -- --ignored runtime_

use std::path::PathBuf;
use std::time::Instant;

use foamcraft_runtime::{Container, DockerRunner, Mount, Runner, StartOptions};

fn runner() -> DockerRunner {
    let r = DockerRunner::local();
    r.check()
        .expect("docker must be running for runtime_ tests");
    assert!(
        r.image_present().expect("image check"),
        "runtime image missing: run `foamcraft runtime build`"
    );
    r
}

/// Stops the container even when an assertion fails.
struct Guard<'a>(&'a DockerRunner, Container);

impl Drop for Guard<'_> {
    fn drop(&mut self) {
        let _ = self.0.stop(&self.1);
    }
}

fn container_exists(id: &str) -> bool {
    std::process::Command::new("docker")
        .args(["container", "inspect", id])
        .output()
        .is_ok_and(|o| o.status.success())
}

#[test]
#[ignore]
fn runtime_container_starts_and_openfoam_answers() {
    let r = runner();
    let c = r.start(&StartOptions::default()).expect("start");
    let _guard = Guard(&r, c.clone());
    let out = r
        .exec_openfoam(&c, "blockMesh -help", &mut |_| {})
        .expect("exec");
    assert_eq!(out.status, 0, "{}", out.stderr);
    assert!(out.stdout.contains("Usage: blockMesh"), "{}", out.stdout);
}

#[test]
#[ignore]
fn runtime_agent_reports_its_version_and_openfoam() {
    let r = runner();
    let c = r.start(&StartOptions::default()).expect("start");
    let _guard = Guard(&r, c.clone());
    let status = r.agent_status(&c).expect("agent status");
    assert_eq!(status.agent_version, foamcraft_core::VERSION);
    assert_eq!(status.openfoam.version(), "v2606");
    assert!(status.memory_bytes.is_some_and(|m| m > 0));
}

#[test]
#[ignore]
fn runtime_stop_removes_the_container() {
    let r = runner();
    let c = r.start(&StartOptions::default()).expect("start");
    assert!(container_exists(&c.id));
    r.stop(&c).expect("stop");
    assert!(!container_exists(&c.id));
}

#[test]
#[ignore]
fn runtime_core_limit_applies() {
    let r = runner();
    let c = r
        .start(&StartOptions {
            cpus: Some(2.0),
            ..Default::default()
        })
        .expect("start");
    let _guard = Guard(&r, c.clone());
    assert_eq!(r.agent_status(&c).expect("status").cpus, 2);
}

fn cavity_case() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../runtime/cases/cavity")
}

/// Measurement for the case file I/O decision (docs/architecture.md): the cavity case writes all
/// 100 time steps. Prints the time; asserts only that the run succeeds.
#[test]
#[ignore]
fn runtime_measure_case_io_on_volume() {
    let r = runner();
    let volume = format!("foamcraft-io-{}", std::process::id());
    r.create_volume(&volume).expect("volume");
    let c = r
        .start(&StartOptions {
            mount: Some(Mount::Volume(volume.clone())),
            ..Default::default()
        })
        .expect("start");
    let guard = Guard(&r, c.clone());
    let t0 = Instant::now();
    r.copy_into(&c, &cavity_case(), "/work/")
        .expect("copy case");
    let copied = t0.elapsed();
    let out = r
        .exec_openfoam(
            &c,
            "cd cavity && blockMesh > log.blockMesh && icoFoam > log.icoFoam",
            &mut |_| {},
        )
        .expect("run");
    let total = t0.elapsed();
    assert_eq!(out.status, 0, "{}", out.stderr);
    let count = r
        .exec(
            &c,
            &["sh", "-c", "ls -d /work/cavity/[0-9]* | wc -l"],
            &mut |_| {},
        )
        .expect("count");
    eprintln!(
        "IO volume: copy {:.2} s, total {:.2} s, {} time directories",
        copied.as_secs_f64(),
        total.as_secs_f64(),
        count.stdout.trim()
    );
    drop(guard);
    r.remove_volume(&volume).expect("remove volume");
}

fn copy_dir(from: &std::path::Path, to: &std::path::Path) {
    std::fs::create_dir_all(to).expect("create dir");
    for entry in std::fs::read_dir(from).expect("read dir").flatten() {
        let target = to.join(entry.file_name());
        if entry.path().is_dir() {
            copy_dir(&entry.path(), &target);
        } else {
            std::fs::copy(entry.path(), target).expect("copy file");
        }
    }
}

/// Same measurement with the case on a host folder mounted into the container. On Docker Desktop
/// the folder must be shared with Docker (a dialog asks once).
#[test]
#[ignore]
fn runtime_measure_case_io_on_bind_mount() {
    let r = runner();
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target/io-bench");
    let case = root.join("cavity");
    if case.exists() {
        std::fs::remove_dir_all(&case).expect("clean old case");
    }
    copy_dir(&cavity_case(), &case);
    let root = root.canonicalize().expect("absolute path");
    // Docker on Windows does not accept the `\\?\` prefix of canonical paths.
    let host = PathBuf::from(
        root.to_string_lossy()
            .trim_start_matches(r"\\?\")
            .to_string(),
    );
    let c = r
        .start(&StartOptions {
            mount: Some(Mount::Bind(host)),
            ..Default::default()
        })
        .expect("start");
    let _guard = Guard(&r, c.clone());
    let t0 = Instant::now();
    let out = r
        .exec_openfoam(
            &c,
            "cd cavity && blockMesh > log.blockMesh && icoFoam > log.icoFoam",
            &mut |_| {},
        )
        .expect("run");
    let total = t0.elapsed();
    assert_eq!(out.status, 0, "{}", out.stderr);
    eprintln!("IO bind mount: total {:.2} s", total.as_secs_f64());
}
