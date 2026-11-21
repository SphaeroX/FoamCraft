// SPDX-License-Identifier: GPL-3.0-or-later
//! Reading OpenFOAM data written by the runtime: field files (ASCII and binary, uniform and
//! non-uniform), the "foam" surface format of the surfaces function object and raw sample sets.

use foamcraft_core::error::FoamError;

fn bad(what: &str) -> FoamError {
    FoamError::new("results.read_failed", what)
}

#[derive(Debug, Clone, PartialEq)]
pub enum FieldData {
    Scalars(Vec<f64>),
    Vectors(Vec<[f64; 3]>),
    UniformScalar(f64),
    UniformVector([f64; 3]),
}

/// A position in the input with simple token helpers.
struct Cursor<'a> {
    s: &'a [u8],
    i: usize,
    binary: bool,
}

impl<'a> Cursor<'a> {
    fn new(s: &'a [u8], binary: bool) -> Self {
        Self { s, i: 0, binary }
    }

    fn skip_space(&mut self) {
        loop {
            while self.i < self.s.len() && self.s[self.i].is_ascii_whitespace() {
                self.i += 1;
            }
            if self.s[self.i..].starts_with(b"//") {
                while self.i < self.s.len() && self.s[self.i] != b'\n' {
                    self.i += 1;
                }
            } else if self.s[self.i..].starts_with(b"/*") {
                match find(&self.s[self.i + 2..], b"*/") {
                    Some(end) => self.i += 2 + end + 2,
                    None => self.i = self.s.len(),
                }
            } else {
                return;
            }
        }
    }

    fn peek(&mut self) -> Option<u8> {
        self.skip_space();
        self.s.get(self.i).copied()
    }

    fn expect(&mut self, c: u8) -> Result<(), FoamError> {
        if self.peek() == Some(c) {
            self.i += 1;
            Ok(())
        } else {
            Err(bad(&format!("expected '{}' at byte {}", c as char, self.i)))
        }
    }

    fn token(&mut self) -> &'a str {
        self.skip_space();
        let start = self.i;
        while self.i < self.s.len() {
            let c = self.s[self.i];
            if c.is_ascii_whitespace() || b"(){};".contains(&c) {
                break;
            }
            self.i += 1;
        }
        std::str::from_utf8(&self.s[start..self.i]).unwrap_or("")
    }

    fn number(&mut self) -> Result<f64, FoamError> {
        let t = self.token();
        t.parse().map_err(|_| bad(&format!("not a number: {t}")))
    }

    fn vector(&mut self) -> Result<[f64; 3], FoamError> {
        self.expect(b'(')?;
        let v = [self.number()?, self.number()?, self.number()?];
        self.expect(b')')?;
        Ok(v)
    }

    fn raw(&mut self, n: usize) -> Result<&'a [u8], FoamError> {
        let end = self.i + n;
        let bytes = self
            .s
            .get(self.i..end)
            .ok_or_else(|| bad("binary list too short"))?;
        self.i = end;
        Ok(bytes)
    }

    /// `N(...)`, `N{v}` or binary `N(bytes)`; `item` reads one ASCII element.
    fn list<T: Clone>(
        &mut self,
        size: usize,
        item: impl Fn(&mut Self) -> Result<T, FoamError>,
        from_bytes: impl Fn(&[u8]) -> T,
    ) -> Result<Vec<T>, FoamError> {
        let n: usize = self
            .token()
            .parse()
            .map_err(|_| bad("list without a length"))?;
        match self.peek() {
            Some(b'{') => {
                self.i += 1;
                let v = item(self)?;
                self.expect(b'}')?;
                Ok(vec![v; n])
            }
            Some(b'(') => {
                self.i += 1;
                let out = if self.binary {
                    self.raw(n * size)?.chunks(size).map(&from_bytes).collect()
                } else {
                    (0..n).map(|_| item(self)).collect::<Result<Vec<_>, _>>()?
                };
                self.expect(b')')?;
                Ok(out)
            }
            _ => Err(bad("expected '(' or '{' after the list length")),
        }
    }
}

fn find(hay: &[u8], needle: &[u8]) -> Option<usize> {
    hay.windows(needle.len()).position(|w| w == needle)
}

fn f64_at(b: &[u8]) -> f64 {
    f64::from_le_bytes(b[..8].try_into().unwrap_or([0; 8]))
}

/// Binary format and the position after the FoamFile header (0 without a header).
fn header(bytes: &[u8]) -> (bool, usize) {
    let Some(start) = find(bytes, b"FoamFile") else {
        return (false, 0);
    };
    let end = find(&bytes[start..], b"}").map_or(start, |e| start + e + 1);
    let head = String::from_utf8_lossy(&bytes[start..end]);
    let binary = head
        .split(';')
        .any(|part| part.split_whitespace().collect::<Vec<_>>() == ["format", "binary"]);
    (binary, end)
}

fn scalar_list(c: &mut Cursor<'_>) -> Result<Vec<f64>, FoamError> {
    c.list(8, |c| c.number(), f64_at)
}

fn vector_list(c: &mut Cursor<'_>) -> Result<Vec<[f64; 3]>, FoamError> {
    c.list(
        24,
        |c| c.vector(),
        |b| [f64_at(b), f64_at(&b[8..]), f64_at(&b[16..])],
    )
}

/// A list of scalars (`N(...)`, `N{v}`; binary when the header says so).
pub fn read_scalar_list(bytes: &[u8]) -> Result<Vec<f64>, FoamError> {
    let (binary, start) = header(bytes);
    let mut c = Cursor::new(&bytes[start..], binary);
    scalar_list(&mut c)
}

/// A list of vectors.
pub fn read_vector_list(bytes: &[u8]) -> Result<Vec<[f64; 3]>, FoamError> {
    let (binary, start) = header(bytes);
    let mut c = Cursor::new(&bytes[start..], binary);
    vector_list(&mut c)
}

/// Points of a surface (a vector list).
pub fn read_points(bytes: &[u8]) -> Result<Vec<[f64; 3]>, FoamError> {
    read_vector_list(bytes)
}

/// Faces `N(3(a b c) ...)` (ASCII).
pub fn read_faces(bytes: &[u8]) -> Result<Vec<Vec<u32>>, FoamError> {
    let (binary, start) = header(bytes);
    if binary {
        return Err(bad("binary face lists are not supported"));
    }
    let mut c = Cursor::new(&bytes[start..], false);
    c.list(
        0,
        |c| {
            let n: usize = c.token().parse().map_err(|_| bad("face size"))?;
            c.expect(b'(')?;
            let f = (0..n)
                .map(|_| c.token().parse::<u32>().map_err(|_| bad("face index")))
                .collect::<Result<Vec<_>, _>>()?;
            c.expect(b')')?;
            Ok(f)
        },
        |_| Vec::new(),
    )
}

/// The internal field of a field file: `uniform value` or `nonuniform List<T> N(...)`.
pub fn read_internal_field(bytes: &[u8]) -> Result<FieldData, FoamError> {
    let (binary, start) = header(bytes);
    let body = &bytes[start..];
    let at = find(body, b"internalField").ok_or_else(|| bad("no internalField"))?;
    let mut c = Cursor::new(&body[at + "internalField".len()..], binary);
    match c.token() {
        "uniform" => {
            if c.peek() == Some(b'(') {
                Ok(FieldData::UniformVector(c.vector()?))
            } else {
                Ok(FieldData::UniformScalar(c.number()?))
            }
        }
        "nonuniform" => match c.token() {
            "List<scalar>" => Ok(FieldData::Scalars(scalar_list(&mut c)?)),
            "List<vector>" => Ok(FieldData::Vectors(vector_list(&mut c)?)),
            other => Err(bad(&format!("unsupported field type {other}"))),
        },
        other => Err(bad(&format!("unexpected {other} after internalField"))),
    }
}

/// Rows of numbers of a raw sample set (`x y z values...`).
pub fn read_raw_set(bytes: &[u8]) -> Result<Vec<Vec<f64>>, FoamError> {
    let text = std::str::from_utf8(bytes).map_err(|_| bad("sample set is not text"))?;
    text.lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .map(|l| {
            l.split_whitespace()
                .map(|t| {
                    t.parse::<f64>()
                        .map_err(|_| bad(&format!("not a number: {t}")))
                })
                .collect()
        })
        .collect()
}
