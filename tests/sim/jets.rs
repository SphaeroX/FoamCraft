// SPDX-License-Identifier: GPL-3.0-or-later
//! Free jets (Task_31): sources in the flow frame, the domain around models and jets, the
//! refinement along each jet and the case changes (open boundaries, velocity constraint).

use foamcraft_core::project::JetSource;
use foamcraft_sim::internal::FieldValueLog;
use foamcraft_sim::jets::{
    FLOW_OBJECT, JetFlow, apply_jets, check_jets, extend_plan, jet_level, to_flow,
};
use foamcraft_sim::mesh::MeshPlan;

fn plan() -> MeshPlan {
    serde_json::from_value(serde_json::json!({
        "yaw_deg": 0.0, "ground": false, "reference_length": 0.4,
        "tunnel_min": [-2.0, -1.0, -1.0], "tunnel_max": [6.0, 1.0, 1.0],
        "background_cell": 0.1, "background_cells": [80, 20, 20], "surface_cell": 0.0125, "surface_level": 3,
        "near_level": 2, "wake_level": 1,
        "near_box": [[-0.2, -0.3, -0.2], [0.6, 0.3, 0.4]],
        "wake_box": [[-0.1, -0.3, -0.2], [2.0, 0.3, 0.4]],
        "layers": 0, "first_layer": 0.001, "expansion_ratio": 1.2,
        "location_in_mesh": [-1.5, 0.1, 0.1], "estimated_cells": 300000
    }))
    .unwrap()
}

fn jet(center: [f64; 3], direction: [f64; 3], d: f64) -> JetSource {
    JetSource {
        shape: "circle".into(),
        center,
        direction,
        size: [d, d],
        speed_m_s: 20.0,
    }
}

#[test]
fn sources_turn_into_the_flow_frame() {
    // Wind direction 90°: world +Y is the flow +X.
    let j = to_flow(&jet([0.0, -1.0, 0.2], [0.0, 1.0, 0.0], 0.1), 90.0);
    assert!(
        (j.center[0] + 1.0).abs() < 1e-9 && j.center[1].abs() < 1e-9,
        "{:?}",
        j.center
    );
    assert!((j.direction[0] - 1.0).abs() < 1e-9, "{:?}", j.direction);
    assert!((j.diameter - 0.1).abs() < 1e-12);
    // A rectangle counts with the diameter of the same area.
    let mut r = jet([0.0; 3], [1.0, 0.0, 0.0], 0.1);
    r.shape = "rectangle".into();
    r.size = [0.2, 0.05];
    let f = to_flow(&r, 0.0);
    assert!((f.diameter - (4.0 * 0.01 / std::f64::consts::PI).sqrt()).abs() < 1e-12);
}

#[test]
fn the_domain_grows_around_the_jets_and_keeps_its_cell_size() {
    let mut p = plan();
    let h = (p.tunnel_max[0] - p.tunnel_min[0]) / f64::from(p.background_cells[0]);
    let jets = vec![JetFlow {
        center: [-3.0, 0.0, 0.1],
        direction: [1.0, 0.0, 0.0],
        diameter: 0.1,
        speed_m_s: 20.0,
    }];
    extend_plan(&mut p, &jets);
    // Ten diameters of room around the source, upstream as well.
    assert!(p.tunnel_min[0] <= -4.0 + 1e-9, "{:?}", p.tunnel_min);
    let h2 = (p.tunnel_max[0] - p.tunnel_min[0]) / f64::from(p.background_cells[0]);
    assert!((h2 - h).abs() < 1e-9, "{h} vs {h2}");
    // The point the mesher keeps lies in the fluid, not in the source.
    assert!(p.location_in_mesh[0] > p.tunnel_min[0]);
}

#[test]
fn the_source_gets_at_least_ten_cells_across() {
    let p = plan();
    // Background cell 0.1 m, jet 0.08 m: 0.1 / 2^n <= 0.008 needs n = 4.
    assert_eq!(jet_level(&p, 0.08), 4);
}

#[test]
fn a_source_inside_the_model_or_blowing_past_it_is_reported() {
    use foamcraft_geometry::{Format, MeshBuilder};
    // Unit cube 0..1.
    let mut b = MeshBuilder::with_capacity(12);
    let c = |x: f32, y: f32, z: f32| [x, y, z];
    let faces = [
        [c(0., 0., 0.), c(1., 1., 0.), c(1., 0., 0.)],
        [c(0., 0., 0.), c(0., 1., 0.), c(1., 1., 0.)],
        [c(0., 0., 1.), c(1., 0., 1.), c(1., 1., 1.)],
        [c(0., 0., 1.), c(1., 1., 1.), c(0., 1., 1.)],
        [c(0., 0., 0.), c(1., 0., 0.), c(1., 0., 1.)],
        [c(0., 0., 0.), c(1., 0., 1.), c(0., 0., 1.)],
        [c(0., 1., 0.), c(1., 1., 1.), c(1., 1., 0.)],
        [c(0., 1., 0.), c(0., 1., 1.), c(1., 1., 1.)],
        [c(0., 0., 0.), c(0., 0., 1.), c(0., 1., 1.)],
        [c(0., 0., 0.), c(0., 1., 1.), c(0., 1., 0.)],
        [c(1., 0., 0.), c(1., 1., 0.), c(1., 1., 1.)],
        [c(1., 0., 0.), c(1., 1., 1.), c(1., 0., 1.)],
    ];
    for [a, bb, cc] in faces {
        b.triangle(a, bb, cc);
    }
    let cube = b.finish(Format::Stl, None);
    let models = [&cube];
    let at = |center: [f64; 3], dir: [f64; 3]| check_jets(&models, &[jet(center, dir, 0.1)]);
    assert!(at([-1.0, 0.5, 0.5], [1.0, 0.0, 0.0]).is_empty());
    assert_eq!(
        at([0.5, 0.5, 0.5], [1.0, 0.0, 0.0]),
        vec!["scenario.jet_inside_model"]
    );
    assert_eq!(
        at([-1.0, 0.5, 0.5], [-1.0, 0.0, 0.0]),
        vec!["scenario.jet_misses_model"]
    );
}

#[test]
fn the_case_gets_open_boundaries_a_constraint_and_calmer_relaxation() {
    let dir = std::env::temp_dir().join(format!("fc-jets-case-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    for sub in ["0", "system"] {
        std::fs::create_dir_all(dir.join(sub)).unwrap();
    }
    let field = |name: &str, internal: &str, inlet: &str| {
        format!(
            "FoamFile {{ object {name}; }}\ninternalField   {internal};\nboundaryField\n{{\n    inlet  {inlet}\n    outlet {{ type zeroGradient; }}\n    sides  {{ type slip; }}\n    model  {{ type noSlip; }}\n}}\n"
        )
    };
    std::fs::write(
        dir.join("0/U"),
        field(
            "U",
            "uniform (20 0 0)",
            "{ type fixedValue; value uniform (20 0 0); }",
        ),
    )
    .unwrap();
    std::fs::write(
        dir.join("0/p"),
        field("p", "uniform 0", "{ type zeroGradient; }"),
    )
    .unwrap();
    std::fs::write(
        dir.join("0/k"),
        field(
            "k",
            "uniform 0.06",
            "{ type fixedValue; value uniform 0.06; }",
        ),
    )
    .unwrap();
    std::fs::write(
        dir.join("0/omega"),
        field(
            "omega",
            "uniform 11",
            "{ type fixedValue; value uniform 11; }",
        ),
    )
    .unwrap();
    std::fs::write(
        dir.join("system/fvSolution"),
        "SIMPLE\n{\n    consistent      true;\n}\nrelaxationFactors\n{\n    equations { U 0.9; \".*\" 0.9; }\n    fields { p 1; }\n}\n",
    )
    .unwrap();
    std::fs::write(
        dir.join("system/snappyHexMeshDict"),
        "geometry\n{\n    near { type searchableBox; min (0 0 0); max (1 1 1); }\n    wake { type searchableBox; min (0 0 0); max (2 1 1); }\n}\ncastellatedMeshControls\n{\n    refinementRegions\n    {\n        near { mode inside; levels ((1E15 2)); }\n        wake { mode inside; levels ((1E15 1)); }\n    }\n}\n",
    )
    .unwrap();
    let jets = vec![JetFlow {
        center: [-0.3, 0.0, 0.075],
        direction: [1.0, 0.0, 0.0],
        diameter: 0.08,
        speed_m_s: 20.0,
    }];
    apply_jets(&dir, &plan(), &jets).unwrap();
    let read = |p: &str| std::fs::read_to_string(dir.join(p)).unwrap();
    let u = read("0/U");
    assert!(u.contains("internalField   uniform (0 0 0);"), "{u}");
    assert!(
        u.contains("inlet  { type pressureInletOutletVelocity;"),
        "{u}"
    );
    assert!(
        u.contains("sides  { type pressureInletOutletVelocity;"),
        "{u}"
    );
    assert!(u.contains("model  { type noSlip; }"), "{u}");
    assert!(read("0/p").contains("outlet { type totalPressure; p0 uniform 0;"));
    assert!(read("0/k").contains("inlet  { type inletOutlet;"));
    let fvo = read("system/fvOptions");
    assert!(fvo.contains("vectorFixedValueConstraint"), "{fvo}");
    assert!(fvo.contains("source cylinder"), "{fvo}");
    assert!(fvo.contains("U (20 0 0)"), "{fvo}");
    let sol = read("system/fvSolution");
    assert!(sol.contains("consistent      false;"), "{sol}");
    assert!(sol.contains("U 0.5;"), "{sol}");
    assert!(sol.contains("fields { p 0.3; }"), "{sol}");
    let snappy = read("system/snappyHexMeshDict");
    assert!(
        snappy.contains("jet1 { type searchableCylinder;"),
        "{snappy}"
    );
    assert!(
        snappy.contains("jet1_core { type searchableCylinder;"),
        "{snappy}"
    );
    assert!(
        snappy.contains("jet1 { mode inside; levels ((1E15 2)); }"),
        "{snappy}"
    );
    assert!(
        snappy.contains("jet1_near { mode inside; levels ((1E15 3)); }"),
        "{snappy}"
    );
    assert!(
        snappy.contains("jet1_core { mode inside; levels ((1E15 4)); }"),
        "{snappy}"
    );
}

#[test]
fn a_far_source_coarsens_the_background_but_not_the_cells_at_the_model() {
    let mut p = plan();
    let before: u64 = p.background_cells.iter().map(|&c| u64::from(c)).product();
    let fine_at_model = p.background_cell / 2f64.powi(p.surface_level as i32);
    let jets = vec![JetFlow {
        center: [-30.0, 15.0, 0.0],
        direction: [1.0, 0.0, 0.0],
        diameter: 1.0,
        speed_m_s: 10.0,
    }];
    extend_plan(&mut p, &jets);
    let after: u64 = p.background_cells.iter().map(|&c| u64::from(c)).product();
    assert!(after <= 2 * before.max(25_000), "{before} -> {after}");
    let still = p.background_cell / 2f64.powi(p.surface_level as i32);
    assert!(
        (still - fine_at_model).abs() < 1e-12,
        "{fine_at_model} vs {still}"
    );
    assert!(p.tunnel_min[0] <= -40.0 + 1e-9 && p.tunnel_max[1] >= 25.0 - 1e-9);
}

#[test]
fn the_case_reports_the_mean_speed_in_the_domain_every_iteration() {
    // The forces on a model far from a source stay near zero while the jet still develops, so
    // the run also watches the mean speed in the domain (Task_31, validation of the free jet).
    let dir = std::env::temp_dir().join(format!("fc-jets-flow-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    for sub in ["0", "system"] {
        std::fs::create_dir_all(dir.join(sub)).unwrap();
    }
    let field =
        "internalField   uniform 0;\nboundaryField\n{\n    inlet  { type zeroGradient; }\n}\n";
    for name in ["0/U", "0/p"] {
        std::fs::write(dir.join(name), field).unwrap();
    }
    std::fs::write(dir.join("system/fvSolution"), "SIMPLE\n{\n}\n").unwrap();
    std::fs::write(dir.join("system/snappyHexMeshDict"), "geometry\n{\n}\n").unwrap();
    std::fs::write(
        dir.join("system/controlDict"),
        "application     simpleFoam;\n\nfunctions\n{\n    forces\n    {\n        type forceCoeffs;\n    }\n    stop\n    {\n        type abort;\n    }\n}\n",
    )
    .unwrap();
    let jets = vec![JetFlow {
        center: [-0.3, 0.0, 0.075],
        direction: [1.0, 0.0, 0.0],
        diameter: 0.08,
        speed_m_s: 20.0,
    }];
    apply_jets(&dir, &plan(), &jets).unwrap();
    let control = std::fs::read_to_string(dir.join("system/controlDict")).unwrap();
    let at = |needle: &str| {
        control
            .find(needle)
            .unwrap_or_else(|| panic!("{needle} missing in\n{control}"))
    };
    // The speed |U| as a field, then its volume average, printed to the solver log.
    let speed = at("jetSpeed { type mag;");
    let flow = at(&format!("{FLOW_OBJECT} {{ type volFieldValue;"));
    assert!(control.contains("field U; result jetSpeed;"), "{control}");
    assert!(
        control.contains("fields (jetSpeed); operation volAverage; regionType all;"),
        "{control}"
    );
    assert!(control.contains("log true;"), "{control}");
    // Both come before the forces, so a force sample always has its speed; nothing else changes.
    assert!(speed < flow && flow < at("    forces\n"), "{control}");
    assert!(control.contains("type forceCoeffs;") && control.contains("type abort;"));
    assert!(control.starts_with("application     simpleFoam;\n\nfunctions\n{\n"));
    assert_eq!(control.matches("type volFieldValue;").count(), 1);
}

#[test]
fn the_mean_speed_is_read_from_the_solver_log() {
    // Lines as OpenFOAM v2606 prints them (recorded from a real run).
    let text = "Time = 1\n\nvolFieldValue jetFlow write:\n    volAverage(region0) of jetSpeed = 0.00045450251\n\n\
                forceCoeffs forces write:\n    Coefficient\tTotal\tPressure\tViscous\tInternal\n    Cd:\t0\t0\t0\t0\n\n\
                Time = 2\n\nvolFieldValue jetFlow write:\n    volAverage(region0) of jetSpeed = 0.0011330477\n\nEnd\n";
    let mut log = FieldValueLog::new(1);
    let complete: Vec<bool> = text.lines().map(|l| log.feed(l)).collect();
    assert_eq!(complete.iter().filter(|c| **c).count(), 2);
    assert_eq!(log.samples.len(), 2);
    assert_eq!(log.samples[0][FLOW_OBJECT], 0.00045450251);
    assert_eq!(log.samples[1][FLOW_OBJECT], 0.0011330477);
    assert_eq!(log.iterations, vec![1.0, 2.0]);
}
