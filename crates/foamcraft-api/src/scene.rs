// SPDX-License-Identifier: GPL-3.0-or-later
//! The 3D scene of a project: models (or the coloured result surface), caps, open edges, role
//! arrows, the wind tunnel, the mesh preview and the flow visualisation of the view settings.
//! The app draws it in its window; `view.render` draws it without a window.

use foamcraft_render::Scene;
use foamcraft_render::glam::Vec3;

/// Scene of the open project: every face and cap gets a pick id (`keys` holds the key per id).
pub struct ProjectScene {
    pub scene: Scene,
    pub keys: Vec<String>,
    /// Sphere that "fit view" frames (the models).
    pub focus: Option<(Vec3, f32)>,
    pub hit_meshes: Vec<(String, foamcraft_geometry::Mesh)>,
}

impl ProjectScene {
    pub fn empty() -> Self {
        Self {
            scene: Scene::empty(),
            keys: Vec::new(),
            focus: None,
            hit_meshes: Vec::new(),
        }
    }
}

/// Colours of caps by role.
const CAP_COLOR: [f32; 3] = [0.20, 0.65, 0.80];
const INLET_COLOR: [f32; 3] = [0.15, 0.70, 0.30];
const OUTLET_COLOR: [f32; 3] = [0.85, 0.25, 0.20];
const OPENING_COLOR: [f32; 3] = [1.0, 0.75, 0.10];

/// Arrow as line segments from `from` along `dir` with length `len`.
fn arrow(from: [f64; 3], dir: [f64; 3], len: f64) -> Vec<Vec<[f32; 3]>> {
    let d = foamcraft_render::glam::DVec3::from(dir).normalize_or_zero();
    let a = foamcraft_render::glam::DVec3::from(from);
    let tip = a + d * len;
    let side = d.any_orthonormal_vector() * len * 0.2;
    let back = tip - d * len * 0.3;
    let f = |v: foamcraft_render::glam::DVec3| [v.x as f32, v.y as f32, v.z as f32];
    vec![
        vec![f(a), f(tip)],
        vec![f(back + side), f(tip), f(back - side)],
    ]
}

/// Surface field `b` − `a` of the compared runs (difference mode), with its range.
pub fn difference_values(
    project: &foamcraft_core::project::Project,
) -> Result<(Vec<f32>, [f64; 2]), foamcraft_core::error::FoamError> {
    use foamcraft_core::error::FoamError;
    let view = &project.manifest.view;
    let cmp = view
        .compare
        .as_ref()
        .ok_or_else(|| FoamError::new("results.none", "no comparison"))?;
    let field = view.surface_field.as_deref().unwrap_or("p");
    let a = crate::results::load_surface(&project.root, &cmp.a)?;
    let b = crate::results::load_surface(&project.root, &cmp.b)?;
    let d = foamcraft_sim::results::difference_field(&a, &b, field).ok_or_else(|| {
        FoamError::new("results.geometry_differs", format!("{} {}", cmp.a, cmp.b))
    })?;
    let (lo, hi) = d.iter().fold((f64::MAX, f64::MIN), |(lo, hi), v| {
        (lo.min(f64::from(*v)), hi.max(f64::from(*v)))
    });
    Ok((d, [lo, hi]))
}

/// The colour scale of the view: low and high end and the unit of the shown surface field
/// (the difference in difference mode); `None` when no result colours the model.
pub fn legend(project: &foamcraft_core::project::Project) -> Option<(f64, f64, String)> {
    let (_, _, lo, hi) = surface_values(project)?;
    let view = &project.manifest.view;
    let field = view.surface_field.as_deref()?;
    let unit = view
        .result_job
        .as_deref()
        .and_then(|job| crate::results::load_meta(&project.root, job).ok())
        .and_then(|m| {
            m.fields
                .into_iter()
                .find(|f| f.name == field && f.location == "surface")
                .map(|f| f.unit)
        })
        .unwrap_or_default();
    Some((lo, hi, unit))
}

/// Surface, values per triangle and colour range of the shown result.
fn surface_values(
    project: &foamcraft_core::project::Project,
) -> Option<(foamcraft_sim::results::Surface, Vec<f32>, f64, f64)> {
    let view = &project.manifest.view;
    let field = view.surface_field.as_ref()?;
    let job = view.result_job.as_ref()?;
    let s = crate::results::load_surface(&project.root, job).ok()?;
    let difference = view
        .compare
        .as_ref()
        .filter(|c| c.mode == "difference")
        .and_then(|_| difference_values(project).ok());
    // A difference is centred on zero: the middle colour means "no change".
    let range = difference
        .as_ref()
        .map(|(_, [lo, hi])| {
            let m = lo.abs().max(hi.abs()).max(1e-12);
            [-m, m]
        })
        .or(view.range);
    let values: Vec<f32> = match difference {
        Some((d, _)) => d,
        None => s.field(field)?.to_vec(),
    };
    let (lo, hi) = range.map_or_else(
        || {
            values.iter().fold((f64::MAX, f64::MIN), |(lo, hi), v| {
                (lo.min(f64::from(*v)), hi.max(f64::from(*v)))
            })
        },
        |[lo, hi]| (lo, hi),
    );
    Some((s, values, lo, hi))
}

/// The wall surface of the result shown in the view, with one colour per triangle.
fn result_surface(
    project: &foamcraft_core::project::Project,
) -> Option<(String, foamcraft_geometry::Mesh, Vec<[f32; 3]>)> {
    let (s, values, lo, hi) = surface_values(project)?;
    let job = project.manifest.view.result_job.as_ref()?;
    let span = (hi - lo).max(1e-30);
    let colors = values
        .iter()
        .map(|v| foamcraft_sim::results::colormap((f64::from(*v) - lo) / span))
        .collect();
    let mesh = foamcraft_geometry::Mesh {
        positions: s.points,
        triangles: s.triangles,
        format: foamcraft_geometry::Format::Stl,
        declared_unit: None,
        face_tags: None,
        part_tags: None,
    };
    Some((format!("result:{job}"), mesh, colors))
}

/// The edges of the mesh preview are drawn unless a result field colours the model (they would
/// hide its colours).
pub fn draws_mesh_edges(view: &foamcraft_core::project::ViewSettings) -> bool {
    view.result_job.is_none() || view.surface_field.is_none()
}

/// Split view: the scenes of run `a` and `b` with one colour range; `None` without a split
/// comparison.
pub fn compare_scenes(
    project: &foamcraft_core::project::Project,
) -> Option<(ProjectScene, ProjectScene)> {
    let cmp = project
        .manifest
        .view
        .compare
        .as_ref()
        .filter(|c| c.mode == "split")?;
    let field = project.manifest.view.surface_field.clone()?;
    let side = |job: &str| {
        let mut p = project.clone();
        p.manifest.view.result_job = Some(job.to_string());
        p.manifest.view.compare = None;
        p
    };
    let (mut a, mut b) = (side(&cmp.a), side(&cmp.b));
    if project.manifest.view.range.is_none() {
        let span = |job: &str| {
            let s = crate::results::load_surface(&project.root, job).ok()?;
            Some(
                s.field(&field)?
                    .iter()
                    .fold((f64::MAX, f64::MIN), |(lo, hi), v| {
                        (lo.min(f64::from(*v)), hi.max(f64::from(*v)))
                    }),
            )
        };
        if let (Some(x), Some(y)) = (span(&cmp.a), span(&cmp.b)) {
            let range = Some([x.0.min(y.0), x.1.max(y.1)]);
            a.manifest.view.range = range;
            b.manifest.view.range = range;
        }
    }
    Some((project_scene(Some(&a)), project_scene(Some(&b))))
}

pub fn project_scene(project: Option<&foamcraft_core::project::Project>) -> ProjectScene {
    use foamcraft_geometry::commands::placement;
    use foamcraft_geometry::surfaces::{face_key, segmented_models};
    use foamcraft_render::scene::{MODEL_COLOR, axes, mesh_from_triangles_with_ids};
    let Some(models) = project.and_then(|p| segmented_models(p).ok()) else {
        return ProjectScene::empty();
    };
    if models.is_empty() {
        return ProjectScene::empty();
    }
    let mut hit_meshes = Vec::new();
    let mut keys = Vec::new();
    let mut meshes = Vec::new();
    for m in &models {
        let base = keys.len() as u32;
        keys.extend(m.segmentation.faces.iter().map(|f| face_key(&m.id, f.id)));
        let ids: Vec<u32> = m
            .segmentation
            .face_of_triangle
            .iter()
            .map(|f| base + f)
            .collect();
        let world = placement(&m.entry).apply(&m.mesh);
        meshes.push(mesh_from_triangles_with_ids(
            &world.positions,
            &world.triangles,
            &ids,
            MODEL_COLOR,
        ));
        hit_meshes.push((m.id.clone(), world));
    }
    // A result field replaces the plain models: the wall surface of the run, coloured.
    if let Some((key, surface, colors)) = project.and_then(result_surface) {
        keys = vec![key.clone()];
        let ids = vec![1; surface.triangles.len()];
        let mut mesh =
            mesh_from_triangles_with_ids(&surface.positions, &surface.triangles, &ids, MODEL_COLOR);
        mesh.vertex_colors = Some(colors.iter().flat_map(|c| [*c; 3]).collect());
        meshes = vec![mesh];
        hit_meshes = vec![(key, surface)];
    }
    // The model cut open along the clip plane of the view.
    if let Some(plane) = project.and_then(|p| p.manifest.view.clip.as_ref()) {
        for m in &mut meshes {
            crate::view::clip(m, plane);
        }
    }
    let models_len = meshes.len();
    // Caps: translucent, coloured by role, pickable; roles as arrows; open edges highlighted.
    let mut overlays = Vec::new();
    if let Some(p) = project {
        use foamcraft_core::project::BoundaryRole;
        use foamcraft_geometry::caps::{cap_meshes, cap_views, open_loops};
        let views = cap_views(p);
        for (key, cap) in cap_meshes(p) {
            keys.push(key.clone());
            let id = keys.len() as u32;
            let role = views.iter().find(|v| v.key == key).and_then(|v| v.role);
            let color = match role {
                Some(BoundaryRole::Inlet) => INLET_COLOR,
                Some(BoundaryRole::Outlet) => OUTLET_COLOR,
                _ => CAP_COLOR,
            };
            // Both sides: a cap is seen from inside and outside.
            let mut triangles = cap.triangles.clone();
            triangles.extend(cap.triangles.iter().map(|t| [t[0], t[2], t[1]]));
            let mut mesh = mesh_from_triangles_with_ids(
                &cap.positions,
                &triangles,
                &vec![id; triangles.len()],
                color,
            );
            mesh.opacity = 0.5;
            meshes.push(mesh);
            hit_meshes.push((key, cap));
        }
        for v in &views {
            let (dir, color) = match v.role {
                Some(BoundaryRole::Inlet) => (v.inward, INLET_COLOR),
                Some(BoundaryRole::Outlet) => (v.inward.map(|x| -x), OUTLET_COLOR),
                _ => continue,
            };
            let start = match v.role {
                // Inlet arrows end at the cap, outlet arrows start there.
                Some(BoundaryRole::Inlet) => [0, 1, 2].map(|k| v.centroid[k] - dir[k] * v.size),
                _ => v.centroid,
            };
            overlays.push(foamcraft_render::scene::polylines(
                &arrow(start, dir, v.size),
                color,
                false,
            ));
        }
        // Slices and isosurfaces of the result.
        overlays.extend(crate::view::flow_meshes(p));
        overlays.extend(crate::particles::streamline_meshes(p));
        let loops: Vec<Vec<[f32; 3]>> = open_loops(p)
            .iter()
            .map(|l| l.iter().map(|x| x.map(|c| c as f32)).collect())
            .collect();
        if !loops.is_empty() {
            overlays.push(foamcraft_render::scene::polylines(
                &loops,
                OPENING_COLOR,
                true,
            ));
        }
    }
    let mut scene = Scene {
        meshes,
        background: Scene::empty().background,
    };
    // Fit view frames the models; axes are sized to them.
    let focus = Scene {
        meshes: scene.meshes[..models_len].to_vec(),
        background: scene.background,
    }
    .bounding_sphere();
    scene.meshes.extend(overlays);
    scene.meshes.extend(axes((focus.1 * 1.6).max(1e-3)));
    // Wind tunnel around the models.
    if let Some(foamcraft_core::project::Scenario::WindTunnel(w)) =
        project.and_then(|p| p.manifest.scenario.as_ref())
    {
        let (mut min, mut max) = ([f64::MAX; 3], [f64::MIN; 3]);
        // Only the models (the first meshes), not the axes.
        for m in &scene.meshes[..models_len] {
            for v in &m.vertices {
                for k in 0..3 {
                    min[k] = min[k].min(f64::from(v.position[k]));
                    max[k] = max[k].max(f64::from(v.position[k]));
                }
            }
        }
        // Mesh preview: model surface of the latest mesh (flow frame, turned back by the yaw).
        let preview = project.and_then(|p| {
            p.manifest.mesh.as_ref()?;
            if !draws_mesh_edges(&p.manifest.view) {
                return None;
            }
            foamcraft_geometry::read_mesh(&p.root.join("cache").join("mesh_preview.obj")).ok()
        });
        if let Some(mut surface) = preview {
            // OpenFOAM boundary faces point out of the fluid, into the model; turn them outward so
            // the edges are drawn just outside the surface.
            foamcraft_geometry::fix_normals(&mut surface);
            let back = foamcraft_geometry::Placement {
                unit: "m".into(),
                rotation_deg: [w.direction_deg, 0.0, 0.0],
                ..Default::default()
            }
            .apply(&surface);
            let offset = ((max[0] - min[0]).max(max[2] - min[2]) * 0.002) as f32;
            scene.meshes.push(foamcraft_render::scene::mesh_edges(
                &back.positions,
                &back.triangles,
                [0.10, 0.12, 0.16],
                offset,
            ));
        }
        let ground = !matches!(w.ground, foamcraft_core::project::Ground::None);
        let t = foamcraft_sim::tunnel_box(min, max, w.direction_deg, w.ground);
        scene.meshes.extend(foamcraft_render::scene::tunnel(
            t.center,
            [t.length, t.width, t.height],
            t.yaw_deg,
            ground,
        ));
    }
    ProjectScene {
        scene,
        keys,
        focus: Some(focus),
        hit_meshes,
    }
}
