// SPDX-License-Identifier: GPL-3.0-or-later
//! Mesh planning for the wind tunnel: background cell size, refinement levels and boxes, boundary
//! layers and the point inside the flow domain. Everything is in the flow frame: the models are
//! turned by -yaw so the flow runs along +X and the tunnel is axis-aligned.

use foamcraft_core::project::{Ground, Quality};
use foamcraft_geometry::Mesh;
use glam::{DMat3, DVec3};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{MediumProperties, tunnel_box};

/// Target y+ of the first cell for wall functions (middle of 30-100).
pub const TARGET_Y_PLUS: f64 = 50.0;
/// Growth of the boundary layer cells.
pub const EXPANSION_RATIO: f64 = 1.2;
/// Cells snappyHexMesh keeps between two refinement levels.
pub const CELLS_BETWEEN_LEVELS: u32 = 3;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct MeshPlan {
    pub yaw_deg: f64,
    pub ground: bool,
    pub reference_length: f64,
    pub tunnel_min: [f64; 3],
    pub tunnel_max: [f64; 3],
    /// Background (blockMesh) cell size, metres.
    pub background_cell: f64,
    pub background_cells: [u32; 3],
    /// Cell size at the model surface, metres.
    pub surface_cell: f64,
    pub surface_level: u32,
    pub near_level: u32,
    pub wake_level: u32,
    pub near_box: [[f64; 3]; 2],
    pub wake_box: [[f64; 3]; 2],
    pub layers: u32,
    pub first_layer: f64,
    pub expansion_ratio: f64,
    pub location_in_mesh: [f64; 3],
    pub estimated_cells: u64,
}

/// First cell height for the target y+ from a flat plate estimate (Cf = 0.0576 Re^-0.2).
pub fn first_layer_thickness(speed: f64, length: f64, nu: f64) -> f64 {
    let re = (speed * length / nu).max(1.0);
    let cf = 0.0576 * re.powf(-0.2);
    let u_tau = speed * (cf / 2.0).sqrt();
    TARGET_Y_PLUS * nu / u_tau
}

/// Rotation from world into the flow frame.
pub fn to_flow_frame(yaw_deg: f64) -> DMat3 {
    DMat3::from_rotation_z(-yaw_deg.to_radians())
}

fn flow_points<'a>(models: &'a [&'a Mesh], yaw_deg: f64) -> impl Iterator<Item = DVec3> + 'a {
    let r = to_flow_frame(yaw_deg);
    models
        .iter()
        .flat_map(|m| m.positions.iter())
        .map(move |p| r * DVec3::new(f64::from(p[0]), f64::from(p[1]), f64::from(p[2])))
}

fn surface_area(models: &[&Mesh]) -> f64 {
    models
        .iter()
        .flat_map(|m| {
            m.triangles.iter().map(|t| {
                let [a, b, c] = t.map(|i| {
                    let p = m.positions[i as usize];
                    DVec3::new(f64::from(p[0]), f64::from(p[1]), f64::from(p[2]))
                });
                (b - a).cross(c - a).length() / 2.0
            })
        })
        .sum()
}

fn clamp_box(lo: DVec3, hi: DVec3, tmin: DVec3, tmax: DVec3) -> [[f64; 3]; 2] {
    [lo.max(tmin).into(), hi.min(tmax).into()]
}

fn volume(b: &[[f64; 3]; 2]) -> f64 {
    (0..3).map(|k| (b[1][k] - b[0][k]).max(0.0)).product()
}

struct Layout {
    tunnel: f64,
    wake: f64,
    near: f64,
    area: f64,
}

/// Cells for surface cell size `hs` with `s` levels: nested regions plus a band at the surface.
fn estimate(layout: &Layout, hs: f64, s: u32) -> f64 {
    let h0 = hs * f64::from(1u32 << s);
    let h_wake = h0 / f64::from(1u32 << s.saturating_sub(2).max(1));
    let h_near = h0 / f64::from(1u32 << s.saturating_sub(1).max(1));
    let background = layout.tunnel / h0.powi(3);
    let wake = layout.wake * (h_wake.powi(-3) - h0.powi(-3)).max(0.0);
    let near = layout.near * (h_near.powi(-3) - h_wake.powi(-3)).max(0.0);
    let band = layout.area * f64::from(CELLS_BETWEEN_LEVELS + 1) / (hs * hs);
    background + wake + near + band
}

/// Refinement level so the background cells are about half the model size.
fn levels_for(hs: f64, length: f64) -> u32 {
    ((length / 2.0 / hs).log2().round() as i64).clamp(2, 10) as u32
}

/// Plans the mesh for models given in world coordinates (metres).
#[allow(clippy::too_many_arguments)]
pub fn plan_wind_tunnel(
    models: &[&Mesh],
    yaw_deg: f64,
    ground: Ground,
    speed: f64,
    medium: &MediumProperties,
    quality: Quality,
    max_cells: u64,
) -> MeshPlan {
    let (mut lo, mut hi) = (DVec3::splat(f64::MAX), DVec3::splat(f64::MIN));
    for p in flow_points(models, yaw_deg) {
        lo = lo.min(p);
        hi = hi.max(p);
    }
    let extent = hi - lo;
    let l = extent.max_element().max(1e-6);
    let t = tunnel_box(lo.into(), hi.into(), 0.0, ground);
    let half = DVec3::new(t.length, t.width, t.height) / 2.0;
    let center = DVec3::from(t.center);
    let (tmin, tmax) = (center - half, center + half);
    let near_box = clamp_box(
        lo - DVec3::splat(0.5 * l),
        hi + DVec3::splat(0.5 * l),
        tmin,
        tmax,
    );
    let wake_box = clamp_box(
        DVec3::new(lo.x - 0.3 * l, lo.y - 0.6 * l, lo.z - 0.6 * l),
        DVec3::new(hi.x + 4.0 * l, hi.y + 0.6 * l, hi.z + 0.8 * l),
        tmin,
        tmax,
    );
    let layout = Layout {
        tunnel: volume(&[tmin.into(), tmax.into()]),
        wake: volume(&wake_box),
        near: volume(&near_box),
        area: surface_area(models),
    };

    // Largest surface cell whose estimate stays within the budget (bisection on log hs).
    let target = max_cells as f64 * 0.95;
    let (mut fine, mut coarse) = ((l / 5000.0).ln(), (l / 4.0).ln());
    for _ in 0..60 {
        let mid = (fine + coarse) / 2.0;
        let hs = mid.exp();
        if estimate(&layout, hs, levels_for(hs, l)) > target {
            fine = mid;
        } else {
            coarse = mid;
        }
    }
    let surface_cell = coarse.exp();
    let s = levels_for(surface_cell, l);
    let h0 = surface_cell * f64::from(1u32 << s);
    let size = tmax - tmin;
    let background_cells = [0, 1, 2].map(|k| ((size[k] / h0).ceil() as u32).max(4));
    let background_cell = (0..3)
        .map(|k| size[k] / f64::from(background_cells[k]))
        .fold(0.0, f64::max);

    let reference_length = extent.x.max(1e-6);
    let first = first_layer_thickness(speed, reference_length, medium.kinematic_viscosity);
    let ratio = 0.5 * surface_cell / first;
    let mut layers = if ratio <= EXPANSION_RATIO {
        1
    } else {
        ((ratio.ln() / EXPANSION_RATIO.ln()).ceil() as u32).clamp(1, 10)
    };
    if quality == Quality::Draft {
        layers = layers.min(3);
    }
    // A first layer thicker than half the surface cell cannot be built; cap it.
    let first_layer = first.min(0.5 * surface_cell);

    // Upstream of the model, off cell faces, far from any body.
    let location_in_mesh = [
        tmin.x + 1.513 * background_cell,
        center.y + 0.137 * background_cell,
        tmin.z + 0.5 * size.z + 0.111 * background_cell,
    ];
    MeshPlan {
        yaw_deg,
        ground: !matches!(ground, Ground::None),
        reference_length,
        tunnel_min: tmin.into(),
        tunnel_max: tmax.into(),
        background_cell,
        background_cells,
        surface_cell,
        surface_level: s,
        near_level: s.saturating_sub(1).max(1),
        wake_level: s.saturating_sub(2).max(1),
        near_box,
        wake_box,
        layers,
        first_layer,
        expansion_ratio: EXPANSION_RATIO,
        location_in_mesh,
        estimated_cells: estimate(&layout, surface_cell, s).round() as u64,
    }
}

/// True when `point` lies inside any closed mesh (ray parity, skewed ray to miss edges).
pub fn inside_any(models: &[&Mesh], point: [f64; 3]) -> bool {
    let o = DVec3::from(point);
    let d = DVec3::new(1.0, 0.000_731, 0.001_237).normalize();
    models.iter().any(|m| {
        let crossings = m
            .triangles
            .iter()
            .filter(|t| {
                let [a, b, c] = t.map(|i| {
                    let p = m.positions[i as usize];
                    DVec3::new(f64::from(p[0]), f64::from(p[1]), f64::from(p[2]))
                });
                let (e1, e2) = (b - a, c - a);
                let p = d.cross(e2);
                let det = e1.dot(p);
                if det.abs() < 1e-18 {
                    return false;
                }
                let s = o - a;
                let u = s.dot(p) / det;
                let q = s.cross(e1);
                let v = d.dot(q) / det;
                u >= 0.0 && v >= 0.0 && u + v <= 1.0 && e2.dot(q) / det > 0.0
            })
            .count();
        crossings % 2 == 1
    })
}
