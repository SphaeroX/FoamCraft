// SPDX-License-Identifier: GPL-3.0-or-later
//! Drawing on finished frames: the colour legend (gradient bar, range and unit) at the bottom
//! right, with a small built-in 5 × 7 pixel font for numbers and units (no font files needed).

/// Rows of a 5 × 7 glyph, top first; bit 4 is the left column.
fn glyph(c: char) -> Option<[u8; 7]> {
    Some(match c {
        '0' => [0x0E, 0x11, 0x13, 0x15, 0x19, 0x11, 0x0E],
        '1' => [0x04, 0x0C, 0x04, 0x04, 0x04, 0x04, 0x0E],
        '2' => [0x0E, 0x11, 0x01, 0x02, 0x04, 0x08, 0x1F],
        '3' => [0x1E, 0x01, 0x01, 0x0E, 0x01, 0x01, 0x1E],
        '4' => [0x02, 0x06, 0x0A, 0x12, 0x1F, 0x02, 0x02],
        '5' => [0x1F, 0x10, 0x1E, 0x01, 0x01, 0x11, 0x0E],
        '6' => [0x06, 0x08, 0x10, 0x1E, 0x11, 0x11, 0x0E],
        '7' => [0x1F, 0x01, 0x02, 0x04, 0x08, 0x08, 0x08],
        '8' => [0x0E, 0x11, 0x11, 0x0E, 0x11, 0x11, 0x0E],
        '9' => [0x0E, 0x11, 0x11, 0x0F, 0x01, 0x02, 0x0C],
        '-' | '−' => [0x00, 0x00, 0x00, 0x1F, 0x00, 0x00, 0x00],
        '.' => [0x00, 0x00, 0x00, 0x00, 0x00, 0x0C, 0x0C],
        '+' => [0x00, 0x04, 0x04, 0x1F, 0x04, 0x04, 0x00],
        'e' => [0x00, 0x00, 0x0E, 0x11, 0x1F, 0x10, 0x0E],
        'P' => [0x1E, 0x11, 0x11, 0x1E, 0x10, 0x10, 0x10],
        'a' => [0x00, 0x00, 0x0E, 0x01, 0x0F, 0x11, 0x0F],
        'm' => [0x00, 0x00, 0x1A, 0x15, 0x15, 0x15, 0x15],
        's' => [0x00, 0x00, 0x0F, 0x10, 0x0E, 0x01, 0x1E],
        'N' => [0x11, 0x19, 0x15, 0x13, 0x11, 0x11, 0x11],
        'W' => [0x11, 0x11, 0x11, 0x15, 0x15, 0x15, 0x0A],
        '/' => [0x01, 0x02, 0x02, 0x04, 0x08, 0x08, 0x10],
        ' ' => [0; 7],
        _ => return None,
    })
}

/// Width in pixels of `text` at `scale`.
fn text_width(text: &str, scale: u32) -> u32 {
    text.chars().filter(|c| glyph(*c).is_some()).count() as u32 * 6 * scale
}

fn put(rgba: &mut [u8], w: u32, h: u32, x: u32, y: u32, c: [u8; 4]) {
    if x < w && y < h {
        let i = ((y * w + x) * 4) as usize;
        rgba[i..i + 4].copy_from_slice(&c);
    }
}

#[allow(clippy::too_many_arguments)] // a frame, a place, a text and its look
fn text(rgba: &mut [u8], w: u32, h: u32, x: u32, y: u32, s: &str, scale: u32, c: [u8; 4]) {
    let mut cx = x;
    for ch in s.chars() {
        let Some(g) = glyph(ch) else { continue };
        for (row, bits) in g.iter().enumerate() {
            for col in 0..5 {
                if bits & (0x10 >> col) != 0 {
                    for dy in 0..scale {
                        for dx in 0..scale {
                            put(
                                rgba,
                                w,
                                h,
                                cx + col * scale + dx,
                                y + row as u32 * scale + dy,
                                c,
                            );
                        }
                    }
                }
            }
        }
        cx += 6 * scale;
    }
}

/// A number with about four significant digits, short for a legend.
pub fn format_value(v: f64) -> String {
    let a = v.abs();
    if a != 0.0 && !(1e-2..1e5).contains(&a) {
        return format!("{v:.2e}");
    }
    let decimals = if a == 0.0 {
        0
    } else {
        (3 - a.log10().floor() as i32).clamp(0, 4) as usize
    };
    let s = format!("{v:.decimals$}");
    if s.contains('.') {
        s.trim_end_matches('0').trim_end_matches('.').to_string()
    } else {
        s
    }
}

/// Draws the legend at the bottom right: a panel with the colour bar (`colors` from low to high)
/// and the range with its unit below.
pub fn draw_legend(
    rgba: &mut [u8],
    w: u32,
    h: u32,
    colors: &[[u8; 3]],
    lo: f64,
    hi: f64,
    unit: &str,
) {
    if colors.is_empty() || w < 64 || h < 48 {
        return;
    }
    let scale = (h / 270).max(1);
    let margin = 8 * scale;
    let bar_w = (w * 3 / 10).max(60);
    let bar_h = 6 * scale;
    let text_h = 7 * scale;
    let panel_w = bar_w + 2 * margin;
    let panel_h = margin + bar_h + 4 * scale + text_h + margin;
    let (px, py) = (
        w.saturating_sub(panel_w + margin),
        h.saturating_sub(panel_h + margin),
    );
    for y in py..py + panel_h {
        for x in px..px + panel_w {
            put(rgba, w, h, x, y, [24, 28, 36, 235]);
        }
    }
    let (bx, by) = (px + margin, py + margin);
    for x in 0..bar_w {
        let i = (x as usize * (colors.len() - 1)) / (bar_w as usize - 1).max(1);
        let [r, g, b] = colors[i];
        for y in 0..bar_h {
            put(rgba, w, h, bx + x, by + y, [r, g, b, 255]);
        }
    }
    let ty = by + bar_h + 4 * scale;
    let white = [235, 238, 242, 255];
    let (lo_s, hi_s) = (format_value(lo), format_value(hi));
    text(rgba, w, h, bx, ty, &lo_s, scale, white);
    let hw = text_width(&hi_s, scale);
    text(
        rgba,
        w,
        h,
        (bx + bar_w).saturating_sub(hw),
        ty,
        &hi_s,
        scale,
        white,
    );
    let uw = text_width(unit, scale);
    text(
        rgba,
        w,
        h,
        bx + (bar_w.saturating_sub(uw)) / 2,
        ty,
        unit,
        scale,
        white,
    );
}
