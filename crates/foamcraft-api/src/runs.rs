// SPDX-License-Identifier: GPL-3.0-or-later
//! Key figures and runs: `results.summary` (stable schema with texts and the reliability light),
//! `results.compare`, `results.export`, and `runs.list`, `runs.rename`, `runs.duplicate`,
//! `runs.delete`.

use std::path::{Path, PathBuf};

use foamcraft_core::command::{Options, Registry, Session};
use foamcraft_core::error::{FoamError, Language, Message, message};
use foamcraft_core::job::JobStore;
use foamcraft_core::project::{MANIFEST_FILE, Project};
use foamcraft_sim::summary::{Figure, Level, Summary, summarize};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::SharedRunner;
use crate::results::cache_dir;
use crate::solve::{RESULTS_DIR, load_result, result_path};

fn texts(code: &str) -> (Message, Message) {
    let get = |lang| {
        message(code, lang).cloned().unwrap_or(Message {
            text: code.into(),
            fix: String::new(),
        })
    };
    (get(Language::En), get(Language::De))
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct FigureText {
    #[serde(flatten)]
    pub figure: Figure,
    pub en: Message,
    pub de: Message,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct CheckText {
    pub code: String,
    pub level: Level,
    pub en: Message,
    pub de: Message,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct ReliabilityText {
    pub level: Level,
    pub checks: Vec<CheckText>,
}

/// `results.summary`: the summary with the explanation of every figure and check.
#[derive(Debug, Serialize, JsonSchema)]
pub struct SummaryText {
    pub schema: u32,
    pub job_id: String,
    pub name: String,
    pub kind: String,
    pub figures: Vec<FigureText>,
    pub reliability: ReliabilityText,
}

fn summary_of(project: &Project, job: Option<&str>) -> Result<(Summary, Value), FoamError> {
    let result = load_result(&project.root.join(RESULTS_DIR), job)?;
    let mut summary = summarize(&result)?;
    if summary.kind == "wind_tunnel"
        && let Some(share) = reverse_flow_area(project, &summary.job_id)
    {
        summary.figures.push(foamcraft_sim::summary::Figure {
            id: "reverse_flow_area".into(),
            value: share * 100.0,
            unit: "%".into(),
            part: None,
        });
    }
    Ok((summary, result))
}

/// Share of the model surface in reverse flow (from the wall shear stress vector), when the run
/// has it.
fn reverse_flow_area(project: &Project, job: &str) -> Option<f64> {
    let s = crate::results::load_surface(&project.root, job).ok()?;
    let a = s.field("attached")?;
    let (mut back, mut total) = (0.0, 0.0);
    for (t, v) in s.triangles.iter().zip(a) {
        let [p, q, r] = t.map(|i| glam::DVec3::from(s.points[i as usize].map(f64::from)));
        let area = 0.5 * (q - p).cross(r - p).length();
        total += area;
        if *v < 0.0 {
            back += area;
        }
    }
    (total > 0.0).then(|| back / total)
}

/// `results.summary` of a job as JSON (with the English and German texts).
pub fn summary_json(project: &Project, job: &str) -> Result<Value, FoamError> {
    let (summary, _) = summary_of(project, Some(job))?;
    serde_json::to_value(with_texts(project, summary))
        .map_err(|e| FoamError::new("results.read_failed", e.to_string()))
}

fn with_texts(project: &Project, s: Summary) -> SummaryText {
    SummaryText {
        schema: s.schema,
        name: run_name(project, &s.job_id),
        job_id: s.job_id,
        kind: s.kind,
        figures: s
            .figures
            .into_iter()
            .map(|figure| {
                let (en, de) = texts(&format!("figure.{}", figure.id));
                FigureText { figure, en, de }
            })
            .collect(),
        reliability: ReliabilityText {
            level: s.reliability.level,
            checks: s
                .reliability
                .checks
                .into_iter()
                .map(|c| {
                    let (en, de) = texts(&c.code);
                    CheckText {
                        code: c.code,
                        level: c.level,
                        en,
                        de,
                    }
                })
                .collect(),
        },
    }
}

pub fn run_name(project: &Project, job: &str) -> String {
    project
        .manifest
        .run_names
        .get(job)
        .cloned()
        .unwrap_or_else(|| job.to_string())
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct JobParams {
    /// Default: the newest result.
    #[serde(default)]
    pub job_id: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CompareManyParams {
    /// Two or more job ids, in the order of the columns.
    pub jobs: Vec<String>,
    /// Run the others are compared with; default: the first.
    #[serde(default)]
    pub reference: Option<String>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct ManyRow {
    pub id: String,
    pub part: Option<String>,
    pub unit: String,
    /// One value per run (null where the run has no such figure).
    pub values: Vec<Option<f64>>,
    /// (value − reference) / |reference| per run.
    pub relative: Vec<Option<f64>>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct ManyComparison {
    pub jobs: Vec<String>,
    pub names: Vec<String>,
    pub reference: String,
    pub rows: Vec<ManyRow>,
}

/// Key figures of several runs side by side, each relative to a reference run.
pub fn compare_many(project: &Project, p: CompareManyParams) -> Result<ManyComparison, FoamError> {
    if p.jobs.len() < 2 {
        return Err(FoamError::new("results.compare_needs_two", ""));
    }
    let reference = p.reference.unwrap_or_else(|| p.jobs[0].clone());
    let summaries = p
        .jobs
        .iter()
        .map(|j| summary_of(project, Some(j)).map(|(s, _)| s))
        .collect::<Result<Vec<_>, _>>()?;
    let ref_index = p.jobs.iter().position(|j| *j == reference).unwrap_or(0);
    let mut keys: Vec<(String, Option<String>, String)> = Vec::new();
    for f in summaries.iter().flat_map(|s| &s.figures) {
        let k = (f.id.clone(), f.part.clone(), f.unit.clone());
        if !keys.contains(&k) {
            keys.push(k);
        }
    }
    let rows = keys
        .into_iter()
        .map(|(id, part, unit)| {
            let values: Vec<Option<f64>> = summaries
                .iter()
                .map(|s| {
                    s.figures
                        .iter()
                        .find(|f| f.id == id && f.part == part)
                        .map(|f| f.value)
                })
                .collect();
            let base = values[ref_index];
            let relative = values
                .iter()
                .map(|v| {
                    v.zip(base)
                        .and_then(|(x, b)| (b.abs() > 1e-30).then(|| (x - b) / b.abs()))
                })
                .collect();
            ManyRow {
                id,
                part,
                unit,
                values,
                relative,
            }
        })
        .collect();
    Ok(ManyComparison {
        names: p.jobs.iter().map(|j| run_name(project, j)).collect(),
        jobs: p.jobs,
        reference,
        rows,
    })
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CompareParams {
    pub a: String,
    pub b: String,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct CompareRow {
    pub id: String,
    pub part: Option<String>,
    pub unit: String,
    pub a: Option<f64>,
    pub b: Option<f64>,
    /// b − a.
    pub difference: Option<f64>,
    /// (b − a) / |a|.
    pub relative: Option<f64>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct Comparison {
    pub a: String,
    pub b: String,
    pub name_a: String,
    pub name_b: String,
    pub rows: Vec<CompareRow>,
}

pub fn compare(project: &Project, p: CompareParams) -> Result<Comparison, FoamError> {
    let (a, _) = summary_of(project, Some(&p.a))?;
    let (b, _) = summary_of(project, Some(&p.b))?;
    let mut keys: Vec<(String, Option<String>, String)> = Vec::new();
    for f in a.figures.iter().chain(&b.figures) {
        let k = (f.id.clone(), f.part.clone(), f.unit.clone());
        if !keys.contains(&k) {
            keys.push(k);
        }
    }
    let find = |s: &Summary, id: &str, part: &Option<String>| {
        s.figures
            .iter()
            .find(|f| f.id == id && &f.part == part)
            .map(|f| f.value)
    };
    let rows = keys
        .into_iter()
        .map(|(id, part, unit)| {
            let (va, vb) = (find(&a, &id, &part), find(&b, &id, &part));
            let difference = va.zip(vb).map(|(x, y)| y - x);
            let relative = va
                .zip(difference)
                .and_then(|(x, d)| (x.abs() > 1e-30).then(|| d / x.abs()));
            CompareRow {
                id,
                part,
                unit,
                a: va,
                b: vb,
                difference,
                relative,
            }
        })
        .collect();
    Ok(Comparison {
        name_a: run_name(project, &p.a),
        name_b: run_name(project, &p.b),
        a: p.a,
        b: p.b,
        rows,
    })
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ExportParams {
    #[serde(default)]
    pub job_id: Option<String>,
    pub path: PathBuf,
    /// `csv` or `json`.
    pub format: String,
}

fn export(project: &Project, p: ExportParams) -> Result<Value, FoamError> {
    let (s, _) = summary_of(project, p.job_id.as_deref())?;
    let text = match p.format.as_str() {
        "csv" => {
            let mut out = String::from("id,part,value,unit\n");
            for f in &s.figures {
                let part = f.part.clone().unwrap_or_default().replace(',', ";");
                out.push_str(&format!("{},{},{},{}\n", f.id, part, f.value, f.unit));
            }
            out
        }
        "json" => serde_json::to_string_pretty(&s)
            .map_err(|e| FoamError::new("results.export_failed", e.to_string()))?,
        other => {
            return Err(FoamError::new(
                "results.export_failed",
                format!("format {other}"),
            ));
        }
    };
    std::fs::write(&p.path, text)
        .map_err(|e| FoamError::new("results.export_failed", e.to_string()))?;
    Ok(json!({"path": p.path, "figures": s.figures.len()}))
}

fn size(path: &Path) -> u64 {
    if path.is_file() {
        return path.metadata().map_or(0, |m| m.len());
    }
    std::fs::read_dir(path)
        .map(|it| it.flatten().map(|e| size(&e.path())).sum())
        .unwrap_or(0)
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct RunInfo {
    pub job_id: String,
    pub name: String,
    pub kind: String,
    /// Unix time of the result.
    pub created: u64,
    /// Drag (wind tunnel) or pressure loss (internal flow) with unit.
    pub key_value: Option<f64>,
    pub key_unit: String,
    pub reliability: Level,
    /// Disk space of the run (job folder, result, cache); `null` when sizes were not asked for.
    pub bytes: Option<u64>,
}

fn run_bytes(root: &Path, job: &str) -> u64 {
    size(&root.join("runs").join(job))
        + size(&result_path(&root.join(RESULTS_DIR), job))
        + size(&cache_dir(root, job))
}

fn list(project: &Project, sizes: bool) -> Vec<RunInfo> {
    let dir = project.root.join(RESULTS_DIR);
    let mut out: Vec<RunInfo> = std::fs::read_dir(&dir)
        .ok()
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|e| {
            let path = e.path();
            let job = path.file_stem()?.to_string_lossy().to_string();
            let value: Value = serde_json::from_str(&std::fs::read_to_string(&path).ok()?).ok()?;
            let s = summarize(&value).ok()?;
            let key = if s.kind == "internal_flow" {
                "pressure_loss_pa"
            } else {
                "drag_n"
            };
            let figure = s.figures.iter().find(|f| f.id == key && f.part.is_none());
            let created = e
                .metadata()
                .and_then(|m| m.modified())
                .ok()
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map_or(0, |d| d.as_secs());
            Some(RunInfo {
                name: run_name(project, &job),
                bytes: sizes.then(|| run_bytes(&project.root, &job)),
                job_id: job,
                kind: s.kind.clone(),
                created,
                key_value: figure.map(|f| f.value),
                key_unit: figure.map_or(String::new(), |f| f.unit.clone()),
                reliability: s.reliability.level,
            })
        })
        .collect();
    out.sort_by(|a, b| (b.created, &b.job_id).cmp(&(a.created, &a.job_id)));
    out
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ListParams {
    /// Measure the disk space of each run (default true). Walking the job folders of large runs
    /// takes long; lists that only need names pass false.
    #[serde(default = "yes")]
    pub sizes: bool,
}

fn yes() -> bool {
    true
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RenameParams {
    pub job_id: String,
    pub name: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DeleteParams {
    pub job_id: String,
}

fn delete(runner: &SharedRunner, s: &mut Session, p: DeleteParams) -> Result<Value, FoamError> {
    let project = s.project()?;
    let root = project.root.clone();
    let results = result_path(&root.join(RESULTS_DIR), &p.job_id);
    let job_dir = root.join("runs").join(&p.job_id);
    if !results.is_file() && !job_dir.is_dir() {
        return Err(FoamError::new("job.not_found", p.job_id));
    }
    if project
        .manifest
        .mesh
        .as_ref()
        .is_some_and(|m| m.job_id == p.job_id)
    {
        return Err(FoamError::new("runs.mesh_in_use", p.job_id));
    }
    let freed = run_bytes(&root, &p.job_id);
    // A job with its own volume (not the mesh volume of the project) frees it too.
    if let Ok(record) = JobStore::new(root.join("runs")).load(&p.job_id) {
        let _ = foamcraft_runtime::jobs::remove_job_resources(runner.as_ref(), &record);
    }
    let _ = std::fs::remove_dir_all(&job_dir);
    let _ = std::fs::remove_file(&results);
    let _ = std::fs::remove_dir_all(cache_dir(&root, &p.job_id));
    project.manifest.runs.retain(|r| r != &p.job_id);
    project.manifest.run_names.remove(&p.job_id);
    if project.manifest.view.result_job.as_deref() == Some(p.job_id.as_str()) {
        project.manifest.view.result_job = None;
        project.manifest.view.surface_field = None;
    }
    project.save()?;
    s.events().emit("project.changed", Value::Null);
    Ok(json!({"job_id": p.job_id, "freed_bytes": freed}))
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DuplicateParams {
    /// Name of the variant; it becomes a folder next to the project.
    pub name: String,
}

fn copy_tree(from: &Path, to: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(to)?;
    for e in std::fs::read_dir(from)?.flatten() {
        let path = e.path();
        if path.is_dir() {
            copy_tree(&path, &to.join(e.file_name()))?;
        } else {
            std::fs::copy(&path, to.join(e.file_name()))?;
        }
    }
    Ok(())
}

/// A copy of the project with its geometry and settings but without runs, results and mesh.
fn duplicate(s: &mut Session, p: DuplicateParams) -> Result<Value, FoamError> {
    let project = s.project()?;
    let folder: String = p
        .name
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == ' ' || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();
    let parent = project.root.parent().unwrap_or(&project.root);
    let target = (1..)
        .map(|n| {
            if n == 1 {
                parent.join(folder.trim())
            } else {
                parent.join(format!("{} {n}", folder.trim()))
            }
        })
        .find(|t| !t.exists())
        .unwrap_or_else(|| parent.join("variant"));
    let io = |e: std::io::Error| FoamError::new("project.storage_failed", e.to_string());
    std::fs::create_dir_all(&target).map_err(io)?;
    // The geometry files; runs, results and cache stay behind.
    let geometry = project.root.join("geometry");
    if geometry.is_dir() {
        copy_tree(&geometry, &target.join("geometry")).map_err(io)?;
    }
    for dir in ["runs", "cache"] {
        std::fs::create_dir_all(target.join(dir)).map_err(io)?;
    }
    let mut manifest = project.manifest.clone();
    manifest.name = p.name.clone();
    manifest.runs.clear();
    manifest.mesh = None;
    manifest.view = Default::default();
    manifest.run_names.clear();
    let text = serde_json::to_string_pretty(&manifest)
        .map_err(|e| FoamError::new("project.storage_failed", e.to_string()))?;
    std::fs::write(target.join(MANIFEST_FILE), text).map_err(io)?;
    Ok(json!({"path": target, "name": p.name}))
}

pub fn register(r: &mut Registry, runner: SharedRunner) {
    r.add(
        "results.summary",
        "All key figures of a run with unit and explanation, and the reliability light with reasons \
         (stable schema, version 1).",
        Options {
            undoable: false,
            errors: &["project.none_open", "run.no_result"],
            example: json!({}),
        },
        |s, p: JobParams| {
            let project = s.project()?;
            let (summary, _) = summary_of(project, p.job_id.as_deref())?;
            Ok(with_texts(project, summary))
        },
    );
    r.add(
        "results.compare",
        "Compare the key figures of two runs: values, difference and relative difference.",
        Options {
            undoable: false,
            errors: &["project.none_open", "run.no_result"],
            example: json!({"a": "job-1", "b": "job-2"}),
        },
        |s, p: CompareParams| compare(s.project()?, p),
    );
    r.add(
        "results.compare_many",
        "Compare the key figures of two or more runs side by side, each relative to a reference run.",
        Options {
            undoable: false,
            errors: &["project.none_open", "run.no_result", "results.compare_needs_two"],
            example: json!({"jobs": ["job-1", "job-2", "job-3"], "reference": "job-1"}),
        },
        |s, p: CompareManyParams| compare_many(s.project()?, p),
    );
    r.add(
        "results.export",
        "Write the key figures of a run as CSV or JSON.",
        Options {
            undoable: false,
            errors: &[
                "project.none_open",
                "run.no_result",
                "results.export_failed",
            ],
            example: json!({"path": "C:/Temp/figures.csv", "format": "csv"}),
        },
        |s, p: ExportParams| export(s.project()?, p),
    );
    r.add(
        "runs.list",
        "List the runs with results: name, kind, key value, reliability and disk space.",
        Options {
            undoable: false,
            errors: &["project.none_open"],
            example: json!({}),
        },
        |s, p: ListParams| Ok(list(s.project()?, p.sizes)),
    );
    r.add(
        "runs.rename",
        "Give a run a name.",
        Options {
            undoable: false,
            errors: &["project.none_open", "job.not_found"],
            example: json!({"job_id": "job-1", "name": "Spoiler flat"}),
        },
        |s, p: RenameParams| {
            let project = s.project()?;
            if !result_path(&project.root.join(RESULTS_DIR), &p.job_id).is_file() {
                return Err(FoamError::new("job.not_found", p.job_id));
            }
            project
                .manifest
                .run_names
                .insert(p.job_id.clone(), p.name.trim().to_string());
            project.save()?;
            s.events().emit("project.changed", Value::Null);
            Ok(json!({"job_id": p.job_id, "name": p.name.trim()}))
        },
    );
    r.add(
        "runs.duplicate",
        "Make a variant: a copy of the project (geometry and settings, no runs) next to it.",
        Options {
            undoable: false,
            errors: &["project.none_open", "project.storage_failed"],
            example: json!({"name": "Helmet variant"}),
        },
        duplicate,
    );
    r.add(
        "runs.delete",
        "Delete a run with its files and cache and free the disk space.",
        Options {
            undoable: false,
            errors: &["project.none_open", "job.not_found", "runs.mesh_in_use"],
            example: json!({"job_id": "job-1"}),
        },
        move |s, p: DeleteParams| delete(&runner, s, p),
    );
}
