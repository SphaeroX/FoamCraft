// SPDX-License-Identifier: GPL-3.0-or-later
//! Units, placement in the world and the only repair FoamCraft does itself: consistent normals.

use std::collections::{HashMap, VecDeque};

use glam::{DMat3, DVec3};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::diagnose::{edge_key, signed_volume};
use crate::mesh::Mesh;

/// Metres per file unit.
pub fn unit_scale(unit: &str) -> Option<f64> {
    match unit {
        "um" => Some(1e-6),
        "mm" => Some(0.001),
        "cm" => Some(0.01),
        "m" => Some(1.0),
        "in" => Some(0.0254),
        "ft" => Some(0.3048),
        _ => None,
    }
}

pub const UNITS: [&str; 6] = ["um", "mm", "cm", "m", "in", "ft"];

/// How a model sits in the world (metres, Z up).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Placement {
    /// Unit of the file coordinates: um, mm, cm, m, in or ft.
    pub unit: String,
    /// Extra uniform scale factor.
    pub scale: f64,
    /// Yaw (about Z), pitch (about Y), roll (about X) in degrees, applied roll first.
    pub rotation_deg: [f64; 3],
    /// Translation in metres after scaling and rotation.
    pub translation: [f64; 3],
}

impl Default for Placement {
    fn default() -> Self {
        Self {
            unit: "m".into(),
            scale: 1.0,
            rotation_deg: [0.0; 3],
            translation: [0.0; 3],
        }
    }
}

impl Placement {
    pub fn rotation(&self) -> DMat3 {
        let [yaw, pitch, roll] = self.rotation_deg.map(f64::to_radians);
        DMat3::from_rotation_z(yaw) * DMat3::from_rotation_y(pitch) * DMat3::from_rotation_x(roll)
    }

    /// The mesh in world coordinates (metres).
    pub fn apply(&self, mesh: &Mesh) -> Mesh {
        let s = unit_scale(&self.unit).unwrap_or(1.0) * self.scale;
        let r = self.rotation();
        let t = DVec3::from(self.translation);
        let positions = mesh
            .positions
            .iter()
            .map(|p| {
                let w = r * (DVec3::new(f64::from(p[0]), f64::from(p[1]), f64::from(p[2])) * s) + t;
                [w.x as f32, w.y as f32, w.z as f32]
            })
            .collect();
        Mesh {
            positions,
            triangles: mesh.triangles.clone(),
            format: mesh.format,
            declared_unit: None,
            face_tags: mesh.face_tags.clone(),
            part_tags: mesh.part_tags.clone(),
        }
    }

    /// The placement turned by `angle` radians about the world axis `axis` through `pivot`
    /// (metres). Scale and unit stay.
    pub fn rotated_about(&self, axis: DVec3, angle: f64, pivot: DVec3) -> Placement {
        let turn = DMat3::from_axis_angle(axis.normalize(), angle);
        let rotation = turn * self.rotation();
        let t = pivot + turn * (DVec3::from(self.translation) - pivot);
        Placement {
            rotation_deg: rotation_to_deg(rotation),
            translation: t.into(),
            ..self.clone()
        }
    }
}

/// Yaw, pitch and roll in degrees of a rotation `Rz(yaw)·Ry(pitch)·Rx(roll)`. At pitch ±90° the
/// roll is set to zero and the yaw carries the whole turn about the vertical.
pub fn rotation_to_deg(r: DMat3) -> [f64; 3] {
    // glam stores columns: r.col(c)[row].
    let m = |row: usize, col: usize| r.col(col)[row];
    let s = (-m(2, 0)).clamp(-1.0, 1.0);
    let pitch = s.asin();
    let (yaw, roll) = if s.abs() > 1.0 - 1e-12 {
        ((-m(0, 1)).atan2(m(1, 1)), 0.0)
    } else {
        (m(1, 0).atan2(m(0, 0)), m(2, 1).atan2(m(2, 2)))
    };
    [yaw, pitch, roll].map(f64::to_degrees)
}

/// Points a model can be moved and turned about, in metres.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Pivots {
    /// Centre of the bounding box.
    pub center: [f64; 3],
    /// Centre of the enclosed volume (the box centre for an open or flat model).
    pub centroid: [f64; 3],
    /// Origin of the model file, i.e. the translation.
    pub origin: [f64; 3],
}

/// Pivots of a model in world coordinates; `origin` is its translation.
pub fn pivots(world: &Mesh, origin: [f64; 3]) -> Pivots {
    let (min, max) = world.bounds();
    let center = [0, 1, 2].map(|k| (f64::from(min[k]) + f64::from(max[k])) / 2.0);
    // Sum over signed tetrahedra (triangle plus the box centre, for accuracy far from zero).
    let c = DVec3::from(center);
    let mut volume = 0.0;
    let mut moment = DVec3::ZERO;
    for t in &world.triangles {
        let [a, b, d] = t.map(|v| DVec3::from(world.positions[v as usize].map(f64::from)) - c);
        let v = a.dot(b.cross(d)) / 6.0;
        volume += v;
        moment += v * (a + b + d) / 4.0;
    }
    let size = DVec3::from(max.map(f64::from)) - DVec3::from(min.map(f64::from));
    let centroid = if volume.abs() > 1e-9 * size.x.max(size.y).max(size.z).powi(3).max(1e-30) {
        (c + moment / volume).into()
    } else {
        center
    };
    Pivots {
        center,
        centroid,
        origin,
    }
}

/// Vertical shift that puts the lowest point of `world` at z = 0.
pub fn ground_offset(world: &Mesh) -> f64 {
    -f64::from(world.bounds().0[2])
}

/// Orients all triangles of each connected part consistently, then turns every part outward
/// (positive volume).
pub fn fix_normals(mesh: &mut Mesh) {
    let n = mesh.triangles.len();
    let mut by_edge: HashMap<u64, Vec<u32>> = HashMap::with_capacity(n * 3 / 2);
    for (i, t) in mesh.triangles.iter().enumerate() {
        for (u, v) in [(t[0], t[1]), (t[1], t[2]), (t[2], t[0])] {
            by_edge.entry(edge_key(u, v)).or_default().push(i as u32);
        }
    }
    let directed =
        |t: [u32; 3], u: u32, v: u32| [(t[0], t[1]), (t[1], t[2]), (t[2], t[0])].contains(&(u, v));
    let mut visited = vec![false; n];
    for start in 0..n {
        if visited[start] {
            continue;
        }
        visited[start] = true;
        let mut part = vec![start];
        let mut queue = VecDeque::from([start]);
        while let Some(i) = queue.pop_front() {
            let t = mesh.triangles[i];
            for (u, v) in [(t[0], t[1]), (t[1], t[2]), (t[2], t[0])] {
                let Some(neighbours) = by_edge.get(&edge_key(u, v)) else {
                    continue;
                };
                // Only manifold edges carry a reliable orientation.
                if neighbours.len() != 2 {
                    continue;
                }
                for &j in neighbours {
                    let j = j as usize;
                    if visited[j] {
                        continue;
                    }
                    // A consistent neighbour walks the shared edge in the opposite direction.
                    if directed(mesh.triangles[j], u, v) {
                        mesh.triangles[j].swap(1, 2);
                    }
                    visited[j] = true;
                    part.push(j);
                    queue.push_back(j);
                }
            }
        }
        if signed_volume(mesh, part.iter().copied()) < 0.0 {
            for &i in &part {
                mesh.triangles[i].swap(1, 2);
            }
        }
    }
}
