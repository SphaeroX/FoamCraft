// SPDX-License-Identifier: GPL-3.0-or-later
//! A project is a folder with a versioned JSON manifest plus assets:
//!
//! ```text
//! <project>/foamcraft.json   manifest
//! <project>/geometry/        imported geometry
//! <project>/runs/            run data copied from the runtime
//! <project>/cache/           derived data that can be rebuilt
//! ```

use std::path::{Path, PathBuf};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::error::FoamError;
use crate::units::DisplayUnits;

pub const MANIFEST_FILE: &str = "foamcraft.json";
/// Version of the manifest format this build writes. Older manifests are migrated on open.
pub const SCHEMA_VERSION: u32 = 11;
const FOLDERS: [&str; 3] = ["geometry", "runs", "cache"];

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Manifest {
    pub schema_version: u32,
    pub name: String,
    /// Unix time in seconds.
    pub created: u64,
    #[serde(default)]
    pub display_units: DisplayUnits,
    /// Job ids of the runs of this project, oldest first.
    #[serde(default)]
    pub runs: Vec<String>,
    /// Imported models (since version 2).
    #[serde(default)]
    pub geometry: Vec<GeometryEntry>,
    /// Names given to faces, by face key (`model-1:face-3`) (since version 3).
    #[serde(default)]
    pub surface_names: std::collections::BTreeMap<String, String>,
    /// Named groups of faces, for example inlets (since version 3).
    #[serde(default)]
    pub groups: Vec<SurfaceGroup>,
    /// What is simulated (since version 5).
    #[serde(default)]
    pub scenario: Option<Scenario>,
    /// The latest mesh of the scenario (since version 6).
    #[serde(default)]
    pub mesh: Option<MeshRecord>,
    /// Caps that close openings of models (since version 7).
    #[serde(default)]
    pub caps: Vec<CapRecord>,
    /// Inlet, outlet or wall per face or cap key (since version 7).
    #[serde(default)]
    pub boundary_roles: std::collections::BTreeMap<String, BoundaryRole>,
    /// What the 3D view shows of the results (since version 8).
    #[serde(default)]
    pub view: ViewSettings,
    /// Names given to runs, by job id (since version 11).
    #[serde(default)]
    pub run_names: std::collections::BTreeMap<String, String>,
}

/// Result display of the 3D view.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ViewSettings {
    /// Job whose result is shown.
    #[serde(default)]
    pub result_job: Option<String>,
    /// Surface field the model is coloured by (`p`, `Cp`, `wall_shear_stress`, `y_plus`).
    #[serde(default)]
    pub surface_field: Option<String>,
    /// Colour scale range; `None` is the range of the field.
    #[serde(default)]
    pub range: Option<[f64; 2]>,
    /// Planes through the flow coloured by a field (since version 9).
    #[serde(default)]
    pub slices: Vec<SliceSpec>,
    /// Surfaces of equal value (since version 9).
    #[serde(default)]
    pub isosurfaces: Vec<IsoSpec>,
    /// The model is cut open along this plane; the side the normal points to is hidden.
    #[serde(default)]
    pub clip: Option<PlaneSpec>,
    /// Animated particles or streamlines (since version 10).
    #[serde(default)]
    pub particles: Option<ParticleSpec>,
    /// Two runs compared in the view (since version 11).
    #[serde(default)]
    pub compare: Option<CompareSpec>,
}

/// Two runs in the view: side by side with one camera (`split`) or the surface field of `b`
/// minus `a` on the model (`difference`, same geometry only).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct CompareSpec {
    pub a: String,
    pub b: String,
    pub mode: String,
}

/// Particles or streamlines started in a region.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ParticleSpec {
    /// `comets` (points with a tail), `smoke` (light, translucent) or `streamlines`.
    pub style: String,
    pub count: u32,
    /// Start value of the random numbers: the same value gives the same animation.
    pub seed: u32,
    /// `speed`, `pressure` or `age` (time since the start).
    pub color_by: String,
    /// Animation speed relative to real time.
    pub speed: f64,
    pub paused: bool,
    pub region: SeedRegionSpec,
}

/// Where particles start: a rectangle or line (origin + s u + t v, s and t in 0..1), the inlets,
/// or the model surface.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct SeedRegionSpec {
    /// `area` (origin, u, v), `inlets` or `surface`.
    pub kind: String,
    #[serde(default)]
    pub origin: [f64; 3],
    #[serde(default)]
    pub u: [f64; 3],
    #[serde(default)]
    pub v: [f64; 3],
}

/// A plane in the world (metres).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct PlaneSpec {
    pub origin: [f64; 3],
    pub normal: [f64; 3],
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct SliceSpec {
    /// Stable id, for example `slice-1`.
    pub id: String,
    #[serde(flatten)]
    pub plane: PlaneSpec,
    /// Volume field: `U_mag`, `p`, `p_total`, `k`, `Q`, `Ux`, `Uy`, `Uz`.
    pub field: String,
    #[serde(default)]
    pub range: Option<[f64; 2]>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct IsoSpec {
    /// Stable id, for example `iso-1`.
    pub id: String,
    /// `U_mag`, `p_total` or `Q`.
    pub field: String,
    pub value: f64,
}

/// A flat cover over an opening of a model.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct CapRecord {
    /// Stable key, for example `model-1:cap-1`.
    pub key: String,
    /// Model id.
    pub model: String,
    /// Loops of the opening in the file coordinates of the model; the first is the outer one.
    pub loops: Vec<Vec<[f64; 3]>>,
}

/// What a face or cap is for the flow.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum BoundaryRole {
    Inlet,
    Outlet,
    Wall,
}

/// Summary of a generated mesh; the mesh itself stays on the job's runtime volume.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct MeshRecord {
    /// Job whose volume holds the meshed case.
    pub job_id: String,
    pub cells: u64,
    /// good, acceptable or poor.
    pub verdict: String,
    pub layers_target: u32,
    /// Mean number of layers built.
    pub layers_achieved: f64,
    /// Catalog codes of the fallbacks that were needed.
    pub fallbacks: Vec<String>,
}

/// The kind of simulation and its physical inputs (SI units).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Scenario {
    /// External flow around the models in a virtual wind tunnel.
    WindTunnel(WindTunnel),
    /// Flow through the models (pipes, housings) from inlets to outlets.
    InternalFlow(InternalFlow),
}

/// Internal flow: medium, quality and what happens at the inlets and outlets (caps or faces
/// marked with a boundary role).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct InternalFlow {
    pub medium: MediumSpec,
    pub quality: Quality,
    pub inlets: Vec<InletSpec>,
    pub outlets: Vec<OutletSpec>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct InletSpec {
    /// Cap or face key with the role inlet.
    pub key: String,
    pub condition: InletCondition,
}

/// What is given at an inlet.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum InletCondition {
    /// Mean velocity normal to the opening, m/s.
    Velocity { m_s: f64 },
    /// Volume flow, m³/s.
    VolumeFlow { m3_s: f64 },
    /// Mass flow, kg/s.
    MassFlow { kg_s: f64 },
    /// Total pressure above the reference, Pa.
    TotalPressure { pa: f64 },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct OutletSpec {
    /// Cap or face key with the role outlet.
    pub key: String,
    /// Static pressure above the reference, Pa.
    pub pressure_pa: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct WindTunnel {
    /// Inflow speed in metres per second.
    pub speed_m_s: f64,
    /// Flow direction in the ground plane, degrees (0 = towards +X, 90 = towards +Y).
    pub direction_deg: f64,
    pub ground: Ground,
    pub medium: MediumSpec,
    pub quality: Quality,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Ground {
    /// Free flow all around the model.
    None,
    /// A fixed floor at the lowest point of the model.
    Fixed,
    /// A floor moving with the flow (planned; computed from Task_39 on).
    Moving,
}

/// Fluid: a preset at a temperature, or custom properties.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum MediumSpec {
    /// Air at 101325 Pa.
    Air {
        temperature_c: f64,
    },
    Water {
        temperature_c: f64,
    },
    Custom {
        /// Density in kilograms per cubic metre.
        density: f64,
        /// Kinematic viscosity in square metres per second.
        kinematic_viscosity: f64,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Quality {
    Draft,
    Standard,
    Fine,
}

/// A named set of faces.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct SurfaceGroup {
    pub name: String,
    /// Face keys such as `model-1:face-3`, sorted.
    pub keys: Vec<String>,
}

fn default_crease_angle() -> f64 {
    30.0
}

/// A model file in `geometry/` and how it sits in the world.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct GeometryEntry {
    /// Stable id, for example `model-1`.
    pub id: String,
    /// Original file name.
    pub name: String,
    /// Path of the triangle mesh used by FoamCraft, relative to the project folder.
    pub file: String,
    /// Original CAD file (STEP, IGES) the mesh was converted from (since version 4).
    #[serde(default)]
    pub source: Option<String>,
    /// Unit of the file coordinates: um, mm, cm, m, in or ft.
    pub unit: String,
    /// Extra uniform scale factor.
    pub scale: f64,
    /// Yaw, pitch, roll in degrees.
    pub rotation_deg: [f64; 3],
    /// Translation in metres.
    pub translation: [f64; 3],
    /// Orient all triangles consistently outward when the model is loaded.
    #[serde(default)]
    pub fix_normals: bool,
    /// Edges sharper than this angle (degrees) separate faces.
    #[serde(default = "default_crease_angle")]
    pub crease_angle_deg: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Project {
    pub root: PathBuf,
    pub manifest: Manifest,
}

fn io(code: &str, path: &Path, e: impl std::fmt::Display) -> FoamError {
    FoamError::new(code, format!("{}: {e}", path.display()))
}

/// Upgrades an older manifest document step by step to [`SCHEMA_VERSION`].
fn migrate(mut doc: Value, from: u32) -> Value {
    if from < 2 {
        // Version 2 added the list of imported models.
        doc["geometry"] = Value::Array(Vec::new());
    }
    if from < 3 {
        // Version 3 added face names and groups; models get the default crease angle.
        doc["surface_names"] = Value::Object(Default::default());
        doc["groups"] = Value::Array(Vec::new());
    }
    // Version 4 added the optional CAD source of a model, version 5 the optional scenario,
    // version 6 the optional mesh record, version 7 caps and boundary roles, version 8 the view
    // settings, version 9 slices, isosurfaces and the clip plane, version 10 particles, version 11
    // run names; nothing to convert.
    doc
}

impl Project {
    /// Creates a new project in `root` (the folder may exist but must not hold a project).
    pub fn create(root: &Path, name: &str) -> Result<Self, FoamError> {
        let manifest_path = root.join(MANIFEST_FILE);
        if manifest_path.exists() {
            return Err(FoamError::new(
                "project.already_exists",
                root.display().to_string(),
            ));
        }
        for folder in FOLDERS {
            std::fs::create_dir_all(root.join(folder))
                .map_err(|e| io("project.write_failed", root, e))?;
        }
        let created = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_secs());
        let project = Self {
            root: root.to_path_buf(),
            manifest: Manifest {
                schema_version: SCHEMA_VERSION,
                name: name.into(),
                created,
                display_units: DisplayUnits::default(),
                runs: Vec::new(),
                geometry: Vec::new(),
                surface_names: Default::default(),
                groups: Vec::new(),
                scenario: None,
                mesh: None,
                caps: Vec::new(),
                boundary_roles: Default::default(),
                view: ViewSettings::default(),
                run_names: Default::default(),
            },
        };
        project.save()?;
        Ok(project)
    }

    pub fn open(root: &Path) -> Result<Self, FoamError> {
        let path = root.join(MANIFEST_FILE);
        if !path.is_file() {
            return Err(FoamError::new(
                "project.not_found",
                root.display().to_string(),
            ));
        }
        let text =
            std::fs::read_to_string(&path).map_err(|e| io("project.invalid_manifest", &path, e))?;
        let doc: Value =
            serde_json::from_str(&text).map_err(|e| io("project.invalid_manifest", &path, e))?;
        let version = doc
            .get("schema_version")
            .and_then(Value::as_u64)
            .ok_or_else(|| io("project.invalid_manifest", &path, "schema_version missing"))?
            as u32;
        if version > SCHEMA_VERSION {
            return Err(FoamError::new(
                "project.schema_too_new",
                format!("version {version}, this build reads up to {SCHEMA_VERSION}"),
            ));
        }
        let mut manifest: Manifest = serde_json::from_value(migrate(doc, version))
            .map_err(|e| io("project.invalid_manifest", &path, e))?;
        manifest.schema_version = SCHEMA_VERSION;
        Ok(Self {
            root: root.to_path_buf(),
            manifest,
        })
    }

    /// Writes the manifest (atomically: temporary file, then rename).
    pub fn save(&self) -> Result<(), FoamError> {
        let path = self.root.join(MANIFEST_FILE);
        let json = serde_json::to_string_pretty(&self.manifest)
            .map_err(|e| io("project.write_failed", &path, e))?;
        let tmp = self.root.join(format!("{MANIFEST_FILE}.tmp"));
        std::fs::write(&tmp, json).map_err(|e| io("project.write_failed", &tmp, e))?;
        std::fs::rename(&tmp, &path).map_err(|e| io("project.write_failed", &path, e))
    }
}
