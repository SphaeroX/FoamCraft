// SPDX-License-Identifier: GPL-3.0-or-later
//! view.*: slices, isosurfaces and the clip plane of the 3D view (stored as the scene of the
//! project), and `view.render`, which draws the scene without a window.

use foamcraft_core::command::{Options, Registry, Session};
use foamcraft_core::error::FoamError;
use foamcraft_core::project::{IsoSpec, PlaneSpec, Project, SliceSpec, ViewSettings};
use foamcraft_render::scene::{Mesh, mesh_from_triangles_with_ids};
use foamcraft_render::{Renderer, camera::OrbitCamera};
use foamcraft_sim::results::{Grid, colormap};
use foamcraft_sim::visual::{FieldMesh, add_derived, isosurface, slice};
use glam::{DMat3, DVec3};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::results::{cache_dir, load_meta};

/// Points along each side of a slice.
pub const SLICE_RESOLUTION: u32 = 160;
/// Fields that isosurfaces can show.
pub const ISO_FIELDS: [&str; 3] = ["U_mag", "p_total", "Q"];

#[derive(Debug, Serialize, JsonSchema)]
pub struct VolumeField {
    pub name: String,
    pub unit: String,
}

fn unit_of(name: &str) -> &'static str {
    match name {
        "p" | "p_total" => "Pa",
        "U_mag" | "Ux" | "Uy" | "Uz" => "m/s",
        "k" => "m²/s²",
        "Q" => "1/s²",
        _ => "-",
    }
}

/// The job whose result the view shows (or the newest).
fn job_of(project: &Project) -> Result<String, FoamError> {
    if let Some(j) = &project.manifest.view.result_job
        && cache_dir(&project.root, j).join("meta.json").is_file()
    {
        return Ok(j.clone());
    }
    std::fs::read_dir(project.root.join(crate::results::CACHE_DIR))
        .ok()
        .into_iter()
        .flatten()
        .flatten()
        .filter(|e| e.path().join("meta.json").is_file())
        .max_by_key(|e| e.metadata().and_then(|m| m.modified()).ok())
        .map(|e| e.file_name().to_string_lossy().to_string())
        .ok_or_else(|| FoamError::new("results.none", ""))
}

fn volume_fields(project: &Project) -> Result<Vec<VolumeField>, FoamError> {
    let meta = load_meta(&project.root, &job_of(project)?)?;
    let mut names: Vec<String> = meta
        .fields
        .iter()
        .filter(|f| f.location == "volume")
        .map(|f| f.name.clone())
        .collect();
    if names.iter().any(|n| n == "Ux") {
        names.extend(["U_mag".to_string(), "Q".to_string()]);
    }
    if names.iter().any(|n| n == "p") && !names.iter().any(|n| n == "p_total") {
        names.push("p_total".into());
    }
    Ok(names
        .into_iter()
        .map(|name| VolumeField {
            unit: unit_of(&name).into(),
            name,
        })
        .collect())
}

/// Grids of the shown result with the derived fields (speed, Q, total pressure).
pub fn load_grids(project: &Project) -> Result<(Vec<Grid>, f64), FoamError> {
    let job = job_of(project)?;
    let meta = load_meta(&project.root, &job)?;
    let mut grids = Vec::new();
    for n in 0..meta.levels.len() {
        let bytes = std::fs::read(cache_dir(&project.root, &job).join(format!("grid_{n}.bin")))
            .map_err(|e| FoamError::new("results.read_failed", e.to_string()))?;
        let mut g = Grid::from_bytes(&bytes)?;
        add_derived(&mut g);
        grids.push(g);
    }
    Ok((grids, meta.yaw_deg))
}

fn check_field(project: &Project, field: &str) -> Result<(), FoamError> {
    if volume_fields(project)?.iter().any(|f| f.name == field) {
        Ok(())
    } else {
        Err(FoamError::new("results.unknown_field", field))
    }
}

fn check_plane(p: &PlaneSpec) -> Result<(), FoamError> {
    let n = DVec3::from(p.normal);
    if p.origin.iter().all(|x| x.is_finite()) && n.is_finite() && n.length() > 1e-12 {
        Ok(())
    } else {
        Err(FoamError::new("view.invalid_plane", format!("{p:?}")))
    }
}

fn next_id(prefix: &str, used: impl Iterator<Item = String>) -> String {
    let used: Vec<String> = used.collect();
    (1..)
        .map(|n| format!("{prefix}-{n}"))
        .find(|id| !used.contains(id))
        .unwrap_or_default()
}

fn changed(s: &mut Session) -> Result<ViewSettings, FoamError> {
    let project = s.project()?;
    project.save()?;
    let view = project.manifest.view.clone();
    s.events().emit("project.changed", serde_json::Value::Null);
    Ok(view)
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AddSlice {
    pub origin: [f64; 3],
    pub normal: [f64; 3],
    pub field: String,
    #[serde(default)]
    pub range: Option<[f64; 2]>,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct UpdateSlice {
    pub id: String,
    #[serde(default)]
    pub origin: Option<[f64; 3]>,
    #[serde(default)]
    pub normal: Option<[f64; 3]>,
    #[serde(default)]
    pub field: Option<String>,
    #[serde(default)]
    pub range: Option<[f64; 2]>,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AddIso {
    pub field: String,
    pub value: f64,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct IdParams {
    pub id: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ClipParams {
    /// Plane to cut the model along; null shows the whole model.
    pub plane: Option<PlaneSpec>,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RenderParams {
    /// PNG file to write.
    pub path: std::path::PathBuf,
    #[serde(default)]
    pub width: Option<u32>,
    #[serde(default)]
    pub height: Option<u32>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct Rendered {
    pub path: String,
    pub width: u32,
    pub height: u32,
}

fn render(project: &Project, p: RenderParams) -> Result<Rendered, FoamError> {
    let (w, h) = (
        p.width.unwrap_or(1280).clamp(16, 8192),
        p.height.unwrap_or(800).clamp(16, 8192),
    );
    let renderer = Renderer::new_headless().ok_or_else(|| FoamError::new("view.no_gpu", ""))?;
    let mut camera = OrbitCamera::default();
    let pixels = if let Some((a, b)) = crate::scene::compare_scenes(project) {
        let (centre, radius) = a.focus.unwrap_or_else(|| a.scene.bounding_sphere());
        camera.fit(centre, radius * 1.1, (w / 2) as f32 / h as f32);
        renderer.render_offscreen_split(
            &renderer.upload(&a.scene),
            &renderer.upload(&b.scene),
            &camera,
            w,
            h,
        )
    } else {
        let built = crate::scene::project_scene(Some(project));
        let (centre, radius) = built.focus.unwrap_or_else(|| built.scene.bounding_sphere());
        camera.fit(centre, radius * 1.1, w as f32 / h as f32);
        renderer.render_offscreen(&built.scene, &camera, w, h)
    };
    foamcraft_render::write_png(&p.path, w, h, &pixels)
        .map_err(|e| FoamError::new("view.write_failed", e.to_string()))?;
    Ok(Rendered {
        path: p.path.display().to_string(),
        width: w,
        height: h,
    })
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CompareParams {
    /// First run (reference); null ends the comparison.
    pub a: Option<String>,
    #[serde(default)]
    pub b: Option<String>,
    /// `split` (default) or `difference`.
    #[serde(default)]
    pub mode: Option<String>,
}

fn set_compare(s: &mut Session, p: CompareParams) -> Result<serde_json::Value, FoamError> {
    let project = s.project()?;
    let Some(a) = p.a else {
        project.manifest.view.compare = None;
        project.save()?;
        s.events().emit("project.changed", serde_json::Value::Null);
        return Ok(json!({"mode": null}));
    };
    let b = p.b.ok_or_else(|| FoamError::new("results.none", "b"))?;
    let mode = p.mode.unwrap_or_else(|| "split".into());
    if mode != "split" && mode != "difference" {
        return Err(FoamError::new("results.none", format!("mode {mode}")));
    }
    for job in [&a, &b] {
        crate::results::load_meta(&project.root, job)?;
    }
    let before = project.manifest.view.clone();
    let view = &mut project.manifest.view;
    view.result_job = Some(a.clone());
    if view.surface_field.is_none() {
        view.surface_field = Some("p".into());
    }
    view.compare = Some(foamcraft_core::project::CompareSpec {
        a: a.clone(),
        b: b.clone(),
        mode: mode.clone(),
    });
    let difference = if mode == "difference" {
        match crate::scene::difference_values(project) {
            Ok((_, [lo, hi])) => Some(json!({"min": lo, "max": hi})),
            Err(e) => {
                project.manifest.view = before;
                return Err(e);
            }
        }
    } else {
        None
    };
    project.save()?;
    s.events().emit("project.changed", serde_json::Value::Null);
    Ok(json!({"mode": mode, "a": a, "b": b, "difference": difference}))
}

fn to_render_mesh(
    m: &FieldMesh,
    turn: DMat3,
    colors: Option<Vec<[f32; 3]>>,
    color: [f32; 3],
) -> Mesh {
    let positions: Vec<[f32; 3]> = m
        .positions
        .iter()
        .map(|p| {
            let w = turn * DVec3::new(f64::from(p[0]), f64::from(p[1]), f64::from(p[2]));
            [w.x as f32, w.y as f32, w.z as f32]
        })
        .collect();
    // Both sides: slices and isosurfaces are seen from either side.
    let mut triangles = m.triangles.clone();
    triangles.extend(m.triangles.iter().map(|t| [t[0], t[2], t[1]]));
    let ids = vec![0; triangles.len()];
    let mut mesh = mesh_from_triangles_with_ids(&positions, &triangles, &ids, color);
    if let Some(c) = colors {
        mesh.vertex_colors = Some(
            triangles
                .iter()
                .flat_map(|t| t.map(|i| c[i as usize]))
                .collect(),
        );
    }
    mesh
}

fn iso_color(field: &str) -> [f32; 3] {
    match field {
        "Q" => [0.55, 0.30, 0.75],
        "p_total" => [0.90, 0.55, 0.15],
        _ => [0.20, 0.65, 0.80],
    }
}

/// Slices and isosurfaces of the view as render meshes (world coordinates).
pub fn flow_meshes(project: &Project) -> Vec<Mesh> {
    let view = &project.manifest.view;
    if view.slices.is_empty() && view.isosurfaces.is_empty() {
        return vec![];
    }
    let Ok((grids, yaw)) = load_grids(project) else {
        return vec![];
    };
    let to_world = DMat3::from_rotation_z(yaw.to_radians());
    let to_flow = to_world.transpose();
    let mut out = Vec::new();
    for s in &view.slices {
        let origin = (to_flow * DVec3::from(s.plane.origin)).into();
        let normal = (to_flow * DVec3::from(s.plane.normal)).into();
        let m = slice(&grids, &s.field, origin, normal, SLICE_RESOLUTION);
        if m.triangles.is_empty() {
            continue;
        }
        let (lo, hi) = s.range.map_or_else(
            || {
                m.values.iter().fold((f64::MAX, f64::MIN), |(a, b), v| {
                    (a.min(f64::from(*v)), b.max(f64::from(*v)))
                })
            },
            |[a, b]| (a, b),
        );
        let span = (hi - lo).max(1e-30);
        let colors = m
            .values
            .iter()
            .map(|v| colormap((f64::from(*v) - lo) / span))
            .collect();
        out.push(to_render_mesh(&m, to_world, Some(colors), [0.5; 3]));
    }
    for iso in &view.isosurfaces {
        let m = isosurface(&grids, &iso.field, iso.value);
        if m.triangles.is_empty() {
            continue;
        }
        let mut mesh = to_render_mesh(&m, to_world, None, iso_color(&iso.field));
        mesh.opacity = 0.5;
        out.push(mesh);
    }
    out
}

/// Hides the triangles on the side of the clip plane the normal points to.
pub fn clip(mesh: &mut Mesh, plane: &PlaneSpec) {
    let o = DVec3::from(plane.origin);
    let n = DVec3::from(plane.normal);
    let keep: Vec<u32> = mesh
        .indices
        .chunks(3)
        .filter(|t| {
            let c = t
                .iter()
                .map(|i| {
                    let p = mesh.vertices[*i as usize].position;
                    DVec3::new(f64::from(p[0]), f64::from(p[1]), f64::from(p[2]))
                })
                .sum::<DVec3>()
                / 3.0;
            (c - o).dot(n) <= 0.0
        })
        .flatten()
        .copied()
        .collect();
    mesh.indices = keep;
}

pub fn register(r: &mut Registry) {
    let none: &'static [&'static str] = &["project.none_open"];
    r.add(
        "view.state",
        "The view settings of the project: shown result, surface field, slices, isosurfaces, clip plane.",
        Options {
            undoable: false,
            errors: none,
            example: json!({}),
        },
        |s, _: crate::results::JobParams| Ok(s.project()?.manifest.view.clone()),
    );
    r.add(
        "view.volume_fields",
        "Volume fields of the shown result that slices and isosurfaces can use.",
        Options {
            undoable: false,
            errors: &["project.none_open", "results.none"],
            example: json!({}),
        },
        |s, _: crate::results::JobParams| volume_fields(s.project()?),
    );
    r.add(
        "view.add_slice",
        "Add a plane through the flow coloured by a volume field.",
        Options {
            undoable: false,
            errors: &["project.none_open", "results.none", "results.unknown_field", "view.invalid_plane"],
            example: json!({"origin": [0.0, 0.0, 0.1], "normal": [0.0, 0.0, 1.0], "field": "U_mag"}),
        },
        |s, p: AddSlice| {
            let project = s.project()?;
            check_field(project, &p.field)?;
            let plane = PlaneSpec {
                origin: p.origin,
                normal: p.normal,
            };
            check_plane(&plane)?;
            let view = &mut project.manifest.view;
            let id = next_id("slice", view.slices.iter().map(|x| x.id.clone()));
            view.slices.push(SliceSpec {
                id,
                plane,
                field: p.field,
                range: p.range,
            });
            changed(s)
        },
    );
    r.add(
        "view.update_slice",
        "Move a slice or change its field or range.",
        Options {
            undoable: false,
            errors: &[
                "project.none_open",
                "view.not_found",
                "results.unknown_field",
                "view.invalid_plane",
            ],
            example: json!({"id": "slice-1", "origin": [0.0, 0.0, 0.2]}),
        },
        |s, p: UpdateSlice| {
            let project = s.project()?;
            if let Some(f) = &p.field {
                check_field(project, f)?;
            }
            let slice = project
                .manifest
                .view
                .slices
                .iter_mut()
                .find(|x| x.id == p.id)
                .ok_or_else(|| FoamError::new("view.not_found", p.id.clone()))?;
            let mut plane = slice.plane.clone();
            if let Some(o) = p.origin {
                plane.origin = o;
            }
            if let Some(n) = p.normal {
                plane.normal = n;
            }
            check_plane(&plane)?;
            slice.plane = plane;
            if let Some(f) = p.field {
                slice.field = f;
            }
            if p.range.is_some() {
                slice.range = p.range;
            }
            changed(s)
        },
    );
    r.add(
        "view.add_isosurface",
        "Show the surface where a field (U_mag, p_total or Q) has a value.",
        Options {
            undoable: false,
            errors: &["project.none_open", "results.none", "results.unknown_field"],
            example: json!({"field": "Q", "value": 1000.0}),
        },
        |s, p: AddIso| {
            let project = s.project()?;
            if !ISO_FIELDS.contains(&p.field.as_str()) {
                return Err(FoamError::new("results.unknown_field", p.field));
            }
            check_field(project, &p.field)?;
            let view = &mut project.manifest.view;
            let id = next_id("iso", view.isosurfaces.iter().map(|x| x.id.clone()));
            view.isosurfaces.push(IsoSpec {
                id,
                field: p.field,
                value: p.value,
            });
            changed(s)
        },
    );
    r.add(
        "view.remove",
        "Remove a slice or an isosurface.",
        Options {
            undoable: false,
            errors: &["project.none_open", "view.not_found"],
            example: json!({"id": "slice-1"}),
        },
        |s, p: IdParams| {
            let view = &mut s.project()?.manifest.view;
            let before = view.slices.len() + view.isosurfaces.len();
            view.slices.retain(|x| x.id != p.id);
            view.isosurfaces.retain(|x| x.id != p.id);
            if view.slices.len() + view.isosurfaces.len() == before {
                return Err(FoamError::new("view.not_found", p.id));
            }
            changed(s)
        },
    );
    r.add(
        "view.set_clip",
        "Cut the model open along a plane (the side the normal points to is hidden); null: whole model.",
        Options {
            undoable: false,
            errors: &["project.none_open", "view.invalid_plane"],
            example: json!({"plane": {"origin": [0.0, 0.0, 0.0], "normal": [0.0, -1.0, 0.0]}}),
        },
        |s, p: ClipParams| {
            if let Some(plane) = &p.plane {
                check_plane(plane)?;
            }
            s.project()?.manifest.view.clip = p.plane;
            changed(s)
        },
    );
    r.add(
        "view.set_compare",
        "Compare two runs in the view: side by side with one camera (split) or the surface field \
         of b minus a (difference, same geometry only). a = null ends the comparison.",
        Options {
            undoable: false,
            errors: &[
                "project.none_open",
                "results.none",
                "results.geometry_differs",
            ],
            example: json!({"a": "job-1", "b": "job-2", "mode": "split"}),
        },
        set_compare,
    );
    r.add(
        "view.render",
        "Draw the current scene of the project into a PNG file (no window needed).",
        Options {
            undoable: false,
            errors: &["project.none_open", "view.no_gpu", "view.write_failed"],
            example: json!({"path": "C:/Temp/scene.png", "width": 1280, "height": 800}),
        },
        |s, p: RenderParams| render(s.project()?, p),
    );
}
