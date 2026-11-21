// SPDX-License-Identifier: GPL-3.0-or-later
//! FoamCraft desktop app: a Tauri window with the native wgpu viewport beneath a transparent
//! web view.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod api;
mod ride;

use std::sync::Mutex;

use foamcraft_api::scene::{ProjectScene, SceneModel, SceneSlice, SceneWind, project_scene};
use foamcraft_render::camera::{CameraTransition, OrbitCamera, named_view};
use foamcraft_render::glam::{Mat4, Vec3};
use foamcraft_render::handles::{
    AxisDrag, HANDLE_KEYS, RingDrag, WIND_KEY, handle_meshes, move_step, snap, wind_arrow_mesh,
    wind_direction_deg,
};
use foamcraft_render::{GpuScene, Renderer, Scene, Viewport, default_instance};
use serde::{Deserialize, Serialize};
use tauri::{Emitter, Manager, WindowEvent};

/// Seconds a change to a standard view takes.
const VIEW_TRANSITION_SECONDS: f32 = 0.35;

struct ViewState {
    renderer: Renderer,
    viewport: Viewport,
    /// Uploaded once per change; orbiting only redraws.
    gpu: GpuScene,
    bounds: (foamcraft_render::glam::Vec3, f32),
    camera: OrbitCamera,
    /// Face key of every pick id (index = id - 1).
    pick_keys: Vec<String>,
    hover: Option<u32>,
    selected: Vec<u32>,
    /// World meshes for finding the clicked point: model id or cap key, mesh.
    hit_meshes: Vec<(String, foamcraft_geometry::Mesh)>,
    /// Flow time per animation frame and whether the particles stand still.
    particle_dt: f32,
    particles_paused: bool,
    /// Measured animation time (frames, seconds) for the log.
    frame_stats: (u32, f64),
    /// Second run of a split comparison, drawn in the right half with the same camera.
    compare: Option<GpuScene>,
    /// Physical pixels covered by the side panels (left, right); a split view uses the rest.
    insets: (u32, u32),
    /// Running move to a standard view and when it started.
    transition: Option<(CameraTransition, std::time::Instant)>,
    /// Plain models of the scene (meshes and bounding sphere) for the handles.
    models: Vec<SceneModel>,
    /// Number of pick keys of the scene itself; handle keys follow.
    scene_keys: usize,
    /// Model and pivot (metres) the handles are shown at.
    handles: Option<(String, [f32; 3])>,
    drag: Option<HandleDrag>,
    /// Wind of a wind tunnel scene and the meshes of its tunnel.
    wind: Option<SceneWind>,
    tunnel_meshes: Vec<usize>,
    /// Show the wind arrow handle (Scenario step).
    show_wind: bool,
    /// Shown slices; they can be dragged along their normal.
    slices: Vec<SceneSlice>,
    /// Project and model ids the camera was last fitted to; a new set fits again.
    fit_key: String,
    /// The ride along one particle, while one is shown.
    flight: Option<ride::Flight>,
    /// Turn of the particle grids into the world (degrees), for picking particles.
    particle_yaw: f32,
    /// Animation speed of the surface flow; `None` when nothing on the surface moves.
    flow_speed: Option<f32>,
    flow_time: f32,
}

/// What a handle drag does.
#[derive(Clone)]
enum DragKind {
    Move(AxisDrag),
    Rotate(RingDrag),
    /// Turning the wind; the value is the new direction in degrees, `start` the old one.
    Wind {
        start: f64,
    },
    /// Moving a slice along its normal; the value is the offset in metres within `range`.
    Slice {
        drag: AxisDrag,
        id: String,
        origin: Vec3,
        range: [f32; 2],
    },
}

/// A running drag of a move or rotate handle: the model is previewed with a matrix, and the
/// end turns into exactly one command.
struct HandleDrag {
    model: String,
    meshes: Vec<usize>,
    pivot: Vec3,
    axis: usize,
    kind: DragKind,
    /// Snap step: metres for moves, radians for turns.
    step: f64,
    /// Current value (snapped unless free): metres or radians.
    value: f64,
}

const AXIS_NAMES: [&str; 3] = ["x", "y", "z"];
const AXES: [Vec3; 3] = [Vec3::X, Vec3::Y, Vec3::Z];

/// Value shown while dragging.
#[derive(Serialize)]
struct HandleValue {
    /// `move` (metres) or `rotate` (degrees).
    kind: &'static str,
    axis: &'static str,
    value: f64,
}

/// The command that finishes a drag.
#[derive(Serialize)]
struct HandleCommand {
    command: &'static str,
    params: serde_json::Value,
}

impl ViewState {
    /// `focus` is the sphere "fit view" frames (the models); default: the whole scene.
    fn set_scene(&mut self, built: ProjectScene) {
        let geometry = built.flow_geometry();
        let ProjectScene {
            scene,
            keys: pick_keys,
            focus,
            hit_meshes,
            models,
            wind,
            tunnel_meshes,
            slices,
            ..
        } = built;
        self.slices = slices;
        self.wind = wind;
        self.tunnel_meshes = tunnel_meshes;
        self.gpu = self.renderer.upload(&scene);
        self.renderer
            .set_surface_flow(&mut self.gpu, geometry.as_ref());
        self.gpu.time = self.flow_time;
        self.bounds = focus.unwrap_or_else(|| scene.bounding_sphere());
        self.scene_keys = pick_keys.len();
        self.pick_keys = pick_keys;
        self.hit_meshes = hit_meshes;
        self.models = models;
        self.drag = None;
        self.hover = None;
        self.selected.clear();
        self.show_handles(None, None);
    }

    /// Uploads the handles at the chosen model and pivot (moved by `offset` while dragging),
    /// or removes them.
    /// `wind` overrides the shown wind direction (degrees) while it is dragged.
    fn show_handles(&mut self, offset: Option<Vec3>, wind: Option<f32>) {
        self.pick_keys.truncate(self.scene_keys);
        let mut meshes = Vec::new();
        let model = self
            .handles
            .as_ref()
            .and_then(|(id, p)| Some((self.models.iter().find(|m| &m.id == id)?, *p)));
        if let Some((model, pivot)) = model {
            let pivot = Vec3::from(pivot) + offset.unwrap_or(Vec3::ZERO);
            let first = self.scene_keys as u32 + 1;
            meshes = handle_meshes(pivot, model.radius, first);
            self.pick_keys
                .extend(HANDLE_KEYS.iter().map(|k| (*k).to_string()));
        }
        if let (true, Some(w)) = (self.show_wind, self.wind) {
            let id = self.pick_keys.len() as u32 + 1;
            let deg = wind.unwrap_or(w.direction_deg);
            meshes.push(wind_arrow_mesh(Vec3::from(w.center), w.radius, deg, id));
            self.pick_keys.push(WIND_KEY.to_string());
        }
        self.renderer.set_handles(&mut self.gpu, &meshes);
    }

    /// Camera ray through a pixel of the window.
    fn ray(&self, x: f32, y: f32) -> (Vec3, Vec3) {
        let (w, h) = self.viewport.size();
        let aspect = w as f32 / h.max(1) as f32;
        let ndc_x = 2.0 * x / w.max(1) as f32 - 1.0;
        let ndc_y = 1.0 - 2.0 * y / h.max(1) as f32;
        self.camera.ray(aspect, ndc_x, ndc_y)
    }

    /// Starts dragging handle `key` at a pixel; false when no handle is shown or the key is
    /// not a handle.
    fn begin_drag(&mut self, key: &str, x: f32, y: f32) -> bool {
        if let Some(id) = key.strip_prefix("slice:") {
            let Some(sl) = self.slices.iter().find(|s| s.id == id).cloned() else {
                return false;
            };
            let origin = Vec3::from(sl.origin);
            let normal = Vec3::from(sl.normal).normalize_or_zero();
            let ray = self.ray(x, y);
            self.drag = Some(HandleDrag {
                model: String::new(),
                meshes: vec![sl.mesh],
                pivot: origin,
                axis: 2,
                kind: DragKind::Slice {
                    drag: AxisDrag::new(origin, normal, ray),
                    id: sl.id,
                    origin,
                    range: sl.range,
                },
                step: 0.0,
                value: 0.0,
            });
            self.transition = None;
            return true;
        }
        if key == WIND_KEY {
            let Some(w) = self.wind.filter(|_| self.show_wind) else {
                return false;
            };
            let start = f64::from(w.direction_deg);
            self.drag = Some(HandleDrag {
                model: String::new(),
                meshes: self.tunnel_meshes.clone(),
                pivot: Vec3::from(w.center),
                axis: 2,
                kind: DragKind::Wind { start },
                step: 5.0,
                value: start,
            });
            self.transition = None;
            return true;
        }
        let Some((id, pivot)) = self.handles.clone() else {
            return false;
        };
        let Some(model) = self.models.iter().find(|m| m.id == id) else {
            return false;
        };
        let Some(i) = HANDLE_KEYS.iter().position(|k| *k == key) else {
            return false;
        };
        let axis = i % 3;
        let pivot = Vec3::from(pivot);
        let ray = self.ray(x, y);
        let (kind, step) = if i < 3 {
            (
                DragKind::Move(AxisDrag::new(pivot, AXES[axis], ray)),
                move_step(f64::from(model.radius) * 2.0),
            )
        } else {
            (
                DragKind::Rotate(RingDrag::new(pivot, AXES[axis], ray)),
                5f64.to_radians(),
            )
        };
        self.drag = Some(HandleDrag {
            model: id,
            meshes: model.meshes.clone(),
            pivot,
            axis,
            kind,
            step,
            value: 0.0,
        });
        self.transition = None;
        true
    }

    /// Follows the mouse during a drag; `free` turns snapping off.
    fn drag_to(&mut self, x: f32, y: f32, free: bool) -> Option<HandleValue> {
        let ray = self.ray(x, y);
        let drag = self.drag.as_mut()?;
        let raw = match &mut drag.kind {
            DragKind::Move(d) => f64::from(d.update(ray)),
            DragKind::Rotate(d) => f64::from(d.update(ray)),
            // Seen edge-on the plane gives no direction: keep the last one.
            DragKind::Wind { .. } => wind_direction_deg(drag.pivot, ray)
                .map(f64::from)
                .unwrap_or(drag.value),
            // Stays inside the grid so the slice never disappears.
            DragKind::Slice { drag: d, range, .. } => {
                f64::from(d.update(ray).clamp(range[0], range[1]))
            }
        };
        drag.value = if free || drag.step <= 0.0 {
            raw
        } else {
            snap(raw, drag.step)
        };
        let axis = AXES[drag.axis];
        let mut wind = None;
        let (model, offset, kind, shown) = match drag.kind {
            DragKind::Move(_) => {
                let t = axis * drag.value as f32;
                (Mat4::from_translation(t), Some(t), "move", drag.value)
            }
            DragKind::Rotate(_) => {
                let p = drag.pivot;
                let m = Mat4::from_translation(p)
                    * Mat4::from_axis_angle(axis, drag.value as f32)
                    * Mat4::from_translation(-p);
                (m, None, "rotate", drag.value.to_degrees())
            }
            DragKind::Wind { start } => {
                let p = drag.pivot;
                let turn = (drag.value - start).to_radians() as f32;
                let m = Mat4::from_translation(p)
                    * Mat4::from_rotation_z(turn)
                    * Mat4::from_translation(-p);
                wind = Some(drag.value as f32);
                (m, None, "wind", drag.value)
            }
            DragKind::Slice { ref id, .. } => {
                let normal = self
                    .slices
                    .iter()
                    .find(|s| &s.id == id)
                    .map_or(Vec3::Z, |s| Vec3::from(s.normal).normalize_or_zero());
                let t = Mat4::from_translation(normal * drag.value as f32);
                (t, None, "slice", drag.value)
            }
        };
        let meshes = drag.meshes.clone();
        let axis_name = AXIS_NAMES[drag.axis];
        for i in meshes {
            self.renderer.set_model_transform(&self.gpu, i, model);
        }
        self.show_handles(offset, wind);
        self.draw();
        Some(HandleValue {
            kind,
            axis: axis_name,
            value: shown,
        })
    }

    /// Ends a drag: the command to run (none when nothing changed).
    fn end_drag(&mut self) -> Option<HandleCommand> {
        let drag = self.drag.take()?;
        let unchanged = match drag.kind {
            DragKind::Wind { start } => (drag.value - start).abs() < 1e-9,
            _ => drag.value.abs() < 1e-12,
        };
        if unchanged {
            self.reset_preview(&drag);
            return None;
        }
        // A source of air (`jet:N`) is moved and turned with its own commands.
        if let Some(n) = drag
            .model
            .strip_prefix("jet:")
            .and_then(|n| n.parse::<usize>().ok())
        {
            let index = n.saturating_sub(1);
            return Some(match drag.kind {
                DragKind::Rotate(_) => HandleCommand {
                    command: "scenario.rotate_jet",
                    params: serde_json::json!({
                        "index": index,
                        "axis": AXIS_NAMES[drag.axis],
                        "angle_deg": drag.value.to_degrees(),
                    }),
                },
                _ => {
                    let mut delta = [0.0; 3];
                    delta[drag.axis] = drag.value;
                    HandleCommand {
                        command: "scenario.move_jet",
                        params: serde_json::json!({"index": index, "delta": delta}),
                    }
                }
            });
        }
        Some(match drag.kind {
            DragKind::Slice { id, origin, .. } => {
                let normal = self
                    .slices
                    .iter()
                    .find(|s| s.id == id)
                    .map_or(Vec3::Z, |s| Vec3::from(s.normal).normalize_or_zero());
                let o = origin + normal * drag.value as f32;
                HandleCommand {
                    command: "view.update_slice",
                    params: serde_json::json!({"id": id, "origin": o.to_array().map(f64::from)}),
                }
            }
            DragKind::Wind { .. } => HandleCommand {
                command: "scenario.set_wind_direction",
                params: serde_json::json!({"direction_deg": drag.value}),
            },
            DragKind::Move(_) => {
                let mut delta = [0.0; 3];
                delta[drag.axis] = drag.value;
                HandleCommand {
                    command: "geometry.move_by",
                    params: serde_json::json!({"id": drag.model, "delta": delta}),
                }
            }
            DragKind::Rotate(_) => HandleCommand {
                command: "geometry.rotate_about",
                params: serde_json::json!({
                    "id": drag.model,
                    "axis": AXIS_NAMES[drag.axis],
                    "angle_deg": drag.value.to_degrees(),
                    "pivot": drag.pivot.to_array().map(f64::from),
                }),
            },
        })
    }

    /// Puts the model back where it was before the drag.
    fn reset_preview(&mut self, drag: &HandleDrag) {
        for &i in &drag.meshes {
            self.renderer
                .set_model_transform(&self.gpu, i, Mat4::IDENTITY);
        }
        self.show_handles(None, None);
        self.draw();
    }

    fn highlight(&mut self) {
        self.renderer
            .set_highlight(&mut self.gpu, self.hover, &self.selected);
    }

    fn pick(&self, x: f32, y: f32) -> Option<u32> {
        let (w, h) = self.viewport.size();
        // The split view of two runs is for looking only.
        if x < 0.0 || y < 0.0 || self.compare.is_some() {
            return None;
        }
        self.renderer
            .pick(&self.gpu, &self.camera, w, h, x as u32, y as u32)
    }

    fn fit(&mut self) {
        self.camera = self.fitted(self.camera);
    }

    /// `camera` moved so the models fill the view.
    fn fitted(&self, mut camera: OrbitCamera) -> OrbitCamera {
        let (center, radius) = self.bounds;
        let (w, h) = self.viewport.size();
        // Each half of a split view is half as wide as the free area.
        let w = if self.compare.is_some() {
            self.span().1 / 2
        } else {
            w
        };
        // A little margin so the model does not touch the window edges.
        camera.fit(center, radius * 1.1, w as f32 / h.max(1) as f32);
        camera
    }

    /// Starts a short move to a standard view that ends fitted. False for an unknown name.
    fn start_view(&mut self, name: &str) -> bool {
        let Some((yaw, pitch)) = named_view(name) else {
            return false;
        };
        let target = self.fitted(OrbitCamera {
            yaw,
            pitch,
            ..self.camera
        });
        let transition = CameraTransition::new(self.camera, target, VIEW_TRANSITION_SECONDS);
        self.transition = Some((transition, std::time::Instant::now()));
        true
    }

    /// Advances a running view change; true while it was running (the view must be drawn).
    fn step_transition(&mut self) -> bool {
        let Some((transition, start)) = self.transition else {
            return false;
        };
        let elapsed = start.elapsed().as_secs_f32();
        self.camera = transition.at(elapsed);
        if transition.finished(elapsed) {
            self.transition = None;
        }
        true
    }

    fn pose(&self) -> CameraPose {
        CameraPose {
            yaw: self.camera.yaw,
            pitch: self.camera.pitch,
        }
    }

    /// Columns of the window not covered by the side panels: (first, count).
    fn span(&self) -> (u32, u32) {
        let (w, _) = self.viewport.size();
        let (l, r) = self.insets;
        if l + r + 2 >= w {
            return (0, w);
        }
        (l, w - l - r)
    }

    /// Lines and marker of the ride in the main view and the inset.
    fn ride_meshes(&mut self) {
        let Some(f) = self.flight.as_mut() else {
            return;
        };
        let main = f.ride.meshes(f.t, &f.ride.spec.view);
        let inset = f.ride.meshes(f.t, "overview");
        self.renderer.set_handles(&mut self.gpu, &main);
        self.renderer.set_handles(&mut f.inset, &inset);
    }

    fn draw(&mut self) {
        let span = self.span();
        if let Some(f) = &self.flight
            && let Some(cam) = f.camera()
        {
            let (_, h) = self.viewport.size();
            let iw = (span.1 / 4).max(16);
            let ih = (iw * h / span.1.max(1)).max(16);
            let m = 12;
            // Right edge of the free area, below the view cube and above the flight bar.
            let rect = [
                (span.0 + span.1).saturating_sub(iw + m),
                (h as f32 * 0.3) as u32,
                iw,
                ih,
            ];
            self.viewport
                .render_inset(&self.renderer, &self.gpu, &cam, &f.inset, &f.before, rect);
            return;
        }
        match &self.compare {
            Some(right) => {
                self.viewport
                    .render_split(&self.renderer, &self.gpu, right, &self.camera, span)
            }
            None => self
                .viewport
                .render(&self.renderer, &self.gpu, &self.camera),
        }
    }
}

/// `None` when no GPU adapter or surface is available; the interface still works then.
struct AppState(Mutex<Option<ViewState>>);

#[derive(Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum CameraAction {
    Orbit {
        yaw: f32,
        pitch: f32,
    },
    Pan {
        dx: f32,
        dy: f32,
    },
    Zoom {
        factor: f32,
    },
    Fit,
    Resize,
    /// A standard view such as `front` or `top_front_right`, reached with a short move.
    View {
        name: String,
    },
}

/// Camera angles for the view cube, radians.
#[derive(Debug, Clone, Copy, Serialize)]
struct CameraPose {
    yaw: f32,
    pitch: f32,
}

#[derive(Serialize)]
struct AppInfo {
    version: &'static str,
}

/// Path of the command line program next to the app (for the MCP configuration).
#[tauri::command]
fn cli_path() -> String {
    let name = if cfg!(windows) {
        "foamcraft.exe"
    } else {
        "foamcraft"
    };
    std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(|d| d.join(name)))
        .map_or_else(|| name.to_string(), |p| p.display().to_string())
}

#[tauri::command]
fn app_info() -> AppInfo {
    AppInfo {
        version: foamcraft_core::VERSION,
    }
}

#[tauri::command]
fn viewport_camera(action: CameraAction, state: tauri::State<'_, AppState>) -> Option<CameraPose> {
    let mut guard = state.0.lock().unwrap_or_else(|e| e.into_inner());
    let view = guard.as_mut()?;
    // Riding first person or chasing, dragging looks around (viewport_ride); the orbit camera
    // only moves in the overview.
    if let Some(f) = view.flight.as_mut().filter(|f| f.view() != "overview")
        && !matches!(action, CameraAction::Resize)
    {
        if let CameraAction::Orbit { yaw, pitch } = action {
            f.apply(ride::RideAction::Look {
                dx: f64::from(yaw),
                dy: f64::from(pitch),
            });
        }
        return Some(view.pose());
    }
    // Moving the camera by hand ends a running view change.
    if !matches!(action, CameraAction::Resize | CameraAction::View { .. }) {
        view.transition = None;
    }
    match action {
        CameraAction::Orbit { yaw, pitch } => view.camera.orbit(yaw, pitch),
        CameraAction::Pan { dx, dy } => view.camera.pan(dx, dy),
        CameraAction::Zoom { factor } => view.camera.zoom(factor),
        CameraAction::Fit => view.fit(),
        CameraAction::Resize => {}
        CameraAction::View { name } => {
            // The clock thread draws the move; the pose comes with `viewport://camera`.
            if !view.start_view(&name) {
                eprintln!("foamcraft: unknown view {name}");
            }
            return Some(view.pose());
        }
    }
    view.draw();
    Some(view.pose())
}

/// Shows the models of the open project (or only the axes) and fits the camera.
#[tauri::command(async)]
fn viewport_sync(api: tauri::State<'_, api::ApiState>, view: tauri::State<'_, AppState>) {
    let split = api.with_project(|p| p.and_then(foamcraft_api::scene::compare_scenes));
    let (built, right) = match split {
        Some((a, b)) => (a, Some(b)),
        None => (api.with_project(project_scene), None),
    };
    let particles = api.with_project(|p| p.and_then(foamcraft_api::particles::setup));
    // Speed of the moving lights and tufts on the surface (none when paused or off).
    let flow_speed = api.with_project(|p| {
        p.and_then(foamcraft_api::surface_flow::animation_speed)
            .map(|s| s as f32)
    });
    // The ride of the view (computed outside the view lock).
    let ride = api.with_project(|p| {
        p.filter(|p| p.manifest.view.ride.is_some())
            .map(foamcraft_api::ride::current)
    });
    let fit_key = api.with_project(|p| {
        p.map_or_else(String::new, |p| {
            let ids: Vec<_> = p.manifest.geometry.iter().map(|g| g.id.as_str()).collect();
            format!("{}|{}", p.root.display(), ids.join(","))
        })
    });
    let mut guard = view.0.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(view) = guard.as_mut() {
        let inset = view.renderer.upload(&built.scene);
        view.flow_speed = flow_speed;
        view.set_scene(built);
        view.compare = right.map(|b| view.renderer.upload(&b.scene));
        if let Some(setup) = particles {
            view.gpu.particles = Some(foamcraft_api::particles::create_gpu(&view.renderer, &setup));
            view.particle_dt = setup.dt;
            view.particles_paused = setup.paused;
            view.particle_yaw = setup.settings.yaw_deg;
        }
        let old = view.flight.take();
        match ride {
            Some(Ok(ride)) => {
                let flight = match old {
                    // The same ride with other settings: stay at the same place on the path.
                    Some(mut f)
                        if f.ride.spec.start == ride.spec.start
                            && f.ride.spec.run == ride.spec.run =>
                    {
                        let s = f.ride.timing.s_at(&f.ride.path, f.t);
                        f.t = ride.timing.t_at(&ride.path, s);
                        f.ride = ride;
                        f.inset = inset;
                        f
                    }
                    Some(f) => ride::Flight::new(ride, inset, f.before),
                    None => ride::Flight::new(ride, inset, view.camera),
                };
                if flight.view() == "overview" && view.camera == flight.before {
                    view.camera = flight.before;
                }
                view.flight = Some(flight);
                view.ride_meshes();
            }
            Some(Err(e)) => eprintln!("foamcraft: ride: {e}"),
            None => {
                // Leaving the ride: move back to the view from before.
                if let Some(f) = old {
                    let from = f
                        .camera()
                        .map_or(view.camera, |c| c.as_orbit(f.before.distance));
                    view.transition = Some((
                        CameraTransition::new(from, f.before, ride::ENTRY_SECONDS),
                        std::time::Instant::now(),
                    ));
                    view.camera = from;
                }
            }
        }
        // Moving or turning a model keeps the camera; a new project or model fits again.
        if view.fit_key != fit_key {
            view.fit_key = fit_key;
            view.fit();
        }
        view.draw();
    }
}

/// Face key under a pixel of the view (physical pixels), or null.
#[tauri::command]
fn viewport_pick(x: f32, y: f32, view: tauri::State<'_, AppState>) -> Option<String> {
    let guard = view.0.lock().unwrap_or_else(|e| e.into_inner());
    let view = guard.as_ref()?;
    let id = view.pick(x, y)?;
    view.pick_keys.get(id as usize - 1).cloned()
}

#[derive(Serialize)]
struct PickPoint {
    key: String,
    /// World point under the pixel (metres), when the ray hits the picked object.
    point: Option<[f64; 3]>,
}

/// Face or cap under a pixel and the 3D point there.
#[tauri::command]
fn viewport_pick_point(x: f32, y: f32, view: tauri::State<'_, AppState>) -> Option<PickPoint> {
    let guard = view.0.lock().unwrap_or_else(|e| e.into_inner());
    let view = guard.as_ref()?;
    let id = view.pick(x, y)?;
    let key = view.pick_keys.get(id as usize - 1)?.clone();
    let (w, h) = view.viewport.size();
    let aspect = w as f32 / h.max(1) as f32;
    let ndc_x = 2.0 * x / w.max(1) as f32 - 1.0;
    let ndc_y = 1.0 - 2.0 * y / h.max(1) as f32;
    let (origin, dir) = view.camera.ray(aspect, ndc_x, ndc_y);
    let owner = view
        .hit_meshes
        .iter()
        .find(|(k, _)| *k == key || key.starts_with(&format!("{k}:")));
    let point = owner.and_then(|(_, mesh)| {
        let o = [origin.x, origin.y, origin.z].map(f64::from);
        let d = [dir.x, dir.y, dir.z].map(f64::from);
        let hit = foamcraft_geometry::raycast(mesh, o, d)?;
        Some([0, 1, 2].map(|k| o[k] + d[k] * hit.distance))
    });
    Some(PickPoint { key, point })
}

/// Highlights the face under the mouse; returns its key.
#[tauri::command]
fn viewport_hover(x: f32, y: f32, view: tauri::State<'_, AppState>) -> Option<String> {
    let mut guard = view.0.lock().unwrap_or_else(|e| e.into_inner());
    let view = guard.as_mut()?;
    let id = view.pick(x, y);
    if id != view.hover {
        view.hover = id;
        view.highlight();
        view.draw();
    }
    id.and_then(|id| view.pick_keys.get(id as usize - 1).cloned())
}

/// Widths covered by the side panels (physical pixels); the split view of two runs uses the
/// rest of the window.
/// Shows the move and rotate handles of `model` at `pivot` (metres); none hides them.
#[tauri::command]
fn viewport_set_handles(
    model: Option<String>,
    pivot: Option<[f32; 3]>,
    view: tauri::State<'_, AppState>,
) {
    let mut guard = view.0.lock().unwrap_or_else(|e| e.into_inner());
    let Some(view) = guard.as_mut() else { return };
    view.handles = model.zip(pivot);
    view.show_handles(None, None);
    view.draw();
}

/// Shows or hides the wind arrow handle (Scenario step of a wind tunnel).
#[tauri::command]
fn viewport_show_wind(show: bool, view: tauri::State<'_, AppState>) {
    let mut guard = view.0.lock().unwrap_or_else(|e| e.into_inner());
    let Some(view) = guard.as_mut() else { return };
    if view.show_wind != show {
        view.show_wind = show;
        view.show_handles(None, None);
        view.draw();
    }
}

/// Starts dragging the handle `key` (`handle:move-x`, ...) at a pixel; false if none.
#[tauri::command]
fn viewport_handle_begin(key: String, x: f32, y: f32, view: tauri::State<'_, AppState>) -> bool {
    let mut guard = view.0.lock().unwrap_or_else(|e| e.into_inner());
    guard.as_mut().is_some_and(|v| v.begin_drag(&key, x, y))
}

/// Follows the mouse during a handle drag; returns the value to show.
#[tauri::command]
fn viewport_handle_drag(
    x: f32,
    y: f32,
    free: bool,
    view: tauri::State<'_, AppState>,
) -> Option<HandleValue> {
    let mut guard = view.0.lock().unwrap_or_else(|e| e.into_inner());
    guard.as_mut()?.drag_to(x, y, free)
}

/// Ends a handle drag; returns the command the interface runs (one undo step).
#[tauri::command]
fn viewport_handle_end(view: tauri::State<'_, AppState>) -> Option<HandleCommand> {
    let mut guard = view.0.lock().unwrap_or_else(|e| e.into_inner());
    guard.as_mut()?.end_drag()
}

/// Cancels a handle drag (Esc): the model goes back.
#[tauri::command]
fn viewport_handle_cancel(view: tauri::State<'_, AppState>) {
    let mut guard = view.0.lock().unwrap_or_else(|e| e.into_inner());
    let Some(view) = guard.as_mut() else { return };
    if let Some(drag) = view.drag.take() {
        view.reset_preview(&drag);
    }
}

#[tauri::command]
fn viewport_insets(left: f32, right: f32, view: tauri::State<'_, AppState>) {
    let mut guard = view.0.lock().unwrap_or_else(|e| e.into_inner());
    let Some(view) = guard.as_mut() else { return };
    let insets = (left.max(0.0) as u32, right.max(0.0) as u32);
    if insets != view.insets {
        view.insets = insets;
        if view.compare.is_some() {
            view.fit();
            view.draw();
        }
    }
}

/// Highlights the selected faces.
#[tauri::command]
fn viewport_set_selection(keys: Vec<String>, view: tauri::State<'_, AppState>) {
    let mut guard = view.0.lock().unwrap_or_else(|e| e.into_inner());
    let Some(view) = guard.as_mut() else { return };
    view.selected = keys
        .iter()
        .filter_map(|k| view.pick_keys.iter().position(|p| p == k))
        .map(|i| i as u32 + 1)
        .collect();
    view.highlight();
    view.draw();
}

/// Steers the playback of the ride (play, pause, jump, look around); answers its clock.
#[tauri::command]
fn viewport_ride(
    action: ride::RideAction,
    view: tauri::State<'_, AppState>,
) -> Option<ride::RideClock> {
    let mut guard = view.0.lock().unwrap_or_else(|e| e.into_inner());
    let view = guard.as_mut()?;
    let f = view.flight.as_mut()?;
    f.apply(action);
    let clock = f.clock();
    view.ride_meshes();
    view.draw();
    Some(clock)
}

/// A start point for a ride at a pixel: the particle under the mouse (within 12 pixels, in
/// front of the model) or, on the model, a point just off the surface on the side of the viewer.
#[tauri::command]
fn viewport_ride_point(x: f32, y: f32, view: tauri::State<'_, AppState>) -> Option<[f64; 3]> {
    let guard = view.0.lock().unwrap_or_else(|e| e.into_inner());
    let view = guard.as_ref()?;
    let (w, h) = view.viewport.size();
    let aspect = w as f32 / h.max(1) as f32;
    let ndc = (
        2.0 * x / w.max(1) as f32 - 1.0,
        1.0 - 2.0 * y / h.max(1) as f32,
    );
    let (origin, dir) = view.camera.ray(aspect, ndc.0, ndc.1);
    // The model under the pixel: its distance hides particles behind it.
    let hit = view.pick(x, y).and_then(|id| {
        let key = view.pick_keys.get(id as usize - 1)?;
        let (_, mesh) = view
            .hit_meshes
            .iter()
            .find(|(k, _)| k == key || key.starts_with(&format!("{k}:")))?;
        let o = origin.to_array().map(f64::from);
        let d = dir.to_array().map(f64::from);
        let hit = foamcraft_geometry::raycast(mesh, o, d)?;
        Some((hit, mesh))
    });
    if let Some(p) = view.gpu.particles.as_ref() {
        let positions = view.renderer.read_particles(p);
        let blocker = hit
            .as_ref()
            .map(|(h, _)| (h.distance as f32) + (origin - view.camera.eye()).length());
        if let Some(world) = ride::nearest_particle(
            &positions,
            view.particle_yaw,
            &view.camera,
            (w, h),
            (x, y),
            12.0,
            blocker,
        ) {
            return Some(world.to_array().map(f64::from));
        }
    }
    let (hit, mesh) = hit?;
    // Off the surface along the face normal, on the side the ray comes from.
    let t = mesh.triangles.get(hit.triangle)?;
    let c = t.map(|i| Vec3::from(mesh.positions[i as usize]));
    let mut n = (c[1] - c[0]).cross(c[2] - c[0]).normalize_or_zero();
    if n.dot(dir) > 0.0 {
        n = -n;
    }
    let offset = 0.02 * view.bounds.1;
    let p = Vec3::from(hit.point.map(|v| v as f32)) + n * offset;
    Some(p.to_array().map(f64::from))
}

fn create_view(window: &tauri::WebviewWindow) -> Option<ViewState> {
    let instance = default_instance();
    let surface = instance.create_surface(window.clone()).ok()?;
    let renderer = Renderer::new(instance, Some(&surface))?;
    let size = window.inner_size().ok()?;
    let viewport = Viewport::new(&renderer, surface, size.width, size.height)?;
    let scene = Scene::empty();
    let gpu = renderer.upload(&scene);
    let mut view = ViewState {
        renderer,
        viewport,
        gpu,
        bounds: scene.bounding_sphere(),
        camera: OrbitCamera::default(),
        pick_keys: Vec::new(),
        hover: None,
        selected: Vec::new(),
        hit_meshes: Vec::new(),
        particle_dt: 0.0,
        particles_paused: true,
        frame_stats: (0, 0.0),
        compare: None,
        insets: (0, 0),
        transition: None,
        models: Vec::new(),
        scene_keys: 0,
        handles: None,
        drag: None,
        wind: None,
        tunnel_meshes: Vec::new(),
        show_wind: false,
        slices: Vec::new(),
        fit_key: String::new(),
        flight: None,
        particle_yaw: 0.0,
        flow_speed: None,
        flow_time: 0.0,
    };
    view.fit();
    Some(view)
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(AppState(Mutex::new(None)))
        .setup(|app| {
            // Animation clock: advances the particles about 60 times a second.
            let handle = app.handle().clone();
            std::thread::spawn(move || {
                loop {
                    std::thread::sleep(std::time::Duration::from_millis(16));
                    let state = handle.state::<AppState>();
                    let mut guard = state.0.lock().unwrap_or_else(|e| e.into_inner());
                    let Some(view) = guard.as_mut() else { continue };
                    if let Some(f) = view.flight.as_mut() {
                        let (flow, changed) = f.tick();
                        let clock = f.clock();
                        if !changed {
                            continue;
                        }
                        // The particles move with the rider's flow time, in steps of at most
                        // the usual particle step.
                        if flow > 0.0 && !view.particles_paused {
                            let base = f64::from(view.particle_dt).max(1e-12);
                            let steps = (flow / base).ceil().clamp(1.0, 32.0) as u32;
                            if let Some(p) = view.gpu.particles.as_mut() {
                                view.renderer.step_particles(
                                    p,
                                    (flow / f64::from(steps)) as f32,
                                    steps,
                                );
                            }
                        }
                        let start = std::time::Instant::now();
                        view.ride_meshes();
                        view.draw();
                        let (frames, secs) = &mut view.frame_stats;
                        *frames += 1;
                        *secs += start.elapsed().as_secs_f64();
                        if *frames == 600 {
                            eprintln!(
                                "foamcraft: ride frame {:.2} ms on average",
                                *secs / f64::from(*frames) * 1000.0
                            );
                            *frames = 0;
                            *secs = 0.0;
                        }
                        let _ = handle.emit("viewport://ride", clock);
                        continue;
                    }
                    if view.step_transition() {
                        let pose = view.pose();
                        let animating = !view.particles_paused && view.gpu.particles.is_some();
                        if !animating {
                            view.draw();
                        }
                        let _ = handle.emit("viewport://camera", pose);
                        if !animating {
                            continue;
                        }
                    }
                    let particles = !view.particles_paused && view.gpu.particles.is_some();
                    if !particles && view.flow_speed.is_none() {
                        continue;
                    }
                    let start = std::time::Instant::now();
                    // Lights on the surface lines and fluttering tufts (Task_33).
                    if let Some(speed) = view.flow_speed {
                        view.flow_time += speed / 60.0;
                        view.gpu.time = view.flow_time;
                    }
                    let dt = view.particle_dt;
                    if let Some(p) = view.gpu.particles.as_mut().filter(|_| particles) {
                        view.renderer.step_particles(p, dt, 1);
                    }
                    view.draw();
                    let (frames, secs) = &mut view.frame_stats;
                    *frames += 1;
                    *secs += start.elapsed().as_secs_f64();
                    if *frames == 600 {
                        eprintln!(
                            "foamcraft: particle frame {:.2} ms on average",
                            *secs / f64::from(*frames) * 1000.0
                        );
                        *frames = 0;
                        *secs = 0.0;
                    }
                }
            });
            app.manage(api::ApiState::new(app.handle()));
            if let Some(window) = app.get_webview_window("main") {
                let view = create_view(&window);
                if view.is_none() {
                    eprintln!("foamcraft: no GPU viewport available");
                }
                let state = app.state::<AppState>();
                let mut guard = state.0.lock().unwrap_or_else(|e| e.into_inner());
                *guard = view;
                if let Some(view) = guard.as_mut() {
                    view.draw();
                }
            }
            Ok(())
        })
        .on_window_event(|window, event| {
            if let WindowEvent::Resized(size) = event {
                let state = window.state::<AppState>();
                let mut guard = state.0.lock().unwrap_or_else(|e| e.into_inner());
                if let Some(view) = guard.as_mut() {
                    view.viewport
                        .resize(&view.renderer, size.width, size.height);
                    view.draw();
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            app_info,
            cli_path,
            viewport_camera,
            viewport_sync,
            viewport_pick,
            viewport_pick_point,
            viewport_hover,
            viewport_set_selection,
            viewport_insets,
            viewport_set_handles,
            viewport_show_wind,
            viewport_handle_begin,
            viewport_handle_drag,
            viewport_handle_end,
            viewport_handle_cancel,
            viewport_ride,
            viewport_ride_point,
            api::api_call
        ])
        .run(tauri::generate_context!())
        .unwrap_or_else(|err| {
            eprintln!("foamcraft: {err}");
            std::process::exit(1);
        });
}
