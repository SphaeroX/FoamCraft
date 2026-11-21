// SPDX-License-Identifier: GPL-3.0-or-later
//! surface.* and group.* commands. A face key is `<model id>:face-<n>`, a part key
//! `<model id>:part-<n>`; numbers come from the deterministic segmentation, so keys survive saving
//! and opening as long as the file and the crease angle stay the same.

use std::collections::{BTreeSet, HashMap};

use foamcraft_core::command::{Options, Registry, Session};
use foamcraft_core::error::FoamError;
use foamcraft_core::project::{GeometryEntry, Project, SurfaceGroup};
use glam::DVec3;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::commands::{load_entry, placement};
use crate::diagnose::edge_key;
use crate::{Mesh, Segmentation, nearest_triangle, segment, unit_scale};

/// One model with its faces, ready for queries.
pub struct SegmentedModel {
    pub id: String,
    pub mesh: Mesh,
    pub segmentation: Segmentation,
    pub entry: GeometryEntry,
}

pub fn face_key(model: &str, face: u32) -> String {
    format!("{model}:face-{face}")
}

pub(crate) fn parse_key(key: &str) -> Option<(&str, u32)> {
    let (model, face) = key.rsplit_once(":face-")?;
    Some((model, face.parse().ok()?))
}

/// Segments every model of the project (file units).
pub fn segmented_models(project: &Project) -> Result<Vec<SegmentedModel>, FoamError> {
    project
        .manifest
        .geometry
        .iter()
        .map(|e| {
            let mesh = load_entry(&project.root, e)?;
            let segmentation = segment(&mesh, e.crease_angle_deg);
            Ok(SegmentedModel {
                id: e.id.clone(),
                mesh,
                segmentation,
                entry: e.clone(),
            })
        })
        .collect()
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct SurfaceInfo {
    pub key: String,
    pub model: String,
    /// File name of the model.
    pub model_name: String,
    pub part: String,
    pub name: Option<String>,
    pub groups: Vec<String>,
    pub triangles: usize,
    pub area_m2: f64,
    /// Mean outward normal in world coordinates.
    pub normal: [f64; 3],
    pub centroid_m: [f64; 3],
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ListParams {
    /// Only faces of this model.
    #[serde(default)]
    pub model: Option<String>,
}

#[derive(Debug, Default, Clone, Copy, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum SelectMode {
    #[default]
    Replace,
    Add,
    Toggle,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SelectParams {
    pub keys: Vec<String>,
    #[serde(default)]
    pub mode: SelectMode,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SelectAtParams {
    /// Point in world coordinates (metres); the nearest face is selected.
    pub point: [f64; 3],
    #[serde(default)]
    pub mode: SelectMode,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct Picked {
    pub key: String,
    pub selection: Vec<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RenameParams {
    pub key: String,
    /// New name; empty removes the name.
    pub name: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct GroupParams {
    pub name: String,
    /// Face keys; default for group.create: the current selection.
    #[serde(default)]
    pub keys: Option<Vec<String>>,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct GroupNameParams {
    pub name: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CreaseParams {
    pub id: String,
    /// Edges sharper than this angle (degrees, 1 to 89) separate faces.
    pub degrees: f64,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct NoParams {}

fn infos(project: &Project, only: Option<&str>) -> Result<Vec<SurfaceInfo>, FoamError> {
    let mut out = Vec::new();
    for m in segmented_models(project)? {
        if only.is_some_and(|o| o != m.id) {
            continue;
        }
        let p = placement(&m.entry);
        let s = unit_scale(&p.unit).unwrap_or(1.0) * p.scale;
        let rot = p.rotation();
        let t = DVec3::from(p.translation);
        for f in &m.segmentation.faces {
            let key = face_key(&m.id, f.id);
            let groups = project
                .manifest
                .groups
                .iter()
                .filter(|g| g.keys.contains(&key))
                .map(|g| g.name.clone())
                .collect();
            out.push(SurfaceInfo {
                name: project.manifest.surface_names.get(&key).cloned(),
                model: m.id.clone(),
                model_name: m.entry.name.clone(),
                part: format!("{}:part-{}", m.id, f.part),
                groups,
                triangles: f.triangles,
                area_m2: f.area * s * s,
                normal: (rot * DVec3::from(f.normal)).into(),
                centroid_m: (rot * (DVec3::from(f.centroid) * s) + t).into(),
                key,
            });
        }
    }
    Ok(out)
}

fn check_keys(project: &Project, keys: &[String]) -> Result<(), FoamError> {
    let models = segmented_models(project)?;
    for key in keys {
        // Caps can be selected like faces.
        if project.manifest.caps.iter().any(|c| &c.key == key) {
            continue;
        }
        let ok = parse_key(key).is_some_and(|(model, face)| {
            models
                .iter()
                .any(|m| m.id == model && face >= 1 && face as usize <= m.segmentation.faces.len())
        });
        if !ok {
            return Err(FoamError::new("surface.not_found", key.clone()));
        }
    }
    Ok(())
}

fn apply_mode(current: &[String], keys: Vec<String>, mode: SelectMode) -> Vec<String> {
    let mut set: BTreeSet<String> = match mode {
        SelectMode::Replace => BTreeSet::new(),
        _ => current.iter().cloned().collect(),
    };
    for key in keys {
        if matches!(mode, SelectMode::Toggle) && set.contains(&key) {
            set.remove(&key);
        } else {
            set.insert(key);
        }
    }
    set.into_iter().collect()
}

/// Faces sharing an edge with any of `faces` (same model).
fn adjacent(model: &SegmentedModel, faces: &BTreeSet<u32>) -> BTreeSet<u32> {
    let mut by_edge: HashMap<u64, Vec<u32>> = HashMap::new();
    for (i, t) in model.mesh.triangles.iter().enumerate() {
        for (u, v) in [(t[0], t[1]), (t[1], t[2]), (t[2], t[0])] {
            by_edge
                .entry(edge_key(u, v))
                .or_default()
                .push(model.segmentation.face_of_triangle[i]);
        }
    }
    let mut out = BTreeSet::new();
    for owners in by_edge.values() {
        if owners.iter().any(|f| faces.contains(f)) {
            out.extend(owners.iter().copied());
        }
    }
    out
}

fn select_at(s: &mut Session, p: SelectAtParams) -> Result<Picked, FoamError> {
    let project = s.project()?;
    let mut best: Option<(String, f64)> = None;
    for m in segmented_models(project)? {
        let world = placement(&m.entry).apply(&m.mesh);
        if let Some(t) = nearest_triangle(&world, p.point) {
            let [a, b, c] =
                world.triangles[t].map(|i| DVec3::from(world.positions[i as usize].map(f64::from)));
            let d = ((a + b + c) / 3.0).distance(DVec3::from(p.point));
            if best.as_ref().is_none_or(|(_, bd)| d < *bd) {
                best = Some((face_key(&m.id, m.segmentation.face_of_triangle[t]), d));
            }
        }
    }
    let (key, _) = best.ok_or_else(|| FoamError::new("surface.not_found", "no model"))?;
    let selection = apply_mode(s.selection(), vec![key.clone()], p.mode);
    s.set_selection(selection.clone());
    Ok(Picked { key, selection })
}

fn group_mut<'a>(project: &'a mut Project, name: &str) -> Result<&'a mut SurfaceGroup, FoamError> {
    project
        .manifest
        .groups
        .iter_mut()
        .find(|g| g.name == name)
        .ok_or_else(|| FoamError::new("group.not_found", name))
}

pub fn register(r: &mut Registry) {
    let none: &'static [&'static str] = &["project.none_open"];
    let keys_err: &'static [&'static str] = &["project.none_open", "surface.not_found"];
    r.add(
        "surface.list",
        "List the faces of all models (or one model) with key, part, name, groups and size.",
        Options {
            undoable: false,
            errors: none,
            example: json!({}),
        },
        |s, p: ListParams| infos(s.project()?, p.model.as_deref()),
    );
    r.add(
        "surface.select",
        "Select faces by key (replace, add or toggle).",
        Options {
            undoable: false,
            errors: keys_err,
            example: json!({"keys": ["model-1:face-1"]}),
        },
        |s, p: SelectParams| {
            check_keys(s.project()?, &p.keys)?;
            let selection = apply_mode(s.selection(), p.keys, p.mode);
            s.set_selection(selection.clone());
            Ok(selection)
        },
    );
    r.add(
        "surface.select_at",
        "Select the face nearest to a point in space (metres); works without a window.",
        Options {
            undoable: false,
            errors: keys_err,
            example: json!({"point": [0.0, 0.0, 0.0]}),
        },
        select_at,
    );
    r.add(
        "surface.select_adjacent",
        "Add the faces that share an edge with the selection.",
        Options {
            undoable: false,
            errors: none,
            example: json!({}),
        },
        |s, _: NoParams| {
            let models = segmented_models(s.project()?)?;
            let mut keys: Vec<String> = s.selection().to_vec();
            for m in &models {
                let faces: BTreeSet<u32> = s
                    .selection()
                    .iter()
                    .filter_map(|k| parse_key(k))
                    .filter(|(model, _)| *model == m.id)
                    .map(|(_, f)| f)
                    .collect();
                if !faces.is_empty() {
                    keys.extend(adjacent(m, &faces).into_iter().map(|f| face_key(&m.id, f)));
                }
            }
            let selection = apply_mode(&[], keys, SelectMode::Add);
            s.set_selection(selection.clone());
            Ok(selection)
        },
    );
    r.add(
        "surface.selection",
        "Report the selected faces.",
        Options {
            undoable: false,
            errors: &[],
            example: json!({}),
        },
        |s, _: NoParams| Ok(s.selection().to_vec()),
    );
    r.add(
        "surface.clear_selection",
        "Clear the selection.",
        Options {
            undoable: false,
            errors: &[],
            example: json!({}),
        },
        |s, _: NoParams| {
            s.set_selection(Vec::new());
            Ok(())
        },
    );
    r.add(
        "surface.rename",
        "Give a face a name (an empty name removes it).",
        Options {
            undoable: true,
            errors: keys_err,
            example: json!({"key": "model-1:face-3", "name": "Inlet face"}),
        },
        |s, p: RenameParams| {
            let project = s.project()?;
            check_keys(project, std::slice::from_ref(&p.key))?;
            if p.name.trim().is_empty() {
                project.manifest.surface_names.remove(&p.key);
            } else {
                project
                    .manifest
                    .surface_names
                    .insert(p.key, p.name.trim().to_string());
            }
            Ok(())
        },
    );
    r.add(
        "group.create",
        "Create a named group of faces (default: the selection).",
        Options {
            undoable: true,
            errors: &[
                "project.none_open",
                "surface.not_found",
                "group.exists",
                "group.empty",
            ],
            example: json!({"name": "Outlet", "keys": ["model-1:face-5"]}),
        },
        |s, p: GroupParams| {
            let keys = p.keys.unwrap_or_else(|| s.selection().to_vec());
            if keys.is_empty() {
                return Err(FoamError::new("group.empty", p.name));
            }
            let project = s.project()?;
            check_keys(project, &keys)?;
            if project.manifest.groups.iter().any(|g| g.name == p.name) {
                return Err(FoamError::new("group.exists", p.name));
            }
            let group = SurfaceGroup {
                name: p.name,
                keys: apply_mode(&[], keys, SelectMode::Add),
            };
            project.manifest.groups.push(group.clone());
            Ok(group)
        },
    );
    r.add(
        "group.add",
        "Add faces to a group (default: the selection).",
        Options {
            undoable: true,
            errors: &["project.none_open", "surface.not_found", "group.not_found"],
            example: json!({"name": "Inlet", "keys": ["model-1:face-3"]}),
        },
        |s, p: GroupParams| {
            let keys = p.keys.unwrap_or_else(|| s.selection().to_vec());
            let project = s.project()?;
            check_keys(project, &keys)?;
            let group = group_mut(project, &p.name)?;
            group.keys = apply_mode(&group.keys, keys, SelectMode::Add);
            Ok(group.clone())
        },
    );
    r.add(
        "group.remove",
        "Remove faces from a group (default: the selection).",
        Options {
            undoable: true,
            errors: &["project.none_open", "group.not_found"],
            example: json!({"name": "Inlet", "keys": ["model-1:face-1"]}),
        },
        |s, p: GroupParams| {
            let keys = p.keys.unwrap_or_else(|| s.selection().to_vec());
            let group = group_mut(s.project()?, &p.name)?;
            group.keys.retain(|k| !keys.contains(k));
            Ok(group.clone())
        },
    );
    r.add(
        "group.delete",
        "Delete a group (its faces stay).",
        Options {
            undoable: true,
            errors: &["project.none_open", "group.not_found"],
            example: json!({"name": "Inlet"}),
        },
        |s, p: GroupNameParams| {
            let project = s.project()?;
            group_mut(project, &p.name)?;
            project.manifest.groups.retain(|g| g.name != p.name);
            Ok(())
        },
    );
    r.add(
        "group.list",
        "List the face groups.",
        Options {
            undoable: false,
            errors: none,
            example: json!({}),
        },
        |s, _: NoParams| Ok(s.project()?.manifest.groups.clone()),
    );
    r.add(
        "geometry.set_crease_angle",
        "Set the angle at which edges split a model into faces (changes face numbers).",
        Options {
            undoable: true,
            errors: &[
                "project.none_open",
                "geometry.not_found",
                "geometry.invalid_angle",
            ],
            example: json!({"id": "model-1", "degrees": 60.0}),
        },
        |s, p: CreaseParams| {
            if !(1.0..=89.0).contains(&p.degrees) {
                return Err(FoamError::new(
                    "geometry.invalid_angle",
                    p.degrees.to_string(),
                ));
            }
            let entry = s
                .project()?
                .manifest
                .geometry
                .iter_mut()
                .find(|e| e.id == p.id)
                .ok_or_else(|| FoamError::new("geometry.not_found", p.id.clone()))?;
            entry.crease_angle_deg = p.degrees;
            Ok(entry.clone())
        },
    );
}
