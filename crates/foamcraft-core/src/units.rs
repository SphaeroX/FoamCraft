// SPDX-License-Identifier: GPL-3.0-or-later
//! Physical quantities. Values are stored and passed in SI units; conversion happens only where a
//! value is shown or entered.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Unit system for display and input.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum DisplayUnits {
    #[default]
    Metric,
    Imperial,
}

macro_rules! quantity {
    ($(#[$doc:meta])* $name:ident, $si:ident) => {
        $(#[$doc])*
        #[derive(Debug, Clone, Copy, Default, PartialEq, PartialOrd, Serialize, Deserialize, JsonSchema)]
        #[serde(transparent)]
        pub struct $name(f64);

        impl $name {
            pub const fn $si(self) -> f64 {
                self.0
            }
        }
    };
}

quantity!(
    /// Length in metres.
    Length,
    metres
);
quantity!(
    /// Velocity in metres per second.
    Velocity,
    metres_per_second
);
quantity!(
    /// Pressure in pascal.
    Pressure,
    pascal
);
quantity!(
    /// Density in kilograms per cubic metre.
    Density,
    kg_per_cubic_metre
);
quantity!(
    /// Kinematic viscosity in square metres per second.
    KinematicViscosity,
    square_metres_per_second
);
quantity!(
    /// Temperature in kelvin.
    Temperature,
    kelvin
);

const INCH: f64 = 0.0254;
const MILE: f64 = 1609.344;

impl Length {
    pub const fn from_metres(m: f64) -> Self {
        Self(m)
    }
    pub fn from_millimetres(mm: f64) -> Self {
        Self(mm / 1000.0)
    }
    pub fn from_inches(inch: f64) -> Self {
        Self(inch * INCH)
    }
    /// Value and unit symbol for display: millimetres or inches.
    pub fn display(self, units: DisplayUnits) -> (f64, &'static str) {
        match units {
            DisplayUnits::Metric => (self.0 * 1000.0, "mm"),
            DisplayUnits::Imperial => (self.0 / INCH, "in"),
        }
    }
}

impl Velocity {
    pub const fn from_metres_per_second(v: f64) -> Self {
        Self(v)
    }
    pub fn from_km_per_hour(v: f64) -> Self {
        Self(v / 3.6)
    }
    pub fn from_miles_per_hour(v: f64) -> Self {
        Self(v * MILE / 3600.0)
    }
    /// Value and unit symbol for display: km/h or mph.
    pub fn display(self, units: DisplayUnits) -> (f64, &'static str) {
        match units {
            DisplayUnits::Metric => (self.0 * 3.6, "km/h"),
            DisplayUnits::Imperial => (self.0 * 3600.0 / MILE, "mph"),
        }
    }
}

impl Pressure {
    pub const fn from_pascal(p: f64) -> Self {
        Self(p)
    }
}

impl Density {
    pub const fn from_kg_per_cubic_metre(rho: f64) -> Self {
        Self(rho)
    }
}

impl KinematicViscosity {
    pub const fn from_square_metres_per_second(nu: f64) -> Self {
        Self(nu)
    }
}

impl Temperature {
    pub const fn from_kelvin(t: f64) -> Self {
        Self(t)
    }
    pub fn from_celsius(t: f64) -> Self {
        Self(t + 273.15)
    }
}
