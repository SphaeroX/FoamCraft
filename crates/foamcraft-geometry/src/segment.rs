// SPDX-License-Identifier: GPL-3.0-or-later
//! Splits a mesh into faces (regions bounded by creases) and parts (connected bodies).
//!
//! Faces and parts are numbered by their lowest triangle index, so the numbers are stable as long
//! as the file and the crease angle stay the same.

use std::collections::HashMap;

use serde::Serialize;

use crate::diagnose::{Components, edge_key};
use crate::mesh::Mesh;

/// Faces smaller than this share of their part's area are merged into their largest neighbour.
/// Scans of organic shapes otherwise break into hundreds of tiny faces nobody can click.
pub const MIN_FACE_AREA_SHARE: f64 = 0.005;

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct FaceInfo {
    /// Face number, starting at 1.
    pub id: u32,
    /// Part (connected body) number, starting at 1.
    pub part: u32,
    pub first_triangle: usize,
    pub triangles: usize,
    /// Area in file units squared.
    pub area: f64,
    /// Area-weighted mean normal (unit length).
    pub normal: [f64; 3],
    pub centroid: [f64; 3],
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Segmentation {
    /// Face number of every triangle.
    pub face_of_triangle: Vec<u32>,
    pub faces: Vec<FaceInfo>,
    pub parts: u32,
}

fn triangle_geometry(mesh: &Mesh, t: [u32; 3]) -> ([f64; 3], f64, [f64; 3]) {
    let [a, b, c] = t.map(|i| mesh.positions[i as usize].map(f64::from));
    let u = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
    let v = [c[0] - a[0], c[1] - a[1], c[2] - a[2]];
    let n = [
        u[1] * v[2] - u[2] * v[1],
        u[2] * v[0] - u[0] * v[2],
        u[0] * v[1] - u[1] * v[0],
    ];
    let len = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt();
    let unit = if len > 0.0 {
        n.map(|x| x / len)
    } else {
        [0.0; 3]
    };
    let centroid = [0, 1, 2].map(|k| (a[k] + b[k] + c[k]) / 3.0);
    (unit, len / 2.0, centroid)
}

pub fn segment(mesh: &Mesh, crease_angle_deg: f64) -> Segmentation {
    let n = mesh.triangles.len();
    let geo: Vec<_> = mesh
        .triangles
        .iter()
        .map(|t| triangle_geometry(mesh, *t))
        .collect();
    let mut by_edge: HashMap<u64, Vec<u32>> = HashMap::with_capacity(n * 3 / 2);
    for (i, t) in mesh.triangles.iter().enumerate() {
        for (u, v) in [(t[0], t[1]), (t[1], t[2]), (t[2], t[0])] {
            by_edge.entry(edge_key(u, v)).or_default().push(i as u32);
        }
    }
    let mut neighbours: Vec<Vec<u32>> = vec![Vec::new(); n];
    for tris in by_edge.values() {
        if tris.len() == 2 {
            let (a, b) = (tris[0], tris[1]);
            neighbours[a as usize].push(b);
            neighbours[b as usize].push(a);
        }
    }
    let cos_limit = crease_angle_deg.to_radians().cos();
    let dot = |a: [f64; 3], b: [f64; 3]| a[0] * b[0] + a[1] * b[1] + a[2] * b[2];

    // CAD meshes bring their faces; otherwise grow regions. Either way region numbers follow the
    // first triangle.
    let mut region = vec![u32::MAX; n];
    let mut count = 0u32;
    if let Some(tags) = mesh.face_tags.as_ref().filter(|t| t.len() == n) {
        let mut number: HashMap<u32, u32> = HashMap::new();
        for (i, tag) in tags.iter().enumerate() {
            let next = number.len() as u32;
            region[i] = *number.entry(*tag).or_insert(next);
        }
        count = number.len() as u32;
    }
    for seed in 0..n {
        if region[seed] != u32::MAX {
            continue;
        }
        region[seed] = count;
        let mut stack = vec![seed];
        while let Some(i) = stack.pop() {
            for &j in &neighbours[i] {
                let j = j as usize;
                if region[j] == u32::MAX && dot(geo[i].0, geo[j].0) >= cos_limit {
                    region[j] = count;
                    stack.push(j);
                }
            }
        }
        count += 1;
    }

    // Parts: connected bodies over shared vertices.
    let mut comps = Components::new(mesh.positions.len());
    for t in &mesh.triangles {
        comps.union(t[0], t[1]);
        comps.union(t[1], t[2]);
    }
    let mut part_number: HashMap<u32, u32> = HashMap::new();
    let cad_parts = mesh
        .part_tags
        .as_ref()
        .filter(|t| t.len() == n && t.iter().all(|v| *v != 0));
    let part_of: Vec<u32> = mesh
        .triangles
        .iter()
        .enumerate()
        .map(|(i, t)| {
            // CAD volumes are the parts when every face belongs to one; else connected bodies.
            let key = match cad_parts {
                Some(tags) => tags[i],
                None => comps.find(t[0]),
            };
            let next = part_number.len() as u32 + 1;
            *part_number.entry(key).or_insert(next)
        })
        .collect();

    // Merge tiny regions into their largest neighbour region of the same part.
    let mut region_area = vec![0.0f64; count as usize];
    for i in 0..n {
        region_area[region[i] as usize] += geo[i].1;
    }
    let mut part_area: HashMap<u32, f64> = HashMap::new();
    for i in 0..n {
        *part_area.entry(part_of[i]).or_default() += geo[i].1;
    }
    let mut merged_into: Vec<u32> = (0..count).collect();
    let mut region_part = vec![0u32; count as usize];
    for i in 0..n {
        region_part[region[i] as usize] = part_of[i];
    }
    let mut members: Vec<Vec<usize>> = vec![Vec::new(); count as usize];
    for (i, r) in region.iter().enumerate() {
        members[*r as usize].push(i);
    }
    // CAD faces are real faces and never merged.
    let merge_small = mesh.face_tags.is_none();
    for r in 0..count as usize {
        if !merge_small {
            break;
        }
        let limit = part_area[&region_part[r]] * MIN_FACE_AREA_SHARE;
        if region_area[r] >= limit {
            continue;
        }
        let best = members[r]
            .iter()
            .flat_map(|&i| neighbours[i].iter().map(|&j| region[j as usize]))
            .filter(|&q| q as usize != r)
            .max_by(|a, b| {
                region_area[*a as usize]
                    .total_cmp(&region_area[*b as usize])
                    .then(b.cmp(a))
            });
        if let Some(q) = best {
            merged_into[r] = q;
        }
    }
    let resolve = |mut r: u32| {
        // Follow merge chains (a tiny region may merge into another tiny one).
        for _ in 0..merged_into.len() {
            let next = merged_into[r as usize];
            if next == r {
                break;
            }
            r = next;
        }
        r
    };

    // Final numbering by first triangle.
    let mut number: HashMap<u32, u32> = HashMap::new();
    let mut face_of_triangle = vec![0u32; n];
    let mut faces: Vec<FaceInfo> = Vec::new();
    for i in 0..n {
        let r = resolve(region[i]);
        let next = number.len() as u32 + 1;
        let id = *number.entry(r).or_insert(next);
        face_of_triangle[i] = id;
        if id as usize > faces.len() {
            faces.push(FaceInfo {
                id,
                part: part_of[i],
                first_triangle: i,
                triangles: 0,
                area: 0.0,
                normal: [0.0; 3],
                centroid: [0.0; 3],
            });
        }
        let f = &mut faces[id as usize - 1];
        let (normal, area, centroid) = geo[i];
        f.triangles += 1;
        f.area += area;
        for k in 0..3 {
            f.normal[k] += normal[k] * area;
            f.centroid[k] += centroid[k] * area;
        }
    }
    for f in &mut faces {
        let len = dot(f.normal, f.normal).sqrt();
        if len > 0.0 {
            f.normal = f.normal.map(|x| x / len);
        }
        if f.area > 0.0 {
            f.centroid = f.centroid.map(|x| x / f.area);
        }
    }
    Segmentation {
        face_of_triangle,
        faces,
        parts: part_number.len() as u32,
    }
}
