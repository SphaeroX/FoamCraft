// SPDX-License-Identifier: GPL-3.0-or-later
//! A small SVG line chart (convergence history) for PDF and HTML alike.

use std::fmt::Write as _;

/// "Nice" tick values covering lo..hi.
fn ticks(lo: f64, hi: f64) -> Vec<f64> {
    let span = (hi - lo).abs().max(1e-12);
    let raw = span / 4.0;
    let mag = 10f64.powf(raw.log10().floor());
    let step = [1.0, 2.0, 5.0, 10.0]
        .iter()
        .map(|m| m * mag)
        .find(|s| *s >= raw)
        .unwrap_or(10.0 * mag);
    let mut t = (lo / step).ceil() * step;
    let mut out = Vec::new();
    while t <= hi + step * 1e-9 && out.len() < 12 {
        out.push(t);
        t += step;
    }
    out
}

/// Line chart of y over x, 640 × 260 user units, axes with labelled ticks.
pub fn line_chart(x: &[f64], y: &[f64], lang: &str) -> String {
    let n = x.len().min(y.len());
    let (x, y) = (&x[..n], &y[..n]);
    let (x0, x1) = (x[0], x[n - 1].max(x[0] + 1e-12));
    // Skip the first tenth when it dwarfs the rest (start-up of the solver).
    let tail = &y[n / 10..];
    let (mut y0, mut y1) = tail
        .iter()
        .fold((f64::MAX, f64::MIN), |(a, b), v| (a.min(*v), b.max(*v)));
    let pad = ((y1 - y0) * 0.1).max(y1.abs() * 0.02).max(1e-9);
    y0 -= pad;
    y1 += pad;
    let (w, h, left, bottom, top, right) = (640.0, 260.0, 64.0, 34.0, 12.0, 16.0);
    let px = |v: f64| left + (v - x0) / (x1 - x0) * (w - left - right);
    let py = |v: f64| top + (1.0 - (v.clamp(y0, y1) - y0) / (y1 - y0)) * (h - top - bottom);
    let mut svg = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 {w} {h}\" width=\"{w}\" height=\"{h}\" font-family=\"serif\" font-size=\"11\">"
    );
    let _ = write!(
        svg,
        "<rect x=\"0\" y=\"0\" width=\"{w}\" height=\"{h}\" fill=\"#ffffff\"/>"
    );
    for t in ticks(y0, y1) {
        let yy = py(t);
        let _ = write!(
            svg,
            "<line x1=\"{left}\" y1=\"{yy:.1}\" x2=\"{:.1}\" y2=\"{yy:.1}\" stroke=\"#dde3ea\"/><text x=\"{:.1}\" y=\"{:.1}\" text-anchor=\"end\" fill=\"#445\">{}</text>",
            w - right,
            left - 6.0,
            yy + 4.0,
            crate::number(t, lang)
        );
    }
    for t in ticks(x0, x1) {
        let xx = px(t);
        let _ = write!(
            svg,
            "<line x1=\"{xx:.1}\" y1=\"{top}\" x2=\"{xx:.1}\" y2=\"{:.1}\" stroke=\"#eef1f5\"/><text x=\"{xx:.1}\" y=\"{:.1}\" text-anchor=\"middle\" fill=\"#445\">{}</text>",
            h - bottom,
            h - bottom + 16.0,
            crate::number(t, lang)
        );
    }
    let _ = write!(
        svg,
        "<rect x=\"{left}\" y=\"{top}\" width=\"{:.1}\" height=\"{:.1}\" fill=\"none\" stroke=\"#667\"/>",
        w - left - right,
        h - top - bottom
    );
    let step = (n / 800).max(1);
    let points: Vec<String> = (0..n)
        .step_by(step)
        .map(|i| format!("{:.1},{:.1}", px(x[i]), py(y[i])))
        .collect();
    let _ = write!(
        svg,
        "<polyline fill=\"none\" stroke=\"#1d6fa5\" stroke-width=\"1.6\" points=\"{}\"/></svg>",
        points.join(" ")
    );
    svg
}
