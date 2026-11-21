// SPDX-License-Identifier: GPL-3.0-or-later
//! Meshing in the real runtime (needs Docker and the runtime image):
//!     cargo test --workspace -- --ignored runtime_

use std::io::Write;
use std::path::{Path, PathBuf};

use foamcraft_core::command::Session;
use serde_json::{Value, json};

/// Budget used by the tests: small enough for a few minutes of meshing.
const BUDGET: u64 = 150_000;

fn write_sphere(path: &Path, r: f32, n: usize) {
    let point = |i: usize, j: usize| {
        if i == 0 || i == n {
            return [0.0, 0.0, r + if i == 0 { r } else { -r }];
        }
        let t = std::f32::consts::PI * i as f32 / n as f32;
        let f = 2.0 * std::f32::consts::PI * (j % n) as f32 / n as f32;
        [
            r * t.sin() * f.cos(),
            r * t.sin() * f.sin(),
            r + r * t.cos(),
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
}

fn mesh_model(name: &str, model: &Path, unit: &str) -> (Value, PathBuf) {
    let reg = foamcraft_api::registry();
    let base = std::env::temp_dir().join(format!("fc-mesh-rt-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&base);
    let mut s = Session::new(base.join("data"));
    let project = base.join("p");
    reg.execute(
        &mut s,
        "project.new",
        json!({"path": project, "name": name}),
    )
    .unwrap();
    reg.execute(
        &mut s,
        "geometry.import",
        json!({"path": model, "unit": unit}),
    )
    .unwrap();
    reg.execute(
        &mut s,
        "scenario.set_wind_tunnel",
        json!({"speed_m_s": 20.0, "direction_deg": 0.0, "ground": "fixed",
               "medium": {"kind": "air", "temperature_c": 20.0}, "quality": "draft"}),
    )
    .unwrap();
    let out = reg
        .execute(&mut s, "mesh.generate", json!({"max_cells": BUDGET}))
        .unwrap_or_else(|e| panic!("{name}: {e}"));
    (out, project)
}

fn check(name: &str, out: &Value, project: &Path) {
    let cells = out["cells"].as_u64().unwrap();
    eprintln!(
        "{name}: {cells} cells, verdict {}, layers {}, fallbacks {}",
        out["quality"]["verdict"], out["layers"], out["fallbacks"]
    );
    // The budget tolerance of the task: within 30 %.
    let ratio = cells as f64 / BUDGET as f64;
    assert!(
        (0.7..=1.3).contains(&ratio),
        "{name}: {cells} cells for a budget of {BUDGET}"
    );
    assert_eq!(
        out["quality"]["failed_checks"], 0,
        "{name}: checkMesh errors"
    );
    let preview = std::fs::read_to_string(project.join("cache/mesh_preview.obj")).expect("preview");
    assert!(
        preview.lines().filter(|l| l.starts_with("f ")).count() > 100,
        "{name}: preview has no faces"
    );
}

#[test]
#[ignore]
fn runtime_cube_is_meshed_within_budget() {
    let cube =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/data/geometry/cube_binary.stl");
    let (out, project) = mesh_model("cube", &cube, "mm");
    check("cube", &out, &project);
}

#[test]
#[ignore]
fn runtime_sphere_is_meshed_within_budget() {
    let path = std::env::temp_dir().join(format!("fc-sphere-mesh-{}.stl", std::process::id()));
    write_sphere(&path, 0.1, 48);
    let (out, project) = mesh_model("sphere", &path, "m");
    check("sphere", &out, &project);
}
