// SPDX-License-Identifier: GPL-3.0-or-later
//! Core library of FoamCraft.

pub mod case;
pub mod command;
pub mod error;
pub mod job;
pub mod log;
pub mod paths;
pub mod project;
pub mod runtime;
pub mod units;

/// Version of the FoamCraft workspace, taken from the workspace manifest.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Trademark notice required wherever OpenFOAM is named (see docs/licensing.md).
pub const TRADEMARK_NOTICE: &str = "This offering is not approved or endorsed by OpenCFD Limited, \
producer and distributor of the OpenFOAM software via www.openfoam.com, and owner of the \
OPENFOAM® and OpenCFD® trade marks.";
