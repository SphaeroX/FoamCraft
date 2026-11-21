// SPDX-License-Identifier: GPL-3.0-or-later
//! Mesh quality verdict (checkMesh) and boundary layer coverage (snappyHexMesh).

use schemars::JsonSchema;
use serde::Serialize;

/// Non-orthogonality (degrees) and skewness up to which a mesh counts as good.
pub const GOOD_NON_ORTHOGONALITY: f64 = 65.0;
pub const GOOD_SKEWNESS: f64 = 4.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Verdict {
    Good,
    /// No failed checks, but some cells are distorted; results stay usable.
    Acceptable,
    /// checkMesh reports failed checks.
    Poor,
}

#[derive(Debug, Clone, PartialEq, Serialize, JsonSchema)]
pub struct MeshQuality {
    pub cells: Option<u64>,
    pub failed_checks: u32,
    pub max_non_orthogonality: Option<f64>,
    pub max_skewness: Option<f64>,
    pub verdict: Verdict,
}

fn number_after(line: &str, key: &str) -> Option<f64> {
    let rest = &line[line.find(key)? + key.len()..];
    rest.split_whitespace()
        .next()?
        .trim_end_matches(['.', ','])
        .parse()
        .ok()
}

pub fn mesh_quality(check_log: &str) -> MeshQuality {
    let mut q = MeshQuality {
        cells: None,
        failed_checks: 0,
        max_non_orthogonality: None,
        max_skewness: None,
        verdict: Verdict::Good,
    };
    for line in check_log.lines() {
        let t = line.trim();
        if let Some(n) = t.strip_prefix("cells:") {
            q.cells = n.trim().parse().ok();
        } else if t.starts_with("Mesh non-orthogonality Max:") {
            q.max_non_orthogonality = number_after(t, "Max:");
        } else if t.starts_with("Max skewness =") {
            q.max_skewness = number_after(t, "=");
        } else if t.starts_with("Failed ") && t.ends_with("mesh checks.") {
            q.failed_checks = number_after(t, "Failed").unwrap_or(1.0) as u32;
        }
    }
    q.verdict = if q.failed_checks > 0 {
        Verdict::Poor
    } else if q.max_non_orthogonality.unwrap_or(0.0) <= GOOD_NON_ORTHOGONALITY
        && q.max_skewness.unwrap_or(0.0) <= GOOD_SKEWNESS
    {
        Verdict::Good
    } else {
        Verdict::Acceptable
    };
    q
}

#[derive(Debug, Clone, PartialEq, Serialize, JsonSchema)]
pub struct LayerReport {
    /// Layers asked for.
    pub target: u32,
    /// Face-weighted mean number of layers built.
    pub achieved: f64,
    /// Face-weighted mean of the reached share of the wanted thickness, percent.
    pub thickness_percent: f64,
}

impl LayerReport {
    /// Achieved share of the target layers (0 to 1).
    pub fn coverage(&self) -> f64 {
        if self.target == 0 {
            1.0
        } else {
            self.achieved / f64::from(self.target)
        }
    }
}

/// Reads the last layer table of a snappyHexMesh log:
/// `patch faces target mesh thickness[m] thickness[%]`.
pub fn layer_report(snappy_log: &str) -> Option<LayerReport> {
    let lines: Vec<&str> = snappy_log.lines().collect();
    let header = lines
        .iter()
        .rposition(|l| l.contains("overall thickness"))?;
    let (mut faces_sum, mut target, mut achieved, mut percent) = (0.0, 0u32, 0.0, 0.0);
    // Skip the header, the unit line and the dashes.
    for line in lines.iter().skip(header + 3) {
        let cols: Vec<&str> = line.split_whitespace().collect();
        if cols.len() < 6 {
            break;
        }
        let (Ok(faces), Ok(t), Ok(mesh), Ok(pct)) = (
            cols[1].parse::<f64>(),
            cols[2].parse::<u32>(),
            cols[3].parse::<f64>(),
            cols[5].parse::<f64>(),
        ) else {
            break;
        };
        faces_sum += faces;
        target = target.max(t);
        achieved += faces * mesh;
        percent += faces * pct;
    }
    (faces_sum > 0.0).then(|| LayerReport {
        target,
        achieved: achieved / faces_sum,
        thickness_percent: percent / faces_sum,
    })
}
