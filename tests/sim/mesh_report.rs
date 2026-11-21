// SPDX-License-Identifier: GPL-3.0-or-later
//! Mesh quality verdict from checkMesh and layer coverage from snappyHexMesh (recorded v2606 logs).

use foamcraft_sim::mesh_report::{Verdict, layer_report, mesh_quality};

fn log(name: &str) -> String {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/data/logs")
        .join(name);
    std::fs::read_to_string(path).unwrap()
}

#[test]
fn layer_table_gives_target_and_achieved_layers() {
    let r = layer_report(&log("snappyHexMesh_layers.log")).expect("layer table");
    assert_eq!(r.target, 1);
    // Face-weighted mean of 0.981 (2594 faces) and 0.974 (10976 faces).
    assert!((r.achieved - 0.975).abs() < 0.001, "{}", r.achieved);
    assert!((r.coverage() - 0.975).abs() < 0.001);
    assert!(
        (r.thickness_percent - 92.4).abs() < 0.1,
        "{}",
        r.thickness_percent
    );
    assert!(layer_report("no table here").is_none());
}

#[test]
fn check_mesh_verdicts() {
    let q = mesh_quality(&log("checkMesh_wind_tunnel.log"));
    assert_eq!(q.cells, Some(188_962));
    assert_eq!(q.failed_checks, 0);
    assert!((q.max_non_orthogonality.unwrap() - 73.995).abs() < 0.01);
    assert!((q.max_skewness.unwrap() - 2.4153).abs() < 0.001);
    assert_eq!(q.verdict, Verdict::Acceptable);

    let good = mesh_quality(&log("checkMesh_ok.log"));
    assert_eq!(good.verdict, Verdict::Good);

    let bad = mesh_quality("    cells:            10\n\nFailed 3 mesh checks.\n");
    assert_eq!(bad.failed_checks, 3);
    assert_eq!(bad.verdict, Verdict::Poor);
}
