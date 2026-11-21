// SPDX-License-Identifier: GPL-3.0-or-later
//! Runner logic against the fake engine: states and error mapping.

use foamcraft_runtime::fake::FakeEngine;
use foamcraft_runtime::{
    Backend, CmdOutput, Container, DockerRunner, IMAGE_TAG, LogLine, Mount, Runner, StartOptions,
};

const GIB: u64 = 1024 * 1024 * 1024;

fn info_json(kernel: &str, mem: u64) -> String {
    format!(
        r#"{{"ServerVersion":"29.9.2","OperatingSystem":"Docker Desktop","KernelVersion":"{kernel}","NCPU":32,"MemTotal":{mem}}}"#
    )
}

fn runner(engine: FakeEngine, windows: bool) -> DockerRunner<FakeEngine> {
    DockerRunner::with_host(engine, windows)
}

fn container() -> Container {
    Container { id: "abc".into() }
}

#[test]
fn missing_docker_maps_to_docker_missing() {
    let r = runner(FakeEngine::missing_binary(), true);
    assert_eq!(r.check().unwrap_err().code, "runtime.docker_missing");
}

#[test]
fn stopped_service_maps_to_daemon_not_running() {
    let windows_message = "error during connect: Get \"http://%2F%2F.%2Fpipe%2FdockerDesktopLinuxEngine/v1.51/info\": open //./pipe/dockerDesktopLinuxEngine: The system cannot find the file specified.";
    let engine = FakeEngine::new().on(&["info"], CmdOutput::failed(1, windows_message));
    assert_eq!(
        runner(engine, true).check().unwrap_err().code,
        "runtime.daemon_not_running"
    );

    let linux_message = "Cannot connect to the Docker daemon at unix:///var/run/docker.sock. Is the docker daemon running?";
    let engine = FakeEngine::new().on(&["info"], CmdOutput::failed(1, linux_message));
    assert_eq!(
        runner(engine, false).check().unwrap_err().code,
        "runtime.daemon_not_running"
    );
}

#[test]
fn other_info_failures_map_to_command_failed() {
    let engine = FakeEngine::new().on(&["info"], CmdOutput::failed(1, "something odd"));
    assert_eq!(
        runner(engine, true).check().unwrap_err().code,
        "runtime.command_failed"
    );
}

#[test]
fn healthy_wsl2_backend_has_no_warnings() {
    let info = info_json("6.6.87.2-microsoft-standard-WSL2", 32 * GIB);
    let engine = FakeEngine::new().on(&["info"], CmdOutput::ok(&info));
    let report = runner(engine, true).check().unwrap();
    assert_eq!(report.backend, Backend::Wsl2);
    assert_eq!(report.cpus, 32);
    assert_eq!(report.memory_bytes, 32 * GIB);
    assert!(report.warnings.is_empty(), "{:?}", report.warnings);
}

#[test]
fn hyper_v_backend_and_little_memory_are_warnings_on_windows() {
    let info = info_json("7.0.14-linuxkit", 2 * GIB);
    let engine = FakeEngine::new().on(&["info"], CmdOutput::ok(&info));
    let report = runner(engine, true).check().unwrap();
    assert_eq!(report.backend, Backend::VirtualMachine);
    let codes: Vec<_> = report.warnings.iter().map(|w| w.code.as_str()).collect();
    assert_eq!(
        codes,
        ["runtime.no_wsl2_backend", "runtime.insufficient_memory"]
    );
}

#[test]
fn linux_host_needs_no_wsl2() {
    let info = info_json("6.8.0-generic", 64 * GIB);
    let engine = FakeEngine::new().on(&["info"], CmdOutput::ok(&info));
    let report = runner(engine, false).check().unwrap();
    assert!(report.warnings.is_empty());
}

#[test]
fn image_presence_comes_from_image_inspect() {
    let engine = FakeEngine::new().on(&["image", "inspect", IMAGE_TAG], CmdOutput::ok("[{}]"));
    assert!(runner(engine, true).image_present().unwrap());
    let engine = FakeEngine::new().on(
        &["image", "inspect", IMAGE_TAG],
        CmdOutput::failed(1, "Error: No such image"),
    );
    assert!(!runner(engine, true).image_present().unwrap());
}

#[test]
fn start_passes_limits_mount_gpu_and_label() {
    let engine = FakeEngine::new().on(&["run"], CmdOutput::ok("abc123\n"));
    let r = runner(engine, true);
    let opts = StartOptions {
        cpus: Some(4.0),
        memory_bytes: Some(2 * GIB),
        mount: Some(Mount::Volume("fc-work".into())),
        gpu: true,
    };
    let c = r.start(&opts).unwrap();
    assert_eq!(c.id, "abc123");
    let call = r
        .engine()
        .calls()
        .into_iter()
        .find(|c| c[0] == "run")
        .unwrap();
    let joined = call.join(" ");
    let memory = format!("--memory {}", 2 * GIB);
    for part in [
        "-d",
        "--init",
        "--cpus 4",
        memory.as_str(),
        "-v fc-work:/work",
        "--gpus all",
        "--label org.foamcraft.managed=true",
    ] {
        assert!(joined.contains(part), "{part} missing in {joined}");
    }
    assert!(joined.ends_with(&format!("{IMAGE_TAG} idle")), "{joined}");
}

#[test]
fn start_without_image_maps_to_image_missing() {
    let message = "Unable to find image 'foamcraft-runtime:0.1.0' locally\npull access denied";
    let engine = FakeEngine::new().on(&["run"], CmdOutput::failed(125, message));
    let err = runner(engine, true)
        .start(&StartOptions::default())
        .unwrap_err();
    assert_eq!(err.code, "runtime.image_missing");
}

#[test]
fn exec_streams_lines_and_returns_exit_code() {
    let engine = FakeEngine::new().on(&["exec"], CmdOutput::with_lines(3, &["a", "b"], &["warn"]));
    let r = runner(engine, true);
    let mut seen = Vec::new();
    let out = r
        .exec(&container(), &["blockMesh"], &mut |l| seen.push(l))
        .unwrap();
    assert_eq!(out.status, 3);
    assert_eq!(
        seen,
        [
            LogLine::Stdout("a".into()),
            LogLine::Stdout("b".into()),
            LogLine::Stderr("warn".into())
        ]
    );
    let call = r
        .engine()
        .calls()
        .into_iter()
        .find(|c| c[0] == "exec")
        .unwrap();
    assert_eq!(call, ["exec", "abc", "blockMesh"]);
}

#[test]
fn agent_status_parses_json() {
    let json = r#"{"agent_version":"0.1.0","openfoam":{"api":"2606","patch":"0","build":"x"},"cpus":4,"memory_bytes":1024,"gpu":{"available":false,"detail":"none"}}"#;
    let engine = FakeEngine::new().on(
        &["exec", "abc", "foamcraft-agent", "status"],
        CmdOutput::ok(json),
    );
    let s = runner(engine, true).agent_status(&container()).unwrap();
    assert_eq!(s.openfoam.version(), "v2606");
    assert_eq!(s.cpus, 4);
}

#[test]
fn broken_agent_answer_maps_to_agent_failed() {
    let engine = FakeEngine::new().on(&["exec"], CmdOutput::ok("not json"));
    let err = runner(engine, true).agent_status(&container()).unwrap_err();
    assert_eq!(err.code, "runtime.agent_failed");
}

#[test]
fn stop_removes_container_and_stop_all_finds_managed_ones() {
    let engine = FakeEngine::new()
        .on(&["ps"], CmdOutput::ok("id1\nid2\n"))
        .on(&["rm"], CmdOutput::ok(""));
    let r = runner(engine, true);
    r.stop(&container()).unwrap();
    assert_eq!(r.stop_all().unwrap(), 2);
    let calls = r.engine().calls();
    let as_strings = |v: &[&str]| v.iter().map(|s| s.to_string()).collect::<Vec<_>>();
    assert!(calls.contains(&as_strings(&["rm", "-f", "abc"])));
    assert!(calls.contains(&as_strings(&["rm", "-f", "id1", "id2"])));
    let ps = calls.iter().find(|c| c[0] == "ps").unwrap();
    assert!(ps.join(" ").contains("label=org.foamcraft.managed=true"));
}

#[test]
fn gpu_passthrough_is_probed_with_a_throwaway_container() {
    let engine = FakeEngine::new().on(&["run", "--rm", "--gpus", "all"], CmdOutput::ok(""));
    assert!(runner(engine, true).gpu_passthrough());
    let engine = FakeEngine::new().on(
        &["run", "--rm", "--gpus", "all"],
        CmdOutput::failed(125, "could not select device driver"),
    );
    assert!(!runner(engine, true).gpu_passthrough());
}

#[test]
fn volumes_are_created_labelled_and_removed() {
    let engine = FakeEngine::new()
        .on(&["volume", "create"], CmdOutput::ok("fc-1\n"))
        .on(&["volume", "rm"], CmdOutput::ok(""));
    let r = runner(engine, true);
    r.create_volume("fc-1").unwrap();
    r.remove_volume("fc-1").unwrap();
    let calls = r.engine().calls();
    assert!(
        calls[0]
            .join(" ")
            .contains("volume create --label org.foamcraft.managed=true fc-1")
    );
    assert_eq!(calls[1].join(" "), "volume rm -f fc-1");
}

#[test]
fn copy_into_and_out_of_container_use_docker_cp() {
    let engine = FakeEngine::new().on(&["cp"], CmdOutput::ok(""));
    let r = runner(engine, true);
    r.copy_into(&container(), std::path::Path::new("case"), "/work/")
        .unwrap();
    r.copy_out(&container(), "/work/case/log", std::path::Path::new("out"))
        .unwrap();
    let calls = r.engine().calls();
    assert_eq!(calls[0], ["cp", "case", "abc:/work/"]);
    assert_eq!(calls[1], ["cp", "abc:/work/case/log", "out"]);
}

mod stage_executor {
    use super::*;
    use foamcraft_core::job::{Stage, StageExecutor};
    use foamcraft_runtime::DockerStageExecutor;

    #[test]
    fn start_runs_the_stage_detached_through_the_agent() {
        let engine = FakeEngine::new().on(&["exec", "-d"], CmdOutput::ok(""));
        let r = runner(engine, true);
        let mut exec = DockerStageExecutor::new(&r, container(), "/work/case");
        exec.start(Stage::Mesh, "snappyHexMesh -overwrite").unwrap();
        let call = &r.engine().calls()[0];
        assert_eq!(
            call,
            &[
                "exec",
                "-d",
                "abc",
                "foamcraft-agent",
                "run-stage",
                "/work/case",
                "mesh",
                "snappyHexMesh -overwrite"
            ]
        );
    }

    #[test]
    fn poll_reads_the_tail_answer() {
        let json = r#"{"text":"Time = 1\n","next_offset":9,"exit":0}"#;
        let engine = FakeEngine::new().on(
            &["exec", "abc", "foamcraft-agent", "tail"],
            CmdOutput::ok(json),
        );
        let r = runner(engine, true);
        let mut exec = DockerStageExecutor::new(&r, container(), "/work/case");
        let poll = exec.poll(Stage::Solve, 42).unwrap();
        assert_eq!(poll.text, "Time = 1\n");
        assert_eq!(poll.exit, Some(0));
        assert_eq!(
            r.engine().calls()[0],
            [
                "exec",
                "abc",
                "foamcraft-agent",
                "tail",
                "/work/case",
                "solve",
                "42"
            ]
        );
    }

    #[test]
    fn poll_of_a_vanished_container_is_run_lost() {
        let engine = FakeEngine::new().on(
            &["exec"],
            CmdOutput::failed(1, "Error response from daemon: No such container: abc"),
        );
        let r = runner(engine, true);
        let mut exec = DockerStageExecutor::new(&r, container(), "/work/case");
        assert_eq!(exec.poll(Stage::Solve, 0).unwrap_err().code, "run.lost");
    }

    #[test]
    fn cancel_removes_the_container() {
        let engine = FakeEngine::new().on(&["rm"], CmdOutput::ok(""));
        let r = runner(engine, true);
        let mut exec = DockerStageExecutor::new(&r, container(), "/work/case");
        exec.cancel().unwrap();
        assert_eq!(r.engine().calls()[0], ["rm", "-f", "abc"]);
    }
}
