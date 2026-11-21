// SPDX-License-Identifier: GPL-3.0-or-later
//! Readers for STL (ASCII and binary), OBJ and 3MF.

use std::io::Read;
use std::path::Path;

use foamcraft_core::error::FoamError;
use quick_xml::events::Event;

use crate::mesh::{Format, Mesh, MeshBuilder};

fn failed(path: &Path, why: impl std::fmt::Display) -> FoamError {
    FoamError::new("geometry.read_failed", format!("{}: {why}", path.display()))
}

/// Format from the file extension.
pub fn format_of(path: &Path) -> Option<Format> {
    let ext = path.extension()?.to_string_lossy().to_ascii_lowercase();
    match ext.as_str() {
        "stl" => Some(Format::Stl),
        "obj" => Some(Format::Obj),
        "3mf" => Some(Format::ThreeMf),
        "step" | "stp" => Some(Format::Step),
        "iges" | "igs" => Some(Format::Iges),
        "msh" => Some(Format::Msh),
        _ => None,
    }
}

/// Reads a mesh; the format follows the file extension.
pub fn read_mesh(path: &Path) -> Result<Mesh, FoamError> {
    let format = format_of(path)
        .ok_or_else(|| FoamError::new("geometry.unsupported_format", path.display().to_string()))?;
    if matches!(format, Format::Step | Format::Iges) {
        // CAD files are converted to a Gmsh mesh first (geometry.import does that).
        return Err(FoamError::new(
            "geometry.cad_unavailable",
            path.display().to_string(),
        ));
    }
    let bytes = std::fs::read(path).map_err(|e| failed(path, e))?;
    let mesh = match format {
        Format::Stl => read_stl(&bytes).map_err(|e| failed(path, e))?,
        Format::Obj => read_obj(&bytes).map_err(|e| failed(path, e))?,
        Format::ThreeMf => read_3mf(&bytes).map_err(|e| failed(path, e))?,
        Format::Msh => crate::msh::read_msh(&bytes).map_err(|e| failed(path, e))?,
        Format::Step | Format::Iges => unreachable!("handled above"),
    };
    if mesh.triangles.is_empty() {
        return Err(FoamError::new("geometry.empty", path.display().to_string()));
    }
    Ok(mesh)
}

fn read_stl(bytes: &[u8]) -> Result<Mesh, String> {
    if bytes.len() >= 84 {
        let count = u32::from_le_bytes([bytes[80], bytes[81], bytes[82], bytes[83]]) as usize;
        // Binary files may also start with "solid"; the exact size decides.
        if bytes.len() == 84 + count * 50 {
            return Ok(read_binary_stl(bytes, count));
        }
    }
    if bytes.starts_with(b"solid") {
        return read_ascii_stl(bytes);
    }
    Err("neither binary nor ASCII STL".into())
}

fn read_binary_stl(bytes: &[u8], count: usize) -> Mesh {
    let mut b = MeshBuilder::with_capacity(count);
    let f = |o: usize| f32::from_le_bytes([bytes[o], bytes[o + 1], bytes[o + 2], bytes[o + 3]]);
    for i in 0..count {
        let o = 84 + i * 50 + 12;
        let p = |k: usize| [f(o + k * 12), f(o + k * 12 + 4), f(o + k * 12 + 8)];
        b.triangle(p(0), p(1), p(2));
    }
    b.finish(Format::Stl, None)
}

fn read_ascii_stl(bytes: &[u8]) -> Result<Mesh, String> {
    let text = std::str::from_utf8(bytes).map_err(|e| e.to_string())?;
    let mut b = MeshBuilder::default();
    let mut corners: Vec<[f32; 3]> = Vec::with_capacity(3);
    let mut tokens = text.split_ascii_whitespace();
    while let Some(tok) = tokens.next() {
        if tok != "vertex" {
            continue;
        }
        let mut p = [0f32; 3];
        for c in &mut p {
            let t = tokens.next().ok_or("vertex is incomplete")?;
            *c = t.parse().map_err(|_| format!("invalid number '{t}'"))?;
        }
        corners.push(p);
        if corners.len() == 3 {
            b.triangle(corners[0], corners[1], corners[2]);
            corners.clear();
        }
    }
    if !corners.is_empty() {
        return Err("facet with fewer than three vertices".into());
    }
    Ok(b.finish(Format::Stl, None))
}

fn read_obj(bytes: &[u8]) -> Result<Mesh, String> {
    let text = String::from_utf8_lossy(bytes);
    let mut vertices: Vec<[f32; 3]> = Vec::new();
    let mut b = MeshBuilder::default();
    for (n, line) in text.lines().enumerate() {
        let mut it = line.split_ascii_whitespace();
        match it.next() {
            Some("v") => {
                let mut p = [0f32; 3];
                for c in &mut p {
                    let t = it
                        .next()
                        .ok_or(format!("line {}: vertex is incomplete", n + 1))?;
                    *c = t
                        .parse()
                        .map_err(|_| format!("line {}: invalid number '{t}'", n + 1))?;
                }
                vertices.push(p);
            }
            Some("f") => {
                let mut ids = Vec::new();
                for t in it {
                    let first = t.split('/').next().unwrap_or("");
                    let i: i64 = first
                        .parse()
                        .map_err(|_| format!("line {}: invalid index '{t}'", n + 1))?;
                    // Negative indices count back from the last vertex.
                    let idx = if i < 0 {
                        vertices.len() as i64 + i
                    } else {
                        i - 1
                    };
                    let p = usize::try_from(idx)
                        .ok()
                        .and_then(|i| vertices.get(i))
                        .ok_or(format!("line {}: index {i} out of range", n + 1))?;
                    ids.push(*p);
                }
                if ids.len() < 3 {
                    return Err(format!(
                        "line {}: face with fewer than three vertices",
                        n + 1
                    ));
                }
                for k in 1..ids.len() - 1 {
                    b.triangle(ids[0], ids[k], ids[k + 1]);
                }
            }
            _ => {}
        }
    }
    Ok(b.finish(Format::Obj, None))
}

fn unit_name(unit: &str) -> &'static str {
    match unit {
        "micron" => "um",
        "centimeter" => "cm",
        "inch" => "in",
        "foot" => "ft",
        "meter" => "m",
        _ => "mm",
    }
}

fn read_3mf(bytes: &[u8]) -> Result<Mesh, String> {
    let mut archive =
        zip::ZipArchive::new(std::io::Cursor::new(bytes)).map_err(|e| format!("zip: {e}"))?;
    let name = (0..archive.len())
        .filter_map(|i| {
            archive
                .name_for_index(i)
                .and_then(|r| r.ok())
                .map(|n| n.to_string())
        })
        .find(|n| n.to_ascii_lowercase().ends_with(".model"))
        .ok_or("no 3D model part in the archive")?;
    let mut xml = String::new();
    archive
        .by_name(&name)
        .map_err(|e| e.to_string())?
        .read_to_string(&mut xml)
        .map_err(|e| e.to_string())?;
    parse_3mf_model(&xml)
}

fn attr(e: &quick_xml::events::BytesStart<'_>, name: &str) -> Option<String> {
    e.attributes()
        .flatten()
        .find(|a| a.key.local_name().as_ref() == name)
        // Attributes read here are numbers and unit names; no entity unescaping needed.
        .map(|a| a.value.to_string())
}

fn parse_3mf_model(xml: &str) -> Result<Mesh, String> {
    let mut reader = quick_xml::Reader::from_str(xml);
    let mut unit = "millimeter".to_string();
    let mut b = MeshBuilder::default();
    let mut object_vertices: Vec<[f32; 3]> = Vec::new();
    loop {
        match reader.read_event().map_err(|e| format!("xml: {e}"))? {
            Event::Eof => break,
            Event::Start(e) | Event::Empty(e) => match e.local_name().as_ref() {
                "model" => {
                    if let Some(u) = attr(&e, "unit") {
                        unit = u;
                    }
                }
                "mesh" => object_vertices.clear(),
                "vertex" => {
                    let c = |n: &str| -> Result<f32, String> {
                        attr(&e, n)
                            .ok_or("vertex coordinate missing")?
                            .parse()
                            .map_err(|_| "invalid vertex coordinate".to_string())
                    };
                    object_vertices.push([c("x")?, c("y")?, c("z")?]);
                }
                "triangle" => {
                    let v = |n: &str| -> Result<[f32; 3], String> {
                        let i: usize = attr(&e, n)
                            .ok_or("triangle index missing")?
                            .parse()
                            .map_err(|_| "invalid triangle index".to_string())?;
                        object_vertices
                            .get(i)
                            .copied()
                            .ok_or("triangle index out of range".into())
                    };
                    b.triangle(v("v1")?, v("v2")?, v("v3")?);
                }
                _ => {}
            },
            _ => {}
        }
    }
    Ok(b.finish(Format::ThreeMf, Some(unit_name(&unit).to_string())))
}
