// SPDX-License-Identifier: GPL-3.0-or-later
//! Reports of a run: the model from a sample result is complete in both languages, the PDF has
//! pages with the key values and the trademark notice, the HTML stands alone, missing images
//! give a hint instead of an error, and two runs give a comparison report.

use foamcraft_report::{ReportInput, ReportModel, View, build, html, pdf, pdf_text, texts};
use serde_json::{Value, json};

const OFFICIAL: &str = "This offering is not approved or endorsed by OpenCFD Limited";

fn result(job: &str, drag: f64) -> Value {
    let n = 120;
    json!({
        "job_id": job, "reason": "stable", "iterations": n, "decided_at_iteration": 110,
        "max_iterations": 900, "fluctuation_percent": 0.4,
        "coefficients": {"cd": drag / 160.0, "cl": 0.1, "cs": 0.0, "cm_pitch": 0.0, "cm_roll": 0.0, "cm_yaw": 0.0},
        "forces": {"drag_n": drag, "lift_n": 1.25, "side_n": 0.0},
        "drag_split": {"pressure_n": drag * 0.9, "viscous_n": drag * 0.1},
        "parts": [],
        "reference": {"area_m2": 0.5, "length_m": 1.0, "speed_m_s": 20.0, "density_kg_m3": 1.2, "dynamic_pressure_pa": 240.0},
        "y_plus": [{"patch": "model", "min": 1.0, "max": 90.0, "average": 45.0}],
        "hints": ["run.stable"], "restarted": false, "cells": 812345, "cores": 8,
        "timings_s": {"solve": 10.0},
        "scenario": {"speed_m_s": 20.0, "direction_deg": 0.0, "ground": "fixed",
                     "medium": {"kind": "air", "temperature_c": 20.0}, "quality": "draft"},
        "mesh": {"job_id": "job-mesh", "cells": 812345, "verdict": "good", "layers_target": 3, "layers_achieved": 2.6, "fallbacks": []},
        "history": {"iterations": (1..=n).collect::<Vec<_>>(),
                    "cd": (1..=n).map(|i| drag / 160.0 + 0.2 * (-(i as f64) / 15.0).exp()).collect::<Vec<_>>(),
                    "cl": (1..=n).map(|_| 0.1).collect::<Vec<_>>(),
                    "residual_iterations": [], "residuals": {}}
    })
}

fn summary(job: &str, drag: f64) -> Value {
    let t =
        |en: &str, de: &str| json!({"en": {"text": en, "fix": ""}, "de": {"text": de, "fix": ""}});
    let fig = |id: &str, value: f64, unit: &str, en: &str, de: &str| {
        let mut f = json!({"id": id, "value": value, "unit": unit, "part": null});
        let tt = t(en, de);
        f["en"] = tt["en"].clone();
        f["de"] = tt["de"].clone();
        f
    };
    let check = |code: &str, level: &str, en: &str, de: &str| {
        let mut c = json!({"code": code, "level": level});
        let tt = t(en, de);
        c["en"] = tt["en"].clone();
        c["de"] = tt["de"].clone();
        c
    };
    json!({
        "schema": 1, "job_id": job, "name": format!("Run {job}"), "kind": "wind_tunnel",
        "figures": [
            fig("cd", drag / 160.0, "-", "Drag coefficient.", "Widerstandsbeiwert."),
            fig("drag_n", drag, "N", "Drag: force along the flow.", "Widerstand: Kraft in Strömungsrichtung."),
            fig("lift_n", 1.25, "N", "Lift.", "Auftrieb."),
            fig("power_w", drag * 20.0, "W", "Power needed.", "Leistungsbedarf."),
        ],
        "reliability": {"level": "yellow", "checks": [
            check("reliability.converged", "green", "The values settled.", "Die Werte haben sich eingependelt."),
            check("reliability.mesh_acceptable", "yellow", "The mesh quality is acceptable.", "Die Netzqualität ist annehmbar."),
        ]}
    })
}

fn input(lang: &str, views: Vec<View>) -> ReportInput {
    ReportInput {
        language: lang.into(),
        title: "Helmet study".into(),
        author: "A. Tester".into(),
        date: "2026-11-21".into(),
        project: "Helmet".into(),
        run_name: "Run 1".into(),
        result: result("job-1", 48.0),
        summary: summary("job-1", 48.0),
        models: vec![("helmet.stl".into(), [0.3, 0.22, 0.18])],
        views,
        surface_field: "p".into(),
        logo_png: None,
        sections: None,
        compare: None,
        app_version: "0.1.0".into(),
        solver_version: "OpenFOAM v2606".into(),
    }
}

fn png_1x1() -> Vec<u8> {
    // A valid 1 × 1 RGBA PNG.
    let mut out = Vec::new();
    {
        let mut enc = png::Encoder::new(&mut out, 1, 1);
        enc.set_color(png::ColorType::Rgba);
        enc.set_depth(png::BitDepth::Eight);
        enc.write_header()
            .unwrap()
            .write_image_data(&[10, 120, 200, 255])
            .unwrap();
    }
    out
}

fn views() -> Vec<View> {
    ["front", "side", "top"]
        .iter()
        .map(|k| View {
            key: (*k).into(),
            png: png_1x1(),
        })
        .collect()
}

fn complete(m: &ReportModel) {
    assert!(!m.summary.is_empty());
    assert!(m.setup.len() >= 5, "{:?}", m.setup);
    assert!(m.mesh.len() >= 3);
    assert!(
        m.convergence
            .chart_svg
            .as_deref()
            .is_some_and(|s| s.contains("<svg"))
    );
    assert_eq!(m.figures.len(), 4);
    assert!(m.figures.iter().all(|f| !f.text.is_empty()));
    assert_eq!(m.reliability.level, "yellow");
    assert_eq!(m.reliability.checks.len(), 2);
    assert!(m.limits.len() >= 5);
    assert!(m.versions.iter().any(|(_, v)| v.contains("v2606")));
    assert!(m.trademark.iter().any(|t| t.contains(OFFICIAL)));
}

#[test]
fn the_model_is_complete_in_both_languages() {
    let en = build(&input("en", views()));
    complete(&en);
    assert!(en.summary[0].contains("48"), "{:?}", en.summary);
    let de = build(&input("de", views()));
    complete(&de);
    assert!(de.summary[0].contains("Widerstand"));
    assert_eq!(
        de.figures[1].text,
        "Widerstand: Kraft in Strömungsrichtung."
    );
    // Every text exists in both languages.
    let (en_keys, de_keys) = (texts("en"), texts("de"));
    let mut a: Vec<_> = en_keys.keys().collect();
    let mut b: Vec<_> = de_keys.keys().collect();
    a.sort();
    b.sort();
    assert_eq!(a, b);
    assert!(
        en_keys
            .values()
            .chain(de_keys.values())
            .all(|v| !v.is_empty())
    );
}

#[test]
fn the_pdf_has_pages_key_values_light_limits_and_the_notice() {
    for lang in ["en", "de"] {
        let model = build(&input(lang, views()));
        let bytes = pdf(&model).unwrap();
        assert_eq!(&bytes[..5], b"%PDF-");
        let (pages, text) = pdf_text(&model).unwrap();
        assert!(pages >= 2, "{pages}");
        let text = text.replace('\u{a0}', " ");
        assert!(text.contains("48"), "{lang}");
        assert!(text.contains("OpenCFD Limited"), "{lang}");
        let (light, limits) = if lang == "en" {
            ("use with care", "Limits of the results")
        } else {
            ("mit Vorsicht nutzen", "Grenzen der Aussagekraft")
        };
        assert!(text.contains(light), "{lang}: {text}");
        assert!(text.contains(limits), "{lang}");
    }
}

#[test]
fn the_html_stands_alone() {
    let page = html(&build(&input("en", views())));
    assert!(page.starts_with("<!DOCTYPE html>"));
    for bad in [
        "src=\"http",
        "href=\"http",
        "url(http",
        "@import",
        "<script src",
        "<link ",
    ] {
        assert!(!page.contains(bad), "{bad}");
    }
    assert_eq!(page.matches("<img").count(), 3);
    assert_eq!(page.matches("<img src=\"data:image/png;base64,").count(), 3);
    assert!(page.contains("<svg"));
    assert!(page.contains(OFFICIAL));
    assert!(page.contains("use with care"));
}

#[test]
fn without_images_the_report_has_a_hint() {
    let model = build(&input("en", vec![]));
    assert!(model.views.is_empty());
    assert!(
        model
            .views_hint
            .as_deref()
            .is_some_and(|h| h.starts_with("No images"))
    );
    let page = html(&model);
    assert!(!page.contains("<img"));
    assert!(page.contains("No images"));
    assert!(pdf(&model).is_ok());
}

#[test]
fn two_runs_give_a_comparison_report() {
    let mut i = input("en", vec![]);
    i.compare = Some(foamcraft_report::CompareInput {
        run_name: "Run 2".into(),
        rows: json!([
            {"id": "drag_n", "part": null, "unit": "N", "a": 48.0, "b": 43.2, "difference": -4.8, "relative": -0.1},
            {"id": "cd", "part": null, "unit": "-", "a": 0.3, "b": 0.27, "difference": -0.03, "relative": -0.1}
        ]),
        summary_b: summary("job-2", 43.2),
    });
    let model = build(&i);
    let cmp = model.comparison.as_ref().unwrap();
    assert_eq!(cmp.rows.len(), 2);
    assert_eq!(model.title_kind, "Comparison report");
    let page = html(&model);
    assert!(page.contains("Run 2") && page.contains("−10"));
    let (_, text) = pdf_text(&model).unwrap();
    assert!(text.contains("Run 2"));
}

#[test]
fn numbers_follow_the_language() {
    use foamcraft_report::number;
    assert_eq!(number(519_195.0, "en"), "519,195");
    assert_eq!(number(519_195.0, "de"), "519.195");
    assert_eq!(number(0.7772, "de"), "0,7772");
    assert_eq!(number(-0.002791, "en"), "−0.002791");
    assert_eq!(number(-4.611e-4, "de"), "−4,611e−4");
    assert_eq!(number(5.034e-4, "en"), "5.034e−4");
    assert_eq!(number(18.3, "en"), "18.3");
}
