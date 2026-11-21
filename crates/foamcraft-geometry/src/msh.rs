// SPDX-License-Identifier: GPL-3.0-or-later
//! Reader for Gmsh meshes (format 4.1, ASCII), as written when STEP and IGES files are converted.
//! Every triangle keeps the tag of its CAD surface; surfaces bounding a volume belong to that part.

use std::collections::HashMap;

use crate::mesh::{Format, Mesh, MeshBuilder};
use crate::transform::fix_normals;

struct Lines<'a> {
    it: std::str::Lines<'a>,
}

impl<'a> Lines<'a> {
    fn next(&mut self) -> Result<&'a str, String> {
        loop {
            let line = self.it.next().ok_or("unexpected end of file")?.trim();
            if !line.is_empty() {
                return Ok(line);
            }
        }
    }

    fn numbers<T: std::str::FromStr>(&mut self) -> Result<Vec<T>, String> {
        self.next()?
            .split_ascii_whitespace()
            .map(|t| t.parse().map_err(|_| format!("invalid number '{t}'")))
            .collect()
    }

    fn skip_to_end(&mut self, section: &str) -> Result<(), String> {
        let end = format!("$End{}", &section[1..]);
        while self.next()? != end {}
        Ok(())
    }
}

fn get<T: Copy>(v: &[T], i: usize) -> Result<T, String> {
    v.get(i)
        .copied()
        .ok_or_else(|| "line too short".to_string())
}

/// Surface tag -> volume tag, from the `$Entities` section.
fn parse_entities(lines: &mut Lines<'_>) -> Result<HashMap<u32, u32>, String> {
    let counts: Vec<usize> = lines.numbers()?;
    let (points, curves, surfaces, volumes) = (
        get(&counts, 0)?,
        get(&counts, 1)?,
        get(&counts, 2)?,
        get(&counts, 3)?,
    );
    for _ in 0..points + curves + surfaces {
        lines.next()?;
    }
    let mut owner = HashMap::new();
    for _ in 0..volumes {
        // tag minX minY minZ maxX maxY maxZ numPhysical physical... numSurfaces surfaces...
        let v: Vec<f64> = lines.numbers()?;
        let tag = get(&v, 0)? as u32;
        let phys = get(&v, 7)? as usize;
        let n = get(&v, 8 + phys)? as usize;
        for k in 0..n {
            let s = get(&v, 9 + phys + k)?.abs() as u32;
            owner.entry(s).or_insert(tag);
        }
    }
    lines.skip_to_end("$Entities")?;
    Ok(owner)
}

fn parse(text: &str) -> Result<Mesh, String> {
    let mut lines = Lines { it: text.lines() };
    let mut owner = HashMap::new();
    let mut nodes: HashMap<u64, [f32; 3]> = HashMap::new();
    let mut triangles: Vec<([u64; 3], u32)> = Vec::new();
    let mut version_ok = false;
    while let Some(line) = lines.it.next() {
        match line.trim() {
            "$MeshFormat" => {
                let v: Vec<String> = lines
                    .next()?
                    .split_ascii_whitespace()
                    .map(String::from)
                    .collect();
                if !v.first().is_some_and(|x| x.starts_with('4'))
                    || v.get(1).map(String::as_str) != Some("0")
                {
                    return Err("only Gmsh ASCII format 4 is supported".into());
                }
                version_ok = true;
                lines.skip_to_end("$MeshFormat")?;
            }
            "$Entities" => owner = parse_entities(&mut lines)?,
            "$Nodes" => {
                let head: Vec<u64> = lines.numbers()?;
                for _ in 0..get(&head, 0)? {
                    let block: Vec<u64> = lines.numbers()?;
                    let (parametric, n) = (get(&block, 2)?, get(&block, 3)? as usize);
                    let mut tags = Vec::with_capacity(n);
                    for _ in 0..n {
                        tags.push(get(&lines.numbers::<u64>()?, 0)?);
                    }
                    for tag in tags {
                        let c: Vec<f64> = lines.numbers()?;
                        if c.len() < 3 + if parametric == 1 { 1 } else { 0 } {
                            return Err("node coordinates incomplete".into());
                        }
                        nodes.insert(tag, [c[0] as f32, c[1] as f32, c[2] as f32]);
                    }
                }
                lines.skip_to_end("$Nodes")?;
            }
            "$Elements" => {
                let head: Vec<u64> = lines.numbers()?;
                for _ in 0..get(&head, 0)? {
                    let block: Vec<u64> = lines.numbers()?;
                    let (dim, entity, kind, n) = (
                        get(&block, 0)?,
                        get(&block, 1)? as u32,
                        get(&block, 2)?,
                        get(&block, 3)?,
                    );
                    for _ in 0..n {
                        let e: Vec<u64> = lines.numbers()?;
                        // Type 2 is the 3-node triangle; other elements (points, lines) are skipped.
                        if dim == 2 && kind == 2 {
                            triangles.push(([get(&e, 1)?, get(&e, 2)?, get(&e, 3)?], entity));
                        }
                    }
                }
                lines.skip_to_end("$Elements")?;
            }
            _ => {}
        }
    }
    if !version_ok {
        return Err("missing $MeshFormat".into());
    }
    let mut b = MeshBuilder::with_capacity(triangles.len());
    let mut face_tags = Vec::with_capacity(triangles.len());
    let mut part_tags = Vec::with_capacity(triangles.len());
    for (t, surface) in &triangles {
        let p = |i: u64| nodes.get(&i).copied().ok_or(format!("unknown node {i}"));
        b.triangle(p(t[0])?, p(t[1])?, p(t[2])?);
        face_tags.push(*surface);
        part_tags.push(owner.get(surface).copied().unwrap_or(0));
    }
    // The unit depends on the conversion (STEP: metres, IGES: file unit); the caller knows it.
    let mut mesh = b.finish(Format::Msh, None);
    mesh.face_tags = Some(face_tags);
    mesh.part_tags = Some(part_tags);
    // Gmsh orients each surface by its own parametrisation; turn everything outward.
    fix_normals(&mut mesh);
    Ok(mesh)
}

pub fn read_msh(bytes: &[u8]) -> Result<Mesh, String> {
    parse(&String::from_utf8_lossy(bytes))
}

/// Length unit of an IGES file from its global section (parameter 14, the unit flag).
pub fn iges_unit(bytes: &[u8]) -> Option<&'static str> {
    let text = String::from_utf8_lossy(bytes);
    // Global section: columns 1-72 of every line marked 'G' in column 73.
    let global: String = text
        .lines()
        .filter(|l| l.len() >= 73 && l.as_bytes()[72] == b'G')
        .map(|l| &l[..72])
        .collect();
    if global.is_empty() {
        return None;
    }
    // Split into parameters; Hollerith strings (nH...) may contain the delimiter.
    let chars: Vec<char> = global.chars().collect();
    let mut params = Vec::new();
    let mut current = String::new();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c.is_ascii_digit() {
            let start = i;
            while i < chars.len() && chars[i].is_ascii_digit() {
                i += 1;
            }
            if i < chars.len() && chars[i] == 'H' {
                let n: usize = chars[start..i].iter().collect::<String>().parse().ok()?;
                current.extend(&chars[i + 1..(i + 1 + n).min(chars.len())]);
                i += 1 + n;
                continue;
            }
            current.extend(&chars[start..i]);
            continue;
        }
        if c == ',' || c == ';' {
            params.push(std::mem::take(&mut current));
        } else {
            current.push(c);
        }
        i += 1;
    }
    let flag: u32 = params.get(13)?.trim().parse().ok()?;
    match flag {
        1 => Some("in"),
        2 => Some("mm"),
        4 => Some("ft"),
        6 => Some("m"),
        9 => Some("um"),
        10 => Some("cm"),
        _ => None,
    }
}
