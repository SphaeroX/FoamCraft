// SPDX-License-Identifier: GPL-3.0-or-later
//! Media, tunnel size rule, cell budget, Reynolds and Mach numbers, unit warning.

use foamcraft_core::project::{Ground, MediumSpec, Quality};
use foamcraft_sim::{
    CellBudget, cell_budget, flow_class, mach_number, medium_properties, tunnel_box, unit_suspect,
};

const GIB: u64 = 1024 * 1024 * 1024;

fn rel(value: f64, expected: f64, tol: f64) {
    assert!(
        (value - expected).abs() <= tol * expected.abs(),
        "{value} vs {expected}"
    );
}

#[test]
fn air_and_water_at_20_degrees() {
    // Reference values: air at 20 °C and 101325 Pa (rho 1.204 kg/m3, nu 1.516e-5 m2/s);
    // water at 20 °C (rho 998.2 kg/m3, nu 1.004e-6 m2/s), common engineering tables.
    let air = medium_properties(&MediumSpec::Air {
        temperature_c: 20.0,
    });
    rel(air.density, 1.204, 0.005);
    rel(air.kinematic_viscosity, 1.516e-5, 0.01);
    rel(air.speed_of_sound.unwrap(), 343.2, 0.005);
    let water = medium_properties(&MediumSpec::Water {
        temperature_c: 20.0,
    });
    rel(water.density, 998.2, 0.002);
    rel(water.kinematic_viscosity, 1.004e-6, 0.02);
    let custom = medium_properties(&MediumSpec::Custom {
        density: 850.0,
        kinematic_viscosity: 2e-5,
    });
    assert_eq!((custom.density, custom.kinematic_viscosity), (850.0, 2e-5));
    assert!(custom.speed_of_sound.is_none());
}

#[test]
fn tunnel_follows_the_rule_with_ground() {
    // Unit cube from (0,0,0) to (1,1,1), flow along +X: 5 L upstream, 15 L downstream, 5 L to each
    // side and above, floor at the lowest point of the model.
    let t = tunnel_box([0.0; 3], [1.0; 3], 0.0, Ground::Fixed);
    rel(t.length, 21.0, 1e-9);
    rel(t.width, 11.0, 1e-9);
    rel(t.height, 6.0, 1e-9);
    rel(t.floor_z, 0.0, 1e-9);
    assert!(
        (t.center[0] - 5.5).abs() < 1e-9 && (t.center[1] - 0.5).abs() < 1e-9,
        "{:?}",
        t.center
    );
}

#[test]
fn tunnel_without_ground_extends_below_and_turns_with_the_flow() {
    let t = tunnel_box([0.0; 3], [2.0, 1.0, 1.0], 0.0, Ground::None);
    // L is the largest model dimension (2 m).
    rel(t.length, 2.0 + 40.0, 1e-9);
    rel(t.height, 1.0 + 20.0, 1e-9);
    rel(t.floor_z, -10.0, 1e-9);
    let turned = tunnel_box([0.0; 3], [2.0, 1.0, 1.0], 90.0, Ground::None);
    // Flow along +Y: the model is 1 m long in the flow direction and 2 m wide.
    rel(turned.length, 1.0 + 40.0, 1e-9);
    rel(turned.width, 2.0 + 20.0, 1e-9);
    assert_eq!(turned.yaw_deg, 90.0);
}

#[test]
fn cell_budget_follows_memory() {
    // 64 GB: no quality level is limited by memory.
    let b: Vec<CellBudget> = [Quality::Draft, Quality::Standard, Quality::Fine]
        .iter()
        .map(|q| cell_budget(64 * GIB, 24, *q))
        .collect();
    assert_eq!(
        b.iter().map(|b| b.cells).collect::<Vec<_>>(),
        [500_000, 2_000_000, 8_000_000]
    );
    assert!(b.iter().all(|b| !b.limited_by_memory));
    assert!(b[0].estimate_s < b[1].estimate_s && b[1].estimate_s < b[2].estimate_s);
    // 8 GB: 60 % usable at 1.5 GB per million cells allows 3.2 million cells.
    let fine = cell_budget(8 * GIB, 8, Quality::Fine);
    assert_eq!(fine.cells, 3_200_000);
    assert!(fine.limited_by_memory);
    assert_eq!(cell_budget(8 * GIB, 8, Quality::Standard).cells, 2_000_000);
}

#[test]
fn reynolds_classes_mach_and_units() {
    assert_eq!(flow_class(500.0), "scenario.flow_viscous");
    assert_eq!(flow_class(5e4), "scenario.flow_laminar");
    assert_eq!(flow_class(2e6), "scenario.flow_turbulent");
    let air = medium_properties(&MediumSpec::Air {
        temperature_c: 20.0,
    });
    assert!(mach_number(50.0, &air).unwrap() < 0.3);
    assert!(mach_number(120.0, &air).unwrap() > 0.3);
    assert!(unit_suspect(0.0004));
    assert!(unit_suspect(500.0));
    assert!(!unit_suspect(0.02));
    assert!(!unit_suspect(4.5));
}
