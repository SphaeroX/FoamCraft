// SPDX-License-Identifier: GPL-3.0-or-later
//! The report as one HTML file: inline styles, images as data URIs, the chart as inline SVG,
//! no scripts and no external references.

use std::fmt::Write as _;

use base64::Engine as _;

use crate::ReportModel;

fn esc(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

const STYLE: &str = "body{font-family:Georgia,'Times New Roman',serif;max-width:880px;margin:2rem auto;padding:0 1rem;color:#1c2330;line-height:1.5}\
h1{font-size:2rem;margin:0}h2{color:#1d4f73;border-bottom:1px solid #d5dde6;padding-bottom:.2rem;margin-top:2rem}\
table{border-collapse:collapse;width:100%;margin:.5rem 0}td,th{border-bottom:1px solid #e3e8ee;padding:.3rem .5rem;text-align:left;vertical-align:top}\
td.num{text-align:right;white-space:nowrap}.cover{border:1px solid #d5dde6;border-radius:8px;padding:1.5rem;margin-bottom:1rem}\
.kind{color:#556;font-size:1.1rem}.light{display:inline-block;padding:.2rem .7rem;border-radius:1rem;color:#fff;font-weight:bold}\
.green{background:#3a9d5d}.yellow{background:#c98f0f}.red{background:#c8423a}ul.checks li.green::marker{color:#3a9d5d}\
ul.checks li.yellow::marker{color:#c98f0f}ul.checks li.red::marker{color:#c8423a}figure{margin:1rem 0}img{max-width:100%;border-radius:6px}\
figcaption{color:#556;font-size:.9rem}.small{color:#556;font-size:.85rem}footer{margin-top:3rem;color:#667;font-size:.8rem}\
svg{max-width:100%;height:auto}";

fn rows(out: &mut String, rows: &[(String, String)]) {
    out.push_str("<table>");
    for (k, v) in rows {
        let _ = write!(out, "<tr><th>{}</th><td>{}</td></tr>", esc(k), esc(v));
    }
    out.push_str("</table>");
}

fn label(m: &ReportModel, key: &str) -> String {
    m.labels
        .get(&key.replace('.', "_"))
        .cloned()
        .unwrap_or_default()
}

pub fn html(m: &ReportModel) -> String {
    let png = |b: &[u8]| base64::engine::general_purpose::STANDARD.encode(b);
    let mut o = String::from("<!DOCTYPE html>\n");
    let _ = write!(
        o,
        "<html lang=\"{}\"><head><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width, initial-scale=1\"><title>{}</title><style>{STYLE}</style></head><body>",
        m.lang,
        esc(&m.title)
    );
    o.push_str("<section class=\"cover\">");
    if let Some(logo) = &m.logo_png {
        let _ = write!(
            o,
            "<img src=\"data:image/png;base64,{}\" alt=\"\" style=\"max-height:64px\">",
            png(logo)
        );
    }
    let _ = write!(
        o,
        "<h1>{}</h1><p class=\"kind\">{}</p>",
        esc(&m.title),
        esc(&m.title_kind)
    );
    rows(&mut o, &m.cover);
    let _ = write!(
        o,
        "<p><span class=\"light {}\">{}</span></p></section>",
        m.reliability.level,
        esc(&m.reliability.label)
    );
    for s in &m.sections {
        let _ = write!(
            o,
            "<section><h2>{}</h2>",
            esc(&label(m, &format!("section.{s}")))
        );
        match s.as_str() {
            "summary" => m.summary.iter().for_each(|p| {
                let _ = write!(o, "<p>{}</p>", esc(p));
            }),
            "setup" => rows(&mut o, &m.setup),
            "mesh" => rows(&mut o, &m.mesh),
            "convergence" => {
                rows(&mut o, &m.convergence.rows);
                if let Some(svg) = &m.convergence.chart_svg {
                    let _ = write!(
                        o,
                        "<figure>{svg}<figcaption>{}</figcaption></figure>",
                        esc(&m.convergence.chart_title)
                    );
                }
                if let Some(n) = &m.convergence.note {
                    let _ = write!(o, "<p class=\"small\">{}</p>", esc(n));
                }
            }
            "figures" => {
                let _ = write!(
                    o,
                    "<table><tr><th></th><th>{}</th><th>{}</th><th>{}</th></tr>",
                    esc(&label(m, "figures.value")),
                    esc(&label(m, "figures.unit")),
                    esc(&label(m, "figures.meaning"))
                );
                for f in &m.figures {
                    let _ = write!(
                        o,
                        "<tr><th>{}</th><td class=\"num\">{}</td><td>{}</td><td>{}</td></tr>",
                        esc(&f.label),
                        esc(&f.value),
                        esc(&f.unit),
                        esc(&f.text)
                    );
                }
                o.push_str("</table>");
            }
            "views" => {
                for v in &m.views {
                    let _ = write!(
                        o,
                        "<figure><img src=\"data:image/png;base64,{}\" alt=\"{}\"><figcaption>{} – {}</figcaption></figure>",
                        png(&v.png),
                        esc(&v.title),
                        esc(&v.title),
                        esc(&v.caption)
                    );
                }
                if let Some(h) = &m.views_hint {
                    let _ = write!(o, "<p class=\"small\">{}</p>", esc(h));
                }
            }
            "reliability" => {
                let _ = write!(
                    o,
                    "<p><span class=\"light {}\">{}</span></p><ul class=\"checks\">",
                    m.reliability.level,
                    esc(&m.reliability.label)
                );
                for (level, text) in &m.reliability.checks {
                    let _ = write!(o, "<li class=\"{level}\">{}</li>", esc(text));
                }
                o.push_str("</ul>");
            }
            "comparison" => {
                if let Some(c) = &m.comparison {
                    o.push_str("<table><tr>");
                    for h in &c.header {
                        let _ = write!(o, "<th>{}</th>", esc(h));
                    }
                    o.push_str("</tr>");
                    for r in &c.rows {
                        o.push_str("<tr>");
                        for (i, cell) in r.iter().enumerate() {
                            if i == 0 {
                                let _ = write!(o, "<th>{}</th>", esc(cell));
                            } else {
                                let _ = write!(o, "<td class=\"num\">{}</td>", esc(cell));
                            }
                        }
                        o.push_str("</tr>");
                    }
                    o.push_str("</table>");
                }
            }
            "limits" => {
                o.push_str("<ul>");
                for l in &m.limits {
                    let _ = write!(o, "<li>{}</li>", esc(l));
                }
                o.push_str("</ul>");
            }
            "versions" => rows(&mut o, &m.versions),
            _ => {}
        }
        o.push_str("</section>");
    }
    let _ = write!(
        o,
        "<footer><h2>{}</h2>",
        esc(&label(m, "section.trademark"))
    );
    for t in &m.trademark {
        let _ = write!(o, "<p>{}</p>", esc(t));
    }
    o.push_str("</footer></body></html>\n");
    o
}
