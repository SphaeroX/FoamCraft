// SPDX-License-Identifier: GPL-3.0-or-later
//! Image and video export: `export.image` (PNG up to 8K, optional transparent background and
//! legend), `export.video` (Motion JPEG AVI, rendered frame by frame offscreen in a background
//! job with progress and cancel), `export.status`, `export.cancel` and `export.preview` (one
//! frame as PNG data for the export dialog). The timeline couples the camera path and the
//! particle animation; everything depends only on the settings, so exports repeat exactly.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, Mutex, OnceLock};

use base64::Engine as _;
use foamcraft_core::command::EventBus;
use foamcraft_core::command::{Options, Registry};
use foamcraft_core::error::FoamError;
use foamcraft_core::project::Project;
use foamcraft_render::camera::OrbitCamera;
use foamcraft_render::camera_path::{CameraKey, interpolate, preset};
use foamcraft_render::video::AviWriter;
use foamcraft_render::{GpuScene, Renderer, Scene};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

/// Resolution presets.
fn resolution(name: &str) -> Option<(u32, u32)> {
    Some(match name {
        "hd" | "720p" => (1280, 720),
        "full_hd" | "1080p" => (1920, 1080),
        "qhd" | "1440p" => (2560, 1440),
        "4k" | "2160p" => (3840, 2160),
        _ => return None,
    })
}

fn size(res: &Option<String>, w: Option<u32>, h: Option<u32>) -> Result<(u32, u32), FoamError> {
    let (dw, dh) = match res {
        Some(r) => resolution(r)
            .ok_or_else(|| FoamError::new("export.invalid", format!("resolution {r}")))?,
        None => (1920, 1080),
    };
    Ok((
        w.unwrap_or(dw).clamp(16, 8192),
        h.unwrap_or(dh).clamp(16, 8192),
    ))
}

/// Camera of an export: a prepared path or own key frames; default the turntable.
#[derive(Debug, Clone, Default, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CameraSpec {
    /// `turntable`, `swing`, `rise`, `approach`, `still` or `ride` (the stored ride of the
    /// view, from start to end unless `duration_s` is given).
    #[serde(default)]
    pub preset: Option<String>,
    /// Own key frames (time in seconds, angles in radians, metres); they win over `preset`.
    #[serde(default)]
    pub keys: Option<Vec<CameraKey>>,
}

/// The scene with what the video needs from it.
struct Prepared {
    /// A copy of the project for the ride (the export runs in a background thread).
    project: Project,
    scene: Scene,
    /// Lines and tufts on the surface (Task_33).
    flow: Option<foamcraft_render::flow::FlowGeometry>,
    base: OrbitCamera,
    particles: Option<crate::particles::ParticleSetup>,
    legend: Option<(f64, f64, String)>,
}

fn prepare(project: &Project, aspect: f32) -> Prepared {
    let built = crate::scene::project_scene(Some(project));
    let mut base = OrbitCamera::default();
    let (centre, radius) = built.focus.unwrap_or_else(|| built.scene.bounding_sphere());
    base.fit(centre, radius * 1.1, aspect);
    Prepared {
        project: project.clone(),
        flow: built.flow_geometry(),
        scene: built.scene,
        base,
        particles: crate::particles::setup(project),
        legend: crate::scene::legend(project),
    }
}

fn is_ride(spec: &CameraSpec) -> bool {
    spec.keys.is_none() && spec.preset.as_deref() == Some("ride")
}

fn path_of(
    spec: &CameraSpec,
    base: &OrbitCamera,
    duration: f32,
) -> Result<Vec<CameraKey>, FoamError> {
    if let Some(keys) = &spec.keys {
        let mut keys = keys.clone();
        keys.sort_by(|a, b| a.t.total_cmp(&b.t));
        return Ok(keys);
    }
    match spec.preset.as_deref().unwrap_or("turntable") {
        "still" | "ride" => Ok(vec![CameraKey::from_camera(0.0, base)]),
        name => preset(name, base, duration)
            .ok_or_else(|| FoamError::new("export.invalid", format!("camera {name}"))),
    }
}

/// The prepared scene on the GPU with its surface flow (at time 0).
fn prepared_gpu(renderer: &Renderer, p: &Prepared) -> GpuScene {
    let mut gpu = renderer.upload(&p.scene);
    renderer.set_surface_flow(&mut gpu, p.flow.as_ref());
    gpu
}

fn add_legend(pixels: &mut [u8], w: u32, h: u32, legend: &Option<(f64, f64, String)>) {
    if let Some((lo, hi, unit)) = legend {
        let colors: Vec<[u8; 3]> = (0..64)
            .map(|i| {
                foamcraft_sim::results::colormap(f64::from(i) / 63.0)
                    .map(|c| (c.clamp(0.0, 1.0) * 255.0).round() as u8)
            })
            .collect();
        foamcraft_render::overlay::draw_legend(pixels, w, h, &colors, *lo, *hi, unit);
    }
}

fn no_gpu() -> FoamError {
    FoamError::new("view.no_gpu", "")
}

fn write_failed(e: impl ToString) -> FoamError {
    FoamError::new("view.write_failed", e.to_string())
}

/// One frame of the timeline: camera at `t`, particles advanced `steps` frames.
struct Timeline {
    renderer: Renderer,
    gpu: GpuScene,
    keys: Vec<CameraKey>,
    base: OrbitCamera,
    dt: f32,
    fps: f32,
    project: Project,
    /// The ride and the uploaded scene of its overview inset.
    ride: Option<(crate::ride::Ride, GpuScene)>,
}

impl Timeline {
    fn new(p: &Prepared, keys: Vec<CameraKey>, fps: f32) -> Result<Self, FoamError> {
        let renderer = Renderer::new_headless().ok_or_else(no_gpu)?;
        let mut gpu = renderer.upload(&p.scene);
        renderer.set_surface_flow(&mut gpu, p.flow.as_ref());
        let mut dt = 0.0;
        if let Some(setup) = &p.particles {
            gpu.particles = Some(crate::particles::create_gpu(&renderer, setup));
            // The app advances `setup.dt` per frame at 60 frames per second; keep that speed.
            dt = if setup.paused {
                0.0
            } else {
                setup.dt * 60.0 / fps
            };
        }
        Ok(Self {
            renderer,
            gpu,
            keys,
            base: p.base,
            dt,
            fps,
            project: p.project.clone(),
            ride: None,
        })
    }

    /// Frames come from the ride: its camera, and the particles move with the rider's flow time.
    fn with_ride(mut self, ride: crate::ride::Ride, scene: &Scene) -> Self {
        let inset = self.renderer.upload(scene);
        self.ride = Some((ride, inset));
        self
    }

    fn frame(&mut self, t: f32, w: u32, h: u32) -> Vec<u8> {
        // The surface flow moves with the timeline: the same time gives the same frame.
        self.gpu.time = crate::surface_flow::time_at(&self.project, f64::from(t));
        if let Some((ride, inset)) = self.ride.as_mut() {
            // Advance the particles by the flow time of this frame, in steps of at most the
            // usual particle step.
            if self.dt > 0.0
                && let Some(p) = self.gpu.particles.as_mut()
            {
                let flow = ride.flow_dt(f64::from(t), 1.0 / f64::from(self.fps)) as f32;
                let base = self.dt * self.fps / 60.0;
                let steps = (flow / base.max(1e-12)).ceil().clamp(1.0, 64.0) as u32;
                if flow > 0.0 {
                    self.renderer.step_particles(p, flow / steps as f32, steps);
                }
            }
            return crate::ride::draw_frame(
                &self.renderer,
                &mut self.gpu,
                inset,
                ride,
                &self.base,
                f64::from(t),
                w,
                h,
            );
        }
        if self.dt > 0.0
            && let Some(p) = self.gpu.particles.as_mut()
        {
            self.renderer.step_particles(p, self.dt, 1);
        }
        let camera = interpolate(&self.keys, t, &self.base);
        self.renderer.render_offscreen_gpu(&self.gpu, &camera, w, h)
    }
}

/// A PNG of the view: the fitted standard view turned by `yaw` (radians) and, when given, at
/// `pitch` (radians), with the colour legend.
pub fn render_view_png(
    project: &Project,
    w: u32,
    h: u32,
    yaw: f32,
    pitch: Option<f32>,
) -> Result<Vec<u8>, FoamError> {
    let prepared = prepare(project, w as f32 / h as f32);
    let renderer = Renderer::new_headless().ok_or_else(no_gpu)?;
    let mut camera = prepared.base;
    camera.yaw += yaw;
    if let Some(p) = pitch {
        camera.pitch = p;
    }
    let mut pixels =
        renderer.render_offscreen_gpu(&prepared_gpu(&renderer, &prepared), &camera, w, h);
    add_legend(&mut pixels, w, h, &prepared.legend);
    let mut png = Vec::new();
    {
        let mut enc = png::Encoder::new(&mut png, w, h);
        enc.set_color(png::ColorType::Rgba);
        enc.set_depth(png::BitDepth::Eight);
        let mut writer = enc.write_header().map_err(write_failed)?;
        writer.write_image_data(&pixels).map_err(write_failed)?;
    }
    Ok(png)
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ImageParams {
    /// PNG file to write.
    pub path: PathBuf,
    /// `hd`, `full_hd`, `qhd` or `4k`; `width` and `height` override it. Default full HD.
    #[serde(default)]
    pub resolution: Option<String>,
    #[serde(default)]
    pub width: Option<u32>,
    #[serde(default)]
    pub height: Option<u32>,
    /// Background fully transparent (for slides and documents).
    #[serde(default)]
    pub transparent: bool,
    /// Colour legend at the bottom right (when a result colours the model). Default on.
    #[serde(default = "yes")]
    pub legend: bool,
    /// Camera; default the fitted standard view.
    #[serde(default)]
    pub camera: Option<CameraKey>,
    /// Screen time (s) of the stored ride: a still from the ride instead of the camera.
    #[serde(default)]
    pub ride_t: Option<f64>,
    /// Long exposure (s of animation, 0.1 to 30): the particles run this long at 60 steps per
    /// second and leave their whole path as light. Needs particles in the view.
    #[serde(default)]
    pub exposure_s: Option<f64>,
}

fn yes() -> bool {
    true
}

fn image(project: &Project, p: ImageParams) -> Result<Value, FoamError> {
    let (w, h) = size(&p.resolution, p.width, p.height)?;
    let mut prepared = prepare(project, w as f32 / h as f32);
    if p.transparent {
        prepared.scene.background[3] = 0.0;
    }
    let renderer = Renderer::new_headless().ok_or_else(no_gpu)?;
    let camera = p
        .camera
        .map_or(prepared.base, |k| interpolate(&[k], 0.0, &prepared.base));
    let mut pixels = match (p.ride_t, p.exposure_s) {
        (Some(_), Some(_)) => {
            return Err(FoamError::new(
                "export.invalid",
                "exposure_s and ride_t together",
            ));
        }
        (Some(t), None) => crate::ride::render_still(project, &renderer, w, h, t, p.transparent)?,
        (None, Some(seconds)) => {
            if !(0.1..=30.0).contains(&seconds) {
                return Err(FoamError::new(
                    "export.invalid",
                    format!("exposure_s {seconds}"),
                ));
            }
            let setup = prepared
                .particles
                .as_ref()
                .filter(|s| s.heavy.is_none())
                .ok_or_else(|| {
                    FoamError::new(
                        "export.invalid",
                        "a long exposure needs massless particles in the view",
                    )
                })?;
            let mut gpu = prepared_gpu(&renderer, &prepared);
            let mut particles = renderer.create_trail_particles(
                &setup.levels,
                &setup.seeds,
                &setup.settings,
                &crate::particles::exposure_trails(setup),
            );
            renderer.set_particle_clip(&mut particles, setup.clip);
            // Spread the particles over their way first: otherwise all of them start on the
            // seed area and the picture is one bright plane.
            let warm = (setup.settings.max_age / setup.dt.max(1e-12))
                .ceil()
                .min(3000.0) as u32;
            renderer.step_particles(&mut particles, setup.dt, warm);
            gpu.particles = Some(particles);
            let steps = (seconds * 60.0).round() as u32;
            renderer.render_offscreen_exposure(&mut gpu, &camera, w, h, steps, setup.dt)
        }
        (None, None) => {
            renderer.render_offscreen_gpu(&prepared_gpu(&renderer, &prepared), &camera, w, h)
        }
    };
    if p.legend {
        add_legend(&mut pixels, w, h, &prepared.legend);
    }
    foamcraft_render::write_png(&p.path, w, h, &pixels).map_err(write_failed)?;
    Ok(json!({"path": p.path, "width": w, "height": h}))
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct VideoParams {
    /// AVI file to write (Motion JPEG).
    pub path: PathBuf,
    #[serde(default)]
    pub resolution: Option<String>,
    #[serde(default)]
    pub width: Option<u32>,
    #[serde(default)]
    pub height: Option<u32>,
    /// Frames per second, default 30.
    #[serde(default)]
    pub fps: Option<u32>,
    /// Length in seconds, default 8.
    #[serde(default)]
    pub duration_s: Option<f32>,
    #[serde(default)]
    pub camera: CameraSpec,
    #[serde(default = "yes")]
    pub legend: bool,
    /// JPEG quality 1..100, default 90.
    #[serde(default)]
    pub quality: Option<u8>,
    /// Block until the video is written (otherwise answer at once with `export_id`).
    #[serde(default)]
    pub wait: bool,
}

#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct ExportStatus {
    pub export_id: String,
    /// `running`, `done`, `failed` or `cancelled`.
    pub state: String,
    pub frame: u32,
    pub frames: u32,
    pub path: PathBuf,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

struct Export {
    cancel: AtomicBool,
    frame: AtomicU32,
    status: Mutex<ExportStatus>,
}

fn exports() -> &'static Mutex<HashMap<String, Arc<Export>>> {
    static EXPORTS: OnceLock<Mutex<HashMap<String, Arc<Export>>>> = OnceLock::new();
    EXPORTS.get_or_init(Default::default)
}

fn status(id: &str) -> Result<ExportStatus, FoamError> {
    let e = exports()
        .lock()
        .ok()
        .and_then(|m| m.get(id).cloned())
        .ok_or_else(|| FoamError::new("export.not_found", id))?;
    let mut s = e
        .status
        .lock()
        .map_or_else(|p| p.into_inner().clone(), |s| s.clone());
    s.frame = e.frame.load(Ordering::SeqCst);
    Ok(s)
}

/// Renders and encodes all frames; removes the file when cancelled or failed.
fn encode(
    export: &Export,
    prepared: &Prepared,
    keys: Vec<CameraKey>,
    p: &VideoParams,
    (w, h, fps, frames): (u32, u32, u32, u32),
    events: &EventBus,
    id: &str,
) -> Result<(), FoamError> {
    let mut timeline = Timeline::new(prepared, keys, fps as f32)?;
    if is_ride(&p.camera) {
        let ride = crate::ride::current(&prepared.project)?;
        timeline = timeline.with_ride(ride, &prepared.scene);
    }
    let mut writer =
        AviWriter::create(&p.path, w, h, fps, p.quality.unwrap_or(90)).map_err(write_failed)?;
    for i in 0..frames {
        if export.cancel.load(Ordering::SeqCst) {
            drop(writer);
            let _ = std::fs::remove_file(&p.path);
            return Err(FoamError::new("export.cancelled", ""));
        }
        let mut pixels = timeline.frame(i as f32 / fps as f32, w, h);
        if p.legend {
            add_legend(&mut pixels, w, h, &prepared.legend);
        }
        if let Err(e) = writer.push_rgba(&pixels) {
            drop(writer);
            let _ = std::fs::remove_file(&p.path);
            return Err(write_failed(e));
        }
        export.frame.store(i + 1, Ordering::SeqCst);
        if i % 10 == 0 || i + 1 == frames {
            events.emit(
                "export.progress",
                json!({"export_id": id, "frame": i + 1, "frames": frames}),
            );
        }
    }
    writer.finish().map_err(|e| {
        let _ = std::fs::remove_file(&p.path);
        write_failed(e)
    })
}

fn video(project: &Project, events: &EventBus, p: VideoParams) -> Result<Value, FoamError> {
    let (w, h) = size(&p.resolution, p.width, p.height)?;
    // Motion JPEG works on 16 × 8 blocks; even sizes keep every player happy.
    let (w, h) = (w & !1, h & !1);
    let fps = p.fps.unwrap_or(30).clamp(1, 120);
    let ride_length = if is_ride(&p.camera) {
        Some(crate::ride::current(project)?.duration() as f32)
    } else {
        None
    };
    let duration = p
        .duration_s
        .or(ride_length)
        .unwrap_or(8.0)
        .clamp(0.1, 600.0);
    let frames = ((duration * fps as f32).round() as u32).max(1);
    let prepared = prepare(project, w as f32 / h as f32);
    let keys = path_of(&p.camera, &prepared.base, duration)?;
    static NEXT: AtomicU32 = AtomicU32::new(1);
    let id = format!(
        "export-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::SeqCst)
    );
    let export = Arc::new(Export {
        cancel: AtomicBool::new(false),
        frame: AtomicU32::new(0),
        status: Mutex::new(ExportStatus {
            export_id: id.clone(),
            state: "running".into(),
            frame: 0,
            frames,
            path: p.path.clone(),
            error: None,
        }),
    });
    if let Ok(mut m) = exports().lock() {
        m.insert(id.clone(), export.clone());
    }
    let wait = p.wait;
    let events = events.clone();
    let run_id = id.clone();
    let job = move || {
        let result = encode(
            &export,
            &prepared,
            keys,
            &p,
            (w, h, fps, frames),
            &events,
            &run_id,
        );
        let mut s = export.status.lock().unwrap_or_else(|e| e.into_inner());
        match &result {
            Ok(()) => s.state = "done".into(),
            Err(e) if e.code == "export.cancelled" => s.state = "cancelled".into(),
            Err(e) => {
                s.state = "failed".into();
                s.error = Some(e.code.clone());
            }
        }
        events.emit(
            "export.finished",
            json!({"export_id": run_id, "state": s.state}),
        );
        result
    };
    if wait {
        job()?;
        return Ok(serde_json::to_value(status(&id)?).unwrap_or_default());
    }
    std::thread::spawn(job);
    Ok(json!({"export_id": id, "path": status(&id)?.path, "frames": frames}))
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct IdParams {
    pub export_id: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PreviewParams {
    #[serde(default)]
    pub width: Option<u32>,
    #[serde(default)]
    pub height: Option<u32>,
    /// Time on the timeline, seconds.
    #[serde(default)]
    pub time_s: f32,
    /// Length of the video the camera path is made for.
    #[serde(default)]
    pub duration_s: Option<f32>,
    #[serde(default)]
    pub camera: CameraSpec,
    #[serde(default = "yes")]
    pub legend: bool,
}

/// One frame of the timeline as PNG data (particles not advanced).
fn preview(project: &Project, p: PreviewParams) -> Result<Value, FoamError> {
    let (w, h) = (
        p.width.unwrap_or(480).clamp(16, 1920),
        p.height.unwrap_or(270).clamp(16, 1080),
    );
    let prepared = prepare(project, w as f32 / h as f32);
    let duration = p.duration_s.unwrap_or(8.0).clamp(0.1, 600.0);
    let keys = path_of(&p.camera, &prepared.base, duration)?;
    let renderer = Renderer::new_headless().ok_or_else(no_gpu)?;
    let camera = interpolate(&keys, p.time_s, &prepared.base);
    let mut pixels =
        renderer.render_offscreen_gpu(&prepared_gpu(&renderer, &prepared), &camera, w, h);
    if p.legend {
        add_legend(&mut pixels, w, h, &prepared.legend);
    }
    let mut png = Vec::new();
    {
        let mut enc = png::Encoder::new(&mut png, w, h);
        enc.set_color(png::ColorType::Rgba);
        enc.set_depth(png::BitDepth::Eight);
        let mut writer = enc.write_header().map_err(write_failed)?;
        writer.write_image_data(&pixels).map_err(write_failed)?;
    }
    Ok(json!({
        "width": w,
        "height": h,
        "png_base64": base64::engine::general_purpose::STANDARD.encode(png),
    }))
}

pub fn register(r: &mut Registry) {
    r.add(
        "export.image",
        "Render the 3D view into a PNG (up to 8K), optionally with transparent background and the \
         colour legend.",
        Options {
            undoable: false,
            errors: &[
                "project.none_open",
                "view.no_gpu",
                "view.write_failed",
                "export.invalid",
                "ride.none",
                "ride.no_path",
            ],
            example: json!({"path": "C:/Temp/view.png", "resolution": "4k", "transparent": true}),
        },
        |s, p: ImageParams| image(s.project()?, p),
    );
    r.add(
        "export.video",
        "Render a video (Motion JPEG AVI) frame by frame: camera path (turntable, swing, rise, \
         approach, still, ride or own key frames) and particle animation on one timeline. Answers at \
         once with export_id unless wait is true.",
        Options {
            undoable: false,
            errors: &[
                "project.none_open",
                "view.no_gpu",
                "view.write_failed",
                "export.invalid",
                "ride.none",
                "ride.no_path",
            ],
            example: json!({"path": "C:/Temp/turntable.avi", "resolution": "full_hd", "fps": 30,
                            "duration_s": 8.0, "camera": {"preset": "turntable"}}),
        },
        |s, p: VideoParams| {
            let events = s.events().clone();
            video(s.project()?, &events, p)
        },
    );
    r.add(
        "export.status",
        "Progress of a video export: state, frames written, total frames.",
        Options {
            undoable: false,
            errors: &["export.not_found"],
            example: json!({"export_id": "export-1-1"}),
        },
        |_, p: IdParams| status(&p.export_id),
    );
    r.add(
        "export.cancel",
        "Cancel a video export; the incomplete file is removed.",
        Options {
            undoable: false,
            errors: &["export.not_found"],
            example: json!({"export_id": "export-1-1"}),
        },
        |_, p: IdParams| {
            let e = exports()
                .lock()
                .ok()
                .and_then(|m| m.get(&p.export_id).cloned())
                .ok_or_else(|| FoamError::new("export.not_found", p.export_id.clone()))?;
            e.cancel.store(true, Ordering::SeqCst);
            Ok(json!({"export_id": p.export_id}))
        },
    );
    r.add(
        "export.preview",
        "One frame of a video timeline as PNG data (base64), for the export dialog.",
        Options {
            undoable: false,
            errors: &["project.none_open", "view.no_gpu", "export.invalid"],
            example: json!({"time_s": 2.0, "duration_s": 8.0, "camera": {"preset": "turntable"}}),
        },
        |s, p: PreviewParams| preview(s.project()?, p),
    );
}
