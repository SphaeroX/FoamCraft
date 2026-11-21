// SPDX-License-Identifier: GPL-3.0-or-later
//! A generated model with one million triangles loads and is diagnosed within a time limit.

use std::io::Write;
use std::time::{Duration, Instant};

use foamcraft_geometry::{diagnose, read_mesh};

/// Time limit for reading and diagnosing one million triangles (binary STL). The geometry crate
/// is built with optimisations even in test builds (Cargo.toml profile override); the reference
/// machine needs well under one second, the limit leaves room for slower machines.
const LIMIT: Duration = Duration::from_secs(10);

/// Writes a closed sphere of `n x n` quads (2 n^2 triangles, poles included) as binary STL.
fn write_sphere(path: &std::path::Path, n: usize) -> usize {
    let point = |i: usize, j: usize| {
        // sin(PI) is not exactly zero in f32; snap the poles so they weld into one vertex.
        if i == 0 || i == n {
            return [0.0, 0.0, if i == 0 { 1.0 } else { -1.0 }];
        }
        let theta = std::f32::consts::PI * i as f32 / n as f32;
        let phi = 2.0 * std::f32::consts::PI * (j % n) as f32 / n as f32;
        [
            theta.sin() * phi.cos(),
            theta.sin() * phi.sin(),
            theta.cos(),
        ]
    };
    let mut tris = Vec::new();
    for i in 0..n {
        for j in 0..n {
            let (a, b, c, d) = (
                point(i, j),
                point(i + 1, j),
                point(i + 1, j + 1),
                point(i, j + 1),
            );
            if i != 0 {
                tris.push([a, b, d]);
            }
            if i != n - 1 {
                tris.push([b, c, d]);
            }
        }
    }
    let mut out = std::io::BufWriter::new(std::fs::File::create(path).unwrap());
    out.write_all(&[0u8; 80]).unwrap();
    out.write_all(&(tris.len() as u32).to_le_bytes()).unwrap();
    for t in &tris {
        out.write_all(&[0u8; 12]).unwrap();
        for p in t {
            for c in p {
                out.write_all(&c.to_le_bytes()).unwrap();
            }
        }
        out.write_all(&[0u8; 2]).unwrap();
    }
    tris.len()
}

#[test]
fn one_million_triangles_load_within_the_limit() {
    let path = std::env::temp_dir().join(format!("fc-sphere-{}.stl", std::process::id()));
    let count = write_sphere(&path, 708);
    assert!(count >= 1_000_000, "{count}");
    let t0 = Instant::now();
    let mesh = read_mesh(&path).unwrap();
    let d = diagnose(&mesh);
    let elapsed = t0.elapsed();
    eprintln!(
        "{count} triangles read and diagnosed in {:.2} s",
        elapsed.as_secs_f64()
    );
    assert_eq!(mesh.triangles.len(), count);
    assert!(d.watertight, "generated sphere must be closed");
    assert!(elapsed < LIMIT, "took {elapsed:?}");
    std::fs::remove_file(&path).unwrap();
}

#[test]
fn one_million_triangles_are_segmented_within_the_limit() {
    let path = std::env::temp_dir().join(format!("fc-sphere-seg-{}.stl", std::process::id()));
    write_sphere(&path, 708);
    let mesh = read_mesh(&path).unwrap();
    let t0 = Instant::now();
    let s = foamcraft_geometry::segment(&mesh, 30.0);
    let elapsed = t0.elapsed();
    eprintln!(
        "segmented {} triangles into {} faces in {:.2} s",
        mesh.triangles.len(),
        s.faces.len(),
        elapsed.as_secs_f64()
    );
    assert_eq!(s.faces.len(), 1);
    assert!(elapsed < LIMIT, "took {elapsed:?}");
    std::fs::remove_file(&path).unwrap();
}
