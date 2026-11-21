// SPDX-License-Identifier: GPL-3.0-or-later
//! Sources of air with real OpenFOAM (Task_31): a source blowing at a model, two sources, and the
//! decay of a free round jet against the literature.
//!
//! Tolerance of the validation, fixed before the first run with a source (2026-11-21) and not to
//! be widened: far from the source the centre-line speed of a round turbulent jet falls as
//! U_axis / U_0 = B d / (x - x_0) with B = 5.8 to 6.1 (Hussein, Capp and George 1994; Pope,
//! "Turbulent Flows", ch. 5). Two-equation models spread the round jet too much (the "round jet
//! anomaly", about 15-25 % too wide, so B too low), the source is a velocity constraint without a
//! nozzle profile, and the draft mesh has 10 cells over d at the source and 2.5 at 30 d. Accepted:
//! 4.0 <= B <= 7.5, fitted on 15 d <= x <= 30 d.

use std::path::PathBuf;

use foamcraft_core::command::{Registry, Session};
use serde_json::{Value, json};

const B_MIN: f64 = 4.0;
const B_MAX: f64 = 7.5;

fn clean(s: &mut Session) {
    if let Ok(project) = s.project() {
        let store = foamcraft_core::job::JobStore::new(project.root.join("runs"));
        let runner = foamcraft_runtime::DockerRunner::local();
        for record in store.list().unwrap_or_default() {
            let _ = foamcraft_runtime::jobs::remove_job_resources(&runner, &record);
        }
    }
}

/// A project with one model (the 20 mm cube in `unit`), a still wind tunnel and these sources.
fn project(name: &str, unit: &str, jets: Value) -> (Registry, Session) {
    let reg = foamcraft_api::registry();
    let base = std::env::temp_dir().join(format!("fc-jet-rt-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&base);
    let mut s = Session::new(base.join("data"));
    reg.execute(
        &mut s,
        "project.new",
        json!({"path": base.join("p"), "name": name}),
    )
    .unwrap();
    let model =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/data/geometry/cube_binary.stl");
    reg.execute(
        &mut s,
        "geometry.import",
        json!({"path": model, "unit": unit}),
    )
    .unwrap();
    reg.execute(
        &mut s,
        "scenario.set_wind_tunnel",
        json!({"speed_m_s": 10.0, "direction_deg": 0.0, "ground": "none",
               "medium": {"kind": "air", "temperature_c": 20.0}, "quality": "draft"}),
    )
    .unwrap();
    reg.execute(&mut s, "scenario.set_jets", json!({"jets": jets}))
        .unwrap();
    (reg, s)
}

fn run(reg: &Registry, s: &mut Session) -> Value {
    let out = reg.execute(s, "run.start", json!({"wait": true}));
    match out {
        Ok(o) => o["result"].clone(),
        Err(e) => {
            clean(s);
            panic!("run failed: {e}")
        }
    }
}

fn jet(center: [f64; 3], d: f64, speed: f64) -> Value {
    json!({"shape": "circle", "center": center, "direction": [1.0, 0.0, 0.0],
           "size": [d, d], "speed_m_s": speed})
}

#[test]
#[ignore = "needs Docker and the runtime image"]
fn runtime_one_and_two_sources_blowing_at_a_model() {
    // 0.2 m cube (unit cm), sources 0.3 m in front of it.
    for (name, jets) in [
        ("one", json!([jet([-0.3, 0.1, 0.1], 0.06, 15.0)])),
        (
            "two",
            json!([
                jet([-0.3, 0.05, 0.1], 0.04, 15.0),
                jet([-0.3, 0.15, 0.1], 0.04, 15.0)
            ]),
        ),
    ] {
        let (reg, mut s) = project(&format!("blow-{name}"), "cm", jets);
        let r = run(&reg, &mut s);
        clean(&mut s);
        eprintln!(
            "{name}: reason {} iterations {} cells {} drag {} N",
            r["reason"], r["iterations"], r["cells"], r["forces"]["drag_n"]
        );
        assert!(
            ["stable", "oscillating", "limit_reached"].contains(&r["reason"].as_str().unwrap()),
            "{name}: {}",
            r["reason"]
        );
        // The jets push the cube downstream.
        assert!(r["forces"]["drag_n"].as_f64().unwrap() > 0.0, "{name}: {r}");
    }
}

/// Centre-line speed of the jet, sampled in the volume of the solve job with OpenFOAM.
fn axis_speed(volume: &str, from: [f64; 3], to: [f64; 3], n: u32) -> Vec<(f64, f64)> {
    let dict = format!(
        "type sets; libs (sampling); interpolationScheme cellPoint; setFormat raw; fields (U);\n\
         sets {{ axis {{ type uniform; axis x; start ({} {} {}); end ({} {} {}); nPoints {n}; }} }}\n",
        from[0], from[1], from[2], to[0], to[1], to[2]
    );
    let script = format!(
        "cd /work/case && cat > system/jetAxis <<'EOT'\n{dict}EOT\n\
         postProcess -func jetAxis -latestTime > /dev/null 2>&1; cat postProcessing/jetAxis/*/axis_U.xy"
    );
    let out = std::process::Command::new("docker")
        .args([
            "run",
            "--rm",
            "--entrypoint",
            "bash",
            "-v",
            &format!("{volume}:/work"),
            "foamcraft-runtime:0.1.0",
            "-lc",
            &script,
        ])
        .output()
        .expect("docker");
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| {
            let v: Vec<f64> = l
                .split_whitespace()
                .filter_map(|x| x.parse().ok())
                .collect();
            (v.len() == 4).then(|| (v[0], v[1]))
        })
        .collect()
}

#[test]
#[ignore = "slow CFD reference case; needs Docker and the runtime image"]
fn validation_round_free_jet_decays_like_the_literature() {
    // A 0.1 m jet at 10 m/s; the only model (a 20 mm cube) sits 1.5 m to the side, out of the
    // way, at the far end of the jet so the jet is refined over 30 d.
    let d = 0.1;
    let u0 = 10.0;
    let source = [-3.0, 1.5, 0.01];
    let (reg, mut s) = project("validation", "mm", json!([jet(source, d, u0)]));
    let r = run(&reg, &mut s);
    let job = r["job_id"].as_str().unwrap().to_string();
    let root = s.project().unwrap().root.clone();
    let record = foamcraft_core::job::JobStore::new(root.join("runs"))
        .load(&job)
        .unwrap();
    let mesh = s.project().unwrap().manifest.mesh.clone().unwrap();
    let mesh_record = foamcraft_core::job::JobStore::new(root.join("runs"))
        .load(&mesh.job_id)
        .unwrap();
    let volume = record
        .volume
        .or(mesh_record.volume)
        .expect("volume of the run");
    let from = [source[0] + 15.0 * d, source[1], source[2]];
    let to = [source[0] + 30.0 * d, source[1], source[2]];
    let samples = axis_speed(&volume, from, to, 16);
    clean(&mut s);
    eprintln!(
        "reason {} cells {} samples {samples:?}",
        r["reason"], r["cells"]
    );
    assert!(samples.len() >= 10, "too few samples: {samples:?}");
    // U0 / U = (x - x0) / (B d): a straight line in x with slope 1 / (B d).
    let pts: Vec<(f64, f64)> = samples
        .iter()
        .filter(|(_, u)| *u > 1e-6)
        .map(|(x, u)| ((x - source[0]) / d, u0 / u))
        .collect();
    let n = pts.len() as f64;
    let (sx, sy) = pts.iter().fold((0.0, 0.0), |a, p| (a.0 + p.0, a.1 + p.1));
    let (mx, my) = (sx / n, sy / n);
    let slope = pts.iter().map(|p| (p.0 - mx) * (p.1 - my)).sum::<f64>()
        / pts.iter().map(|p| (p.0 - mx).powi(2)).sum::<f64>();
    let b = 1.0 / slope;
    let x0 = mx - my / slope;
    eprintln!("decay constant B = {b:.2}, virtual origin x0 = {x0:.1} d");
    assert!(
        (B_MIN..=B_MAX).contains(&b),
        "B = {b:.2} outside {B_MIN}..{B_MAX} (literature 5.8-6.1)"
    );
}
