// SPDX-License-Identifier: GPL-3.0-or-later
//! Indexed triangle mesh.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Format {
    Stl,
    Obj,
    #[serde(rename = "3mf")]
    ThreeMf,
    Step,
    Iges,
    /// Gmsh mesh converted from STEP or IGES.
    Msh,
}

/// Triangle mesh in file units. Vertices with identical coordinates are merged, so the topology
/// (shared edges) is explicit.
#[derive(Debug, Clone, PartialEq)]
pub struct Mesh {
    pub positions: Vec<[f32; 3]>,
    pub triangles: Vec<[u32; 3]>,
    pub format: Format,
    /// Unit written in the file (3MF), `None` when the format has no unit.
    pub declared_unit: Option<String>,
    /// CAD face of every triangle (meshes converted from STEP or IGES).
    pub face_tags: Option<Vec<u32>>,
    /// CAD volume of every triangle, 0 when the face bounds no volume.
    pub part_tags: Option<Vec<u32>>,
}

impl Mesh {
    /// Axis-aligned bounds `(min, max)`; zeros for an empty mesh.
    pub fn bounds(&self) -> ([f32; 3], [f32; 3]) {
        if self.positions.is_empty() {
            return ([0.0; 3], [0.0; 3]);
        }
        let mut min = [f32::INFINITY; 3];
        let mut max = [f32::NEG_INFINITY; 3];
        for p in &self.positions {
            for k in 0..3 {
                min[k] = min[k].min(p[k]);
                max[k] = max[k].max(p[k]);
            }
        }
        (min, max)
    }
}

/// Collects triangles from loose corner coordinates and merges equal vertices.
#[derive(Default)]
pub struct MeshBuilder {
    positions: Vec<[f32; 3]>,
    triangles: Vec<[u32; 3]>,
    index: HashMap<[u32; 3], u32>,
}

fn key(p: [f32; 3]) -> [u32; 3] {
    // -0.0 and 0.0 are the same point.
    p.map(|c| if c == 0.0 { 0u32 } else { c.to_bits() })
}

impl MeshBuilder {
    pub fn with_capacity(triangles: usize) -> Self {
        Self {
            positions: Vec::with_capacity(triangles / 2),
            triangles: Vec::with_capacity(triangles),
            index: HashMap::with_capacity(triangles / 2),
        }
    }

    pub fn vertex(&mut self, p: [f32; 3]) -> u32 {
        let next = self.positions.len() as u32;
        let id = *self.index.entry(key(p)).or_insert(next);
        if id == next {
            self.positions.push(p);
        }
        id
    }

    pub fn triangle(&mut self, a: [f32; 3], b: [f32; 3], c: [f32; 3]) {
        let t = [self.vertex(a), self.vertex(b), self.vertex(c)];
        self.triangles.push(t);
    }

    pub fn indexed_triangle(&mut self, t: [u32; 3]) {
        self.triangles.push(t);
    }

    pub fn finish(self, format: Format, declared_unit: Option<String>) -> Mesh {
        Mesh {
            positions: self.positions,
            triangles: self.triangles,
            format,
            declared_unit,
            face_tags: None,
            part_tags: None,
        }
    }
}
