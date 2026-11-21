// SPDX-License-Identifier: GPL-3.0-or-later
//! Sample projects (self-made, CC0, in `samples/`): `samples.list` and `samples.create`, which
//! writes a copy into a new folder and opens it.

use std::path::PathBuf;

use foamcraft_core::command::{Options, Registry, Session};
use foamcraft_core::error::FoamError;
use foamcraft_core::project::{MANIFEST_FILE, Manifest};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};

struct Sample {
    id: &'static str,
    manifest: &'static str,
    model: &'static [u8],
    /// Title and description, English and German.
    texts: [(&'static str, &'static str); 2],
}

const SAMPLES: [Sample; 2] = [
    Sample {
        id: "wind_tunnel",
        manifest: include_str!("../../../samples/wind_tunnel/foamcraft.json"),
        model: include_bytes!("../../../samples/wind_tunnel/geometry/model-1.stl"),
        texts: [
            (
                "Rounded body",
                "A block with rounded front and side edges in a wind tunnel at 20 m/s: drag, lift and the flow around a body.",
            ),
            (
                "Abgerundeter Körper",
                "Ein Block mit abgerundeten Vorder- und Seitenkanten im Windkanal bei 20 m/s: Widerstand, Auftrieb und die Umströmung eines Körpers.",
            ),
        ],
    },
    Sample {
        id: "manifold",
        manifest: include_str!("../../../samples/manifold/foamcraft.json"),
        model: include_bytes!("../../../samples/manifold/geometry/model-1.stl"),
        texts: [
            (
                "Pipe manifold",
                "A header pipe with three branches, water at 0.5 m/s: pressure loss and how the flow divides.",
            ),
            (
                "Rohrverteiler",
                "Ein Sammelrohr mit drei Abgängen, Wasser mit 0,5 m/s: Druckverlust und wie sich die Strömung aufteilt.",
            ),
        ],
    },
];

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CreateParams {
    /// `wind_tunnel` or `manifold`.
    pub sample: String,
    /// New project folder (must not contain a project yet). Default: a free folder named after
    /// the sample in the default projects folder.
    #[serde(default)]
    pub path: Option<PathBuf>,
    /// Language of the project name, `en` (default) or `de`.
    #[serde(default)]
    pub language: Option<String>,
}

fn create(s: &mut Session, p: CreateParams) -> Result<Value, FoamError> {
    let sample = SAMPLES
        .iter()
        .find(|x| x.id == p.sample)
        .ok_or_else(|| FoamError::new("samples.unknown", p.sample.clone()))?;
    let de = p.language.as_deref() == Some("de");
    let name = sample.texts[usize::from(de)].0;
    let path = match p.path {
        Some(path) => path,
        None => foamcraft_core::paths::free_folder(s.projects_dir(), name),
    };
    if path.join(MANIFEST_FILE).exists() {
        return Err(FoamError::new(
            "project.already_exists",
            path.display().to_string(),
        ));
    }
    let io = |e: std::io::Error| FoamError::new("project.write_failed", e.to_string());
    let mut manifest: Manifest = serde_json::from_str(sample.manifest)
        .map_err(|e| FoamError::new("project.invalid_manifest", e.to_string()))?;
    manifest.name = name.to_string();
    manifest.created = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());
    std::fs::create_dir_all(path.join("geometry")).map_err(io)?;
    std::fs::write(path.join("geometry/model-1.stl"), sample.model).map_err(io)?;
    let text = serde_json::to_string_pretty(&manifest)
        .map_err(|e| FoamError::new("project.write_failed", e.to_string()))?;
    std::fs::write(path.join(MANIFEST_FILE), text).map_err(io)?;
    foamcraft_core::command::opened(s, foamcraft_core::project::Project::open(&path)?);
    Ok(json!({"path": path, "name": manifest.name, "sample": sample.id}))
}

pub fn register(r: &mut Registry) {
    r.add(
        "samples.list",
        "List the sample projects with title and description in English and German.",
        Options {
            undoable: false,
            errors: &[],
            example: json!({}),
        },
        |_, _: crate::results::JobParams| {
            Ok(SAMPLES
                .iter()
                .map(|x| {
                    json!({
                        "id": x.id,
                        "en": {"title": x.texts[0].0, "description": x.texts[0].1},
                        "de": {"title": x.texts[1].0, "description": x.texts[1].1},
                    })
                })
                .collect::<Vec<_>>())
        },
    );
    r.add(
        "samples.create",
        "Create a sample project (wind_tunnel or manifold) in a new folder (default: the projects folder) and open it.",
        Options {
            undoable: false,
            errors: &[
                "samples.unknown",
                "project.already_exists",
                "project.write_failed",
            ],
            example: json!({"sample": "wind_tunnel", "path": "C:/Users/me/CFD/Sample"}),
        },
        create,
    );
}
