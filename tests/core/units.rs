// SPDX-License-Identifier: GPL-3.0-or-later
//! Physical quantities are SI internally; conversion only for display.

use foamcraft_core::units::{DisplayUnits, Length, Velocity};

#[test]
fn quantities_store_si_values() {
    assert_eq!(Length::from_millimetres(250.0).metres(), 0.25);
    assert!((Velocity::from_km_per_hour(36.0).metres_per_second() - 10.0).abs() < 1e-12);
    assert!((Velocity::from_miles_per_hour(1.0).metres_per_second() - 0.44704).abs() < 1e-12);
}

#[test]
fn display_follows_the_unit_system() {
    let v = Velocity::from_metres_per_second(10.0);
    assert_eq!(v.display(DisplayUnits::Metric), (36.0, "km/h"));
    let (mph, unit) = v.display(DisplayUnits::Imperial);
    assert_eq!(unit, "mph");
    assert!((mph - 22.369362920544).abs() < 1e-9);
    assert_eq!(
        Length::from_metres(1.5).display(DisplayUnits::Metric),
        (1500.0, "mm")
    );
    assert_eq!(
        Length::from_metres(0.0254)
            .display(DisplayUnits::Imperial)
            .1,
        "in"
    );
}

#[test]
fn quantities_serialize_as_plain_si_numbers() {
    let json = serde_json::to_string(&Velocity::from_metres_per_second(12.5)).unwrap();
    assert_eq!(json, "12.5");
}
