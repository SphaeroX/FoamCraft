// SPDX-License-Identifier: GPL-3.0-or-later
//! Log parsing against logs recorded with OpenFOAM v2606 (tests/data/logs).

use foamcraft_core::log::{LogEvent, LogParser, LogSummary, parse_all};

fn log(name: &str) -> String {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/data/logs")
        .join(name);
    std::fs::read_to_string(path).expect("read log")
}

fn summary(name: &str) -> LogSummary {
    LogSummary::from_events(&parse_all(&log(name)))
}

#[test]
fn simple_foam_iterations_residuals_and_end() {
    let s = summary("simpleFoam_ok.log");
    assert_eq!(s.last_iteration, Some(200.0));
    assert_eq!(s.iterations, 21);
    assert!(s.errors.is_empty(), "{:?}", s.errors);
    assert!(s.ended);
    for field in ["Ux", "Uy", "Uz", "p", "k", "omega"] {
        assert!(s.residuals.contains_key(field), "{field} missing");
    }
    assert!(s.execution_time.is_some_and(|t| t > 0.0));
}

#[test]
fn first_iteration_residuals_are_exact() {
    let events = parse_all(&log("simpleFoam_ok.log"));
    let first: Vec<_> = events
        .iter()
        .skip_while(|e| !matches!(e, LogEvent::Iteration { .. }))
        .take_while(|e| !matches!(e, LogEvent::ExecutionTime { .. }))
        .cloned()
        .collect();
    assert_eq!(first[0], LogEvent::Iteration { time: 1.0 });
    assert!(first.contains(&LogEvent::Residual {
        field: "Ux".into(),
        initial: 1.0
    }));
    assert!(first.contains(&LogEvent::Residual {
        field: "omega".into(),
        initial: 0.064217322
    }));
}

#[test]
fn block_mesh_and_decompose_end_cleanly() {
    for name in [
        "blockMesh_ok.log",
        "decomposePar_ok.log",
        "reconstructPar_ok.log",
    ] {
        let s = summary(name);
        assert!(s.ended, "{name}");
        assert!(s.errors.is_empty(), "{name}: {:?}", s.errors);
    }
}

#[test]
fn snappy_hex_mesh_phases() {
    let events = parse_all(&log("snappyHexMesh_ok.log"));
    let phases: Vec<_> = events
        .iter()
        .filter_map(|e| match e {
            LogEvent::Phase(p) => Some(p.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(phases, ["refine", "snap"]);
    assert!(summary("snappyHexMesh_ok.log").ended);
}

#[test]
fn check_mesh_reports_cells_and_ok() {
    let s = summary("checkMesh_ok.log");
    assert_eq!(s.cells, Some(15504));
    assert_eq!(s.mesh_ok, Some(true));
}

#[test]
fn failed_mesh_checks_are_an_error() {
    let text = "    cells:            100\n\nFailed 2 mesh checks.\n\nEnd\n";
    let s = LogSummary::from_events(&parse_all(text));
    assert_eq!(s.mesh_ok, Some(false));
    assert_eq!(s.errors[0].code, "mesh.check_failed");
}

#[test]
fn fatal_io_error_is_reported_with_its_message() {
    let s = summary("snappyHexMesh_fatal_io_error.log");
    assert!(!s.ended);
    assert_eq!(s.errors.len(), 1);
    assert_eq!(s.errors[0].code, "run.foam_fatal_error");
    assert!(
        s.errors[0].detail.contains("errorReduction"),
        "{}",
        s.errors[0].detail
    );
}

#[test]
fn floating_point_exception_means_divergence() {
    let s = summary("icoFoam_sigfpe.log");
    assert_eq!(s.errors.len(), 1, "{:?}", s.errors);
    assert_eq!(s.errors[0].code, "run.diverged");
}

#[test]
fn nan_residuals_mean_divergence_once() {
    let s = summary("icoFoam_nan.log");
    assert_eq!(s.errors[0].code, "run.diverged");
    assert_eq!(
        s.errors.iter().filter(|e| e.code == "run.diverged").count(),
        1
    );
}

#[test]
fn parser_works_line_by_line() {
    let mut p = LogParser::default();
    assert!(
        p.feed("Time = 7")
            .contains(&LogEvent::Iteration { time: 7.0 })
    );
    let events = p.feed(
        "GAMG:  Solving for p, Initial residual = 0.25, Final residual = 0.01, No Iterations 4",
    );
    assert_eq!(
        events,
        [LogEvent::Residual {
            field: "p".into(),
            initial: 0.25
        }]
    );
    assert_eq!(
        p.feed("ExecutionTime = 1.5 s  ClockTime = 2 s"),
        [LogEvent::ExecutionTime { seconds: 1.5 }]
    );
    assert_eq!(p.feed("End"), [LogEvent::End]);
}
