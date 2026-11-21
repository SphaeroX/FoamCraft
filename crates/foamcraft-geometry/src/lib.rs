// SPDX-License-Identifier: GPL-3.0-or-later
//! Geometry of FoamCraft: reading triangle models, diagnosing them, placing them in the world.
//! FoamCraft does not edit CAD geometry; it loads, aligns, scales and (later) closes openings.

pub mod caps;
pub mod commands;
mod diagnose;
mod mesh;
mod msh;
pub use msh::iges_unit;
pub mod openings;
mod query;
mod read;
mod segment;
pub mod surfaces;
mod transform;

pub use commands::{CadConverter, SharedCadConverter};
pub use diagnose::{Diagnosis, Finding, Severity, diagnose, suggest_unit};
pub use mesh::{Format, Mesh, MeshBuilder};
pub use query::{Hit, nearest_triangle, raycast};
pub use read::{format_of, read_mesh};
pub use segment::{FaceInfo, MIN_FACE_AREA_SHARE, Segmentation, segment};
pub use transform::{
    Pivots, Placement, UNITS, fix_normals, ground_offset, pivots, rotation_to_deg, unit_scale,
};
