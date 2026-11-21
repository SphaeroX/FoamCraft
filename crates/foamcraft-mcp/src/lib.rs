// SPDX-License-Identifier: GPL-3.0-or-later
//! The MCP server of FoamCraft. [`Server`] holds the curated tools on top of the command API and
//! runs them either in its own session (no window, offscreen rendering) or in the running app
//! through the bridge ([`foamcraft_api::bridge`]), where the user follows every step.
//! [`McpHandler`] adapts it to the MCP protocol (`rmcp`, stdio transport).

mod handler;
mod texts;

use std::path::{Component, Path, PathBuf};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use foamcraft_api::bridge::BridgeClient;
use foamcraft_core::command::{Registry, Session};
use foamcraft_core::error::{FoamError, Language, message};
use serde_json::{Value, json};

pub use handler::McpHandler;
pub use texts::{GUIDE, GUIDE_URI, INSTRUCTIONS};

/// One part of a tool answer.
#[derive(Debug, Clone)]
pub enum Output {
    Json(Value),
    Png(Vec<u8>),
}

#[derive(Debug, Clone)]
pub struct ToolDef {
    pub name: String,
    pub description: String,
    pub input_schema: Value,
}

#[allow(clippy::large_enum_variant)] // one value per server
enum Backend {
    Local {
        registry: Registry,
        session: Mutex<Session>,
    },
    App(BridgeClient),
}

pub struct Server {
    backend: Backend,
    /// Command descriptions and parameter schemas (the same build as the app).
    specs: Registry,
    roots: Vec<PathBuf>,
    data_dir: PathBuf,
}

/// Curated tools: tool name, command, description for an agent.
const CURATED: &[(&str, &str, &str)] = &[
    (
        "project_new",
        "project.new",
        "Create a FoamCraft project in an empty folder (inside the allowed roots) and open it.",
    ),
    (
        "project_open",
        "project.open",
        "Open an existing FoamCraft project folder.",
    ),
    (
        "project_info",
        "project.info",
        "Describe the open project: name, schema, runs.",
    ),
    (
        "geometry_import",
        "geometry.import",
        "Import a model (STL, OBJ, 3MF, STEP, IGES) into the open project. Give the unit of the file coordinates; the answer contains the diagnosis (closed, normals, size).",
    ),
    (
        "geometry_info",
        "geometry.info",
        "Check a model: diagnosis (holes, normals), size and placement.",
    ),
    (
        "geometry_list",
        "geometry.list",
        "List the models of the open project.",
    ),
    (
        "geometry_set_transform",
        "geometry.set_transform",
        "Rotate, move or scale a model (degrees, metres).",
    ),
    (
        "geometry_pivot",
        "geometry.pivot",
        "Points a model can be moved and turned about: box centre, volume centre and origin (metres).",
    ),
    (
        "geometry_move_by",
        "geometry.move_by",
        "Move a model by a distance along the world axes (metres), one undo step.",
    ),
    (
        "geometry_rotate_about",
        "geometry.rotate_about",
        "Turn a model about the world axis x, y or z through a pivot (center, centroid, origin or a point), degrees.",
    ),
    (
        "surface_select_at",
        "surface.select_at",
        "Select the face of a model nearest to a point in metres (for naming faces or marking inlets and outlets).",
    ),
    (
        "geometry_cap_opening",
        "geometry.cap_opening",
        "Close an opening of a model nearest to a point (metres) with a flat cap, for internal flow.",
    ),
    (
        "scenario_set_wind_tunnel",
        "scenario.set_wind_tunnel",
        "Set up external flow around the models in a virtual wind tunnel: speed (m/s), direction (degrees), ground, medium and quality (draft, standard, fine). Returns reference values and the cell budget.",
    ),
    (
        "scenario_list",
        "scenario.list",
        "List the named scenarios of the project (id, name, settings, mesh, runs) and the active one.",
    ),
    (
        "scenario_add",
        "scenario.add",
        "Add a named scenario as a copy of the active one (or empty); activate it with scenario_activate before changing it.",
    ),
    (
        "scenario_activate",
        "scenario.activate",
        "Make a scenario active: scenario_set_* and run_start work on the active scenario.",
    ),
    (
        "run_start_all",
        "run.start_all",
        "Run all set-up scenarios (or the given ids) one after another; meshes are shared when they fit. Follow with run_status per job.",
    ),
    (
        "results_compare_many",
        "results.compare_many",
        "Compare the key figures of two or more runs side by side against a reference run.",
    ),
    (
        "scenario_set_jets",
        "scenario.set_jets",
        "Let the air come from up to four sources (circle or rectangle: centre, direction, size in metres, speed) in still surroundings, like fans or nozzles; an empty list goes back to the wind tunnel. Forces in newtons are meaningful; coefficients refer to the fastest jet.",
    ),
    (
        "scenario_set_wind_direction",
        "scenario.set_wind_direction",
        "Turn the wind of the wind tunnel (degrees in the ground plane, 0 = towards +X, 90 = towards +Y); everything else stays.",
    ),
    (
        "scenario_set_internal_flow",
        "scenario.set_internal_flow",
        "Set up flow through the models from inlets to outlets (caps or faces with a boundary role). Returns hydraulic diameters, Reynolds number and the regime.",
    ),
    (
        "scenario_set_boundary_role",
        "scenario.set_boundary_role",
        "Mark faces or caps as inlet, outlet or wall (for internal flow).",
    ),
    (
        "scenario_info",
        "scenario.info",
        "Describe the scenario of the open project with its reference values.",
    ),
    (
        "run_start",
        "run.start",
        "Start meshing and solving the open project. Returns at once with the job id; use run_wait or run_status to follow it.",
    ),
    (
        "run_status",
        "run.status",
        "State of a job: pending, running (with stage), succeeded, failed (with error code) or cancelled.",
    ),
    (
        "run_cancel",
        "run.cancel",
        "Cancel a running job; all its processes end.",
    ),
    (
        "run_log",
        "run.log",
        "Last lines of the log of a job, for diagnosing a failure.",
    ),
    (
        "results_summary",
        "results.summary",
        "Key figures of a run with units (SI), a short explanation each, and the reliability light (green, yellow, red) with its reasons. Default: the newest run.",
    ),
    (
        "results_probe",
        "results.probe",
        "Value of a result field at a point in metres, on the surface or in the flow.",
    ),
    (
        "results_compare",
        "results.compare",
        "Compare the key figures of two runs: a, b, b − a and (b − a)/|a|.",
    ),
    (
        "runs_list",
        "runs.list",
        "List the runs with results: name, kind, key value, reliability and disk space.",
    ),
    (
        "view_set_surface_field",
        "view.set_surface_field",
        "Colour the model by a surface field (p, Cp, wall_shear_stress, y_plus) before rendering.",
    ),
    (
        "view_add_slice",
        "view.add_slice",
        "Show a plane through the flow coloured by a volume field before rendering.",
    ),
    (
        "view_apply_preset",
        "view.apply_preset",
        "Replace the view with a ready-made view of the result (smoke_lines, smoke_curtain, mid_slice, surface_pressure, vortices, wake, oil_film, tufts, rain, light_trails, ribbons; internal flow: internal_path, internal_slice, oil_film, tufts, light_trails, ribbons). view.presets lists the ones that fit.",
    ),
    (
        "view_set_particles",
        "view.set_particles",
        "Show animated particles: a preset (wind_tunnel_smoke, internal_flow), style (comets, smoke, streamlines, trails: long glowing trails, ribbons and tubes: streamlines that twist with the swirl and carry running time marks), count, colour (speed, pressure, age, slip; swirl for ribbons and tubes). Trails take trail_length (flow s), width (px), glow, exposure and long_exposure (half-life s, null off). With inertia (material fine_dust, spray, rain, sand or snow, or diameter_m and density_kg_m3) they have mass and hit the model; inertia: null makes them massless again.",
    ),
    (
        "particles_materials",
        "particles.materials",
        "Material presets of heavy particles with diameter, density, impact behaviour and the Stokes number in the shown run (below 0.05 they follow the flow like smoke, above 2 they fly almost straight).",
    ),
    (
        "results_impact",
        "results.impact",
        "Compute where heavy particles hit the model (impact map, surface fields impact and impact_speed) and which share hits, with its uncertainty. Answers at once with impact_id; follow it with results_impact_status.",
    ),
    (
        "results_impact_status",
        "results.impact_status",
        "Progress of an impact map; when done the map with share_percent and uncertainty_percent.",
    ),
    (
        "view_set_surface_flow",
        "view.set_surface_flow",
        "Show the flow on the model surface: lines like an oil film (where they run together the flow separates) and wool tufts (flat where attached, fluttering or hanging where separated). Parts left out keep their settings; on: false hides a part. Then view_render.",
    ),
    (
        "ride_suggest",
        "ride.suggest",
        "Up to five worthwhile rides along one particle of the shown result (towards the stagnation point, close along the surface, fastest spot, wake, strongest turn, longest way), each with a start point in the world.",
    ),
    (
        "ride_start",
        "ride.start",
        "Ride along one particle: from a suggestion id or through a world point (m), calm or coaster, first_person, chase or overview. Answers with the readings along the path (speed, Cp, pressure, wall distance, acceleration) and the events.",
    ),
    (
        "ride_telemetry",
        "ride.telemetry",
        "Readings and events of the stored ride: what the particle goes through on its way.",
    ),
    (
        "ride_stop",
        "ride.stop",
        "End the ride and remove it from the view.",
    ),
    (
        "export_image",
        "export.image",
        "Write the 3D view as a PNG file (resolution hd, full_hd, qhd, 4k or width/height; transparent background; colour legend). exposure_s (0.1 to 30) takes a long exposure of the particles: their whole paths as light, the same every time. The path must lie inside the allowed roots.",
    ),
    (
        "export_video",
        "export.video",
        "Render a video (Motion JPEG AVI) of the 3D view with a camera path (turntable, swing, rise, approach, still, ride or own key frames) and the particle animation. Answers at once with export_id; follow it with export_status.",
    ),
    (
        "export_status",
        "export.status",
        "Progress of a video export: state (running, done, failed, cancelled), frames written, total frames.",
    ),
    (
        "export_cancel",
        "export.cancel",
        "Cancel a video export; the incomplete file is removed.",
    ),
    (
        "export_report",
        "export.report",
        "Write a report of a run as PDF and/or self-contained HTML (language en or de): setup, mesh, convergence, key figures with explanations, views, reliability light, limits and trademark notice. compare_with gives a comparison report of two runs.",
    ),
];

fn compact(v: &Value) -> Value {
    v.clone()
}

impl Server {
    /// Own session without a window.
    pub fn local(registry: Registry, data_dir: PathBuf, roots: Vec<PathBuf>) -> Self {
        Self {
            backend: Backend::Local {
                registry,
                session: Mutex::new(Session::new(data_dir.clone())),
            },
            specs: foamcraft_api::registry(),
            roots: roots.iter().map(|r| normalize(r)).collect(),
            data_dir,
        }
    }

    /// Commands run in the running app (its data folder holds the bridge file).
    pub fn app(data_dir: &Path, roots: Vec<PathBuf>) -> Result<Self, FoamError> {
        Ok(Self {
            backend: Backend::App(BridgeClient::connect(data_dir)?),
            specs: foamcraft_api::registry(),
            roots: roots.iter().map(|r| normalize(r)).collect(),
            data_dir: data_dir.to_path_buf(),
        })
    }

    pub fn instructions(&self) -> &'static str {
        INSTRUCTIONS
    }

    pub fn guide(&self) -> &'static str {
        GUIDE
    }

    fn schema_of(&self, command: &str) -> Value {
        self.specs
            .describe(command)
            .map_or_else(|| json!({"type": "object"}), |s| s.params.clone())
    }

    pub fn tools(&self) -> Vec<ToolDef> {
        let mut tools: Vec<ToolDef> = CURATED
            .iter()
            .map(|(name, command, description)| {
                let mut schema = self.schema_of(command);
                if matches!(*command, "run.start" | "export.video")
                    && let Some(props) = schema["properties"].as_object_mut()
                {
                    props.remove("wait");
                }
                ToolDef {
                    name: (*name).into(),
                    description: (*description).into(),
                    input_schema: schema,
                }
            })
            .collect();
        let extra = [
            (
                "run_wait",
                "Wait until a job has finished or the time limit is reached (default 600 s, at most 3600 s). Answers with finished true/false and the state; call again to keep waiting.",
                json!({"type": "object", "properties": {
                    "job_id": {"type": "string"},
                    "timeout_s": {"type": "number", "minimum": 0, "maximum": 3600}
                }, "required": ["job_id"], "additionalProperties": false}),
            ),
            (
                "view_render",
                "Render the 3D view of the open project (models, result colours, slices, particles) and answer with the PNG image. Optional path (inside the allowed roots) also keeps the file.",
                json!({"type": "object", "properties": {
                    "width": {"type": "integer", "minimum": 16, "maximum": 4096},
                    "height": {"type": "integer", "minimum": 16, "maximum": 4096},
                    "path": {"type": "string", "description": "PNG file to write as well."},
                    "view": {"type": "string", "enum": foamcraft_api::view::VIEW_NAMES,
                             "description": "Standard view: front looks from -X against the default flow; edges and corners combine sides, e.g. top_front_right. Default: default."},
                    "ride_t": {"type": "number", "minimum": 0,
                               "description": "Screen time (s) of the stored ride: a still from the ride's view (see ride_start) instead of the standard view."}
                }, "additionalProperties": false}),
            ),
            (
                "commands_list",
                "List every command of the FoamCraft command API with a one-line summary (generic access beyond the curated tools).",
                json!({"type": "object", "properties": {}, "additionalProperties": false}),
            ),
            (
                "command_describe",
                "Describe one command: summary, parameter schema, result schema, error codes and an example.",
                json!({"type": "object", "properties": {"name": {"type": "string"}},
                       "required": ["name"], "additionalProperties": false}),
            ),
            (
                "command_run",
                "Run any command of the command API with JSON parameters (see commands_list and command_describe).",
                json!({"type": "object", "properties": {
                    "name": {"type": "string"},
                    "params": {"type": "object"}
                }, "required": ["name"], "additionalProperties": false}),
            ),
        ];
        tools.extend(extra.into_iter().map(|(n, d, s)| ToolDef {
            name: n.into(),
            description: d.into(),
            input_schema: s,
        }));
        tools
    }

    /// Commands that create a project in the default projects folder when no path is given get a
    /// free folder inside the first root instead: an agent never writes outside its roots.
    fn with_default_path(&self, command: &str, mut params: Value) -> Value {
        let name = match command {
            "project.new" => params["name"].as_str(),
            "samples.create" => params["sample"].as_str(),
            _ => None,
        };
        if let (Some(name), Some(root), true) =
            (name, self.roots.first(), params.get("path").is_none())
        {
            let folder = foamcraft_core::paths::free_folder(root, name);
            params["path"] = json!(folder);
        }
        params
    }

    fn execute(&self, command: &str, params: Value) -> Result<Value, FoamError> {
        let params = self.with_default_path(command, params);
        self.check_paths(&params)?;
        match &self.backend {
            Backend::Local { registry, session } => {
                let mut s = session.lock().unwrap_or_else(|e| e.into_inner());
                registry.execute(&mut s, command, params)
            }
            Backend::App(client) => client.execute(command, params),
        }
    }

    /// Every path-like argument must be absolute and inside one of the roots.
    fn check_paths(&self, params: &Value) -> Result<(), FoamError> {
        let mut found = Vec::new();
        collect_paths(params, None, &mut found);
        for p in found {
            let path = Path::new(&p);
            let inside =
                path.is_absolute() && self.roots.iter().any(|r| starts_with(&normalize(path), r));
            if !inside {
                return Err(FoamError::new("mcp.path_outside_roots", p));
            }
        }
        Ok(())
    }

    /// Runs a tool; errors keep their catalog code.
    pub fn call(&self, tool: &str, args: Value) -> Result<Vec<Output>, FoamError> {
        let args = if args.is_null() { json!({}) } else { args };
        if !args.is_object() {
            return Err(FoamError::new(
                "mcp.invalid_arguments",
                "arguments must be an object",
            ));
        }
        if let Some((_, command, _)) = CURATED.iter().find(|(n, _, _)| *n == tool) {
            let mut params = args;
            if matches!(*command, "run.start" | "export.video") {
                params["wait"] = json!(false);
            }
            let out = self.execute(command, params)?;
            let out = if *command == "results.summary" {
                compact_summary(&out)
            } else {
                compact(&out)
            };
            return Ok(vec![Output::Json(out)]);
        }
        match tool {
            "run_wait" => self.wait(&args),
            "view_render" => self.render(&args),
            "commands_list" => Ok(vec![Output::Json(Value::Array(
                self.specs
                    .specs()
                    .map(|s| json!({"name": s.name, "summary": s.summary}))
                    .collect(),
            ))]),
            "command_describe" => {
                let name = args["name"].as_str().unwrap_or_default();
                let spec = self
                    .specs
                    .describe(name)
                    .ok_or_else(|| FoamError::new("command.unknown", name))?;
                Ok(vec![Output::Json(json!({
                    "name": spec.name, "summary": spec.summary, "params": spec.params,
                    "result": spec.result, "errors": spec.errors, "example": spec.example,
                }))])
            }
            "command_run" => {
                let name = args["name"]
                    .as_str()
                    .ok_or_else(|| FoamError::new("mcp.invalid_arguments", "name"))?;
                let params = args.get("params").cloned().unwrap_or_else(|| json!({}));
                Ok(vec![Output::Json(self.execute(name, params)?)])
            }
            _ => Err(FoamError::new("mcp.unknown_tool", tool)),
        }
    }

    fn wait(&self, args: &Value) -> Result<Vec<Output>, FoamError> {
        let job = args["job_id"]
            .as_str()
            .ok_or_else(|| FoamError::new("mcp.invalid_arguments", "job_id"))?;
        let limit = Duration::from_secs_f64(
            args["timeout_s"]
                .as_f64()
                .unwrap_or(600.0)
                .clamp(0.0, 3600.0),
        );
        let start = Instant::now();
        loop {
            let status = self.execute("run.status", json!({"job_id": job}))?;
            let state = status["state"]["state"].as_str().unwrap_or_default();
            // Finished means also evaluated: the result and the fields are stored.
            let finished =
                matches!(state, "succeeded" | "failed" | "cancelled") && status["busy"] != true;
            if finished || start.elapsed() >= limit {
                return Ok(vec![Output::Json(json!({
                    "job_id": job,
                    "finished": finished,
                    "state": status["state"],
                    "waited_s": start.elapsed().as_secs_f64().round(),
                }))]);
            }
            std::thread::sleep(
                Duration::from_millis(500).min(limit.saturating_sub(start.elapsed())),
            );
        }
    }

    fn render(&self, args: &Value) -> Result<Vec<Output>, FoamError> {
        let width = args["width"].as_u64().unwrap_or(1280).clamp(16, 4096);
        let height = args["height"].as_u64().unwrap_or(800).clamp(16, 4096);
        let keep = args["path"].as_str().map(PathBuf::from);
        let path = keep.clone().unwrap_or_else(|| {
            self.data_dir
                .join("mcp")
                .join(format!("render-{}.png", std::process::id()))
        });
        if let Some(dir) = path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        let mut params = json!({"path": path, "width": width, "height": height});
        if let Some(view) = args.get("view").filter(|v| !v.is_null()) {
            params["view"] = view.clone();
        }
        if let Some(t) = args.get("ride_t").filter(|v| !v.is_null()) {
            params["ride_t"] = t.clone();
        }
        if keep.is_some() {
            self.check_paths(&params)?;
        }
        // The temporary file lies in the data folder, outside the roots: no path check there.
        let out = match &self.backend {
            Backend::Local { registry, session } => {
                let mut s = session.lock().unwrap_or_else(|e| e.into_inner());
                registry.execute(&mut s, "view.render", params)?
            }
            Backend::App(client) => client.execute("view.render", params)?,
        };
        let bytes =
            std::fs::read(&path).map_err(|e| FoamError::new("view.write_failed", e.to_string()))?;
        if keep.is_none() {
            let _ = std::fs::remove_file(&path);
        }
        Ok(vec![Output::Png(bytes), Output::Json(out)])
    }
}

/// Keys whose string values are file system paths.
fn is_path_key(key: &str) -> bool {
    matches!(
        key,
        "path" | "folder" | "file" | "dir" | "directory" | "source" | "root"
    ) || key.ends_with("_path")
        || key.ends_with("_dir")
}

fn collect_paths(v: &Value, key: Option<&str>, out: &mut Vec<String>) {
    match v {
        Value::String(s) if key.is_some_and(is_path_key) => out.push(s.clone()),
        Value::Array(items) => items.iter().for_each(|i| collect_paths(i, key, out)),
        Value::Object(map) => map
            .iter()
            .for_each(|(k, v)| collect_paths(v, Some(k.as_str()), out)),
        _ => {}
    }
}

/// Absolute path without `.` and `..` (lexically; the path need not exist).
fn normalize(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for c in path.components() {
        match c {
            Component::ParentDir => {
                out.pop();
            }
            Component::CurDir => {}
            other => out.push(other),
        }
    }
    out
}

fn starts_with(path: &Path, root: &Path) -> bool {
    if cfg!(windows) {
        let lower = |p: &Path| p.to_string_lossy().replace('/', "\\").to_lowercase();
        let (p, r) = (lower(path), lower(root));
        let r = r.trim_end_matches('\\');
        p == r || p.starts_with(&format!("{r}\\"))
    } else {
        path.starts_with(root)
    }
}

/// The summary with the English text per figure and check (compact for an agent).
fn compact_summary(v: &Value) -> Value {
    let mut out = v.clone();
    let strip = |item: &mut Value| {
        if let Some(obj) = item.as_object_mut() {
            let text = obj
                .get("en")
                .and_then(|m| m["text"].as_str())
                .unwrap_or_default()
                .to_string();
            obj.remove("en");
            obj.remove("de");
            obj.insert("text".into(), json!(text));
        }
    };
    if let Some(figures) = out["figures"].as_array_mut() {
        figures.iter_mut().for_each(strip);
    }
    if let Some(checks) = out["reliability"]["checks"].as_array_mut() {
        checks.iter_mut().for_each(strip);
    }
    out
}

/// The error as an agent reads it: code, English text and suggestion, detail.
pub fn error_json(e: &FoamError) -> Value {
    let m = message(&e.code, Language::En);
    json!({
        "code": e.code,
        "message": m.map(|m| m.text.clone()).unwrap_or_default(),
        "fix": m.map(|m| m.fix.clone()).unwrap_or_default(),
        "detail": e.detail,
    })
}
