// SPDX-License-Identifier: GPL-3.0-or-later
//! Scene content: triangle meshes with per-vertex normals.

use bytemuck::{Pod, Zeroable};
use glam::Vec3;

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct Vertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    /// Pick id of the triangle this vertex belongs to (0 = not pickable).
    pub pick: u32,
}

/// A triangle mesh with one colour.
#[derive(Debug, Clone)]
pub struct Mesh {
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>,
    /// Linear RGB colour.
    pub color: [f32; 3],
    /// Indices are line pairs instead of triangles.
    pub lines: bool,
    /// Below 1 the mesh is drawn translucent (every other pixel, so no sorting is needed).
    pub opacity: f32,
    /// Linear RGB colour per vertex (for example a field shown with a colour scale); replaces
    /// `color` when set.
    pub vertex_colors: Option<Vec<[f32; 3]>>,
}

#[derive(Debug, Clone)]
pub struct Scene {
    pub meshes: Vec<Mesh>,
    /// Background in sRGB, 0..1.
    pub background: [f64; 4],
}

impl Scene {
    /// Unit cube centred at the origin, used to check the render path.
    pub fn test_cube() -> Self {
        Self {
            meshes: vec![cube_mesh(Vec3::ZERO, 1.0, [0.85, 0.45, 0.12])],
            background: BACKGROUND,
        }
    }

    /// Bounding sphere of all meshes (centre, radius). An empty scene gives a unit sphere.
    pub fn bounding_sphere(&self) -> (Vec3, f32) {
        let mut min = Vec3::splat(f32::INFINITY);
        let mut max = Vec3::splat(f32::NEG_INFINITY);
        for v in self.meshes.iter().flat_map(|m| m.vertices.iter()) {
            let p = Vec3::from(v.position);
            min = min.min(p);
            max = max.max(p);
        }
        if !min.is_finite() || !max.is_finite() {
            return (Vec3::ZERO, 1.0);
        }
        let center = (min + max) / 2.0;
        (center, ((max - min).length() / 2.0).max(1e-6))
    }

    /// Background as 8-bit sRGB, exactly as an offscreen frame stores it.
    pub fn background_rgba8(&self) -> [u8; 4] {
        self.background
            .map(|c| (c.clamp(0.0, 1.0) * 255.0).round() as u8)
    }
}

/// Default viewport background (sRGB): a calm dark grey-blue.
const BACKGROUND: [f64; 4] = [36.0 / 255.0, 40.0 / 255.0, 48.0 / 255.0, 1.0];

/// Axis-aligned cube with flat-shaded faces.
pub fn cube_mesh(center: Vec3, size: f32, color: [f32; 3]) -> Mesh {
    let h = size / 2.0;
    let faces: [(Vec3, Vec3, Vec3); 6] = [
        (Vec3::X, Vec3::Y, Vec3::Z),
        (Vec3::NEG_X, Vec3::NEG_Y, Vec3::Z),
        (Vec3::Y, Vec3::NEG_X, Vec3::Z),
        (Vec3::NEG_Y, Vec3::X, Vec3::Z),
        (Vec3::Z, Vec3::X, Vec3::Y),
        (Vec3::NEG_Z, Vec3::X, Vec3::NEG_Y),
    ];
    let mut vertices = Vec::with_capacity(24);
    let mut indices = Vec::with_capacity(36);
    for (normal, u, v) in faces {
        let base = vertices.len() as u32;
        for (su, sv) in [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)] {
            let p = center + (normal + u * su + v * sv) * h;
            vertices.push(Vertex {
                position: p.into(),
                normal: normal.into(),
                pick: 0,
            });
        }
        indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
    }
    Mesh {
        vertices,
        indices,
        color,
        lines: false,
        opacity: 1.0,
        vertex_colors: None,
    }
}

/// Vertex positions and triangles of one model, in world coordinates.
pub type ModelRef<'a> = (&'a [[f32; 3]], &'a [[u32; 3]]);

/// Colour of imported models.
pub const MODEL_COLOR: [f32; 3] = [0.72, 0.75, 0.80];
const AXIS_COLORS: [[f32; 3]; 3] = [[0.86, 0.25, 0.22], [0.30, 0.72, 0.32], [0.25, 0.45, 0.90]];

/// Flat-shaded mesh from indexed triangles (one normal per triangle).
pub fn mesh_from_triangles(
    positions: &[[f32; 3]],
    triangles: &[[u32; 3]],
    color: [f32; 3],
) -> Mesh {
    mesh_from_triangles_with_ids(positions, triangles, &[], color)
}

/// Like [`mesh_from_triangles`], with a pick id per triangle (missing ids are 0).
pub fn mesh_from_triangles_with_ids(
    positions: &[[f32; 3]],
    triangles: &[[u32; 3]],
    pick_ids: &[u32],
    color: [f32; 3],
) -> Mesh {
    let mut vertices = Vec::with_capacity(triangles.len() * 3);
    for (k, t) in triangles.iter().enumerate() {
        let pick = pick_ids.get(k).copied().unwrap_or(0);
        let [a, b, c] = t.map(|i| Vec3::from(positions[i as usize]));
        let n = (b - a).cross(c - a).normalize_or_zero();
        for p in [a, b, c] {
            vertices.push(Vertex {
                position: p.into(),
                normal: n.into(),
                pick,
            });
        }
    }
    let indices = (0..vertices.len() as u32).collect();
    Mesh {
        vertices,
        indices,
        color,
        lines: false,
        opacity: 1.0,
        vertex_colors: None,
    }
}

/// Coordinate axes at the origin: X red, Y green, Z blue.
pub fn axes(length: f32) -> Vec<Mesh> {
    let thickness = length * 0.02;
    [Vec3::X, Vec3::Y, Vec3::Z]
        .into_iter()
        .zip(AXIS_COLORS)
        .map(|(dir, color)| {
            let size = dir * length + (Vec3::ONE - dir) * thickness;
            box_mesh(dir * length / 2.0, size, color)
        })
        .collect()
}

/// Axis-aligned box with the given centre and edge lengths.
pub fn box_mesh(center: Vec3, size: Vec3, color: [f32; 3]) -> Mesh {
    let mut mesh = cube_mesh(Vec3::ZERO, 1.0, color);
    for v in &mut mesh.vertices {
        v.position = (Vec3::from(v.position) * size + center).into();
    }
    mesh
}

impl Scene {
    /// Imported models (world coordinates, metres) plus coordinate axes sized to the models.
    pub fn from_models(models: &[ModelRef<'_>]) -> Self {
        let mut meshes: Vec<Mesh> = models
            .iter()
            .map(|(p, t)| mesh_from_triangles(p, t, MODEL_COLOR))
            .collect();
        let extent = Scene {
            meshes: meshes.clone(),
            background: BACKGROUND,
        }
        .bounding_sphere()
        .1;
        // Longer than the model so the axes stick out of it.
        meshes.extend(axes((extent * 1.6).max(1e-3)));
        Self {
            meshes,
            background: BACKGROUND,
        }
    }

    /// Empty scene with only the axes (no project or no models).
    pub fn empty() -> Self {
        Self {
            meshes: axes(0.5),
            background: BACKGROUND,
        }
    }
}

const TUNNEL_COLOR: [f32; 3] = [0.45, 0.62, 0.80];
const GROUND_COLOR: [f32; 3] = [0.30, 0.32, 0.36];
const ARROW_COLOR: [f32; 3] = [0.95, 0.55, 0.15];

/// Rotates a mesh about the vertical axis through `pivot` and moves it there.
fn place(mut mesh: Mesh, pivot: Vec3, yaw_deg: f64) -> Mesh {
    let rot = glam::Quat::from_rotation_z((yaw_deg as f32).to_radians());
    for v in &mut mesh.vertices {
        v.position = (rot * Vec3::from(v.position) + pivot).into();
        v.normal = (rot * Vec3::from(v.normal)).into();
    }
    mesh
}

/// Arrow along +X from `start` with `length` and shaft `thickness` (local coordinates).
fn arrow(start: Vec3, length: f32, thickness: f32, color: [f32; 3]) -> Vec<Mesh> {
    let tip_length = length * 0.3;
    let shaft = box_mesh(
        start + Vec3::X * (length - tip_length) / 2.0,
        Vec3::new(length - tip_length, thickness, thickness),
        color,
    );
    // Pyramid tip: square base of three shaft widths.
    let base_x = start.x + length - tip_length;
    let r = thickness * 1.5;
    let apex = Vec3::new(start.x + length, start.y, start.z);
    let c = [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)]
        .map(|(a, b)| Vec3::new(base_x, start.y + a * r, start.z + b * r));
    let mut tris: Vec<[Vec3; 3]> = (0..4).map(|k| [c[k], c[(k + 1) % 4], apex]).collect();
    tris.push([c[0], c[3], c[2]]);
    tris.push([c[0], c[2], c[1]]);
    let positions: Vec<[f32; 3]> = tris.iter().flatten().map(|p| (*p).into()).collect();
    let triangles: Vec<[u32; 3]> = (0..tris.len() as u32)
        .map(|i| [3 * i, 3 * i + 1, 3 * i + 2])
        .collect();
    vec![shaft, mesh_from_triangles(&positions, &triangles, color)]
}

/// Wind tunnel outline (edges), ground plate and a flow arrow. `size` is length (along the flow),
/// width and height; the tunnel is turned by `yaw_deg` about its centre.
pub fn tunnel(center: [f64; 3], size: [f64; 3], yaw_deg: f64, ground: bool) -> Vec<Mesh> {
    let c = Vec3::new(center[0] as f32, center[1] as f32, center[2] as f32);
    let [l, w, h] = size.map(|v| v as f32);
    let t = l.max(w).max(h) * 0.003;
    let mut meshes = Vec::new();
    // Twelve edges: four along each axis.
    for (axis, len, a, b) in [(Vec3::X, l, w, h), (Vec3::Y, w, l, h), (Vec3::Z, h, l, w)] {
        let (u, v) = if axis == Vec3::X {
            (Vec3::Y, Vec3::Z)
        } else if axis == Vec3::Y {
            (Vec3::X, Vec3::Z)
        } else {
            (Vec3::X, Vec3::Y)
        };
        for (su, sv) in [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)] {
            let offset = u * su * a / 2.0 + v * sv * b / 2.0;
            let size = axis * len + (Vec3::ONE - axis) * t;
            meshes.push(place(box_mesh(offset, size, TUNNEL_COLOR), c, yaw_deg));
        }
    }
    if ground {
        let plate = box_mesh(
            Vec3::new(0.0, 0.0, -h / 2.0 - t / 2.0),
            Vec3::new(l, w, t),
            GROUND_COLOR,
        );
        meshes.push(place(plate, c, yaw_deg));
    }
    for part in arrow(
        Vec3::new(-l / 2.0, 0.0, 0.0),
        l * 0.12,
        l * 0.006,
        ARROW_COLOR,
    ) {
        meshes.push(place(part, c, yaw_deg));
    }
    meshes
}

/// Edges of a triangle mesh as lines, pushed outwards by `offset` along the vertex normals so
/// they stay visible over the same surface.
pub fn mesh_edges(
    positions: &[[f32; 3]],
    triangles: &[[u32; 3]],
    color: [f32; 3],
    offset: f32,
) -> Mesh {
    let mut normals = vec![Vec3::ZERO; positions.len()];
    for t in triangles {
        let [a, b, c] = t.map(|i| Vec3::from(positions[i as usize]));
        let n = (b - a).cross(c - a);
        for i in t {
            normals[*i as usize] += n;
        }
    }
    let vertices = positions
        .iter()
        .zip(&normals)
        .map(|(p, n)| {
            let n = n.normalize_or_zero();
            Vertex {
                position: (Vec3::from(*p) + n * offset).into(),
                normal: n.into(),
                pick: 0,
            }
        })
        .collect();
    let mut edges = std::collections::BTreeSet::new();
    for t in triangles {
        for (u, v) in [(t[0], t[1]), (t[1], t[2]), (t[2], t[0])] {
            edges.insert((u.min(v), u.max(v)));
        }
    }
    let indices = edges.into_iter().flat_map(|(u, v)| [u, v]).collect();
    Mesh {
        vertices,
        indices,
        color,
        lines: true,
        opacity: 1.0,
        vertex_colors: None,
    }
}

/// Line segments along polylines (closed ones also join the last point to the first).
pub fn polylines(lines: &[Vec<[f32; 3]>], color: [f32; 3], closed: bool) -> Mesh {
    let mut vertices = Vec::new();
    let mut indices = Vec::new();
    for line in lines {
        let base = vertices.len() as u32;
        vertices.extend(line.iter().map(|p| Vertex {
            position: *p,
            normal: [0.0, 0.0, 1.0],
            pick: 0,
        }));
        let n = line.len() as u32;
        let segments = if closed { n } else { n.saturating_sub(1) };
        for i in 0..segments {
            indices.push(base + i);
            indices.push(base + (i + 1) % n);
        }
    }
    Mesh {
        vertices,
        indices,
        color,
        lines: true,
        opacity: 1.0,
        vertex_colors: None,
    }
}
