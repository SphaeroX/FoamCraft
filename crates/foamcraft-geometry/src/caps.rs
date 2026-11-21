// SPDX-License-Identifier: GPL-3.0-or-later
//! Commands for internal flow models: openings, caps, boundary roles and the flood test.
//!
//! Caps are stored as the loops of the opening in the model's file coordinates, so they follow
//! the model when it is moved, turned or rescaled. Their keys (`model-1:cap-1`) never change.

use foamcraft_core::command::{Options, Registry, Session};
use foamcraft_core::error::{FoamError, Language, Message, message};
use foamcraft_core::project::{BoundaryRole, CapRecord, Project};
use glam::DVec3;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::commands::{load_entry, placement};
use crate::mesh::{Format, Mesh, MeshBuilder};
use crate::openings::{
    FluidModel, GeometryKind, Opening, cap_triangles, classify, opening_near, openings,
};
use crate::segment::segment;
use crate::surfaces::parse_key;
use crate::transform::{Placement, unit_scale};

type P = [f64; 3];

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ModelParams {
    /// Model id, for example `model-1`.
    pub model: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CapParams {
    pub model: String,
    /// Point near the opening in world coordinates (metres), for example where the user clicked.
    pub point: [f64; 3],
    /// Face that was clicked (needed for closed bodies with passages): its loop nearest to the
    /// point is closed.
    #[serde(default)]
    pub face: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct KeyParams {
    /// Cap key, for example `model-1:cap-1`.
    pub key: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RoleParams {
    /// Face keys (`model-1:face-3`) or cap keys (`model-1:cap-1`).
    pub keys: Vec<String>,
    pub role: BoundaryRole,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct Warning {
    pub code: String,
    pub en: Message,
    pub de: Message,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct OpeningInfo {
    /// Centre in world coordinates (metres).
    pub centroid_m: P,
    /// Unit normal in world coordinates.
    pub normal: P,
    pub area_m2: f64,
    pub planar: bool,
    /// Key of the cap that closes this opening, if any.
    pub cap: Option<String>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct CapInfo {
    pub key: String,
    pub model: String,
    pub centroid_m: P,
    pub normal: P,
    pub area_m2: f64,
    pub role: Option<BoundaryRole>,
    pub warnings: Vec<Warning>,
}

#[derive(Debug, Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    OpenShell,
    ThickWalled,
    ClosedVolume,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct OpeningsReport {
    pub model: String,
    pub kind: Kind,
    /// Passages through a thick-walled body.
    pub passages: u32,
    pub openings: Vec<OpeningInfo>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct FluidReport {
    pub sealed: bool,
    /// A point inside the fluid region (world, metres).
    pub fluid_point_m: Option<P>,
    pub fluid_volume_m3: f64,
    pub caps: usize,
    pub warnings: Vec<Warning>,
}

fn warning(code: &str) -> Warning {
    let get = |lang| {
        message(code, lang).cloned().unwrap_or(Message {
            text: code.into(),
            fix: String::new(),
        })
    };
    Warning {
        code: code.into(),
        en: get(Language::En),
        de: get(Language::De),
    }
}

fn scale_of(p: &Placement) -> f64 {
    unit_scale(&p.unit).unwrap_or(1.0) * p.scale
}

fn to_world(p: &Placement, x: P) -> P {
    let w = p.rotation() * (DVec3::from(x) * scale_of(p)) + DVec3::from(p.translation);
    w.into()
}

fn to_model(p: &Placement, x: P) -> P {
    let m = p.rotation().transpose() * (DVec3::from(x) - DVec3::from(p.translation)) / scale_of(p);
    m.into()
}

fn direction_to_world(p: &Placement, n: P) -> P {
    (p.rotation() * DVec3::from(n)).normalize_or_zero().into()
}

fn model_mesh(project: &Project, id: &str) -> Result<(Mesh, Placement, f64), FoamError> {
    let entry = project
        .manifest
        .geometry
        .iter()
        .find(|e| e.id == id)
        .ok_or_else(|| FoamError::new("geometry.not_found", id))?;
    Ok((
        load_entry(&project.root, entry)?,
        placement(entry),
        entry.crease_angle_deg,
    ))
}

fn opening_of(record: &CapRecord) -> Opening {
    // Recomputes plane, area and flatness from the stored loops.
    crate::openings::opening_from_loops(record.loops.clone())
}

fn same_opening(a: &Opening, b: &Opening) -> bool {
    let d = DVec3::from(a.centroid).distance(DVec3::from(b.centroid));
    d < 1e-6 * (1.0 + a.area.sqrt()) + 0.01 * a.area.sqrt()
}

fn cap_info(project: &Project, record: &CapRecord) -> CapInfo {
    let o = opening_of(record);
    let (pl, s) = project
        .manifest
        .geometry
        .iter()
        .find(|e| e.id == record.model)
        .map(|e| {
            let pl = placement(e);
            let s = scale_of(&pl);
            (pl, s)
        })
        .unwrap_or_default();
    CapInfo {
        key: record.key.clone(),
        model: record.model.clone(),
        centroid_m: to_world(&pl, o.centroid),
        normal: direction_to_world(&pl, o.normal),
        area_m2: o.area * s * s,
        role: project.manifest.boundary_roles.get(&record.key).copied(),
        warnings: if o.planar {
            vec![]
        } else {
            vec![warning("geometry.opening_not_flat")]
        },
    }
}

fn report(project: &Project, id: &str) -> Result<OpeningsReport, FoamError> {
    let (mesh, pl, _) = model_mesh(project, id)?;
    let s = scale_of(&pl);
    let caps: Vec<(String, Opening)> = project
        .manifest
        .caps
        .iter()
        .filter(|c| c.model == id)
        .map(|c| (c.key.clone(), opening_of(c)))
        .collect();
    let (kind, passages) = match classify(&mesh) {
        GeometryKind::OpenShell => (Kind::OpenShell, 0),
        GeometryKind::ThickWalled { passages } => (Kind::ThickWalled, passages),
        GeometryKind::ClosedVolume => (Kind::ClosedVolume, 0),
    };
    let openings = openings(&mesh)
        .into_iter()
        .map(|o| OpeningInfo {
            centroid_m: to_world(&pl, o.centroid),
            normal: direction_to_world(&pl, o.normal),
            area_m2: o.area * s * s,
            planar: o.planar,
            cap: caps
                .iter()
                .find(|(_, c)| same_opening(c, &o))
                .map(|(k, _)| k.clone()),
        })
        .collect();
    Ok(OpeningsReport {
        model: id.into(),
        kind,
        passages,
        openings,
    })
}

fn cap_opening(s: &mut Session, p: CapParams) -> Result<CapInfo, FoamError> {
    let project = s.project()?;
    let (mesh, pl, crease) = model_mesh(project, &p.model)?;
    let point = to_model(&pl, p.point);
    let seg;
    let face = match &p.face {
        Some(key) => {
            let (model, face) =
                parse_key(key).ok_or_else(|| FoamError::new("surface.not_found", key.clone()))?;
            if model != p.model {
                return Err(FoamError::new("surface.not_found", key.clone()));
            }
            seg = segment(&mesh, crease);
            Some((&seg, face))
        }
        None => None,
    };
    let opening = opening_near(&mesh, point, face)
        .ok_or_else(|| FoamError::new("geometry.no_opening", p.model.clone()))?;
    if let Some(existing) = project
        .manifest
        .caps
        .iter()
        .find(|c| c.model == p.model && same_opening(&opening_of(c), &opening))
    {
        return Err(FoamError::new(
            "geometry.already_capped",
            existing.key.clone(),
        ));
    }
    let next = (1..)
        .find(|n| {
            let key = format!("{}:cap-{n}", p.model);
            !project.manifest.caps.iter().any(|c| c.key == key)
        })
        .unwrap_or(1);
    let record = CapRecord {
        key: format!("{}:cap-{next}", p.model),
        model: p.model.clone(),
        loops: opening.loops.clone(),
    };
    project.manifest.caps.push(record.clone());
    Ok(cap_info(project, &record))
}

/// Cap triangles of a project's models in world coordinates, per model.
pub fn world_caps(project: &Project, model: &str) -> Vec<Vec<[P; 3]>> {
    let Some(entry) = project.manifest.geometry.iter().find(|e| e.id == model) else {
        return vec![];
    };
    let pl = placement(entry);
    project
        .manifest
        .caps
        .iter()
        .filter(|c| c.model == model)
        .map(|c| {
            cap_triangles(&opening_of(c))
                .into_iter()
                .map(|t| t.map(|x| to_world(&pl, x)))
                .collect()
        })
        .collect()
}

/// Cap surfaces of all models as one mesh per cap (world coordinates), with their keys.
pub fn cap_meshes(project: &Project) -> Vec<(String, Mesh)> {
    let mut out = Vec::new();
    for entry in &project.manifest.geometry {
        let keys: Vec<String> = project
            .manifest
            .caps
            .iter()
            .filter(|c| c.model == entry.id)
            .map(|c| c.key.clone())
            .collect();
        for (key, tris) in keys.into_iter().zip(world_caps(project, &entry.id)) {
            let mut b = MeshBuilder::default();
            for [a, bb, c] in tris {
                b.triangle(
                    a.map(|x| x as f32),
                    bb.map(|x| x as f32),
                    c.map(|x| x as f32),
                );
            }
            out.push((key, b.finish(Format::Stl, None)));
        }
    }
    out
}

/// Flood test of the project: models in the world with their caps.
pub fn fluid_report(project: &Project) -> Result<FluidReport, FoamError> {
    let mut worlds = Vec::new();
    for entry in &project.manifest.geometry {
        let mesh = load_entry(&project.root, entry)?;
        worlds.push((entry.id.clone(), placement(entry).apply(&mesh)));
    }
    let models: Vec<FluidModel<'_>> = worlds
        .iter()
        .map(|(id, mesh)| FluidModel {
            mesh,
            caps: world_caps(project, id),
        })
        .collect();
    // Voxels of an eighth of the smallest cap, so narrow passages are resolved.
    let smallest = models
        .iter()
        .flat_map(|m| m.caps.iter())
        .map(|cap| crate::openings::triangles_area(cap).sqrt())
        .fold(f64::MAX, f64::min);
    let c =
        crate::openings::fluid_check_with(&models, (smallest < f64::MAX).then_some(smallest / 8.0));
    let mut warnings = Vec::new();
    if !c.sealed {
        warnings.push(warning("geometry.not_sealed"));
    }
    let caps = models.iter().map(|m| m.caps.len()).sum();
    Ok(FluidReport {
        sealed: c.sealed,
        fluid_point_m: c.fluid_point,
        fluid_volume_m3: c.fluid_volume,
        caps,
        warnings,
    })
}

/// True for an existing face key or cap key of the project.
fn known_key(project: &Project, key: &str) -> bool {
    if project.manifest.caps.iter().any(|c| c.key == key) {
        return true;
    }
    let Some((model, face)) = parse_key(key) else {
        return false;
    };
    project
        .manifest
        .geometry
        .iter()
        .find(|e| e.id == model)
        .and_then(|e| {
            let mesh = load_entry(&project.root, e).ok()?;
            Some(face >= 1 && face as usize <= segment(&mesh, e.crease_angle_deg).faces.len())
        })
        .unwrap_or(false)
}

pub fn register(r: &mut Registry) {
    r.add(
        "geometry.openings",
        "Describe a model for internal flow: kind (open shell, thick-walled, closed volume) and its openings.",
        Options {
            undoable: false,
            errors: &["project.none_open", "geometry.not_found"],
            example: json!({"model": "model-1"}),
        },
        |s, p: ModelParams| report(s.project()?, &p.model),
    );
    r.add(
        "geometry.cap_opening",
        "Close the opening of a model nearest to a point (metres) with a flat cap; for closed bodies \
         also give the clicked face.",
        Options {
            undoable: true,
            errors: &[
                "project.none_open",
                "geometry.not_found",
                "surface.not_found",
                "geometry.no_opening",
                "geometry.already_capped",
            ],
            example: json!({"model": "model-1", "point": [0.0, 0.0, 0.0], "face": "model-1:face-1"}),
        },
        cap_opening,
    );
    r.add(
        "cap.remove",
        "Remove a cap (and its boundary role).",
        Options {
            undoable: true,
            errors: &["project.none_open", "geometry.cap_not_found"],
            example: json!({"key": "model-1:cap-1"}),
        },
        |s, p: KeyParams| {
            let project = s.project()?;
            let before = project.manifest.caps.len();
            project.manifest.caps.retain(|c| c.key != p.key);
            if project.manifest.caps.len() == before {
                return Err(FoamError::new("geometry.cap_not_found", p.key));
            }
            project.manifest.boundary_roles.remove(&p.key);
            Ok(json!({"removed": p.key}))
        },
    );
    r.add(
        "cap.list",
        "List the caps of the project with position, area, role and warnings.",
        Options {
            undoable: false,
            errors: &["project.none_open"],
            example: json!({}),
        },
        |s, _: crate::surfaces::NoParams| {
            let project = s.project()?;
            Ok(project
                .manifest
                .caps
                .iter()
                .map(|c| cap_info(project, c))
                .collect::<Vec<_>>())
        },
    );
    r.add(
        "geometry.fluid_check",
        "Flood test: is the fluid region closed off, and where is a point inside it?",
        Options {
            undoable: false,
            errors: &["project.none_open", "geometry.read_failed"],
            example: json!({}),
        },
        |s, _: crate::surfaces::NoParams| fluid_report(s.project()?),
    );
    r.add(
        "scenario.boundary_roles",
        "List the faces and caps marked as inlet, outlet or wall.",
        Options {
            undoable: false,
            errors: &["project.none_open"],
            example: json!({}),
        },
        |s, _: crate::surfaces::NoParams| Ok(s.project()?.manifest.boundary_roles.clone()),
    );
    r.add(
        "scenario.set_boundary_role",
        "Mark faces or caps as inlet, outlet or wall.",
        Options {
            undoable: true,
            errors: &["project.none_open", "surface.not_found"],
            example: json!({"keys": ["model-1:cap-1"], "role": "inlet"}),
        },
        |s, p: RoleParams| {
            let project = s.project()?;
            if let Some(bad) = p.keys.iter().find(|k| !known_key(project, k)) {
                return Err(FoamError::new("surface.not_found", bad.clone()));
            }
            for k in &p.keys {
                project.manifest.boundary_roles.insert(k.clone(), p.role);
            }
            Ok(project.manifest.boundary_roles.clone())
        },
    );
}

/// Loops of the openings that have no cap yet (world coordinates), for highlighting.
pub fn open_loops(project: &Project) -> Vec<Vec<P>> {
    let mut out = Vec::new();
    for entry in &project.manifest.geometry {
        let Ok(mesh) = load_entry(&project.root, entry) else {
            continue;
        };
        let pl = placement(entry);
        let caps: Vec<Opening> = project
            .manifest
            .caps
            .iter()
            .filter(|c| c.model == entry.id)
            .map(opening_of)
            .collect();
        for o in openings(&mesh) {
            if caps.iter().any(|c| same_opening(c, &o)) {
                continue;
            }
            out.extend(
                o.loops
                    .iter()
                    .map(|l| l.iter().map(|x| to_world(&pl, *x)).collect::<Vec<_>>()),
            );
        }
    }
    out
}

/// A cap in world coordinates for display: key, centre, unit normal pointing into the model's
/// bounding box centre, size and role.
pub struct CapView {
    pub key: String,
    pub centroid: P,
    pub inward: P,
    pub size: f64,
    pub role: Option<BoundaryRole>,
}

pub fn cap_views(project: &Project) -> Vec<CapView> {
    project
        .manifest
        .caps
        .iter()
        .filter_map(|c| {
            let entry = project.manifest.geometry.iter().find(|e| e.id == c.model)?;
            let mesh = load_entry(&project.root, entry).ok()?;
            let pl = placement(entry);
            let (lo, hi) = mesh.bounds();
            let centre = to_world(&pl, [0, 1, 2].map(|k| f64::from(lo[k] + hi[k]) / 2.0));
            let info = cap_info(project, c);
            let n = DVec3::from(info.normal);
            let towards = DVec3::from(centre) - DVec3::from(info.centroid_m);
            let inward = if n.dot(towards) >= 0.0 { n } else { -n };
            Some(CapView {
                key: c.key.clone(),
                centroid: info.centroid_m,
                inward: inward.into(),
                size: info.area_m2.sqrt(),
                role: info.role,
            })
        })
        .collect()
}

/// Size and position of a boundary (cap or face) in the world.
#[derive(Debug, Clone, PartialEq, Serialize, JsonSchema)]
pub struct BoundarySize {
    pub area_m2: f64,
    /// Length of the edge around it (all loops), m.
    pub perimeter_m: f64,
    pub centroid_m: P,
    pub normal: P,
}

fn loops_length(loops: &[Vec<P>]) -> f64 {
    loops
        .iter()
        .map(|l| {
            (0..l.len())
                .map(|i| DVec3::from(l[i]).distance(DVec3::from(l[(i + 1) % l.len()])))
                .sum::<f64>()
        })
        .sum()
}

/// Area, perimeter, centre and normal of a cap key or a face key.
pub fn boundary_size(project: &Project, key: &str) -> Option<BoundarySize> {
    if let Some(cap) = project.manifest.caps.iter().find(|c| c.key == key) {
        let entry = project
            .manifest
            .geometry
            .iter()
            .find(|e| e.id == cap.model)?;
        let pl = placement(entry);
        let s = scale_of(&pl);
        let o = opening_of(cap);
        return Some(BoundarySize {
            area_m2: o.area * s * s,
            perimeter_m: loops_length(&o.loops) * s,
            centroid_m: to_world(&pl, o.centroid),
            normal: direction_to_world(&pl, o.normal),
        });
    }
    let (model, face) = parse_key(key)?;
    let entry = project.manifest.geometry.iter().find(|e| e.id == model)?;
    let mesh = load_entry(&project.root, entry).ok()?;
    let seg = segment(&mesh, entry.crease_angle_deg);
    let info = seg.faces.iter().find(|f| f.id == face)?;
    let pl = placement(entry);
    let s = scale_of(&pl);
    let loops: Vec<Vec<P>> = crate::openings::face_loops(&mesh, &seg, face)
        .iter()
        .map(|l| {
            l.iter()
                .map(|i| mesh.positions[*i as usize].map(f64::from))
                .collect()
        })
        .collect();
    Some(BoundarySize {
        area_m2: info.area * s * s,
        perimeter_m: loops_length(&loops) * s,
        centroid_m: to_world(&pl, info.centroid),
        normal: direction_to_world(&pl, info.normal),
    })
}
