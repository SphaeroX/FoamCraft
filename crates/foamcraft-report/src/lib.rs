// SPDX-License-Identifier: GPL-3.0-or-later
//! Reports of FoamCraft runs. [`build`] turns a stored result, its key figures
//! (`results.summary`, schema 1) and rendered views into a [`ReportModel`]; [`pdf`] lays it out
//! with an own Typst template and the embedded Libertinus Serif font, [`html`] writes one
//! self-contained HTML file (images inline, no external references). English and German.

mod chart;
mod html;
mod typst_pdf_out;

use std::collections::BTreeMap;

use serde::Serialize;
use serde_json::Value;

pub use html::html;
pub use typst_pdf_out::{pdf, pdf_text};

/// Sections in report order; `sections` in [`ReportInput`] picks a subset.
pub const SECTIONS: &[&str] = &[
    "summary",
    "setup",
    "mesh",
    "convergence",
    "figures",
    "views",
    "reliability",
    "comparison",
    "limits",
    "versions",
];

/// A rendered standard view (`front`, `side` or `top`) as PNG.
#[derive(Debug, Clone)]
pub struct View {
    pub key: String,
    pub png: Vec<u8>,
}

/// The second run of a comparison report.
#[derive(Debug, Clone)]
pub struct CompareInput {
    pub run_name: String,
    /// Rows of `results.compare`.
    pub rows: Value,
    /// `results.summary` of the second run.
    pub summary_b: Value,
}

/// Everything a report is made from.
#[derive(Debug, Clone)]
pub struct ReportInput {
    /// `en` or `de`.
    pub language: String,
    pub title: String,
    pub author: String,
    /// Date as shown, for example `2026-11-21`.
    pub date: String,
    pub project: String,
    pub run_name: String,
    /// The stored result of the run (`results/<job>.json`).
    pub result: Value,
    /// `results.summary` of the run (with `en` and `de` texts).
    pub summary: Value,
    /// Models: file name and size in metres.
    pub models: Vec<(String, [f64; 3])>,
    pub views: Vec<View>,
    /// Surface field the views are coloured by.
    pub surface_field: String,
    pub logo_png: Option<Vec<u8>>,
    /// Sections to include (default: all).
    pub sections: Option<Vec<String>>,
    pub compare: Option<CompareInput>,
    pub app_version: String,
    pub solver_version: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct FigureRow {
    pub label: String,
    pub value: String,
    pub unit: String,
    pub text: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct Reliability {
    /// `green`, `yellow` or `red`.
    pub level: String,
    pub label: String,
    /// Level and reason of every check.
    pub checks: Vec<(String, String)>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Convergence {
    pub rows: Vec<(String, String)>,
    pub chart_title: String,
    pub chart_svg: Option<String>,
    pub note: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ViewModel {
    pub key: String,
    pub title: String,
    pub caption: String,
    #[serde(skip)]
    pub png: Vec<u8>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ComparisonModel {
    pub name_a: String,
    pub name_b: String,
    pub header: Vec<String>,
    pub rows: Vec<Vec<String>>,
}

/// The finished content of a report, ready for PDF and HTML.
#[derive(Debug, Clone, Serialize)]
pub struct ReportModel {
    pub lang: String,
    pub title: String,
    /// "Simulation report" or "Comparison report".
    pub title_kind: String,
    pub cover: Vec<(String, String)>,
    pub labels: BTreeMap<String, String>,
    /// Sections shown, in order.
    pub sections: Vec<String>,
    pub summary: Vec<String>,
    pub setup: Vec<(String, String)>,
    pub mesh: Vec<(String, String)>,
    pub convergence: Convergence,
    pub figures: Vec<FigureRow>,
    pub views: Vec<ViewModel>,
    pub views_hint: Option<String>,
    pub reliability: Reliability,
    pub comparison: Option<ComparisonModel>,
    pub limits: Vec<String>,
    pub versions: Vec<(String, String)>,
    /// Notice in the language of the report, and the official English one.
    pub trademark: Vec<String>,
    #[serde(skip)]
    pub logo_png: Option<Vec<u8>>,
}

/// The texts of the report in one language (key → text).
pub fn texts(lang: &str) -> BTreeMap<String, String> {
    static ALL: std::sync::OnceLock<Value> = std::sync::OnceLock::new();
    let all = ALL.get_or_init(|| {
        serde_json::from_str(include_str!("../data/texts.json")).unwrap_or(Value::Null)
    });
    all[if lang == "de" { "de" } else { "en" }]
        .as_object()
        .map(|m| {
            m.iter()
                .map(|(k, v)| (k.clone(), v.as_str().unwrap_or_default().to_string()))
                .collect()
        })
        .unwrap_or_default()
}

/// Short labels of the key figures, shared with the interface (`figures.label.<id>`).
fn figure_label(lang: &str, id: &str) -> String {
    static EN: std::sync::OnceLock<Value> = std::sync::OnceLock::new();
    static DE: std::sync::OnceLock<Value> = std::sync::OnceLock::new();
    let dict = if lang == "de" {
        DE.get_or_init(|| {
            serde_json::from_str(include_str!("../../../app/src/i18n/de.json"))
                .unwrap_or(Value::Null)
        })
    } else {
        EN.get_or_init(|| {
            serde_json::from_str(include_str!("../../../app/src/i18n/en.json"))
                .unwrap_or(Value::Null)
        })
    };
    dict["figures"]["label"][id]
        .as_str()
        .map_or_else(|| id.to_string(), String::from)
}

/// A number with about four significant digits in the style of the language (decimal comma and
/// thousands dot in German, typographic minus).
pub fn number(v: f64, lang: &str) -> String {
    let a = v.abs();
    // English style first: decimal point, thousands comma.
    let body = if a != 0.0 && !(1e-3..1e7).contains(&a) {
        format!("{a:.3e}")
    } else if a >= 1e4 {
        let digits = format!("{a:.0}");
        let mut out = String::new();
        for (i, c) in digits.chars().enumerate() {
            if i > 0 && (digits.len() - i) % 3 == 0 {
                out.push(',');
            }
            out.push(c);
        }
        out
    } else {
        let decimals = if a == 0.0 {
            0
        } else {
            (3 - a.log10().floor() as i32).clamp(0, 6) as usize
        };
        let s = format!("{a:.decimals$}");
        if s.contains('.') {
            s.trim_end_matches('0').trim_end_matches('.').to_string()
        } else {
            s
        }
    };
    let body = body.replace("e-", "e−");
    let body = if lang == "de" {
        body.chars()
            .map(|c| match c {
                '.' => ',',
                ',' => '.',
                c => c,
            })
            .collect()
    } else {
        body
    };
    let zero = body.chars().all(|c| matches!(c, '0' | '.' | ','));
    if v < 0.0 && !zero {
        format!("−{body}")
    } else {
        body
    }
}

fn fill(template: &str, values: &[(&str, String)]) -> String {
    values.iter().fold(template.to_string(), |s, (k, v)| {
        s.replace(&format!("{{{k}}}"), v)
    })
}

fn f(v: &Value, path: &[&str]) -> Option<f64> {
    path.iter().try_fold(v, |v, k| v.get(*k))?.as_f64()
}

fn s<'a>(v: &'a Value, path: &[&str]) -> Option<&'a str> {
    path.iter().try_fold(v, |v, k| v.get(*k))?.as_str()
}

fn figure_value(summary: &Value, id: &str) -> Option<f64> {
    summary["figures"]
        .as_array()?
        .iter()
        .find(|f| f["id"] == id && f["part"].is_null())?["value"]
        .as_f64()
}

pub fn build(input: &ReportInput) -> ReportModel {
    let lang = if input.language == "de" { "de" } else { "en" };
    let t = texts(lang);
    let tr = |k: &str| t.get(k).cloned().unwrap_or_else(|| k.to_string());
    let num = |v: f64| number(v, lang);
    let r = &input.result;
    let sm = &input.summary;
    let kind = s(sm, &["kind"]).unwrap_or("wind_tunnel");
    let internal = kind == "internal_flow";

    // Reliability.
    let level = s(sm, &["reliability", "level"])
        .unwrap_or("red")
        .to_string();
    let checks: Vec<(String, String)> = sm["reliability"]["checks"]
        .as_array()
        .cloned()
        .unwrap_or_default()
        .iter()
        .map(|c| {
            (
                c["level"].as_str().unwrap_or("red").to_string(),
                c[lang]["text"].as_str().unwrap_or_default().to_string(),
            )
        })
        .collect();
    let reliability = Reliability {
        label: tr(&format!("light.{level}")),
        level: level.clone(),
        checks: checks.clone(),
    };

    // Summary in plain words.
    let iterations = f(r, &["iterations"]).unwrap_or(0.0);
    let mut summary = Vec::new();
    if internal {
        summary.push(fill(
            &tr("summary.internal"),
            &[
                (
                    "loss",
                    num(figure_value(sm, "pressure_loss_pa").unwrap_or(0.0)),
                ),
                (
                    "zeta",
                    num(figure_value(sm, "loss_coefficient").unwrap_or(0.0)),
                ),
            ],
        ));
    } else {
        summary.push(fill(
            &tr("summary.wind"),
            &[
                (
                    "speed",
                    num(f(r, &["scenario", "speed_m_s"]).unwrap_or(0.0)),
                ),
                (
                    "direction",
                    num(f(r, &["scenario", "direction_deg"]).unwrap_or(0.0)),
                ),
                ("drag", num(figure_value(sm, "drag_n").unwrap_or(0.0))),
                ("cd", num(figure_value(sm, "cd").unwrap_or(0.0))),
                ("lift", num(figure_value(sm, "lift_n").unwrap_or(0.0))),
                ("power", num(figure_value(sm, "power_w").unwrap_or(0.0))),
            ],
        ));
    }
    let settled = s(r, &["reason"]) == Some("stable");
    summary.push(fill(
        &tr(if settled {
            "summary.converged"
        } else {
            "summary.not_converged"
        }),
        &[("iterations", num(iterations))],
    ));
    let reasons: Vec<String> = checks
        .iter()
        .filter(|(l, _)| l != "green")
        .map(|(_, text)| text.clone())
        .collect();
    summary.push(
        fill(
            &tr("summary.light"),
            &[
                ("light", reliability.label.clone()),
                ("reasons", reasons.join(" ")),
            ],
        )
        .trim()
        .to_string(),
    );

    // Setup.
    let mut setup = vec![(tr("setup.kind"), tr(&format!("kind.{kind}")))];
    for (name, size) in &input.models {
        setup.push((
            format!("{} – {}", tr("setup.models"), name),
            format!("{} × {} × {} m", num(size[0]), num(size[1]), num(size[2])),
        ));
    }
    let sc = &r["scenario"];
    if !internal {
        if let Some(v) = f(sc, &["speed_m_s"]) {
            setup.push((tr("setup.speed"), format!("{} m/s", num(v))));
        }
        if let Some(v) = f(sc, &["direction_deg"]) {
            setup.push((tr("setup.direction"), format!("{} °", num(v))));
        }
        if let Some(g) = s(sc, &["ground"]) {
            setup.push((tr("setup.ground"), tr(&format!("ground.{g}"))));
        }
        if let Some(a) = f(r, &["reference", "area_m2"]) {
            setup.push((tr("setup.reference_area"), format!("{} m²", num(a))));
        }
    }
    if let Some(m) = s(sc, &["medium", "kind"]) {
        setup.push((tr("setup.medium"), tr(&format!("medium.{m}"))));
    }
    if let Some(v) = f(sc, &["medium", "temperature_c"]) {
        setup.push((tr("setup.temperature"), format!("{} °C", num(v))));
    }
    if let Some(q) = s(sc, &["quality"]) {
        setup.push((tr("setup.quality"), tr(&format!("quality.{q}"))));
    }

    // Mesh.
    let m = &r["mesh"];
    let mut mesh = Vec::new();
    if let Some(c) = f(m, &["cells"]).or(f(r, &["cells"])) {
        mesh.push((tr("mesh.cells"), num(c)));
    }
    if let Some(v) = s(m, &["verdict"]) {
        mesh.push((tr("mesh.verdict"), tr(&format!("verdict.{v}"))));
    }
    if let (Some(a), Some(b)) = (f(m, &["layers_target"]), f(m, &["layers_achieved"])) {
        mesh.push((tr("mesh.layers"), format!("{}, {}", num(a), num(b))));
    }
    let fallbacks = m["fallbacks"].as_array().map_or(0, Vec::len);
    mesh.push((tr("mesh.fallbacks"), num(fallbacks as f64)));

    // Convergence.
    let its: Vec<f64> = r["history"]["iterations"]
        .as_array()
        .map(|a| a.iter().filter_map(Value::as_f64).collect())
        .unwrap_or_default();
    let (series_key, chart_key) = if internal {
        ("pressure_loss", "convergence.chart")
    } else {
        ("cd", "convergence.chart")
    };
    let ys: Vec<f64> = r["history"][series_key]
        .as_array()
        .map(|a| a.iter().filter_map(Value::as_f64).collect())
        .unwrap_or_default();
    let chart_svg = (its.len() >= 2 && ys.len() >= 2).then(|| chart::line_chart(&its, &ys, lang));
    let mut rows = vec![(tr("convergence.iterations"), num(iterations))];
    if let Some(d) = f(r, &["decided_at_iteration"]) {
        rows.push((tr("convergence.decided"), num(d)));
    }
    if let Some(fl) = f(r, &["fluctuation_percent"]) {
        rows.push((tr("convergence.fluctuation"), format!("{} %", num(fl))));
    }
    let convergence = Convergence {
        rows,
        chart_title: tr(chart_key),
        note: chart_svg.is_none().then(|| tr("convergence.no_history")),
        chart_svg,
    };

    // Key figures.
    let figure_rows = |sm: &Value| -> Vec<FigureRow> {
        sm["figures"]
            .as_array()
            .cloned()
            .unwrap_or_default()
            .iter()
            .map(|fg| {
                let id = fg["id"].as_str().unwrap_or_default();
                let mut label = figure_label(lang, id);
                if let Some(p) = fg["part"].as_str() {
                    label = format!("{label} ({p})");
                }
                let unit = fg["unit"].as_str().unwrap_or_default();
                FigureRow {
                    label,
                    value: num(fg["value"].as_f64().unwrap_or(0.0)),
                    unit: if unit == "-" {
                        String::new()
                    } else {
                        unit.to_string()
                    },
                    text: fg[lang]["text"].as_str().unwrap_or_default().to_string(),
                }
            })
            .collect()
    };
    let figures = figure_rows(sm);

    // Views.
    let field = figure_label(lang, &input.surface_field);
    let field_name = if field == input.surface_field {
        input.surface_field.clone()
    } else {
        field
    };
    let views: Vec<ViewModel> = input
        .views
        .iter()
        .map(|v| ViewModel {
            key: v.key.clone(),
            title: tr(&format!("views.{}", v.key)),
            caption: fill(&tr("views.caption"), &[("field", field_name.clone())]),
            png: v.png.clone(),
        })
        .collect();
    let views_hint = views.is_empty().then(|| tr("views.hint"));

    // Comparison.
    let comparison = input.compare.as_ref().map(|c| {
        let rows = c
            .rows
            .as_array()
            .cloned()
            .unwrap_or_default()
            .iter()
            .map(|row| {
                let id = row["id"].as_str().unwrap_or_default();
                let mut label = figure_label(lang, id);
                if let Some(p) = row["part"].as_str() {
                    label = format!("{label} ({p})");
                }
                let unit = row["unit"].as_str().filter(|u| *u != "-").unwrap_or("");
                let cell = |k: &str| row[k].as_f64().map_or("–".to_string(), num);
                vec![
                    if unit.is_empty() {
                        label
                    } else {
                        format!("{label} [{unit}]")
                    },
                    cell("a"),
                    cell("b"),
                    cell("difference"),
                    row["relative"]
                        .as_f64()
                        .map_or("–".to_string(), |x| format!("{} %", num(x * 100.0))),
                ]
            })
            .collect();
        ComparisonModel {
            name_a: input.run_name.clone(),
            name_b: c.run_name.clone(),
            header: vec![
                String::new(),
                format!("{} ({})", tr("comparison.value_a"), input.run_name),
                format!("{} ({})", tr("comparison.value_b"), c.run_name),
                tr("comparison.difference"),
                tr("comparison.relative"),
            ],
            rows,
        }
    });

    let limits = ["rans", "turbulence", "wall", "mesh", "blunt", "check"]
        .iter()
        .map(|k| tr(&format!("limits.{k}")))
        .collect();
    let versions = vec![
        (tr("versions.app"), input.app_version.clone()),
        (tr("versions.solver"), input.solver_version.clone()),
        (tr("versions.created"), input.date.clone()),
    ];
    let mut trademark = vec![tr("trademark")];
    if lang != "en" {
        trademark.push(texts("en")["trademark"].clone());
    }

    let wanted: Vec<String> = input
        .sections
        .clone()
        .unwrap_or_else(|| SECTIONS.iter().map(|s| s.to_string()).collect());
    let sections = SECTIONS
        .iter()
        .filter(|s| wanted.iter().any(|w| w == *s))
        .filter(|s| **s != "comparison" || comparison.is_some())
        .map(|s| s.to_string())
        .collect();

    let mut cover = vec![
        (tr("project"), input.project.clone()),
        (
            tr("run"),
            match &input.compare {
                Some(c) => format!("{} / {}", input.run_name, c.run_name),
                None => input.run_name.clone(),
            },
        ),
        (tr("date"), input.date.clone()),
    ];
    if !input.author.trim().is_empty() {
        cover.push((tr("author"), input.author.clone()));
    }
    let labels = t
        .iter()
        .filter(|(k, _)| k.starts_with("section.") || k.starts_with("figures.") || *k == "contents")
        .map(|(k, v)| (k.replace('.', "_"), v.clone()))
        .collect();

    ReportModel {
        lang: lang.into(),
        title: input.title.clone(),
        title_kind: tr(if input.compare.is_some() {
            "comparison_report"
        } else {
            "report"
        }),
        cover,
        labels,
        sections,
        summary,
        setup,
        mesh,
        convergence,
        figures,
        views,
        views_hint,
        reliability,
        comparison,
        limits,
        versions,
        trademark,
        logo_png: input.logo_png.clone(),
    }
}
