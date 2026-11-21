// SPDX-License-Identifier: GPL-3.0-or-later
//! Video files: Motion JPEG in an AVI container (RIFF, `idx1` index). Every frame is a baseline
//! JPEG with 4:2:0 chroma ([`crate::jpeg`]); no codec patents are involved and Windows plays it without extra codecs. Files stay
//! below 1 GiB (the limit of a plain AVI without the OpenDML extension).

use std::fs::File;
use std::io::{BufWriter, Error, Seek, SeekFrom, Write};
use std::path::Path;

/// A plain AVI must stay below this size.
pub const MAX_BYTES: u64 = 1 << 30;

pub struct AviWriter {
    out: BufWriter<File>,
    width: u32,
    height: u32,
    fps: u32,
    quality: u8,
    /// Offset of each frame chunk relative to the `movi` tag, and its size.
    index: Vec<(u32, u32)>,
    movi_start: u64,
    pos: u64,
    largest: u32,
}

fn tag(out: &mut impl Write, t: &[u8; 4]) -> Result<(), Error> {
    out.write_all(t)
}

fn u32le(out: &mut impl Write, v: u32) -> Result<(), Error> {
    out.write_all(&v.to_le_bytes())
}

fn u16le(out: &mut impl Write, v: u16) -> Result<(), Error> {
    out.write_all(&v.to_le_bytes())
}

/// Header up to the `movi` list. Sizes and counts are written again by `finish`.
fn header(
    width: u32,
    height: u32,
    fps: u32,
    frames: u32,
    largest: u32,
    riff: u32,
    movi: u32,
) -> Result<Vec<u8>, Error> {
    let mut h = Vec::new();
    tag(&mut h, b"RIFF")?;
    u32le(&mut h, riff)?;
    tag(&mut h, b"AVI ")?;
    // hdrl: avih (56 bytes) + strl (strh 56 + strf 40).
    tag(&mut h, b"LIST")?;
    u32le(&mut h, 4 + (8 + 56) + (12 + (8 + 56) + (8 + 40)))?;
    tag(&mut h, b"hdrl")?;
    tag(&mut h, b"avih")?;
    u32le(&mut h, 56)?;
    u32le(&mut h, 1_000_000 / fps.max(1))?; // microseconds per frame
    u32le(&mut h, largest.saturating_mul(fps))?; // max bytes per second
    u32le(&mut h, 0)?; // padding granularity
    u32le(&mut h, 0x10)?; // AVIF_HASINDEX
    u32le(&mut h, frames)?;
    u32le(&mut h, 0)?; // initial frames
    u32le(&mut h, 1)?; // streams
    u32le(&mut h, largest)?; // suggested buffer size
    u32le(&mut h, width)?;
    u32le(&mut h, height)?;
    for _ in 0..4 {
        u32le(&mut h, 0)?;
    }
    tag(&mut h, b"LIST")?;
    u32le(&mut h, 4 + (8 + 56) + (8 + 40))?;
    tag(&mut h, b"strl")?;
    tag(&mut h, b"strh")?;
    u32le(&mut h, 56)?;
    tag(&mut h, b"vids")?;
    tag(&mut h, b"MJPG")?;
    u32le(&mut h, 0)?; // flags
    u16le(&mut h, 0)?; // priority
    u16le(&mut h, 0)?; // language
    u32le(&mut h, 0)?; // initial frames
    u32le(&mut h, 1)?; // scale
    u32le(&mut h, fps)?; // rate: rate / scale = frames per second
    u32le(&mut h, 0)?; // start
    u32le(&mut h, frames)?; // length
    u32le(&mut h, largest)?; // suggested buffer size
    u32le(&mut h, u32::MAX)?; // quality: default
    u32le(&mut h, 0)?; // sample size
    u16le(&mut h, 0)?; // frame rectangle
    u16le(&mut h, 0)?;
    u16le(&mut h, width as u16)?;
    u16le(&mut h, height as u16)?;
    tag(&mut h, b"strf")?;
    u32le(&mut h, 40)?;
    u32le(&mut h, 40)?; // BITMAPINFOHEADER size
    u32le(&mut h, width)?;
    u32le(&mut h, height)?;
    u16le(&mut h, 1)?; // planes
    u16le(&mut h, 24)?; // bits per pixel
    tag(&mut h, b"MJPG")?; // compression
    u32le(&mut h, width * height * 3)?;
    for _ in 0..4 {
        u32le(&mut h, 0)?;
    }
    tag(&mut h, b"LIST")?;
    u32le(&mut h, movi)?;
    tag(&mut h, b"movi")?;
    Ok(h)
}

impl AviWriter {
    /// `quality` 1..100 of the JPEG frames.
    pub fn create(
        path: &Path,
        width: u32,
        height: u32,
        fps: u32,
        quality: u8,
    ) -> Result<Self, Error> {
        if width == 0
            || height == 0
            || width > u32::from(u16::MAX)
            || height > u32::from(u16::MAX)
            || fps == 0
        {
            return Err(Error::other("invalid video size or frame rate"));
        }
        let mut out = BufWriter::new(File::create(path)?);
        let h = header(width, height, fps, 0, 0, 0, 4)?;
        out.write_all(&h)?;
        let pos = h.len() as u64;
        Ok(Self {
            out,
            width,
            height,
            fps,
            quality: quality.clamp(1, 100),
            index: Vec::new(),
            movi_start: pos - 4,
            pos,
            largest: 0,
        })
    }

    /// Adds one frame of tightly packed RGBA8 pixels (alpha is dropped).
    pub fn push_rgba(&mut self, rgba: &[u8]) -> Result<(), Error> {
        let (w, h) = (self.width, self.height);
        if rgba.len() != (w * h * 4) as usize {
            return Err(Error::other("frame size does not match the video"));
        }
        let rgb: Vec<u8> = rgba
            .as_chunks::<4>()
            .0
            .iter()
            .flat_map(|p| [p[0], p[1], p[2]])
            .collect();
        let jpeg = crate::jpeg::encode_rgb(&rgb, w, h, self.quality);
        let len = jpeg.len() as u32;
        let padded = u64::from(len) + u64::from(len & 1);
        if self.pos + 8 + padded + 16 * (self.index.len() as u64 + 1) + 8 > MAX_BYTES {
            return Err(Error::other("video larger than 1 GiB"));
        }
        self.index.push(((self.pos - self.movi_start) as u32, len));
        tag(&mut self.out, b"00dc")?;
        u32le(&mut self.out, len)?;
        self.out.write_all(&jpeg)?;
        if len & 1 == 1 {
            self.out.write_all(&[0])?;
        }
        self.pos += 8 + padded;
        self.largest = self.largest.max(len);
        Ok(())
    }

    pub fn frames(&self) -> u32 {
        self.index.len() as u32
    }

    /// Writes the index and the final sizes and counts.
    pub fn finish(mut self) -> Result<(), Error> {
        tag(&mut self.out, b"idx1")?;
        u32le(&mut self.out, 16 * self.index.len() as u32)?;
        for (offset, len) in &self.index {
            tag(&mut self.out, b"00dc")?;
            u32le(&mut self.out, 0x10)?; // AVIIF_KEYFRAME
            u32le(&mut self.out, *offset)?;
            u32le(&mut self.out, *len)?;
        }
        let end = self.pos + 8 + 16 * self.index.len() as u64;
        let movi = (self.pos - self.movi_start) as u32;
        let h = header(
            self.width,
            self.height,
            self.fps,
            self.frames(),
            self.largest,
            (end - 8) as u32,
            movi,
        )?;
        let mut file = self.out.into_inner().map_err(|e| e.into_error())?;
        file.seek(SeekFrom::Start(0))?;
        file.write_all(&h)?;
        file.flush()
    }
}

/// The JPEG frames of an AVI written by [`AviWriter`], in order.
pub fn read_frames(path: &Path) -> Result<Vec<Vec<u8>>, Error> {
    let b = std::fs::read(path)?;
    let u32_at = |at: usize| -> Result<u32, Error> {
        b.get(at..at + 4)
            .and_then(|s| s.try_into().ok())
            .map(u32::from_le_bytes)
            .ok_or_else(|| Error::other("truncated AVI"))
    };
    let movi = b
        .windows(4)
        .position(|w| w == b"movi")
        .ok_or_else(|| Error::other("no movi list"))?;
    let mut at = movi + 4;
    let mut frames = Vec::new();
    while b.get(at..at + 4) == Some(b"00dc".as_slice()) {
        let len = u32_at(at + 4)? as usize;
        let data = b
            .get(at + 8..at + 8 + len)
            .ok_or_else(|| Error::other("truncated frame"))?;
        frames.push(data.to_vec());
        at += 8 + len + (len & 1);
    }
    Ok(frames)
}
