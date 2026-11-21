// SPDX-License-Identifier: GPL-3.0-or-later
//! OpenFOAM case templates kept as data files in `templates/` and embedded at compile time.
//!
//! Placeholders have the form `{{NAME}}`; every template lists defaults for its placeholders.

use std::path::Path;

use crate::error::FoamError;

pub struct CaseTemplate {
    pub name: &'static str,
    /// Relative path and contents.
    pub files: &'static [(&'static str, &'static str)],
    pub defaults: &'static [(&'static str, &'static str)],
}

macro_rules! template_files {
    ($dir:literal: $($path:literal),* $(,)?) => {
        &[$(($path, include_str!(concat!("../templates/", $dir, "/", $path)))),*]
    };
}

/// A 1 m cube in a wind tunnel at 10 m/s; used to test the job pipeline.
pub const REFERENCE_CUBE: CaseTemplate = CaseTemplate {
    name: "reference_cube",
    files: template_files!("reference_cube":
        "system/controlDict",
        "system/blockMeshDict",
        "system/snappyHexMeshDict",
        "system/fvSchemes",
        "system/fvSolution",
        "system/decomposeParDict",
        "constant/transportProperties",
        "constant/turbulenceProperties",
        "0/U",
        "0/p",
        "0/k",
        "0/omega",
        "0/nut",
    ),
    defaults: &[("END_TIME", "200"), ("NPROCS", "1")],
};

/// External flow in a wind tunnel; mesh and boundary values are filled in by `foamcraft-sim`.
pub const WIND_TUNNEL: CaseTemplate = CaseTemplate {
    name: "wind_tunnel",
    files: template_files!("wind_tunnel":
        "system/controlDict",
        "system/blockMeshDict",
        "system/snappyHexMeshDict",
        "system/fvSchemes",
        "system/fvSolution",
        "system/decomposeParDict",
        "constant/transportProperties",
        "constant/turbulenceProperties",
        "0/U",
        "0/p",
        "0/k",
        "0/omega",
        "0/nut",
    ),
    defaults: &[("END_TIME", "500"), ("FUNCTIONS", "")],
};

/// Internal flow: box around the models, fields written by the case generator.
pub const INTERNAL_FLOW: CaseTemplate = CaseTemplate {
    name: "internal_flow",
    files: template_files!("internal_flow":
        "system/controlDict",
        "system/blockMeshDict",
        "system/snappyHexMeshDict",
        "system/fvSchemes",
        "system/fvSolution",
        "system/decomposeParDict",
        "constant/transportProperties",
        "constant/turbulenceProperties",
    ),
    defaults: &[("END_TIME", "1000"), ("FUNCTIONS", "")],
};

/// Replaces `{{NAME}}` with the value from `values`, else from `defaults`.
pub fn render_template(text: &str, values: &[(&str, &str)], defaults: &[(&str, &str)]) -> String {
    let mut out = text.to_string();
    for (name, value) in values.iter().chain(defaults) {
        out = out.replace(&format!("{{{{{name}}}}}"), value);
    }
    out
}

impl CaseTemplate {
    /// Writes the case into `dir` (created if needed).
    pub fn write(&self, dir: &Path, values: &[(&str, &str)]) -> Result<(), FoamError> {
        let io = |e: std::io::Error| FoamError::new("job.storage_failed", e.to_string());
        for (path, text) in self.files {
            let target = dir.join(path);
            if let Some(parent) = target.parent() {
                std::fs::create_dir_all(parent).map_err(io)?;
            }
            std::fs::write(&target, render_template(text, values, self.defaults)).map_err(io)?;
        }
        Ok(())
    }
}
