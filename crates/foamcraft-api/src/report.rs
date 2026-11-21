// SPDX-License-Identifier: GPL-3.0-or-later
//! `export.report`: a report of a run (or two runs) as PDF and/or self-contained HTML, in English
//! or German. Key figures come from `results.summary`; the standard views are rendered offscreen
//! when the result fields of the run are in the cache, otherwise the report says so.

use std::path::PathBuf;

use foamcraft_core::command::{Options, Registry};
use foamcraft_core::error::FoamError;
use foamcraft_core::project::Project;
use foamcraft_report::{CompareInput, ReportInput, View, build};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};

use crate::solve::{RESULTS_DIR, load_result};

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ReportParams {
    /// File name of the report; the extension is set per format (`.pdf`, `.html`).
    pub path: PathBuf,
    /// `pdf` and/or `html`; default both.
    #[serde(default)]
    pub formats: Option<Vec<String>>,
    /// `en` (default) or `de`.
    #[serde(default)]
    pub language: Option<String>,
    /// Run; default the newest result.
    #[serde(default)]
    pub job_id: Option<String>,
    /// Second run for a comparison report.
    #[serde(default)]
    pub compare_with: Option<String>,
    /// Default: the project name.
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub author: Option<String>,
    /// PNG shown on the cover.
    #[serde(default)]
    pub logo_path: Option<PathBuf>,
    /// Sections to include (summary, setup, mesh, convergence, figures, views, reliability,
    /// comparison, limits, versions); default all.
    #[serde(default)]
    pub sections: Option<Vec<String>>,
}

/// Today as YYYY-MM-DD (UTC).
fn today() -> String {
    let days = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs() / 86_400) as i64;
    // Civil date from days since 1970-01-01 (proleptic Gregorian).
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    format!("{y:04}-{m:02}-{d:02}")
}

/// Sizes of the placed models, metres.
fn models(project: &Project) -> Vec<(String, [f64; 3])> {
    use foamcraft_geometry::commands::placement;
    foamcraft_geometry::surfaces::segmented_models(project)
        .unwrap_or_default()
        .iter()
        .map(|m| {
            let world = placement(&m.entry).apply(&m.mesh);
            let (mut lo, mut hi) = ([f64::MAX; 3], [f64::MIN; 3]);
            for p in &world.positions {
                for k in 0..3 {
                    lo[k] = lo[k].min(f64::from(p[k]));
                    hi[k] = hi[k].max(f64::from(p[k]));
                }
            }
            (
                m.entry.name.clone(),
                [0, 1, 2].map(|k| (hi[k] - lo[k]).max(0.0)),
            )
        })
        .collect()
}

/// Front, side and top view coloured by the surface field; none without result fields or GPU.
fn views(project: &Project, job: &str) -> (Vec<View>, String) {
    let field = project
        .manifest
        .view
        .surface_field
        .clone()
        .unwrap_or_else(|| "p".into());
    if crate::results::load_meta(&project.root, job).is_err() {
        return (Vec::new(), field);
    }
    let mut p = project.clone();
    p.manifest.view.result_job = Some(job.to_string());
    p.manifest.view.surface_field = Some(field.clone());
    p.manifest.view.compare = None;
    p.manifest.view.particles = None;
    let shots = [
        ("front", 0.0, None),
        ("side", -30f32, Some(0.05f32)),
        ("top", 60.0, Some(1.35)),
    ];
    let views = shots
        .iter()
        .filter_map(|(key, yaw_deg, pitch)| {
            let png = crate::export::render_view_png(&p, 1600, 1000, yaw_deg.to_radians(), *pitch)
                .ok()?;
            Some(View {
                key: (*key).into(),
                png,
            })
        })
        .collect();
    (views, field)
}

fn report(project: &Project, p: ReportParams) -> Result<Value, FoamError> {
    let results = project.root.join(RESULTS_DIR);
    let result = load_result(&results, p.job_id.as_deref())?;
    let job = result["job_id"].as_str().unwrap_or_default().to_string();
    let summary = crate::runs::summary_json(project, &job)?;
    let compare = match &p.compare_with {
        Some(b) => Some(CompareInput {
            run_name: crate::runs::run_name(project, b),
            rows: serde_json::to_value(
                crate::runs::compare(
                    project,
                    crate::runs::CompareParams {
                        a: job.clone(),
                        b: b.clone(),
                    },
                )?
                .rows,
            )
            .unwrap_or_default(),
            summary_b: crate::runs::summary_json(project, b)?,
        }),
        None => None,
    };
    let logo_png = match &p.logo_path {
        Some(path) => Some(
            std::fs::read(path)
                .map_err(|e| FoamError::new("export.invalid", format!("logo: {e}")))?,
        ),
        None => None,
    };
    let (views, surface_field) = views(project, &job);
    let input = ReportInput {
        language: p.language.clone().unwrap_or_else(|| "en".into()),
        title: p
            .title
            .clone()
            .unwrap_or_else(|| project.manifest.name.clone()),
        author: p.author.clone().unwrap_or_default(),
        date: today(),
        project: project.manifest.name.clone(),
        run_name: crate::runs::run_name(project, &job),
        result,
        summary,
        models: models(project),
        views,
        surface_field,
        logo_png,
        sections: p.sections.clone(),
        compare,
        app_version: foamcraft_core::VERSION.to_string(),
        solver_version: format!(
            "OpenFOAM {} ({})",
            foamcraft_runtime::OPENFOAM_RELEASE,
            foamcraft_runtime::IMAGE_TAG
        ),
    };
    let model = build(&input);
    let formats = p
        .formats
        .clone()
        .unwrap_or_else(|| vec!["pdf".into(), "html".into()]);
    let failed = |e: String| FoamError::new("export.report_failed", e);
    let mut out = json!({"pdf": null, "html": null, "images": model.views.len()});
    for format in &formats {
        let path = p.path.with_extension(format);
        let bytes = match format.as_str() {
            "pdf" => foamcraft_report::pdf(&model).map_err(failed)?,
            "html" => foamcraft_report::html(&model).into_bytes(),
            other => return Err(FoamError::new("export.invalid", format!("format {other}"))),
        };
        std::fs::write(&path, bytes)
            .map_err(|e| FoamError::new("view.write_failed", e.to_string()))?;
        out[format.as_str()] = json!(path);
    }
    Ok(out)
}

pub fn register(r: &mut Registry) {
    r.add(
        "export.report",
        "Write a report of a run (or a comparison of two runs) as PDF and/or self-contained HTML in \
         English or German: setup, mesh, convergence, key figures with explanations, views, \
         reliability light, limits of the results and the trademark notice.",
        Options {
            undoable: false,
            errors: &[
                "project.none_open",
                "run.no_result",
                "export.invalid",
                "export.report_failed",
                "view.write_failed",
            ],
            example: json!({"path": "C:/Temp/report", "language": "de", "formats": ["pdf", "html"]}),
        },
        |s, p: ReportParams| report(s.project()?, p),
    );
}
