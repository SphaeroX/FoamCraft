// SPDX-License-Identifier: GPL-3.0-or-later
//! Wind tunnel rules: tunnel size, cell budget, flow classification and plausibility warnings.
//! The rules are documented in docs/architecture.md ("Wind tunnel").

use foamcraft_core::project::{Ground, Quality};
use schemars::JsonSchema;
use serde::Serialize;

/// Tunnel distances in multiples of the largest model dimension L.
pub const UPSTREAM: f64 = 5.0;
pub const DOWNSTREAM: f64 = 15.0;
pub const SIDE: f64 = 5.0;
pub const ABOVE: f64 = 5.0;
pub const BELOW: f64 = 5.0;

/// Share of the memory a simulation may use.
pub const USABLE_MEMORY_SHARE: f64 = 0.6;
/// Peak memory per million cells (snappyHexMesh is the peak); re-measured in Task_09 and Task_33.
pub const BYTES_PER_MILLION_CELLS: f64 = 1.5 * 1024.0 * 1024.0 * 1024.0;
/// Solver speed for the time estimate: cell iterations per core and second (measured on the
/// reference case in Task_03: about 140 000; rounded down).
pub const CELL_ITERATIONS_PER_CORE_SECOND: f64 = 100_000.0;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, JsonSchema)]
pub struct TunnelBox {
    /// Centre in world coordinates (metres).
    pub center: [f64; 3],
    /// Size along the flow.
    pub length: f64,
    /// Size across the flow (horizontal).
    pub width: f64,
    pub height: f64,
    /// Rotation about Z: the tunnel's length axis points along the flow.
    pub yaw_deg: f64,
    /// Height of the tunnel floor (the ground when there is one).
    pub floor_z: f64,
}

/// Tunnel around model bounds (`min`, `max`, world, metres) for flow at `yaw_deg`.
pub fn tunnel_box(min: [f64; 3], max: [f64; 3], yaw_deg: f64, ground: Ground) -> TunnelBox {
    let (s, c) = yaw_deg.to_radians().sin_cos();
    // Model box in the flow frame (x along the flow, y across): project the 8 corners.
    let mut lo = [f64::MAX; 2];
    let mut hi = [f64::MIN; 2];
    for i in 0..8 {
        let x = if i & 1 == 0 { min[0] } else { max[0] };
        let y = if i & 2 == 0 { min[1] } else { max[1] };
        let along = x * c + y * s;
        let across = -x * s + y * c;
        lo = [lo[0].min(along), lo[1].min(across)];
        hi = [hi[0].max(along), hi[1].max(across)];
    }
    let l = (max[0] - min[0]).max(max[1] - min[1]).max(max[2] - min[2]);
    let x0 = lo[0] - UPSTREAM * l;
    let x1 = hi[0] + DOWNSTREAM * l;
    let y0 = lo[1] - SIDE * l;
    let y1 = hi[1] + SIDE * l;
    let floor_z = match ground {
        Ground::None => min[2] - BELOW * l,
        Ground::Fixed | Ground::Moving => min[2],
    };
    let top = max[2] + ABOVE * l;
    let (cx, cy) = ((x0 + x1) / 2.0, (y0 + y1) / 2.0);
    TunnelBox {
        center: [cx * c - cy * s, cx * s + cy * c, (floor_z + top) / 2.0],
        length: x1 - x0,
        width: y1 - y0,
        height: top - floor_z,
        yaw_deg,
        floor_z,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, JsonSchema)]
pub struct CellBudget {
    pub cells: u64,
    pub iterations: u32,
    /// Rough wall-clock estimate in seconds (meshing plus solving).
    pub estimate_s: f64,
    /// The quality level wanted more cells than the memory allows.
    pub limited_by_memory: bool,
}

/// Cells and iterations of a quality level, limited by memory.
pub fn cell_budget(memory_bytes: u64, cores: u32, quality: Quality) -> CellBudget {
    let (wanted, iterations) = match quality {
        Quality::Draft => (500_000u64, 300u32),
        Quality::Standard => (2_000_000, 500),
        Quality::Fine => (8_000_000, 800),
    };
    let max_cells =
        (memory_bytes as f64 * USABLE_MEMORY_SHARE / BYTES_PER_MILLION_CELLS * 1e6).round() as u64;
    // Round the memory limit to 100 000 cells so the numbers stay readable.
    let max_cells = max_cells / 100_000 * 100_000;
    let cells = wanted.min(max_cells.max(100_000));
    let cores = f64::from(cores.max(1));
    // Meshing: roughly 20 000 cells per second and core, solving: see the constant above.
    let estimate_s = cells as f64 / (20_000.0 * cores)
        + cells as f64 * f64::from(iterations) / (CELL_ITERATIONS_PER_CORE_SECOND * cores);
    CellBudget {
        cells,
        iterations,
        estimate_s,
        limited_by_memory: cells < wanted,
    }
}

/// Catalog code describing the flow regime for a Reynolds number based on the model length.
pub fn flow_class(reynolds: f64) -> &'static str {
    if reynolds < 1e3 {
        "scenario.flow_viscous"
    } else if reynolds < 5e5 {
        "scenario.flow_laminar"
    } else {
        "scenario.flow_turbulent"
    }
}

/// Model sizes outside 1 mm to 200 m are most likely a wrong unit.
pub fn unit_suspect(largest_extent_m: f64) -> bool {
    !(1e-3..=200.0).contains(&largest_extent_m)
}
