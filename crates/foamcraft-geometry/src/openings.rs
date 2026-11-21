// SPDX-License-Identifier: GPL-3.0-or-later
//! Openings of internal flow models and the caps that close them.
//!
//! Three kinds of models are told apart: an open shell (its boundary loops are the openings), a
//! thick-walled body with passages (a closed solid of genus ≥ 1; the openings are holes in its
//! flat outer faces) and a closed body whose inside is the fluid. Caps are flat triangulations of
//! the opening polygons (with holes for ring-shaped openings). A flood test on a voxel grid checks
//! that the fluid region is closed and finds a point inside it.

use std::collections::{HashMap, VecDeque};

use crate::diagnose::edge_key;
use crate::mesh::Mesh;
use crate::segment::{Segmentation, segment};

/// Openings whose points deviate more than this share of their diameter from a plane are not flat.
pub const NONPLANAR_LIMIT: f64 = 0.02;
/// Crease angle for finding flat faces of thick-walled bodies.
const FACE_CREASE_DEG: f64 = 30.0;
/// Voxels along the largest extent in the flood test.
pub const FLOOD_RESOLUTION: f64 = 96.0;

type P = [f64; 3];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GeometryKind {
    /// Surfaces with open edges: the edges around each opening form a loop.
    OpenShell,
    /// A closed solid with `passages` tunnels through it (pipes, housings with bores).
    ThickWalled { passages: u32 },
    /// A closed solid without tunnels: its inside is the fluid.
    ClosedVolume,
}

/// An opening: its outer loop and, for ring-shaped openings, inner loops (holes).
#[derive(Debug, Clone, PartialEq)]
pub struct Opening {
    /// Loops in model coordinates; the first is the outer one.
    pub loops: Vec<Vec<P>>,
    pub centroid: P,
    /// Unit normal of the best-fit plane.
    pub normal: P,
    /// Area of the opening (outer minus holes) in model units squared.
    pub area: f64,
    /// Largest distance of a loop point from the best-fit plane relative to the diameter.
    pub deviation: f64,
    pub planar: bool,
}

fn sub(a: P, b: P) -> P {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}
fn dot(a: P, b: P) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
fn cross(a: P, b: P) -> P {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}
fn norm(a: P) -> f64 {
    dot(a, a).sqrt()
}
fn unit(a: P) -> P {
    let l = norm(a);
    if l > 0.0 {
        a.map(|x| x / l)
    } else {
        [0.0, 0.0, 1.0]
    }
}

fn pos(mesh: &Mesh, i: u32) -> P {
    mesh.positions[i as usize].map(f64::from)
}

/// Chains directed edges into closed loops of vertex ids.
fn chain(edges: &[(u32, u32)]) -> Vec<Vec<u32>> {
    let mut next: HashMap<u32, Vec<u32>> = HashMap::new();
    for &(a, b) in edges {
        next.entry(a).or_default().push(b);
    }
    let mut loops = Vec::new();
    let mut starts: Vec<u32> = edges.iter().map(|e| e.0).collect();
    starts.sort_unstable();
    for start in starts {
        if next.get(&start).is_none_or(|v| v.is_empty()) {
            continue;
        }
        let mut lp = vec![start];
        let mut cur = start;
        while let Some(n) = next.get_mut(&cur).and_then(|v| v.pop()) {
            if n == start {
                break;
            }
            lp.push(n);
            cur = n;
            if lp.len() > edges.len() {
                break;
            }
        }
        if lp.len() >= 3 {
            loops.push(lp);
        }
    }
    loops
}

/// Loops of edges that belong to only one triangle.
pub fn boundary_loops(mesh: &Mesh) -> Vec<Vec<u32>> {
    let mut count: HashMap<u64, u32> = HashMap::new();
    for t in &mesh.triangles {
        for k in 0..3 {
            *count.entry(edge_key(t[k], t[(k + 1) % 3])).or_default() += 1;
        }
    }
    let edges: Vec<(u32, u32)> = mesh
        .triangles
        .iter()
        .flat_map(|t| (0..3).map(move |k| (t[k], t[(k + 1) % 3])))
        .filter(|&(a, b)| count.get(&edge_key(a, b)) == Some(&1))
        .collect();
    chain(&edges)
}

/// Loops around one face: edges whose neighbour triangle belongs to another face.
pub(crate) fn face_loops(mesh: &Mesh, seg: &Segmentation, face: u32) -> Vec<Vec<u32>> {
    let mut owners: HashMap<u64, Vec<u32>> = HashMap::new();
    for (i, t) in mesh.triangles.iter().enumerate() {
        for k in 0..3 {
            owners
                .entry(edge_key(t[k], t[(k + 1) % 3]))
                .or_default()
                .push(seg.face_of_triangle[i]);
        }
    }
    let mut edges = Vec::new();
    for (i, t) in mesh.triangles.iter().enumerate() {
        if seg.face_of_triangle[i] != face {
            continue;
        }
        for k in 0..3 {
            let (a, b) = (t[k], t[(k + 1) % 3]);
            let o = &owners[&edge_key(a, b)];
            // Used once by this face: the other side is another face or nothing.
            if o.iter().filter(|f| **f == face).count() == 1 {
                edges.push((a, b));
            }
        }
    }
    chain(&edges)
}

/// Kind of model from its topology.
pub fn classify(mesh: &Mesh) -> GeometryKind {
    if !boundary_loops(mesh).is_empty() {
        return GeometryKind::OpenShell;
    }
    // Euler characteristic per connected body: chi = V - E + F = 2 - 2 g.
    let mut edges: HashMap<u64, ()> = HashMap::new();
    for t in &mesh.triangles {
        for k in 0..3 {
            edges.insert(edge_key(t[k], t[(k + 1) % 3]), ());
        }
    }
    let mut used = vec![false; mesh.positions.len()];
    for t in &mesh.triangles {
        for &v in t {
            used[v as usize] = true;
        }
    }
    let v = used.iter().filter(|u| **u).count() as i64;
    let chi = v - edges.len() as i64 + mesh.triangles.len() as i64;
    let bodies = crate::diagnose::diagnose(mesh).bodies as i64;
    let genus = ((2 * bodies - chi) / 2).max(0) as u32;
    if genus == 0 {
        GeometryKind::ClosedVolume
    } else {
        GeometryKind::ThickWalled { passages: genus }
    }
}

/// Best-fit plane (Newell normal through the centroid) and the in-plane basis.
struct Plane {
    origin: P,
    normal: P,
    u: P,
    v: P,
}

impl Plane {
    fn of(points: &[P]) -> Self {
        let n = points.len().max(1) as f64;
        let mut c = [0.0; 3];
        for p in points {
            for k in 0..3 {
                c[k] += p[k] / n;
            }
        }
        let mut normal = [0.0; 3];
        for i in 0..points.len() {
            let a = points[i];
            let b = points[(i + 1) % points.len()];
            normal[0] += (a[1] - b[1]) * (a[2] + b[2]);
            normal[1] += (a[2] - b[2]) * (a[0] + b[0]);
            normal[2] += (a[0] - b[0]) * (a[1] + b[1]);
        }
        let normal = unit(normal);
        let helper = if normal[0].abs() < 0.9 {
            [1.0, 0.0, 0.0]
        } else {
            [0.0, 1.0, 0.0]
        };
        let u = unit(cross(normal, helper));
        let v = cross(normal, u);
        Self {
            origin: c,
            normal,
            u,
            v,
        }
    }

    fn project(&self, p: P) -> [f64; 2] {
        let d = sub(p, self.origin);
        [dot(d, self.u), dot(d, self.v)]
    }
}

fn signed_area(poly: &[[f64; 2]]) -> f64 {
    let mut a = 0.0;
    for i in 0..poly.len() {
        let p = poly[i];
        let q = poly[(i + 1) % poly.len()];
        a += p[0] * q[1] - q[0] * p[1];
    }
    a / 2.0
}

fn inside_polygon(poly: &[[f64; 2]], p: [f64; 2]) -> bool {
    let mut inside = false;
    let mut j = poly.len() - 1;
    for i in 0..poly.len() {
        let (a, b) = (poly[i], poly[j]);
        if (a[1] > p[1]) != (b[1] > p[1])
            && p[0] < (b[0] - a[0]) * (p[1] - a[1]) / (b[1] - a[1]) + a[0]
        {
            inside = !inside;
        }
        j = i;
    }
    inside
}

fn diameter(points: &[P]) -> f64 {
    let mut lo = [f64::MAX; 3];
    let mut hi = [f64::MIN; 3];
    for p in points {
        for k in 0..3 {
            lo[k] = lo[k].min(p[k]);
            hi[k] = hi[k].max(p[k]);
        }
    }
    norm(sub(hi, lo))
}

struct LoopInfo {
    points: Vec<P>,
    plane: Plane,
    area: f64,
    diameter: f64,
}

fn loop_info(points: Vec<P>) -> LoopInfo {
    let plane = Plane::of(&points);
    let poly: Vec<[f64; 2]> = points.iter().map(|p| plane.project(*p)).collect();
    let area = signed_area(&poly).abs();
    let diameter = diameter(&points);
    LoopInfo {
        points,
        plane,
        area,
        diameter,
    }
}

/// Groups loops into openings: a coplanar loop inside another one is its hole.
fn group(loops: Vec<Vec<P>>) -> Vec<Opening> {
    let infos: Vec<LoopInfo> = loops.into_iter().map(loop_info).collect();
    let mut parent: Vec<Option<usize>> = vec![None; infos.len()];
    for (j, inner) in infos.iter().enumerate() {
        let mut best: Option<usize> = None;
        for (i, outer) in infos.iter().enumerate() {
            if i == j || inner.area >= outer.area {
                continue;
            }
            let coplanar = dot(inner.plane.normal, outer.plane.normal).abs() > 0.98
                && dot(
                    sub(inner.plane.origin, outer.plane.origin),
                    outer.plane.normal,
                )
                .abs()
                    < 0.02 * outer.diameter;
            let poly: Vec<[f64; 2]> = outer
                .points
                .iter()
                .map(|p| outer.plane.project(*p))
                .collect();
            if coplanar
                && inside_polygon(&poly, outer.plane.project(inner.points[0]))
                && best.is_none_or(|b| infos[b].area > outer.area)
            {
                best = Some(i);
            }
        }
        parent[j] = best;
    }
    let mut out = Vec::new();
    for (i, info) in infos.iter().enumerate() {
        if parent[i].is_some() {
            continue;
        }
        let holes: Vec<&LoopInfo> = (0..infos.len())
            .filter(|j| parent[*j] == Some(i))
            .map(|j| &infos[j])
            .collect();
        let deviation = info
            .points
            .iter()
            .map(|p| dot(sub(*p, info.plane.origin), info.plane.normal).abs())
            .fold(0.0, f64::max)
            / info.diameter.max(1e-12);
        let mut loops = vec![info.points.clone()];
        loops.extend(holes.iter().map(|h| h.points.clone()));
        out.push(Opening {
            loops,
            centroid: info.plane.origin,
            normal: info.plane.normal,
            area: info.area - holes.iter().map(|h| h.area).sum::<f64>(),
            deviation,
            planar: deviation <= NONPLANAR_LIMIT,
        });
    }
    out
}

/// An opening from stored loops (outer first, then holes).
pub fn opening_from_loops(loops: Vec<Vec<P>>) -> Opening {
    let mut it = loops.into_iter();
    let outer = loop_info(it.next().unwrap_or_default());
    let holes: Vec<LoopInfo> = it.map(loop_info).collect();
    let deviation = outer
        .points
        .iter()
        .map(|p| dot(sub(*p, outer.plane.origin), outer.plane.normal).abs())
        .fold(0.0, f64::max)
        / outer.diameter.max(1e-12);
    let mut all = vec![outer.points.clone()];
    all.extend(holes.iter().map(|h| h.points.clone()));
    Opening {
        loops: all,
        centroid: outer.plane.origin,
        normal: outer.plane.normal,
        area: outer.area - holes.iter().map(|h| h.area).sum::<f64>(),
        deviation,
        planar: deviation <= NONPLANAR_LIMIT,
    }
}

fn points_of(mesh: &Mesh, lp: &[u32]) -> Vec<P> {
    lp.iter().map(|i| pos(mesh, *i)).collect()
}

/// True when a ray from `origin` along `dir` leaves the mesh without hitting it.
fn escapes(mesh: &Mesh, origin: P, dir: P) -> bool {
    let start = [0, 1, 2].map(|k| origin[k] + dir[k] * 1e-6 * (1.0 + norm(origin)));
    crate::query::raycast(mesh, start, dir).is_none()
}

/// Openings of a model in model coordinates.
pub fn openings(mesh: &Mesh) -> Vec<Opening> {
    match classify(mesh) {
        GeometryKind::OpenShell => {
            let loops = boundary_loops(mesh)
                .iter()
                .map(|l| points_of(mesh, l))
                .collect();
            group(loops)
        }
        GeometryKind::ClosedVolume => Vec::new(),
        GeometryKind::ThickWalled { .. } => thick_mouths(mesh),
    }
}

/// Holes in flat faces that can be seen from outside: the mouths of the passages.
fn thick_mouths(mesh: &Mesh) -> Vec<Opening> {
    let seg = segment(mesh, FACE_CREASE_DEG);
    let mut out = Vec::new();
    for face in &seg.faces {
        let tris: Vec<usize> = (0..mesh.triangles.len())
            .filter(|i| seg.face_of_triangle[*i] == face.id)
            .collect();
        // Flat: every triangle normal close to the face normal.
        let flat = tris.iter().all(|i| {
            let t = mesh.triangles[*i];
            let [a, b, c] = t.map(|k| pos(mesh, k));
            dot(unit(cross(sub(b, a), sub(c, a))), face.normal) > 0.999
        });
        if !flat {
            continue;
        }
        let Some(probe) = tris.iter().max_by(|x, y| {
            let area = |i: usize| {
                let [a, b, c] = mesh.triangles[i].map(|k| pos(mesh, k));
                norm(cross(sub(b, a), sub(c, a)))
            };
            area(**x).total_cmp(&area(**y))
        }) else {
            continue;
        };
        let [a, b, c] = mesh.triangles[*probe].map(|k| pos(mesh, k));
        let centre = [0, 1, 2].map(|k| (a[k] + b[k] + c[k]) / 3.0);
        if !escapes(mesh, centre, face.normal) {
            continue;
        }
        let loops: Vec<LoopInfo> = face_loops(mesh, &seg, face.id)
            .iter()
            .map(|l| loop_info(points_of(mesh, l)))
            .collect();
        if loops.len() < 2 {
            continue;
        }
        let outer = loops
            .iter()
            .enumerate()
            .max_by(|x, y| x.1.area.total_cmp(&y.1.area))
            .map(|(i, _)| i)
            .unwrap_or(0);
        for (i, l) in loops.into_iter().enumerate() {
            if i != outer {
                out.extend(group(vec![l.points]));
            }
        }
    }
    out
}

fn distance_to_loop(points: &[P], p: P) -> f64 {
    let mut best = f64::MAX;
    for i in 0..points.len() {
        let a = points[i];
        let b = points[(i + 1) % points.len()];
        let ab = sub(b, a);
        let t = (dot(sub(p, a), ab) / dot(ab, ab).max(1e-300)).clamp(0.0, 1.0);
        let q = [0, 1, 2].map(|k| a[k] + ab[k] * t);
        best = best.min(norm(sub(p, q)));
    }
    best
}

/// The opening closest to `point` (model coordinates). For closed bodies, `face` names the face
/// that was clicked; the loop of that face nearest to the point is the opening.
pub fn opening_near(mesh: &Mesh, point: P, face: Option<(&Segmentation, u32)>) -> Option<Opening> {
    let candidates: Vec<Opening> = match face {
        Some((seg, id)) if classify(mesh) != GeometryKind::OpenShell => face_loops(mesh, seg, id)
            .iter()
            .flat_map(|l| group(vec![points_of(mesh, l)]))
            .collect(),
        _ => openings(mesh),
    };
    candidates.into_iter().min_by(|a, b| {
        let da = a
            .loops
            .iter()
            .map(|l| distance_to_loop(l, point))
            .fold(f64::MAX, f64::min);
        let db = b
            .loops
            .iter()
            .map(|l| distance_to_loop(l, point))
            .fold(f64::MAX, f64::min);
        da.total_cmp(&db)
    })
}

/// Triangles that close the opening (outer loop minus holes), in model coordinates.
pub fn cap_triangles(opening: &Opening) -> Vec<[P; 3]> {
    let plane = Plane::of(&opening.loops[0]);
    // One vertex list: outer counter-clockwise, holes clockwise, holes bridged into the outer.
    let mut pts3: Vec<P> = Vec::new();
    let mut pts2: Vec<[f64; 2]> = Vec::new();
    let mut ring = |points: &[P], ccw: bool| -> Vec<usize> {
        let poly: Vec<[f64; 2]> = points.iter().map(|p| plane.project(*p)).collect();
        let mut idx: Vec<usize> = (0..points.len())
            .map(|i| {
                pts3.push(points[i]);
                pts2.push(poly[i]);
                pts3.len() - 1
            })
            .collect();
        if (signed_area(&poly) > 0.0) != ccw {
            idx.reverse();
        }
        idx
    };
    let mut outer = ring(&opening.loops[0], true);
    let mut holes: Vec<Vec<usize>> = opening.loops[1..].iter().map(|h| ring(h, false)).collect();
    // Bridge holes in order of their rightmost point.
    holes.sort_by(|a, b| {
        let mx = |h: &Vec<usize>| h.iter().map(|i| pts2[*i][0]).fold(f64::MIN, f64::max);
        mx(b).total_cmp(&mx(a))
    });
    for hole in holes {
        let (hi, &hv) = hole
            .iter()
            .enumerate()
            .max_by(|a, b| pts2[*a.1][0].total_cmp(&pts2[*b.1][0]))
            .unwrap_or((0, &hole[0]));
        let h = pts2[hv];
        // Closest outer vertex whose bridge crosses no edge.
        let mut best: Option<(usize, f64)> = None;
        for (oi, &ov) in outer.iter().enumerate() {
            let o = pts2[ov];
            let d = (o[0] - h[0]).powi(2) + (o[1] - h[1]).powi(2);
            if best.is_some_and(|(_, bd)| bd <= d) {
                continue;
            }
            let crosses = |ring: &[usize]| {
                (0..ring.len()).any(|k| {
                    let (a, b) = (ring[k], ring[(k + 1) % ring.len()]);
                    a != ov
                        && b != ov
                        && a != hv
                        && b != hv
                        && segments_cross(pts2[a], pts2[b], o, h)
                })
            };
            if !crosses(&outer) && !crosses(&hole) {
                best = Some((oi, d));
            }
        }
        let Some((oi, _)) = best else { continue };
        let ov = outer[oi];
        let mut merged: Vec<usize> = outer[..=oi].to_vec();
        for k in 0..=hole.len() {
            merged.push(hole[(hi + k) % hole.len()]);
        }
        merged.push(ov);
        merged.extend_from_slice(&outer[oi + 1..]);
        outer = merged;
    }
    ear_clip(&outer, &pts2)
        .into_iter()
        .map(|t| t.map(|i| pts3[i]))
        .collect()
}

fn orient(a: [f64; 2], b: [f64; 2], c: [f64; 2]) -> f64 {
    (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0])
}

fn segments_cross(a: [f64; 2], b: [f64; 2], c: [f64; 2], d: [f64; 2]) -> bool {
    let (d1, d2) = (orient(c, d, a), orient(c, d, b));
    let (d3, d4) = (orient(a, b, c), orient(a, b, d));
    (d1 * d2 < 0.0) && (d3 * d4 < 0.0)
}

fn in_triangle(p: [f64; 2], a: [f64; 2], b: [f64; 2], c: [f64; 2]) -> bool {
    orient(a, b, p) > 0.0 && orient(b, c, p) > 0.0 && orient(c, a, p) > 0.0
}

/// Ear clipping of a simple counter-clockwise polygon (bridged holes allowed).
fn ear_clip(poly: &[usize], pts: &[[f64; 2]]) -> Vec<[usize; 3]> {
    let mut v: Vec<usize> = poly.to_vec();
    let mut out = Vec::new();
    let mut guard = 0;
    while v.len() > 3 && guard < 10 * poly.len() * poly.len() + 100 {
        guard += 1;
        let n = v.len();
        let mut clipped = false;
        for i in 0..n {
            let (a, b, c) = (v[(i + n - 1) % n], v[i], v[(i + 1) % n]);
            if orient(pts[a], pts[b], pts[c]) <= 0.0 {
                continue;
            }
            let blocked = v.iter().any(|&q| {
                q != a
                    && q != b
                    && q != c
                    && pts[q] != pts[a]
                    && pts[q] != pts[b]
                    && pts[q] != pts[c]
                    && in_triangle(pts[q], pts[a], pts[b], pts[c])
            });
            if !blocked {
                out.push([a, b, c]);
                v.remove(i);
                clipped = true;
                break;
            }
        }
        if !clipped {
            // Degenerate rest (collinear points): fan it.
            for i in 1..v.len() - 1 {
                out.push([v[0], v[i], v[i + 1]]);
            }
            return out;
        }
    }
    if v.len() == 3 {
        out.push([v[0], v[1], v[2]]);
    }
    out
}

pub fn triangles_area(tris: &[[P; 3]]) -> f64 {
    tris.iter()
        .map(|[a, b, c]| norm(cross(sub(*b, *a), sub(*c, *a))) / 2.0)
        .sum()
}

/// A model for the flood test: its mesh (world coordinates) and its caps.
pub struct FluidModel<'a> {
    pub mesh: &'a Mesh,
    pub caps: Vec<Vec<[P; 3]>>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FluidCheck {
    /// There is a fluid region closed off from the outside.
    pub sealed: bool,
    /// A point deep inside the fluid region (for meshing).
    pub fluid_point: Option<P>,
    /// Volume of the fluid region (voxel estimate).
    pub fluid_volume: f64,
    /// Edge length of the voxels.
    pub cell: f64,
}

struct Grid {
    origin: P,
    h: f64,
    n: [usize; 3],
}

impl Grid {
    fn index(&self, i: usize, j: usize, k: usize) -> usize {
        (k * self.n[1] + j) * self.n[0] + i
    }
    fn cell_of(&self, p: P) -> [usize; 3] {
        [0, 1, 2].map(|a| {
            (((p[a] - self.origin[a]) / self.h).floor().max(0.0) as usize).min(self.n[a] - 1)
        })
    }
    fn centre(&self, c: [usize; 3]) -> P {
        [0, 1, 2].map(|a| self.origin[a] + (c[a] as f64 + 0.5) * self.h)
    }
}

/// Flood test: walls and caps block, the outside is flooded from the grid border; what stays dry
/// and is not inside the material of a closed body is fluid.
pub fn fluid_check(models: &[FluidModel<'_>]) -> FluidCheck {
    fluid_check_with(models, None)
}

/// Most voxels the flood test uses (memory: about three bytes each).
pub const MAX_VOXELS: f64 = 3.0e7;

/// Flood test with voxels of at most `max_cell` (for example an eighth of the smallest opening),
/// as long as the grid stays below [`MAX_VOXELS`].
pub fn fluid_check_with(models: &[FluidModel<'_>], max_cell: Option<f64>) -> FluidCheck {
    let mut lo = [f64::MAX; 3];
    let mut hi = [f64::MIN; 3];
    for m in models {
        for p in &m.mesh.positions {
            for k in 0..3 {
                lo[k] = lo[k].min(f64::from(p[k]));
                hi[k] = hi[k].max(f64::from(p[k]));
            }
        }
    }
    let empty = FluidCheck {
        sealed: false,
        fluid_point: None,
        fluid_volume: 0.0,
        cell: 0.0,
    };
    if lo[0] > hi[0] {
        return empty;
    }
    let extent = (0..3).map(|k| hi[k] - lo[k]).fold(0.0, f64::max).max(1e-9);
    let mut h = (extent / FLOOD_RESOLUTION).min(max_cell.unwrap_or(f64::MAX));
    let voxels = |h: f64| (0..3).map(|k| (hi[k] - lo[k]) / h + 5.0).product::<f64>();
    if voxels(h) > MAX_VOXELS {
        h *= (voxels(h) / MAX_VOXELS).cbrt();
    }
    let origin = lo.map(|x| x - 2.0 * h);
    let n = [0, 1, 2].map(|k| ((hi[k] - lo[k]) / h).ceil() as usize + 5);
    let grid = Grid { origin, h, n };
    let total = n[0] * n[1] * n[2];
    let mut wall = vec![false; total];
    let mut mark = |tri: [P; 3]| {
        let [a, b, c] = tri;
        let longest = norm(sub(b, a)).max(norm(sub(c, a))).max(norm(sub(c, b)));
        let steps = ((longest / (h / 3.0)).ceil() as usize).max(1);
        for i in 0..=steps {
            for j in 0..=steps - i {
                let (u, v) = (i as f64 / steps as f64, j as f64 / steps as f64);
                let p = [0, 1, 2].map(|k| a[k] + u * (b[k] - a[k]) + v * (c[k] - a[k]));
                let cell = grid.cell_of(p);
                wall[grid.index(cell[0], cell[1], cell[2])] = true;
            }
        }
    };
    for m in models {
        for t in &m.mesh.triangles {
            mark(t.map(|i| pos(m.mesh, i)));
        }
        for cap in &m.caps {
            for t in cap {
                mark(*t);
            }
        }
    }
    // Material of closed bodies with passages: column parity along x.
    let mut material = vec![false; total];
    for m in models {
        if !matches!(classify(m.mesh), GeometryKind::ThickWalled { .. }) {
            continue;
        }
        fill_inside(m.mesh, &grid, &mut material);
    }
    // Flood the outside.
    let mut outside = vec![false; total];
    let mut queue = VecDeque::from([[0usize, 0, 0]]);
    outside[0] = true;
    while let Some(c) = queue.pop_front() {
        for (axis, delta) in [(0, -1i64), (0, 1), (1, -1), (1, 1), (2, -1), (2, 1)] {
            let mut d = c;
            let v = c[axis] as i64 + delta;
            if v < 0 || v >= n[axis] as i64 {
                continue;
            }
            d[axis] = v as usize;
            let id = grid.index(d[0], d[1], d[2]);
            if !outside[id] && !wall[id] {
                outside[id] = true;
                queue.push_back(d);
            }
        }
    }
    let fluid: Vec<bool> = (0..total)
        .map(|i| !outside[i] && !wall[i] && !material[i])
        .collect();
    let count = fluid.iter().filter(|f| **f).count();
    if count == 0 {
        return FluidCheck { cell: h, ..empty };
    }
    // Deepest fluid cell: distance from every non-fluid cell.
    let mut dist = vec![u32::MAX; total];
    let mut queue = VecDeque::new();
    for k in 0..n[2] {
        for j in 0..n[1] {
            for i in 0..n[0] {
                let id = grid.index(i, j, k);
                if !fluid[id] {
                    dist[id] = 0;
                    queue.push_back([i, j, k]);
                }
            }
        }
    }
    let mut deepest = (0u32, [0usize; 3]);
    while let Some(c) = queue.pop_front() {
        let dc = dist[grid.index(c[0], c[1], c[2])];
        for (axis, delta) in [(0, -1i64), (0, 1), (1, -1), (1, 1), (2, -1), (2, 1)] {
            let mut d = c;
            let v = c[axis] as i64 + delta;
            if v < 0 || v >= n[axis] as i64 {
                continue;
            }
            d[axis] = v as usize;
            let id = grid.index(d[0], d[1], d[2]);
            if dist[id] == u32::MAX {
                dist[id] = dc + 1;
                if dc + 1 > deepest.0 {
                    deepest = (dc + 1, d);
                }
                queue.push_back(d);
            }
        }
    }
    FluidCheck {
        sealed: true,
        fluid_point: Some(grid.centre(deepest.1)),
        // Wall voxels next to fluid are about half fluid.
        fluid_volume: (count as f64 + 0.5 * wall_next_to_fluid(&grid, &wall, &fluid) as f64)
            * h
            * h
            * h,
        cell: h,
    }
}

/// Marks the voxels inside a closed mesh (ray parity along x per voxel column).
fn fill_inside(mesh: &Mesh, grid: &Grid, inside: &mut [bool]) {
    let [nx, ny, nz] = grid.n;
    // Triangles binned by the columns their y-z bounds cover.
    let mut bins: Vec<Vec<usize>> = vec![Vec::new(); ny * nz];
    for (t, tri) in mesh.triangles.iter().enumerate() {
        let ps = tri.map(|i| pos(mesh, i));
        let (y0, y1) = (
            ps.iter().map(|p| p[1]).fold(f64::MAX, f64::min),
            ps.iter().map(|p| p[1]).fold(f64::MIN, f64::max),
        );
        let (z0, z1) = (
            ps.iter().map(|p| p[2]).fold(f64::MAX, f64::min),
            ps.iter().map(|p| p[2]).fold(f64::MIN, f64::max),
        );
        let j0 = grid.cell_of([0.0, y0, 0.0].map(|x| x))[1];
        let j1 = grid.cell_of([0.0, y1, 0.0])[1];
        let k0 = grid.cell_of([0.0, 0.0, z0])[2];
        let k1 = grid.cell_of([0.0, 0.0, z1])[2];
        for k in k0..=k1 {
            for j in j0..=j1 {
                bins[k * ny + j].push(t);
            }
        }
    }
    for k in 0..nz {
        for j in 0..ny {
            // Slightly off the voxel centre so the ray does not run along edges.
            let y = grid.origin[1] + (j as f64 + 0.5 + 1.3e-4) * grid.h;
            let z = grid.origin[2] + (k as f64 + 0.5 + 2.7e-4) * grid.h;
            let mut xs: Vec<f64> = Vec::new();
            for &t in &bins[k * ny + j] {
                let [a, b, c] = mesh.triangles[t].map(|i| pos(mesh, i));
                // Barycentric test in the y-z projection.
                let det = (b[1] - a[1]) * (c[2] - a[2]) - (c[1] - a[1]) * (b[2] - a[2]);
                if det.abs() < 1e-300 {
                    continue;
                }
                let u = ((y - a[1]) * (c[2] - a[2]) - (c[1] - a[1]) * (z - a[2])) / det;
                let v = ((b[1] - a[1]) * (z - a[2]) - (y - a[1]) * (b[2] - a[2])) / det;
                if u >= 0.0 && v >= 0.0 && u + v <= 1.0 {
                    xs.push(a[0] + u * (b[0] - a[0]) + v * (c[0] - a[0]));
                }
            }
            xs.sort_by(f64::total_cmp);
            for pair in xs.chunks(2) {
                let [x0, x1] = [pair[0], *pair.get(1).unwrap_or(&pair[0])];
                for i in 0..nx {
                    let x = grid.origin[0] + (i as f64 + 0.5) * grid.h;
                    if x > x0 && x < x1 {
                        inside[grid.index(i, j, k)] = true;
                    }
                }
            }
        }
    }
}

fn wall_next_to_fluid(grid: &Grid, wall: &[bool], fluid: &[bool]) -> usize {
    let [nx, ny, nz] = grid.n;
    let mut count = 0;
    for k in 1..nz - 1 {
        for j in 1..ny - 1 {
            for i in 1..nx - 1 {
                let id = grid.index(i, j, k);
                if !wall[id] {
                    continue;
                }
                let near = [
                    grid.index(i - 1, j, k),
                    grid.index(i + 1, j, k),
                    grid.index(i, j - 1, k),
                    grid.index(i, j + 1, k),
                    grid.index(i, j, k - 1),
                    grid.index(i, j, k + 1),
                ];
                if near.iter().any(|n| fluid[*n]) {
                    count += 1;
                }
            }
        }
    }
    count
}
