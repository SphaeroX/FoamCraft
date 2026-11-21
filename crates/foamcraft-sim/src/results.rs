// SPDX-License-Identifier: GPL-3.0-or-later
//! Result data for display: the colour scale, surface fields, volume fields sampled on regular
//! grids (with a mask for solid and outside points) and their compact binary cache format.

use foamcraft_core::error::FoamError;
use serde::{Deserialize, Serialize};

/// Linear sRGB from OKLab (B. Ottosson, "A perceptual color space for image processing", 2020).
fn oklab_to_linear(l: f64, a: f64, b: f64) -> [f64; 3] {
    let l_ = l + 0.396_337_777_4 * a + 0.215_803_757_3 * b;
    let m_ = l - 0.105_561_345_8 * a - 0.063_854_172_8 * b;
    let s_ = l - 0.089_484_177_5 * a - 1.291_485_548 * b;
    let (l3, m3, s3) = (l_ * l_ * l_, m_ * m_ * m_, s_ * s_ * s_);
    [
        4.076_741_662_1 * l3 - 3.307_711_591_3 * m3 + 0.230_969_929_2 * s3,
        -1.268_438_004_6 * l3 + 2.609_757_401_1 * m3 - 0.341_319_396_5 * s3,
        -0.004_196_086_3 * l3 - 0.703_418_614_7 * m3 + 1.707_614_701 * s3,
    ]
}

/// OKLab lightness of a linear sRGB colour.
pub fn oklab_lightness(c: [f32; 3]) -> f64 {
    let [r, g, b] = c.map(f64::from);
    let l = 0.412_221_470_8 * r + 0.536_332_536_3 * g + 0.051_445_992_9 * b;
    let m = 0.211_903_498_2 * r + 0.680_699_545_1 * g + 0.107_396_956_6 * b;
    let s = 0.088_302_461_9 * r + 0.281_718_837_6 * g + 0.629_978_700_5 * b;
    0.210_454_255_3 * l.cbrt() + 0.793_617_785 * m.cbrt() - 0.004_072_046_8 * s.cbrt()
}

/// FoamCraft colour scale for `t` in 0..1 (linear sRGB): lightness rises evenly from dark blue
/// to light yellow, so the order stays readable with red-green colour vision deficiency and in
/// grey scale.
pub fn colormap(t: f64) -> [f32; 3] {
    let t = t.clamp(0.0, 1.0);
    // Starts at a medium lightness so dark ends stay readable on shaded surfaces.
    let l = 0.45 + 0.50 * t;
    // Hue turns from blue (b < 0) through teal to yellow (b > 0); chroma stays moderate so the
    // colours stay inside sRGB.
    let a = -0.06 * (std::f64::consts::PI * t).sin();
    let b = -0.10 + 0.26 * t;
    oklab_to_linear(l, a, b).map(|c| c.clamp(0.0, 1.0) as f32)
}

/// Grid dimensions for a box with at most `max_points` points, cells as cubic as possible.
pub fn level_dims(size: [f64; 3], max_points: u64) -> [u32; 3] {
    let volume: f64 = size.iter().map(|s| s.max(1e-12)).product();
    let h = (volume / max_points as f64).cbrt();
    let mut d = size.map(|s| ((s / h).floor() as u32).max(2));
    while d.iter().map(|x| u64::from(*x)).product::<u64>() > max_points {
        let k = (0..3).max_by_key(|k| d[*k]).unwrap_or(0);
        d[k] -= 1;
    }
    d
}

/// A regular grid as OpenFOAM's `array` sample set places it: point i of n along an axis lies at
/// min + (i + 1) / (n + 1) · (max - min).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GridSpec {
    pub min: [f64; 3],
    pub max: [f64; 3],
    pub dims: [u32; 3],
}

impl GridSpec {
    pub fn step(&self) -> [f64; 3] {
        [0, 1, 2].map(|k| (self.max[k] - self.min[k]) / f64::from(self.dims[k] + 1))
    }

    pub fn point(&self, i: u32, j: u32, k: u32) -> [f64; 3] {
        let s = self.step();
        let ijk = [i, j, k];
        [0, 1, 2].map(|a| self.min[a] + f64::from(ijk[a] + 1) * s[a])
    }

    pub fn len(&self) -> usize {
        self.dims.iter().map(|d| *d as usize).product()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn index(&self, i: u32, j: u32, k: u32) -> usize {
        ((k * self.dims[1] + j) * self.dims[0] + i) as usize
    }

    /// Fractional grid coordinates of a point.
    fn coords(&self, p: [f64; 3]) -> [f64; 3] {
        let s = self.step();
        [0, 1, 2].map(|a| (p[a] - self.min[a]) / s[a] - 1.0)
    }
}

/// Sampled values on a grid; `mask` is true where the point lay in a solid or outside the fluid.
#[derive(Debug, Clone, PartialEq)]
pub struct Grid {
    pub spec: GridSpec,
    pub names: Vec<String>,
    /// One value list per name, `spec.len()` entries each (0 where masked).
    pub values: Vec<Vec<f32>>,
    pub mask: Vec<bool>,
}

/// Builds a grid from raw sample rows `x y z values...`; missing points are masked.
pub fn grid_from_rows(spec: &GridSpec, names: &[&str], rows: &[Vec<f64>]) -> Grid {
    let n = spec.len();
    let mut values = vec![vec![0.0f32; n]; names.len()];
    let mut mask = vec![true; n];
    for row in rows {
        let c = spec.coords([row[0], row[1], row[2]]);
        let ijk = c.map(|x| x.round());
        if (0..3).any(|a| ijk[a] < 0.0 || ijk[a] >= f64::from(spec.dims[a])) {
            continue;
        }
        let id = spec.index(ijk[0] as u32, ijk[1] as u32, ijk[2] as u32);
        mask[id] = false;
        for (f, v) in values.iter_mut().enumerate() {
            v[id] = row.get(3 + f).copied().unwrap_or(0.0) as f32;
        }
    }
    Grid {
        spec: spec.clone(),
        names: names.iter().map(|s| s.to_string()).collect(),
        values,
        mask,
    }
}

impl Grid {
    /// Trilinear value of a field at a point; `None` outside the grid or next to a masked point.
    pub fn sample(&self, field: &str, p: [f64; 3]) -> Option<f64> {
        let f = self.names.iter().position(|n| n == field)?;
        let c = self.spec.coords(p);
        let d = self.spec.dims;
        let mut base = [0u32; 3];
        let mut frac = [0.0; 3];
        for a in 0..3 {
            if c[a] < 0.0 || c[a] > f64::from(d[a] - 1) {
                return None;
            }
            let b = (c[a].floor() as u32).min(d[a].saturating_sub(2));
            base[a] = b;
            frac[a] = c[a] - f64::from(b);
        }
        let mut sum = 0.0;
        for corner in 0..8u32 {
            let o = [corner & 1, (corner >> 1) & 1, (corner >> 2) & 1];
            let id = self
                .spec
                .index(base[0] + o[0], base[1] + o[1], base[2] + o[2]);
            if self.mask[id] {
                return None;
            }
            let w: f64 = (0..3)
                .map(|a| if o[a] == 1 { frac[a] } else { 1.0 - frac[a] })
                .product();
            sum += w * f64::from(self.values[f][id]);
        }
        Some(sum)
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = Vec::new();
        write_json(&mut out, &(&self.spec, &self.names));
        for v in &self.values {
            write_f32s(&mut out, v);
        }
        out.extend(self.mask.iter().map(|m| u8::from(*m)));
        out
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, FoamError> {
        let mut r = Reader { b: bytes, i: 0 };
        let (spec, names): (GridSpec, Vec<String>) = r.json()?;
        let n = spec.len();
        let values = (0..names.len())
            .map(|_| r.f32s(n))
            .collect::<Result<Vec<_>, _>>()?;
        let mask = r.take(n)?.iter().map(|b| *b != 0).collect();
        Ok(Self {
            spec,
            names,
            values,
            mask,
        })
    }
}

/// A triangulated surface with values per triangle.
#[derive(Debug, Clone, PartialEq)]
pub struct Surface {
    pub points: Vec<[f32; 3]>,
    pub triangles: Vec<[u32; 3]>,
    pub fields: Vec<(String, Vec<f32>)>,
}

/// Field `name` of `b` minus that of `a`, per triangle; `None` when the field is missing or the
/// two surfaces are not the same geometry (same points and triangles).
pub fn difference_field(a: &Surface, b: &Surface, name: &str) -> Option<Vec<f32>> {
    if a.triangles != b.triangles
        || a.points.len() != b.points.len()
        || a.points
            .iter()
            .zip(&b.points)
            .any(|(p, q)| (0..3).any(|k| (p[k] - q[k]).abs() > 1e-7))
    {
        return None;
    }
    let (fa, fb) = (a.field(name)?, b.field(name)?);
    (fa.len() == fb.len()).then(|| fa.iter().zip(fb).map(|(x, y)| y - x).collect())
}

impl Surface {
    pub fn field(&self, name: &str) -> Option<&[f32]> {
        self.fields
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, v)| v.as_slice())
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = Vec::new();
        let names: Vec<&str> = self.fields.iter().map(|(n, _)| n.as_str()).collect();
        write_json(&mut out, &(self.points.len(), self.triangles.len(), names));
        write_f32s(&mut out, &self.points.concat());
        for t in &self.triangles {
            for i in t {
                out.extend(i.to_le_bytes());
            }
        }
        for (_, v) in &self.fields {
            write_f32s(&mut out, v);
        }
        out
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, FoamError> {
        let mut r = Reader { b: bytes, i: 0 };
        let (np, nt, names): (usize, usize, Vec<String>) = r.json()?;
        let flat = r.f32s(np * 3)?;
        let points = flat.chunks(3).map(|c| [c[0], c[1], c[2]]).collect();
        let raw = r.take(nt * 12)?;
        let triangles = raw
            .chunks(12)
            .map(|c| {
                [0, 4, 8].map(|o| u32::from_le_bytes(c[o..o + 4].try_into().unwrap_or([0; 4])))
            })
            .collect();
        let fields = names
            .into_iter()
            .map(|n| Ok((n, r.f32s(nt)?)))
            .collect::<Result<Vec<_>, FoamError>>()?;
        Ok(Self {
            points,
            triangles,
            fields,
        })
    }
}

fn write_json(out: &mut Vec<u8>, value: &impl Serialize) {
    let json = serde_json::to_vec(value).unwrap_or_default();
    out.extend((json.len() as u32).to_le_bytes());
    out.extend(json);
}

fn write_f32s(out: &mut Vec<u8>, v: &[f32]) {
    for x in v {
        out.extend(x.to_le_bytes());
    }
}

struct Reader<'a> {
    b: &'a [u8],
    i: usize,
}

fn corrupt() -> FoamError {
    FoamError::new("results.read_failed", "cache file is damaged")
}

impl<'a> Reader<'a> {
    fn take(&mut self, n: usize) -> Result<&'a [u8], FoamError> {
        let s = self.b.get(self.i..self.i + n).ok_or_else(corrupt)?;
        self.i += n;
        Ok(s)
    }

    fn json<T: for<'de> Deserialize<'de>>(&mut self) -> Result<T, FoamError> {
        let len = u32::from_le_bytes(self.take(4)?.try_into().map_err(|_| corrupt())?) as usize;
        serde_json::from_slice(self.take(len)?).map_err(|_| corrupt())
    }

    fn f32s(&mut self, n: usize) -> Result<Vec<f32>, FoamError> {
        Ok(self
            .take(n * 4)?
            .chunks(4)
            .map(|c| f32::from_le_bytes(c.try_into().unwrap_or([0; 4])))
            .collect())
    }
}

/// Grid points per level (64³): with U, p, total pressure and k about 6.5 MB per level.
pub const POINTS_PER_LEVEL: u64 = 262_144;

/// The postProcess dictionary that writes the wall surface with its fields (`walls`) and samples
/// the volume fields on the grid levels (`grid`, sets `level0`, `level1`, ...).
pub fn extraction_dict(patches: &str, turbulent: bool, levels: &[GridSpec]) -> String {
    let wall_fields = if turbulent {
        "p wallShearStress yPlus"
    } else {
        "p wallShearStress"
    };
    let volume_fields = if turbulent { "U p k" } else { "U p" };
    let mut out = format!(
        "FoamFile {{ version 2.0; format ascii; class dictionary; object foamcraftResults; }}\n\
         functions\n{{\n    \
         wallShearStress {{ type wallShearStress; libs (fieldFunctionObjects); patches ({patches}); writeFields true; log false; }}\n"
    );
    if turbulent {
        out.push_str(
            "    yPlus { type yPlus; libs (fieldFunctionObjects); writeFields true; log false; }\n",
        );
    }
    out.push_str(&format!(
        "    walls\n    {{\n        type surfaces; libs (sampling); surfaceFormat foam;\n        \
         formatOptions {{ foam {{ format ascii; }} }}\n        fields ({wall_fields}); interpolate false;\n        \
         surfaces {{ model {{ type patch; patches ({patches}); triangulate true; }} }}\n    }}\n    \
         grid\n    {{\n        type sets; libs (sampling); setFormat raw; interpolationScheme cellPoint;\n        \
         fields ({volume_fields});\n        sets\n        {{\n"
    ));
    for (n, l) in levels.iter().enumerate() {
        let span = [0, 1, 2].map(|k| l.max[k] - l.min[k]);
        out.push_str(&format!(
            "            level{n} {{ type array; axis xyz; origin ({:e} {:e} {:e}); rotation {{ type none; }} \
             pointsDensity ({} {} {}); spanBox ({:e} {:e} {:e}); }}\n",
            l.min[0], l.min[1], l.min[2], l.dims[0], l.dims[1], l.dims[2], span[0], span[1], span[2]
        ));
    }
    out.push_str("        }\n    }\n}\n");
    out
}

/// Column names of a raw set file from its name (`level0_k_p_U.xy` → x y z k p Ux Uy Uz values);
/// `U` is the only vector field.
pub fn set_columns(file_name: &str) -> Vec<String> {
    let stem = file_name.trim_end_matches(".xy");
    let mut parts = stem.split('_');
    parts.next();
    let mut out = Vec::new();
    for f in parts {
        if f == "U" {
            out.extend(["Ux", "Uy", "Uz"].map(String::from));
        } else {
            out.push(f.to_string());
        }
    }
    out
}
