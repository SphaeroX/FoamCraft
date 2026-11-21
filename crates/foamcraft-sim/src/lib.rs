// SPDX-License-Identifier: GPL-3.0-or-later
//! Simulation setup: scenarios, media, wind tunnel rules and (later) case generation.

pub mod case;
pub mod commands;
pub mod distance;
pub mod foam_io;
pub mod heavy;
pub mod heavy_gpu;
pub mod internal;
pub mod internal_case;
pub mod jets;
mod medium;
pub mod mesh;
pub mod mesh_progress;
pub mod mesh_report;
pub mod mesh_steps;
pub mod particles;
mod projection;
pub mod results;
pub mod ribbons;
pub mod ride;
pub mod solve;
pub mod summary;
pub mod surface_flow;
mod tunnel;
pub mod visual;

pub use medium::{MediumProperties, mach_number, medium_properties};
pub use projection::{flow_direction, frontal_area, length_along};
pub use tunnel::{
    ABOVE, BELOW, BYTES_PER_MILLION_CELLS, CellBudget, DOWNSTREAM, SIDE, TunnelBox, UPSTREAM,
    USABLE_MEMORY_SHARE, cell_budget, flow_class, tunnel_box, unit_suspect,
};
