// SPDX-License-Identifier: GPL-3.0-or-later
//! Results for display: after a run, OpenFOAM writes the wall surface with its fields and samples
//! the volume fields on regular grids (`postProcess` in the runtime); FoamCraft converts them into
//! a compact binary cache in `<project>/cache/results/<job id>/` and answers probes from it.

use std::path::{Path, PathBuf};

use foamcraft_core::command::{Options, Registry, Session};
use foamcraft_core::error::FoamError;
use foamcraft_core::project::Project;
use foamcraft_runtime::jobs::case_dir;
use foamcraft_runtime::{Container, Mount, StartOptions};
use foamcraft_sim::foam_io::{
    read_faces, read_points, read_raw_set, read_scalar_list, read_vector_list,
};
use foamcraft_sim::results::{
    Grid, GridSpec, Surface, colormap, extraction_dict, grid_from_rows, set_columns,
};
use glam::{DMat3, DVec3};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::SharedRunner;

/// Folder of the result caches inside a project.
pub const CACHE_DIR: &str = "cache/results";
/// The result caches of a project may use this much disk space; older ones are removed first.
pub const CACHE_LIMIT_BYTES: u64 = 256 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct FieldInfo {
    pub name: String,
    pub unit: String,
    /// `surface` or `volume`.
    pub location: String,
    pub min: f64,
    pub max: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct LevelInfo {
    pub min: [f64; 3],
    pub max: [f64; 3],
    pub dims: [u32; 3],
}

/// `meta.json` of a result cache.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct CacheMeta {
    pub job_id: String,
    pub fields: Vec<FieldInfo>,
    /// Grid levels, coarse to fine (in the flow frame for wind tunnels: turn by `yaw_deg`).
    pub levels: Vec<LevelInfo>,
    pub yaw_deg: f64,
    pub bytes: u64,
    /// Colour scale samples (sRGB hex, low to high) for legends.
    pub colormap: Vec<String>,
}

/// What the extraction needs to know about a finished job.
pub struct ExtractInput {
    pub job_id: String,
    pub volume: String,
    pub project_root: PathBuf,
    /// Wall patches, for example `"model.*"` or `walls` (without brackets).
    pub patches: String,
    pub turbulent: bool,
    pub levels: Vec<GridSpec>,
    pub yaw_deg: f64,
    pub density: f64,
    /// Speed of the dynamic pressure for the pressure coefficient.
    pub reference_speed: f64,
}

fn io(e: impl std::fmt::Display) -> FoamError {
    FoamError::new("results.read_failed", e.to_string())
}

pub fn cache_dir(root: &Path, job_id: &str) -> PathBuf {
    root.join(CACHE_DIR).join(job_id)
}

fn yaw(deg: f64) -> DMat3 {
    DMat3::from_rotation_z(deg.to_radians())
}

fn hex(c: [f32; 3]) -> String {
    // Linear to sRGB for the interface.
    let s = c.map(|x| {
        let x = f64::from(x);
        let v = if x <= 0.003_130_8 {
            12.92 * x
        } else {
            1.055 * x.powf(1.0 / 2.4) - 0.055
        };
        (v.clamp(0.0, 1.0) * 255.0).round() as u8
    });
    format!("#{:02x}{:02x}{:02x}", s[0], s[1], s[2])
}

/// Samples of the colour scale for legends.
fn colormap_hex() -> Vec<String> {
    (0..=8).map(|i| hex(colormap(f64::from(i) / 8.0))).collect()
}

fn range(v: &[f32]) -> (f64, f64) {
    v.iter().fold((f64::MAX, f64::MIN), |(lo, hi), x| {
        (lo.min(f64::from(*x)), hi.max(f64::from(*x)))
    })
}

/// The single time folder below a postProcessing output folder.
fn time_dir(dir: &Path) -> Result<PathBuf, FoamError> {
    std::fs::read_dir(dir)
        .map_err(io)?
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.is_dir())
        .max()
        .ok_or_else(|| io(format!("no output in {}", dir.display())))
}

fn read(path: &Path) -> Result<Vec<u8>, FoamError> {
    std::fs::read(path).map_err(|e| io(format!("{}: {e}", path.display())))
}

/// Runs the extraction in the runtime, converts the output and writes the cache.
pub fn extract(runner: &SharedRunner, input: &ExtractInput) -> Result<CacheMeta, FoamError> {
    let r = runner.as_ref();
    let container = r.start(&StartOptions {
        mount: Some(Mount::Volume(input.volume.clone())),
        ..Default::default()
    })?;
    let raw = input
        .project_root
        .join("runs")
        .join(&input.job_id)
        .join("extract");
    let outcome = (|| {
        let dict = extraction_dict(&input.patches, input.turbulent, &input.levels);
        let script = format!(
            "cd {case} && rm -rf postProcessing/walls postProcessing/grid && \
             cat > system/foamcraftResults <<'FOAMCRAFT_EOF'\n{dict}FOAMCRAFT_EOF\n\
             simpleFoam -postProcess -dict system/foamcraftResults -latestTime > log.results 2>&1 \
             || {{ tail -n 30 log.results; exit 1; }}",
            case = case_dir()
        );
        let out = r.exec_openfoam(&container, &script, &mut |_| {})?;
        if out.status != 0 {
            return Err(FoamError::new("results.extract_failed", out.stdout));
        }
        let _ = std::fs::remove_dir_all(&raw);
        std::fs::create_dir_all(&raw).map_err(io)?;
        for part in ["walls", "grid"] {
            r.copy_out(
                &container,
                &format!("{}/postProcessing/{part}", case_dir()),
                &raw.join(part),
            )?;
        }
        convert(input, &raw)
    })();
    let _ = r.stop(&Container { id: container.id });
    let _ = std::fs::remove_dir_all(&raw);
    outcome
}

fn convert(input: &ExtractInput, raw: &Path) -> Result<CacheMeta, FoamError> {
    let rho = input.density;
    let q = 0.5 * input.reference_speed * input.reference_speed;
    let turn = yaw(input.yaw_deg);
    // Wall surface.
    let model = time_dir(&raw.join("walls"))?.join("model");
    let points: Vec<[f32; 3]> = read_points(&read(&model.join("points"))?)?
        .into_iter()
        .map(|p| {
            let w = turn * DVec3::from(p);
            [w.x as f32, w.y as f32, w.z as f32]
        })
        .collect();
    let triangles: Vec<[u32; 3]> = read_faces(&read(&model.join("faces"))?)?
        .into_iter()
        .filter(|f| f.len() == 3)
        .map(|f| [f[0], f[1], f[2]])
        .collect();
    let p = read_scalar_list(&read(&model.join("scalarField/p"))?)?;
    let wss = read_vector_list(&read(&model.join("vectorField/wallShearStress"))?)?;
    let mut fields = vec![
        (
            "p".to_string(),
            p.iter().map(|v| (v * rho) as f32).collect::<Vec<_>>(),
        ),
        (
            "Cp".to_string(),
            p.iter().map(|v| (v / q.max(1e-30)) as f32).collect(),
        ),
        (
            "wall_shear_stress".to_string(),
            wss.iter()
                .map(|v| (DVec3::from(*v).length() * rho) as f32)
                .collect(),
        ),
    ];
    if let Ok(bytes) = std::fs::read(model.join("scalarField/yPlus")) {
        fields.push((
            "y_plus".to_string(),
            read_scalar_list(&bytes)?
                .iter()
                .map(|v| *v as f32)
                .collect(),
        ));
    }
    let surface = Surface {
        points,
        triangles,
        fields,
    };
    let dir = cache_dir(&input.project_root, &input.job_id);
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).map_err(io)?;
    let mut bytes = 0u64;
    let surface_bytes = surface.to_bytes();
    bytes += surface_bytes.len() as u64;
    std::fs::write(dir.join("surface.bin"), surface_bytes).map_err(io)?;
    let unit = |name: &str| match name {
        "p" | "wall_shear_stress" | "p_total" => "Pa",
        "U" => "m/s",
        "k" => "m²/s²",
        _ => "-",
    };
    let mut infos: Vec<FieldInfo> = surface
        .fields
        .iter()
        .map(|(n, v)| {
            let (min, max) = range(v);
            FieldInfo {
                name: n.clone(),
                unit: unit(n).into(),
                location: "surface".into(),
                min,
                max,
            }
        })
        .collect();
    // Volume grids.
    let grid_time = time_dir(&raw.join("grid"))?;
    let mut levels = Vec::new();
    let mut volume_ranges: std::collections::BTreeMap<String, (f64, f64)> = Default::default();
    for (n, spec) in input.levels.iter().enumerate() {
        let Some(file) = std::fs::read_dir(&grid_time)
            .map_err(io)?
            .flatten()
            .map(|e| e.path())
            .find(|p| {
                p.file_name()
                    .is_some_and(|f| f.to_string_lossy().starts_with(&format!("level{n}_")))
            })
        else {
            continue;
        };
        let columns = set_columns(&file.file_name().unwrap_or_default().to_string_lossy());
        let rows = read_raw_set(&read(&file)?)?;
        let names: Vec<&str> = columns.iter().map(String::as_str).collect();
        let mut grid = grid_from_rows(spec, &names, &rows);
        // Pressure in Pa and the total pressure p + rho |U|² / 2.
        let col = |g: &Grid, name: &str| g.names.iter().position(|c| c == name);
        if let (Some(pi), Some(ux)) = (col(&grid, "p"), col(&grid, "Ux")) {
            let total: Vec<f32> = (0..grid.spec.len())
                .map(|i| {
                    let u2: f64 = (0..3)
                        .map(|k| f64::from(grid.values[ux + k][i]).powi(2))
                        .sum();
                    (rho * (f64::from(grid.values[pi][i]) + 0.5 * u2)) as f32
                })
                .collect();
            for v in grid.values[pi].iter_mut() {
                *v = (f64::from(*v) * rho) as f32;
            }
            grid.names.push("p_total".into());
            grid.values.push(total);
        }
        for (name, values) in grid.names.iter().zip(&grid.values) {
            let live: Vec<f32> = values
                .iter()
                .zip(&grid.mask)
                .filter(|(_, m)| !**m)
                .map(|(v, _)| *v)
                .collect();
            let (lo, hi) = range(&live);
            let e = volume_ranges.entry(name.clone()).or_insert((lo, hi));
            *e = (e.0.min(lo), e.1.max(hi));
        }
        let b = grid.to_bytes();
        bytes += b.len() as u64;
        std::fs::write(dir.join(format!("grid_{n}.bin")), b).map_err(io)?;
        levels.push(LevelInfo {
            min: spec.min,
            max: spec.max,
            dims: spec.dims,
        });
    }
    for (name, (min, max)) in volume_ranges {
        infos.push(FieldInfo {
            unit: unit(name.trim_end_matches(['x', 'y', 'z'])).into(),
            name,
            location: "volume".into(),
            min,
            max,
        });
    }
    let meta = CacheMeta {
        job_id: input.job_id.clone(),
        fields: infos,
        levels,
        yaw_deg: input.yaw_deg,
        bytes,
        colormap: colormap_hex(),
    };
    std::fs::write(
        dir.join("meta.json"),
        serde_json::to_string_pretty(&meta).map_err(io)?,
    )
    .map_err(io)?;
    limit_cache(&input.project_root, &input.job_id);
    Ok(meta)
}

fn dir_size(dir: &Path) -> u64 {
    std::fs::read_dir(dir)
        .map(|it| {
            it.flatten()
                .filter_map(|e| e.metadata().ok())
                .map(|m| m.len())
                .sum()
        })
        .unwrap_or(0)
}

/// Removes the oldest result caches until all fit into [`CACHE_LIMIT_BYTES`].
fn limit_cache(root: &Path, keep: &str) {
    let base = root.join(CACHE_DIR);
    let mut dirs: Vec<(std::time::SystemTime, PathBuf, u64)> = std::fs::read_dir(&base)
        .map(|it| {
            it.flatten()
                .filter(|e| e.path().is_dir())
                .map(|e| {
                    let t = e
                        .metadata()
                        .and_then(|m| m.modified())
                        .unwrap_or(std::time::UNIX_EPOCH);
                    (t, e.path(), dir_size(&e.path()))
                })
                .collect()
        })
        .unwrap_or_default();
    dirs.sort();
    let mut total: u64 = dirs.iter().map(|d| d.2).sum();
    for (_, path, size) in dirs {
        if total <= CACHE_LIMIT_BYTES {
            break;
        }
        if path.file_name().is_some_and(|n| n == keep) {
            continue;
        }
        let _ = std::fs::remove_dir_all(&path);
        total -= size;
    }
}

/// The job whose cache is shown: the view's job, else the newest cache.
fn current_job(project: &Project, job: Option<&str>) -> Result<String, FoamError> {
    if let Some(j) = job.or(project.manifest.view.result_job.as_deref())
        && cache_dir(&project.root, j).join("meta.json").is_file()
    {
        return Ok(j.to_string());
    }
    std::fs::read_dir(project.root.join(CACHE_DIR))
        .ok()
        .into_iter()
        .flatten()
        .flatten()
        .filter(|e| e.path().join("meta.json").is_file())
        .max_by_key(|e| e.metadata().and_then(|m| m.modified()).ok())
        .map(|e| e.file_name().to_string_lossy().to_string())
        .ok_or_else(|| FoamError::new("results.none", ""))
}

pub fn load_meta(root: &Path, job: &str) -> Result<CacheMeta, FoamError> {
    let text = std::fs::read_to_string(cache_dir(root, job).join("meta.json")).map_err(io)?;
    serde_json::from_str(&text).map_err(io)
}

pub fn load_surface(root: &Path, job: &str) -> Result<Surface, FoamError> {
    Surface::from_bytes(&read(&cache_dir(root, job).join("surface.bin"))?)
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct JobParams {
    /// Default: the result shown in the view, else the newest.
    #[serde(default)]
    pub job_id: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ProbeParams {
    /// Point in the world, metres (for example where the user clicked).
    pub point: [f64; 3],
    /// Field name; default: the surface field of the view, else `p`.
    #[serde(default)]
    pub field: Option<String>,
    #[serde(default)]
    pub job_id: Option<String>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct Probe {
    pub field: String,
    pub value: f64,
    pub unit: String,
    /// `surface` or `volume`.
    pub location: String,
    /// Distance of the point from the surface triangle used, m.
    pub distance_m: Option<f64>,
}

fn probe(project: &Project, p: ProbeParams) -> Result<Probe, FoamError> {
    let job = current_job(project, p.job_id.as_deref())?;
    let meta = load_meta(&project.root, &job)?;
    let field = p
        .field
        .or_else(|| project.manifest.view.surface_field.clone())
        .unwrap_or_else(|| "p".into());
    let info = meta
        .fields
        .iter()
        .find(|f| f.name == field)
        .ok_or_else(|| FoamError::new("results.unknown_field", field.clone()))?;
    let point = DVec3::from(p.point);
    if info.location == "surface" {
        let s = load_surface(&project.root, &job)?;
        let values = s.field(&field).unwrap_or_default();
        let (best, dist) = s
            .triangles
            .iter()
            .enumerate()
            .map(|(i, t)| {
                let c = t
                    .iter()
                    .map(|k| DVec3::from(s.points[*k as usize].map(f64::from)))
                    .sum::<DVec3>()
                    / 3.0;
                (i, c.distance(point))
            })
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .ok_or_else(|| FoamError::new("results.none", "empty surface"))?;
        return Ok(Probe {
            value: f64::from(values[best]),
            unit: info.unit.clone(),
            location: "surface".into(),
            distance_m: Some(dist),
            field,
        });
    }
    // Volume: finest level that has a value; grids are in the flow frame.
    let local = yaw(-meta.yaw_deg) * point;
    for n in (0..meta.levels.len()).rev() {
        let grid = Grid::from_bytes(&read(
            &cache_dir(&project.root, &job).join(format!("grid_{n}.bin")),
        )?)?;
        if let Some(v) = grid.sample(&field, local.into()) {
            return Ok(Probe {
                field,
                value: v,
                unit: info.unit.clone(),
                location: "volume".into(),
                distance_m: None,
            });
        }
    }
    Err(FoamError::new("results.outside", format!("{:?}", p.point)))
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SurfaceFieldParams {
    /// Field to colour the model by; null shows the model without results.
    pub field: Option<String>,
    /// Colour scale range; default: the range of the field.
    #[serde(default)]
    pub range: Option<[f64; 2]>,
    #[serde(default)]
    pub job_id: Option<String>,
}

fn set_surface_field(s: &mut Session, p: SurfaceFieldParams) -> Result<Value, FoamError> {
    let project = s.project()?;
    if let Some(field) = &p.field {
        let job = current_job(project, p.job_id.as_deref())?;
        let meta = load_meta(&project.root, &job)?;
        if !meta
            .fields
            .iter()
            .any(|f| &f.name == field && f.location == "surface")
        {
            return Err(FoamError::new("results.unknown_field", field.clone()));
        }
        if let Some([lo, hi]) = p.range
            && !(lo.is_finite() && hi.is_finite() && hi > lo)
        {
            return Err(FoamError::new(
                "results.invalid_range",
                format!("{lo} {hi}"),
            ));
        }
        project.manifest.view.result_job = Some(job);
    }
    project.manifest.view.surface_field = p.field;
    project.manifest.view.range = p.range;
    project.save()?;
    let view = serde_json::to_value(&project.manifest.view).map_err(io)?;
    s.events().emit("project.changed", Value::Null);
    Ok(view)
}

pub fn register(r: &mut Registry) {
    r.add(
        "results.list_fields",
        "List the result fields of a run (surface and volume) with unit and range, the grid levels \
         and the colour scale.",
        Options {
            undoable: false,
            errors: &["project.none_open", "results.none"],
            example: json!({}),
        },
        |s, p: JobParams| {
            let project = s.project()?;
            let job = current_job(project, p.job_id.as_deref())?;
            let mut meta = load_meta(&project.root, &job)?;
            meta.colormap = colormap_hex();
            Ok(meta)
        },
    );
    r.add(
        "results.probe",
        "Value of a result field at a point (metres): on the surface or in the volume.",
        Options {
            undoable: false,
            errors: &[
                "project.none_open",
                "results.none",
                "results.unknown_field",
                "results.outside",
            ],
            example: json!({"point": [0.0, 0.0, 0.01], "field": "Cp"}),
        },
        |s, p: ProbeParams| probe(s.project()?, p),
    );
    r.add(
        "view.set_surface_field",
        "Colour the model by a surface result field (null: no colouring), with an optional range.",
        Options {
            undoable: false,
            errors: &[
                "project.none_open",
                "results.none",
                "results.unknown_field",
                "results.invalid_range",
            ],
            example: json!({"field": "p"}),
        },
        set_surface_field,
    );
}
