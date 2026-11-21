// SPDX-License-Identifier: GPL-3.0-or-later
//! PNG output of offscreen frames.

use std::io::{BufWriter, Error};
use std::path::Path;

/// Writes tightly packed RGBA8 pixels as a PNG file.
pub fn write_png(path: &Path, width: u32, height: u32, rgba: &[u8]) -> Result<(), Error> {
    let file = std::fs::File::create(path)?;
    let mut encoder = png::Encoder::new(BufWriter::new(file), width, height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header().map_err(Error::other)?;
    writer.write_image_data(rgba).map_err(Error::other)?;
    writer.finish().map_err(Error::other)
}
