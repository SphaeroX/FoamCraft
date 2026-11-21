// SPDX-License-Identifier: GPL-3.0-or-later
//! FoamCraft desktop app: a Tauri window with the native wgpu viewport beneath a transparent
//! web view.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod api;

use std::sync::Mutex;

use foamcraft_api::scene::{ProjectScene, project_scene};
use foamcraft_render::camera::OrbitCamera;
use foamcraft_render::{GpuScene, Renderer, Scene, Viewport, default_instance};
use serde::{Deserialize, Serialize};
use tauri::{Manager, WindowEvent};

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
}

impl ViewState {
    /// `focus` is the sphere "fit view" frames (the models); default: the whole scene.
    fn set_scene(&mut self, built: ProjectScene) {
        let ProjectScene {
            scene,
            keys: pick_keys,
            focus,
            hit_meshes,
        } = built;
        self.gpu = self.renderer.upload(&scene);
        self.bounds = focus.unwrap_or_else(|| scene.bounding_sphere());
        self.pick_keys = pick_keys;
        self.hit_meshes = hit_meshes;
        self.hover = None;
        self.selected.clear();
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
        let (center, radius) = self.bounds;
        let (w, h) = self.viewport.size();
        // Each half of a split view is half as wide as the free area.
        let w = if self.compare.is_some() {
            self.span().1 / 2
        } else {
            w
        };
        // A little margin so the model does not touch the window edges.
        self.camera
            .fit(center, radius * 1.1, w as f32 / h.max(1) as f32);
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

    fn draw(&mut self) {
        let span = self.span();
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
    Orbit { yaw: f32, pitch: f32 },
    Pan { dx: f32, dy: f32 },
    Zoom { factor: f32 },
    Fit,
    Resize,
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
fn viewport_camera(action: CameraAction, state: tauri::State<'_, AppState>) {
    let mut guard = state.0.lock().unwrap_or_else(|e| e.into_inner());
    let Some(view) = guard.as_mut() else { return };
    match action {
        CameraAction::Orbit { yaw, pitch } => view.camera.orbit(yaw, pitch),
        CameraAction::Pan { dx, dy } => view.camera.pan(dx, dy),
        CameraAction::Zoom { factor } => view.camera.zoom(factor),
        CameraAction::Fit => view.fit(),
        CameraAction::Resize => {}
    }
    view.draw();
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
    let mut guard = view.0.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(view) = guard.as_mut() {
        view.set_scene(built);
        view.compare = right.map(|b| view.renderer.upload(&b.scene));
        if let Some(setup) = particles {
            let gpu = view
                .renderer
                .create_particles(&setup.levels, &setup.seeds, &setup.settings);
            view.gpu.particles = Some(gpu);
            view.particle_dt = setup.dt;
            view.particles_paused = setup.paused;
        }
        view.fit();
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
                    if view.particles_paused || view.gpu.particles.is_none() {
                        continue;
                    }
                    let start = std::time::Instant::now();
                    let dt = view.particle_dt;
                    if let Some(p) = view.gpu.particles.as_mut() {
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
            api::api_call
        ])
        .run(tauri::generate_context!())
        .unwrap_or_else(|err| {
            eprintln!("foamcraft: {err}");
            std::process::exit(1);
        });
}
