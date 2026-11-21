// SPDX-License-Identifier: GPL-3.0-or-later
//! Sample projects: listed with texts in both languages, created as projects of the current
//! schema, with geometry that passes the diagnosis and a ready scenario.

use foamcraft_core::command::{Registry, Session};
use foamcraft_core::project::SCHEMA_VERSION;
use serde_json::{Value, json};

fn session(name: &str) -> (Registry, Session, std::path::PathBuf) {
    let base = std::env::temp_dir().join(format!("fc-samples-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&base);
    (
        foamcraft_api::registry(),
        Session::new(base.join("data")),
        base,
    )
}

fn run(reg: &Registry, s: &mut Session, name: &str, params: Value) -> Value {
    reg.execute(s, name, params)
        .unwrap_or_else(|e| panic!("{name}: {e}"))
}

#[test]
fn samples_are_listed_in_both_languages() {
    let (reg, mut s, _) = session("list");
    let list = run(&reg, &mut s, "samples.list", json!({}));
    let ids: Vec<&str> = list
        .as_array()
        .unwrap()
        .iter()
        .map(|x| x["id"].as_str().unwrap())
        .collect();
    assert_eq!(ids, ["wind_tunnel", "manifold"]);
    for item in list.as_array().unwrap() {
        for lang in ["en", "de"] {
            assert!(!item[lang]["title"].as_str().unwrap().is_empty());
            assert!(!item[lang]["description"].as_str().unwrap().is_empty());
        }
    }
}

#[test]
fn the_wind_tunnel_sample_is_a_closed_body_with_a_scenario() {
    let (reg, mut s, base) = session("wind");
    let out = run(
        &reg,
        &mut s,
        "samples.create",
        json!({"sample": "wind_tunnel", "path": base.join("wind"), "language": "de"}),
    );
    assert_eq!(out["name"], "Abgerundeter Körper");
    // The sample is open now.
    let info = run(&reg, &mut s, "project.info", json!({}));
    assert_eq!(info["schema_version"], SCHEMA_VERSION);
    let model = run(&reg, &mut s, "geometry.info", json!({"id": "model-1"}));
    assert_eq!(model["diagnosis"]["watertight"], true);
    assert_eq!(model["diagnosis"]["inconsistent_normals"], false);
    let scenario = run(&reg, &mut s, "scenario.info", json!({}));
    assert_eq!(scenario["scenario"]["kind"], "wind_tunnel");
    // A second copy in the same place is refused, the first stays intact.
    let err = reg
        .execute(
            &mut s,
            "samples.create",
            json!({"sample": "wind_tunnel", "path": base.join("wind")}),
        )
        .unwrap_err();
    assert_eq!(err.code, "project.already_exists");
}

#[test]
fn the_manifold_sample_is_sealed_with_inlet_and_outlets() {
    let (reg, mut s, base) = session("manifold");
    run(
        &reg,
        &mut s,
        "samples.create",
        json!({"sample": "manifold", "path": base.join("m")}),
    );
    let check = run(&reg, &mut s, "geometry.fluid_check", json!({}));
    assert_eq!(check["sealed"], true);
    assert_eq!(check["caps"], 4);
    let roles = run(&reg, &mut s, "scenario.boundary_roles", json!({}));
    let text = roles.to_string();
    assert!(text.contains("inlet") && text.contains("outlet"), "{text}");
    let scenario = run(&reg, &mut s, "scenario.info", json!({}));
    assert_eq!(scenario["scenario"]["kind"], "internal_flow");
}

/// Removes the containers and volumes of the jobs of the open project.
fn clean(s: &mut Session) {
    if let Ok(project) = s.project() {
        let store = foamcraft_core::job::JobStore::new(project.root.join("runs"));
        let runner = foamcraft_runtime::DockerRunner::local();
        for record in store.list().unwrap_or_default() {
            let _ = foamcraft_runtime::jobs::remove_job_resources(&runner, &record);
        }
    }
}

fn run_sample(sample: &str) -> Value {
    let (reg, mut s, base) = session(&format!("rt-{sample}"));
    run(
        &reg,
        &mut s,
        "samples.create",
        json!({"sample": sample, "path": base.join("p")}),
    );
    let out = reg.execute(&mut s, "run.start", json!({"wait": true}));
    clean(&mut s);
    out.unwrap_or_else(|e| panic!("{sample}: {e}"))["result"].clone()
}

#[test]
#[ignore]
fn runtime_the_wind_tunnel_sample_runs_in_draft_quality() {
    let r = run_sample("wind_tunnel");
    let cd = r["coefficients"]["cd"].as_f64().unwrap();
    // A rounded block with a sharp rear: between streamlined and blunt.
    assert!((0.2..1.2).contains(&cd), "{r}");
}

#[test]
#[ignore]
fn runtime_the_manifold_sample_runs_in_draft_quality() {
    let r = run_sample("manifold");
    assert!(r["pressure_loss_pa"].as_f64().unwrap() > 0.0, "{r}");
    assert!(r["mass_balance_error"].as_f64().unwrap() < 0.05, "{r}");
}

#[test]
fn a_sample_without_a_path_goes_to_the_default_folder_and_never_overwrites() {
    let (reg, mut s, base) = session("default");
    s.set_projects_dir(base.join("projects"));
    let first = run(
        &reg,
        &mut s,
        "samples.create",
        json!({"sample": "wind_tunnel", "language": "de"}),
    );
    assert_eq!(
        first["path"],
        json!(base.join("projects").join("Abgerundeter Körper"))
    );
    let second = run(
        &reg,
        &mut s,
        "samples.create",
        json!({"sample": "wind_tunnel", "language": "de"}),
    );
    assert_eq!(
        second["path"],
        json!(base.join("projects").join("Abgerundeter Körper 2"))
    );
    // Both copies are complete projects; the second one is open.
    for folder in ["Abgerundeter Körper", "Abgerundeter Körper 2"] {
        let root = base.join("projects").join(folder);
        assert!(root.join("foamcraft.json").is_file(), "{folder}");
        assert!(root.join("geometry/model-1.stl").is_file(), "{folder}");
    }
    let info = run(&reg, &mut s, "project.info", json!({}));
    assert_eq!(
        info["path"],
        json!(base.join("projects").join("Abgerundeter Körper 2"))
    );
}
