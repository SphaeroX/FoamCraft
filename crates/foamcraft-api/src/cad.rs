// SPDX-License-Identifier: GPL-3.0-or-later
//! STEP and IGES conversion with Gmsh in the runtime container.

use std::path::Path;

use foamcraft_core::error::FoamError;
use foamcraft_geometry::CadConverter;
use foamcraft_runtime::StartOptions;

use crate::SharedRunner;

/// Curvature resolution: elements per full circle on curved CAD faces.
const ELEMENTS_PER_CIRCLE: u32 = 24;

pub struct DockerCad {
    pub runner: SharedRunner,
}

fn failed(detail: impl Into<String>) -> FoamError {
    FoamError::new("geometry.read_failed", detail)
}

impl CadConverter for DockerCad {
    fn convert(&self, input: &Path, output: &Path) -> Result<String, FoamError> {
        let runner = self.runner.as_ref();
        let ext = input
            .extension()
            .map_or(String::new(), |e| e.to_string_lossy().into_owned());
        let inside = format!("/tmp/model.{ext}");
        // STEP files are converted to metres; for IGES Open CASCADE cannot change the unit, so the
        // mesh keeps the file unit from the IGES global section.
        let is_iges = matches!(ext.to_ascii_lowercase().as_str(), "iges" | "igs");
        let unit = if is_iges {
            let bytes = std::fs::read(input).map_err(|e| failed(e.to_string()))?;
            foamcraft_geometry::iges_unit(&bytes)
                .unwrap_or("mm")
                .to_string()
        } else {
            "m".to_string()
        };
        let container = runner.start(&StartOptions::default())?;
        let result = (|| {
            runner.copy_into(&container, input, &inside)?;
            // Coordinates in metres; CAD faces stay separate surfaces in the mesh.
            let curvature = ELEMENTS_PER_CIRCLE.to_string();
            let mut args = vec![
                "gmsh",
                &inside,
                "-2",
                "-format",
                "msh41",
                "-o",
                "/tmp/model.msh",
                "-clcurv",
                &curvature,
            ];
            if !is_iges {
                args.extend(["-setstring", "Geometry.OCCTargetUnit", "M"]);
            }
            let out = runner.exec(&container, &args, &mut |_| {})?;
            let log = format!("{}{}", out.stdout, out.stderr);
            if out.status != 0 || log.contains("Error") {
                let tail: Vec<&str> = log
                    .lines()
                    .filter(|l| l.contains("Error"))
                    .take(3)
                    .collect();
                return Err(failed(format!("gmsh: {}", tail.join(" "))));
            }
            runner.copy_out(&container, "/tmp/model.msh", output)?;
            Ok(unit)
        })();
        let _ = runner.stop(&container);
        result
    }
}
