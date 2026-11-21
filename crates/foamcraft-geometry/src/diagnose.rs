// SPDX-License-Identifier: GPL-3.0-or-later
//! Mesh diagnosis: watertightness, orientation, duplicates, degenerate triangles, bodies, size.

use std::collections::{HashMap, HashSet};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::mesh::Mesh;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    Info,
    Warning,
}

/// One diagnosis message. `code` is a catalog entry with English and German text.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Finding {
    pub code: String,
    pub severity: Severity,
    pub count: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Diagnosis {
    pub triangles: u64,
    pub vertices: u64,
    pub watertight: bool,
    pub boundary_loops: u64,
    pub non_manifold_edges: u64,
    /// Neighbouring triangles disagree about the outside.
    pub inconsistent_normals: bool,
    pub duplicate_triangles: u64,
    pub degenerate_triangles: u64,
    pub bodies: u64,
    /// Extent in file units (x, y, z).
    pub size: [f64; 3],
    /// Signed volume in file units cubed; negative when a closed mesh points inwards.
    pub volume: f64,
    pub findings: Vec<Finding>,
}

/// Union-find over vertex ids.
pub(crate) struct Components {
    parent: Vec<u32>,
}

impl Components {
    pub(crate) fn new(n: usize) -> Self {
        Self {
            parent: (0..n as u32).collect(),
        }
    }

    pub(crate) fn find(&mut self, mut x: u32) -> u32 {
        while self.parent[x as usize] != x {
            let p = self.parent[x as usize];
            self.parent[x as usize] = self.parent[p as usize];
            x = p;
        }
        x
    }

    pub(crate) fn union(&mut self, a: u32, b: u32) {
        let (ra, rb) = (self.find(a), self.find(b));
        if ra != rb {
            self.parent[ra as usize] = rb;
        }
    }
}

pub(crate) fn edge_key(a: u32, b: u32) -> u64 {
    let (lo, hi) = if a < b { (a, b) } else { (b, a) };
    (u64::from(lo) << 32) | u64::from(hi)
}

fn sub(a: [f32; 3], b: [f32; 3]) -> [f64; 3] {
    [
        f64::from(a[0] - b[0]),
        f64::from(a[1] - b[1]),
        f64::from(a[2] - b[2]),
    ]
}

fn cross(u: [f64; 3], v: [f64; 3]) -> [f64; 3] {
    [
        u[1] * v[2] - u[2] * v[1],
        u[2] * v[0] - u[0] * v[2],
        u[0] * v[1] - u[1] * v[0],
    ]
}

/// Signed volume of a closed mesh (divergence theorem).
pub(crate) fn signed_volume(mesh: &Mesh, tris: impl Iterator<Item = usize>) -> f64 {
    tris.map(|i| {
        let t = mesh.triangles[i];
        let [a, b, c] = t.map(|v| mesh.positions[v as usize].map(f64::from));
        a[0] * (b[1] * c[2] - b[2] * c[1]) - a[1] * (b[0] * c[2] - b[2] * c[0])
            + a[2] * (b[0] * c[1] - b[1] * c[0])
    })
    .sum::<f64>()
        / 6.0
}

pub fn diagnose(mesh: &Mesh) -> Diagnosis {
    let (min, max) = mesh.bounds();
    let size = [0, 1, 2].map(|k| f64::from(max[k] - min[k]));
    let diag2 = size
        .iter()
        .map(|s| s * s)
        .sum::<f64>()
        .max(f64::MIN_POSITIVE);

    // Undirected edge -> (use count, sum of directions); a consistently oriented shared edge is
    // used once in each direction, so the sum is zero.
    let mut edges: HashMap<u64, (u32, i32)> = HashMap::with_capacity(mesh.triangles.len() * 3 / 2);
    let mut seen: HashSet<[u32; 3]> = HashSet::with_capacity(mesh.triangles.len());
    let mut duplicate_triangles = 0;
    let mut degenerate_triangles = 0;
    let mut bodies = Components::new(mesh.positions.len());
    for t in &mesh.triangles {
        let [a, b, c] = *t;
        let mut sorted = *t;
        sorted.sort_unstable();
        if !seen.insert(sorted) {
            duplicate_triangles += 1;
        }
        let area2 = {
            let n = cross(
                sub(mesh.positions[b as usize], mesh.positions[a as usize]),
                sub(mesh.positions[c as usize], mesh.positions[a as usize]),
            );
            n.iter().map(|x| x * x).sum::<f64>()
        };
        // Relative to the model size, so millimetre and metre models are judged alike.
        if a == b || b == c || a == c || area2 < diag2 * diag2 * 1e-24 {
            degenerate_triangles += 1;
        }
        for (u, v) in [(a, b), (b, c), (c, a)] {
            let e = edges.entry(edge_key(u, v)).or_insert((0, 0));
            e.0 += 1;
            e.1 += if u < v { 1 } else { -1 };
        }
        bodies.union(a, b);
        bodies.union(b, c);
    }

    let mut boundary = Components::new(mesh.positions.len());
    let mut boundary_vertices = HashSet::new();
    let mut non_manifold_edges = 0;
    let mut inconsistent = 0u64;
    for (key, (count, dir)) in &edges {
        match count {
            1 => {
                let (lo, hi) = ((key >> 32) as u32, (*key & 0xffff_ffff) as u32);
                boundary.union(lo, hi);
                boundary_vertices.insert(lo);
                boundary_vertices.insert(hi);
            }
            2 if *dir != 0 => inconsistent += 1,
            2 => {}
            _ => non_manifold_edges += 1,
        }
    }
    let boundary_loops = boundary_vertices
        .iter()
        .map(|v| boundary.find(*v))
        .collect::<HashSet<_>>()
        .len() as u64;
    let used: HashSet<u32> = mesh.triangles.iter().flatten().copied().collect();
    let body_count = used
        .iter()
        .map(|v| bodies.find(*v))
        .collect::<HashSet<_>>()
        .len() as u64;
    let volume = signed_volume(mesh, 0..mesh.triangles.len());
    let watertight = boundary_loops == 0 && non_manifold_edges == 0;

    let mut findings = Vec::new();
    let mut warn = |code: &str, count: u64| {
        findings.push(Finding {
            code: code.into(),
            severity: Severity::Warning,
            count,
        })
    };
    if boundary_loops > 0 {
        warn("geometry.open", boundary_loops);
    }
    if non_manifold_edges > 0 {
        warn("geometry.non_manifold", non_manifold_edges);
    }
    if inconsistent > 0 {
        warn("geometry.flipped_normals", inconsistent);
    } else if watertight && volume < 0.0 {
        warn("geometry.inverted", 1);
    }
    if duplicate_triangles > 0 {
        warn("geometry.duplicate_triangles", duplicate_triangles);
    }
    if degenerate_triangles > 0 {
        warn("geometry.degenerate_triangles", degenerate_triangles);
    }
    if body_count > 1 {
        findings.push(Finding {
            code: "geometry.multiple_bodies".into(),
            severity: Severity::Info,
            count: body_count,
        });
    }
    if findings.iter().all(|f| f.severity == Severity::Info) {
        findings.push(Finding {
            code: "geometry.ok".into(),
            severity: Severity::Info,
            count: 0,
        });
    }

    Diagnosis {
        triangles: mesh.triangles.len() as u64,
        vertices: used.len() as u64,
        watertight,
        boundary_loops,
        non_manifold_edges,
        inconsistent_normals: inconsistent > 0,
        duplicate_triangles,
        degenerate_triangles,
        bodies: body_count,
        size,
        volume,
        findings,
    }
}

/// Suggested unit from the largest extent: models larger than 5 file units are assumed to be in
/// millimetres (parts drawn in mm are rarely smaller than 5 mm; full-size objects in metres rarely
/// exceed a few metres in a CFD case).
pub fn suggest_unit(largest_extent: f64) -> &'static str {
    if largest_extent > 5.0 { "mm" } else { "m" }
}
