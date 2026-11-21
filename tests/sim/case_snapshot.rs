// SPDX-License-Identifier: GPL-3.0-or-later
//! Snapshot of the generated wind tunnel case for the 20 mm cube (tests/data/snapshots).
//! Update a snapshot only with a stated reason: FOAMCRAFT_UPDATE_SNAPSHOTS=1.

use std::path::{Path, PathBuf};

use foamcraft_core::project::{Ground, MediumSpec, Quality, WindTunnel};
use foamcraft_geometry::Placement;
use foamcraft_sim::case::{ModelSurface, patch_word, write_wind_tunnel_case};
use foamcraft_sim::medium_properties;
use foamcraft_sim::mesh::plan_wind_tunnel;

const FILES: [&str; 14] = [
    "system/controlDict",
    "system/blockMeshDict",
    "system/snappyHexMeshDict",
    "system/fvSchemes",
    "system/fvSolution",
    "system/decomposeParDict",
    "constant/transportProperties",
    "constant/turbulenceProperties",
    "0/U",
    "0/p",
    "0/k",
    "0/omega",
    "0/nut",
    "constant/triSurface/model.stl",
];

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests")
}

fn generate(dir: &Path) {
    let mesh =
        foamcraft_geometry::read_mesh(&root().join("data/geometry/cube_binary.stl")).unwrap();
    let world = Placement {
        unit: "mm".into(),
        ..Default::default()
    }
    .apply(&mesh);
    let wind = WindTunnel {
        speed_m_s: 20.0,
        direction_deg: 0.0,
        ground: Ground::Fixed,
        medium: MediumSpec::Air {
            temperature_c: 20.0,
        },
        quality: Quality::Draft,
    };
    let medium = medium_properties(&wind.medium);
    let plan = plan_wind_tunnel(
        &[&world],
        0.0,
        wind.ground,
        wind.speed_m_s,
        &medium,
        wind.quality,
        200_000,
    );
    // The top face (two triangles) is a group "Inlet face"; the rest is the default region.
    let regions = (0..world.triangles.len())
        .map(|i| {
            if i == 2 || i == 3 {
                patch_word("Inlet face")
            } else {
                "body".into()
            }
        })
        .collect();
    let models = [ModelSurface {
        mesh: &world,
        regions,
    }];
    write_wind_tunnel_case(dir, &plan, &models, &wind, &medium, 300).unwrap();
}

#[test]
fn generated_cube_case_matches_the_snapshot() {
    let dir = std::env::temp_dir().join(format!("fc-snap-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    generate(&dir);
    let snapshots = root().join("data/snapshots/wind_tunnel_cube");
    for file in FILES {
        let got = std::fs::read_to_string(dir.join(file)).unwrap();
        let path = snapshots.join(file);
        if std::env::var_os("FOAMCRAFT_UPDATE_SNAPSHOTS").is_some() {
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(&path, &got).unwrap();
        }
        let want =
            std::fs::read_to_string(&path).unwrap_or_else(|_| panic!("snapshot {file} missing"));
        assert_eq!(
            got.replace("\r\n", "\n"),
            want.replace("\r\n", "\n"),
            "{file} differs"
        );
    }
}

#[test]
fn patch_words_are_valid_openfoam_names() {
    assert_eq!(patch_word("Inlet face"), "Inlet_face");
    assert_eq!(patch_word("2nd"), "g2nd");
    assert_eq!(patch_word("Einlass-Ü"), "Einlass__");
}

#[test]
fn every_placeholder_is_filled() {
    let dir = std::env::temp_dir().join(format!("fc-snap-fill-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    generate(&dir);
    for file in FILES {
        let text = std::fs::read_to_string(dir.join(file)).unwrap();
        assert!(!text.contains("{{"), "placeholder left in {file}");
    }
    let snappy = std::fs::read_to_string(dir.join("system/snappyHexMeshDict")).unwrap();
    assert!(snappy.contains("Inlet_face { level"));
    let stl = std::fs::read_to_string(dir.join("constant/triSurface/model.stl")).unwrap();
    assert!(stl.contains("solid Inlet_face") && stl.contains("solid body"));
}
