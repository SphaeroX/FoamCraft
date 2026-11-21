// SPDX-License-Identifier: GPL-3.0-or-later
//! geometry.* commands: import models into the project and place them.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use foamcraft_core::command::{Options, Registry, Session};
use foamcraft_core::error::{FoamError, Language, Message, message};
use foamcraft_core::project::{GeometryEntry, Project};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::{
    Diagnosis, Format, Mesh, Pivots, Placement, Severity, diagnose, fix_normals, format_of,
    ground_offset, pivots, read_mesh, suggest_unit, unit_scale,
};

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ImportParams {
    /// Model file (STL, OBJ or 3MF).
    pub path: String,
    /// Unit of the file coordinates; default: unit written in the file, else a suggestion from
    /// the model size.
    #[serde(default)]
    pub unit: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct IdParams {
    pub id: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct UnitParams {
    pub id: String,
    /// um, mm, cm, m, in or ft.
    pub unit: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TransformParams {
    pub id: String,
    /// Yaw, pitch, roll in degrees.
    #[serde(default)]
    pub rotation_deg: Option<[f64; 3]>,
    /// Translation in metres.
    #[serde(default)]
    pub translation: Option<[f64; 3]>,
    /// Extra uniform scale factor.
    #[serde(default)]
    pub scale: Option<f64>,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct MoveByParams {
    pub id: String,
    /// Shift in metres along world X, Y and Z.
    pub delta: [f64; 3],
}

/// A world axis.
#[derive(Debug, Clone, Copy, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum Axis {
    X,
    Y,
    Z,
}

impl Axis {
    fn vector(self) -> glam::DVec3 {
        match self {
            Axis::X => glam::DVec3::X,
            Axis::Y => glam::DVec3::Y,
            Axis::Z => glam::DVec3::Z,
        }
    }
}

/// A named pivot of the model.
#[derive(Debug, Clone, Copy, Default, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum PivotKind {
    /// Centre of the bounding box.
    #[default]
    Center,
    /// Centre of the enclosed volume.
    Centroid,
    /// Origin of the model file.
    Origin,
}

/// A named pivot or a point in metres.
#[derive(Debug, Clone, Copy, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum PivotSpec {
    Named(PivotKind),
    Point([f64; 3]),
}

impl Default for PivotSpec {
    fn default() -> Self {
        PivotSpec::Named(PivotKind::Center)
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RotateAboutParams {
    pub id: String,
    /// World axis to turn about: x, y or z.
    pub axis: Axis,
    /// Angle in degrees, counter-clockwise looking against the axis.
    pub angle_deg: f64,
    /// center (default), centroid, origin or a point [x, y, z] in metres.
    #[serde(default)]
    pub pivot: PivotSpec,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct LocalizedFinding {
    pub code: String,
    pub severity: Severity,
    pub count: u64,
    pub en: Message,
    pub de: Message,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct Bounds {
    pub min: [f64; 3],
    pub max: [f64; 3],
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct GeometryInfo {
    #[serde(flatten)]
    pub entry: GeometryEntry,
    /// Format of the imported file (stl, obj, 3mf, step, iges).
    pub format: String,
    pub suggested_unit: String,
    pub diagnosis: Diagnosis,
    pub messages: Vec<LocalizedFinding>,
    /// Extent in the world, metres.
    pub size_m: [f64; 3],
    pub bounds_m: Bounds,
}

/// A model in `geometry.list`: its entry and the keys of its caps.
#[derive(Debug, Serialize, JsonSchema)]
pub struct ListedModel {
    #[serde(flatten)]
    pub entry: GeometryEntry,
    pub caps: Vec<String>,
}

pub fn placement(entry: &GeometryEntry) -> Placement {
    Placement {
        unit: entry.unit.clone(),
        scale: entry.scale,
        rotation_deg: entry.rotation_deg,
        translation: entry.translation,
    }
}

/// The model as stored (file units), with the normal fix applied if requested.
pub fn load_entry(project: &Path, entry: &GeometryEntry) -> Result<Mesh, FoamError> {
    let mut mesh = read_mesh(&project.join(&entry.file))?;
    if entry.fix_normals {
        fix_normals(&mut mesh);
    }
    Ok(mesh)
}

/// All models of a project in world coordinates (metres), for display.
pub fn world_meshes(project: &Project) -> Vec<(String, Mesh)> {
    project
        .manifest
        .geometry
        .iter()
        .filter_map(|e| {
            let mesh = load_entry(&project.root, e).ok()?;
            Some((e.id.clone(), placement(e).apply(&mesh)))
        })
        .collect()
}

fn localize(d: &Diagnosis) -> Vec<LocalizedFinding> {
    d.findings
        .iter()
        .map(|f| {
            let text = |lang| {
                message(&f.code, lang).cloned().unwrap_or(Message {
                    text: f.code.clone(),
                    fix: String::new(),
                })
            };
            LocalizedFinding {
                code: f.code.clone(),
                severity: f.severity,
                count: f.count,
                en: text(Language::En),
                de: text(Language::De),
            }
        })
        .collect()
}

fn info(project: &Path, entry: &GeometryEntry) -> Result<GeometryInfo, FoamError> {
    let mesh = load_entry(project, entry)?;
    let diagnosis = diagnose(&mesh);
    let largest = diagnosis.size.iter().copied().fold(0.0, f64::max);
    let world = placement(entry).apply(&mesh);
    let (min, max) = world.bounds();
    let min = min.map(f64::from);
    let max = max.map(f64::from);
    let original = entry.source.as_deref().unwrap_or(&entry.file);
    let format = Path::new(original)
        .extension()
        .map_or(String::new(), |e| e.to_string_lossy().to_ascii_lowercase());
    Ok(GeometryInfo {
        format,
        entry: entry.clone(),
        suggested_unit: suggest_unit(largest).into(),
        messages: localize(&diagnosis),
        diagnosis,
        size_m: [0, 1, 2].map(|k| max[k] - min[k]),
        bounds_m: Bounds { min, max },
    })
}

fn find<'a>(project: &'a mut Project, id: &str) -> Result<&'a mut GeometryEntry, FoamError> {
    project
        .manifest
        .geometry
        .iter_mut()
        .find(|e| e.id == id)
        .ok_or_else(|| FoamError::new("geometry.not_found", id))
}

fn check_unit(unit: &str) -> Result<(), FoamError> {
    unit_scale(unit)
        .map(|_| ())
        .ok_or_else(|| FoamError::new("geometry.invalid_unit", unit))
}

/// Converts CAD files (STEP, IGES) into a Gmsh mesh that keeps the CAD faces.
pub trait CadConverter: Send + Sync {
    /// Writes a Gmsh 4.1 ASCII mesh of `input` to `output` and returns the unit of its
    /// coordinates (for example `m` or `mm`).
    fn convert(&self, input: &Path, output: &Path) -> Result<String, FoamError>;
}

pub type SharedCadConverter = Arc<dyn CadConverter>;

fn write_failed(e: std::io::Error) -> FoamError {
    FoamError::new("project.write_failed", e.to_string())
}

fn import(
    s: &mut Session,
    p: ImportParams,
    cad: Option<&SharedCadConverter>,
) -> Result<GeometryInfo, FoamError> {
    let source = PathBuf::from(&p.path);
    let format = format_of(&source)
        .ok_or_else(|| FoamError::new("geometry.unsupported_format", p.path.clone()))?;
    let is_cad = matches!(format, Format::Step | Format::Iges);
    if is_cad && cad.is_none() {
        return Err(FoamError::new("geometry.cad_unavailable", p.path.clone()));
    }
    if !source.is_file() {
        return Err(FoamError::new("geometry.read_failed", p.path.clone()));
    }
    let project = s.project()?;
    let next = project
        .manifest
        .geometry
        .iter()
        .filter_map(|e| e.id.strip_prefix("model-")?.parse::<u32>().ok())
        .max()
        .unwrap_or(0)
        + 1;
    let id = format!("model-{next}");
    let ext = match format {
        Format::Stl => "stl",
        Format::Obj => "obj",
        Format::ThreeMf => "3mf",
        Format::Step => "step",
        Format::Iges => "iges",
        Format::Msh => "msh",
    };
    let dir = project.root.join("geometry");
    std::fs::create_dir_all(&dir).map_err(write_failed)?;
    let copy = format!("geometry/{id}.{ext}");
    std::fs::copy(&source, project.root.join(&copy)).map_err(write_failed)?;
    let (file, source_file, cad_unit) = match cad {
        Some(converter) if is_cad => {
            let mesh_file = format!("geometry/{id}.msh");
            let unit =
                converter.convert(&project.root.join(&copy), &project.root.join(&mesh_file))?;
            (mesh_file, Some(copy), Some(unit))
        }
        _ => (copy, None, None),
    };
    let mesh = read_mesh(&project.root.join(&file))?;
    let largest = diagnose(&mesh).size.iter().copied().fold(0.0, f64::max);
    let unit = p
        .unit
        .or(cad_unit)
        .or(mesh.declared_unit.clone())
        .unwrap_or_else(|| suggest_unit(largest).to_string());
    check_unit(&unit)?;
    let entry = GeometryEntry {
        id,
        name: source
            .file_name()
            .map_or(String::new(), |n| n.to_string_lossy().into_owned()),
        file,
        source: source_file,
        unit,
        scale: 1.0,
        rotation_deg: [0.0; 3],
        translation: [0.0; 3],
        fix_normals: false,
        crease_angle_deg: 30.0,
    };
    project.manifest.geometry.push(entry.clone());
    info(&project.root, &entry)
}

/// Applies `change` to the entry and returns its new description.
fn update(
    s: &mut Session,
    id: &str,
    change: impl FnOnce(&Path, &mut GeometryEntry) -> Result<(), FoamError>,
) -> Result<GeometryInfo, FoamError> {
    let project = s.project()?;
    let root = project.root.clone();
    let entry = find(project, id)?;
    change(&root, entry)?;
    let entry = entry.clone();
    info(&root, &entry)
}

/// The pivots of a model in world coordinates.
fn entry_pivots(root: &Path, e: &GeometryEntry) -> Result<Pivots, FoamError> {
    let world = placement(e).apply(&load_entry(root, e)?);
    Ok(pivots(&world, e.translation))
}

/// Geometry and surface commands without CAD import (STEP and IGES report
/// `geometry.cad_unavailable`).
pub fn register(r: &mut Registry) {
    register_with_cad(r, None);
}

/// Geometry and surface commands; `cad` converts STEP and IGES files.
pub fn register_with_cad(r: &mut Registry, cad: Option<SharedCadConverter>) {
    crate::surfaces::register(r);
    crate::caps::register(r);
    register_geometry(r, cad);
}

fn register_geometry(r: &mut Registry, cad: Option<SharedCadConverter>) {
    let errors: &'static [&'static str] = &["project.none_open", "geometry.not_found"];
    r.add(
        "geometry.import",
        "Import an STL, OBJ or 3MF model into the project and diagnose it.",
        Options {
            undoable: true,
            errors: &[
                "project.none_open",
                "geometry.read_failed",
                "geometry.unsupported_format",
                "geometry.empty",
                "geometry.invalid_unit",
                "geometry.cad_unavailable",
            ],
            example: json!({"path": "C:/Models/part.stl"}),
        },
        move |s, p: ImportParams| import(s, p, cad.as_ref()),
    );
    r.add(
        "geometry.list",
        "List the models of the project.",
        Options {
            undoable: false,
            errors: &["project.none_open"],
            example: json!({}),
        },
        |s, _: crate::commands::Empty| {
            let m = &s.project()?.manifest;
            Ok(m.geometry
                .iter()
                .map(|e| ListedModel {
                    entry: e.clone(),
                    caps: m
                        .caps
                        .iter()
                        .filter(|c| c.model == e.id)
                        .map(|c| c.key.clone())
                        .collect(),
                })
                .collect::<Vec<_>>())
        },
    );
    r.add(
        "geometry.info",
        "Describe a model: diagnosis, size and placement.",
        Options {
            undoable: false,
            errors,
            example: json!({"id": "model-1"}),
        },
        |s, p: IdParams| update(s, &p.id, |_, _| Ok(())),
    );
    r.add(
        "geometry.set_unit",
        "Set the unit of a model's file coordinates.",
        Options {
            undoable: true,
            errors: &[
                "project.none_open",
                "geometry.not_found",
                "geometry.invalid_unit",
            ],
            example: json!({"id": "model-1", "unit": "cm"}),
        },
        |s, p: UnitParams| {
            check_unit(&p.unit)?;
            update(s, &p.id, |_, e| {
                e.unit = p.unit;
                Ok(())
            })
        },
    );
    r.add(
        "geometry.set_transform",
        "Rotate, move or scale a model.",
        Options {
            undoable: true,
            errors,
            example: json!({"id": "model-1", "rotation_deg": [0, 0, 90], "translation": [0, 0, 0.1]}),
        },
        |s, p: TransformParams| {
            update(s, &p.id, |_, e| {
                if let Some(r) = p.rotation_deg {
                    e.rotation_deg = r;
                }
                if let Some(t) = p.translation {
                    e.translation = t;
                }
                if let Some(k) = p.scale {
                    e.scale = k;
                }
                Ok(())
            })
        },
    );
    r.add(
        "geometry.pivot",
        "Points a model can be moved and turned about: box centre, volume centre and origin (metres).",
        Options {
            undoable: false,
            errors,
            example: json!({"id": "model-1"}),
        },
        |s, p: IdParams| {
            let project = s.project()?;
            let root = project.root.clone();
            let entry = find(project, &p.id)?;
            entry_pivots(&root, entry)
        },
    );
    r.add(
        "geometry.move_by",
        "Move a model by a distance along the world axes (metres).",
        Options {
            undoable: true,
            errors,
            example: json!({"id": "model-1", "delta": [0.1, 0, 0]}),
        },
        |s, p: MoveByParams| {
            update(s, &p.id, |_, e| {
                for k in 0..3 {
                    e.translation[k] += p.delta[k];
                }
                Ok(())
            })
        },
    );
    r.add(
        "geometry.rotate_about",
        "Turn a model about a world axis through a pivot (box centre by default).",
        Options {
            undoable: true,
            errors,
            example: json!({"id": "model-1", "axis": "z", "angle_deg": 90, "pivot": "center"}),
        },
        |s, p: RotateAboutParams| {
            update(s, &p.id, |root, e| {
                let point = match p.pivot {
                    PivotSpec::Point(q) => q,
                    PivotSpec::Named(kind) => {
                        let all = entry_pivots(root, e)?;
                        match kind {
                            PivotKind::Center => all.center,
                            PivotKind::Centroid => all.centroid,
                            PivotKind::Origin => all.origin,
                        }
                    }
                };
                let turned = placement(e).rotated_about(
                    p.axis.vector(),
                    p.angle_deg.to_radians(),
                    glam::DVec3::from(point),
                );
                e.rotation_deg = turned.rotation_deg;
                e.translation = turned.translation;
                Ok(())
            })
        },
    );
    r.add(
        "geometry.place_on_ground",
        "Move a model up or down so its lowest point is at z = 0.",
        Options {
            undoable: true,
            errors,
            example: json!({"id": "model-1"}),
        },
        |s, p: IdParams| {
            update(s, &p.id, |root, e| {
                let world = placement(e).apply(&load_entry(root, e)?);
                e.translation[2] += ground_offset(&world);
                Ok(())
            })
        },
    );
    r.add(
        "geometry.fix_normals",
        "Orient all triangles of a model consistently outward.",
        Options {
            undoable: true,
            errors,
            example: json!({"id": "model-1"}),
        },
        |s, p: IdParams| {
            update(s, &p.id, |_, e| {
                e.fix_normals = true;
                Ok(())
            })
        },
    );
    r.add(
        "geometry.remove",
        "Remove a model from the project (its file stays in geometry/).",
        Options {
            undoable: true,
            errors,
            example: json!({"id": "model-1"}),
        },
        |s, p: IdParams| {
            let project = s.project()?;
            find(project, &p.id)?;
            project.manifest.geometry.retain(|e| e.id != p.id);
            Ok(())
        },
    );
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Empty {}
