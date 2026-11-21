// SPDX-License-Identifier: GPL-3.0-or-later
//! Fluid properties from presets.

use foamcraft_core::project::MediumSpec;
use schemars::JsonSchema;
use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, JsonSchema)]
pub struct MediumProperties {
    /// kg/m³
    pub density: f64,
    /// m²/s
    pub kinematic_viscosity: f64,
    /// m/s; `None` when unknown (custom media) or irrelevant.
    pub speed_of_sound: Option<f64>,
}

const P_ATM: f64 = 101_325.0;
/// Specific gas constant of dry air, J/(kg K).
const R_AIR: f64 = 287.05;
const GAMMA_AIR: f64 = 1.4;

pub fn medium_properties(spec: &MediumSpec) -> MediumProperties {
    match *spec {
        MediumSpec::Air { temperature_c } => {
            let t = temperature_c + 273.15;
            let density = P_ATM / (R_AIR * t);
            // Sutherland's law (reference 1.716e-5 Pa s at 273.15 K, constant 110.4 K).
            let mu = 1.716e-5 * (t / 273.15).powf(1.5) * (273.15 + 110.4) / (t + 110.4);
            MediumProperties {
                density,
                kinematic_viscosity: mu / density,
                speed_of_sound: Some((GAMMA_AIR * R_AIR * t).sqrt()),
            }
        }
        MediumSpec::Water { temperature_c } => {
            let t = temperature_c;
            // Thiesen's density formula (0-40 °C).
            let density = 1000.0
                * (1.0 - (t + 288.9414) / (508_929.2 * (t + 68.129_63)) * (t - 3.9863).powi(2));
            // Vogel equation for the dynamic viscosity (T in K).
            let mu = 2.414e-5 * 10f64.powf(247.8 / (t + 273.15 - 140.0));
            MediumProperties {
                density,
                kinematic_viscosity: mu / density,
                speed_of_sound: Some(1481.0),
            }
        }
        MediumSpec::Custom {
            density,
            kinematic_viscosity,
        } => MediumProperties {
            density,
            kinematic_viscosity,
            speed_of_sound: None,
        },
    }
}

/// Mach number, `None` when the speed of sound is unknown.
pub fn mach_number(speed: f64, medium: &MediumProperties) -> Option<f64> {
    medium.speed_of_sound.map(|c| speed / c)
}
