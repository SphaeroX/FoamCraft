// SPDX-License-Identifier: GPL-3.0-or-later
//! Internal flow case: mesh plan and the generated boundary conditions (snapshot in
//! tests/data/snapshots/internal_pipe; update only with a stated reason:
//! FOAMCRAFT_UPDATE_SNAPSHOTS=1).

use std::path::PathBuf;

use foamcraft_core::project::{InletCondition, MediumSpec};
use foamcraft_geometry::Placement;
use foamcraft_sim::case::ModelSurface;
use foamcraft_sim::internal_case::{Boundary, plan_internal, write_internal_case};
use foamcraft_sim::medium_properties;

const FILES: [&str; 11] = [
    "system/controlDict",
    "system/blockMeshDict",
    "system/snappyHexMeshDict",
    "constant/turbulenceProperties",
    "constant/transportProperties",
    "0/U",
    "0/p",
    "0/k",
    "0/omega",
    "0/nut",
    "constant/triSurface/model.stl",
];

#[test]
fn the_plan_meets_the_budget_and_resolves_the_opening() {
    // Pipe r = 10 mm, 50 mm long.
    let volume = std::f64::consts::PI * 1e-4 * 0.05;
    let plan = plan_internal(
        [0.0, -0.01, -0.01],
        [0.05, 0.01, 0.01],
        volume,
        [0.025, 0.0, 0.0],
        0.02,
        100_000,
        false,
    );
    let ratio = plan.estimated_cells as f64 / 100_000.0;
    assert!((0.8..=1.05).contains(&ratio), "{}", plan.estimated_cells);
    assert!(plan.cells_across > 10.0, "{}", plan.cells_across);
    for k in 0..3 {
        assert!(
            plan.box_min[k] < [0.0, -0.01, -0.01][k] && plan.box_max[k] > [0.05, 0.01, 0.01][k]
        );
        let n = f64::from(plan.cells[k]);
        assert!(((plan.box_max[k] - plan.box_min[k]) / n - plan.cell).abs() < 1e-12);
    }
    // A big box around a thin fluid region limits the background cells.
    let thin = plan_internal(
        [0.0; 3],
        [1.0, 1.0, 1.0],
        1e-6,
        [0.5; 3],
        0.001,
        100_000,
        false,
    );
    let background: f64 = thin.cells.iter().map(|c| f64::from(*c)).product();
    // Three times the budget plus two padding cells per side.
    assert!(background < 4.0e5, "{background}");
}

#[test]
fn generated_pipe_case_matches_the_snapshot() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests");
    let mesh =
        foamcraft_geometry::read_mesh(&root.join("data/geometry/internal/pipe_open.stl")).unwrap();
    let world = Placement {
        unit: "mm".into(),
        ..Default::default()
    }
    .apply(&mesh);
    let regions = vec!["wall".to_string(); world.triangles.len()];
    let models = [ModelSurface {
        mesh: &world,
        regions,
    }];
    let medium = medium_properties(&MediumSpec::Water {
        temperature_c: 20.0,
    });
    let boundaries = [
        Boundary {
            region: "in_1".into(),
            inlet: Some(InletCondition::Velocity { m_s: 2.0 }),
            outlet_pa: None,
            area_m2: std::f64::consts::PI * 1e-4,
            hydraulic_diameter_m: 0.02,
        },
        Boundary {
            region: "out_1".into(),
            inlet: None,
            outlet_pa: Some(100.0),
            area_m2: std::f64::consts::PI * 1e-4,
            hydraulic_diameter_m: 0.02,
        },
    ];
    let plan = plan_internal(
        [0.0, -0.01, -0.01],
        [0.05, 0.01, 0.01],
        std::f64::consts::PI * 1e-4 * 0.05,
        [0.025, 0.0, 0.0],
        0.02,
        100_000,
        true,
    );
    let dir = std::env::temp_dir().join(format!("fc-internal-snap-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    write_internal_case(&dir, &plan, &models, &boundaries, &medium, 1500).unwrap();
    let snapshots = root.join("data/snapshots/internal_pipe");
    for file in FILES {
        let got =
            std::fs::read_to_string(dir.join(file)).unwrap_or_else(|_| panic!("{file} missing"));
        assert!(!got.contains("{{"), "placeholder left in {file}");
        let path = snapshots.join(file);
        if std::env::var_os("FOAMCRAFT_UPDATE_SNAPSHOTS").is_some() {
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(&path, &got).unwrap();
        }
        let want =
            std::fs::read_to_string(&path).unwrap_or_else(|_| panic!("snapshot {file} missing"));
        assert_eq!(
            got.replace("\r\n", "\n"),
            want.replace("\r\n", "\n"),
            "{file} differs"
        );
    }
    let u = std::fs::read_to_string(dir.join("0/U")).unwrap();
    // 2 m/s through pi 1e-4 m²: 6.283e-4 m³/s.
    assert!(
        u.contains("model_in_1 { type flowRateInletVelocity; volumetricFlowRate constant 6.283"),
        "{u}"
    );
    let p = std::fs::read_to_string(dir.join("0/p")).unwrap();
    // 100 Pa / 998.2 kg/m³ (kinematic pressure).
    assert!(
        p.contains("model_out_1 { type fixedValue; value uniform 1.00"),
        "{p}"
    );
}
