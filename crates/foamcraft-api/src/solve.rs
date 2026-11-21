// SPDX-License-Identifier: GPL-3.0-or-later
//! Steady runs of the project's scenario (wind tunnel or internal flow): reuse the mesh, solve
//! with simpleFoam, stop when the watched values settle, restart once with cautious numerics after
//! a divergence and store the result object in `<project>/results/<job id>.json`.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use foamcraft_core::command::EventBus;
use foamcraft_core::error::FoamError;
use foamcraft_core::job::{
    JobEvent, JobRecord, JobState, JobStore, RunOptions, Stage, choose_cores,
};
use foamcraft_core::project::{InternalFlow, MeshRecord, Project, Scenario, WindTunnel};
use foamcraft_runtime::jobs::{case_dir, core_limit, execute_job};
use foamcraft_runtime::{Container, Mount, StartOptions};
use foamcraft_sim::case::{ModelSurface, write_wind_tunnel_case};
use foamcraft_sim::internal::{
    BoundaryValues, FieldValueLog, FlowRegime, InternalFlowResult, InternalHistory,
    evaluate_openings,
};
use foamcraft_sim::internal_case::{InternalPlan, write_internal_case};
use foamcraft_sim::jets::FLOW_OBJECT;
use foamcraft_sim::mesh::MeshPlan;
use foamcraft_sim::solve::{
    Assessment, Coefficients, ConvergenceSettings, History, PART_PREFIX, PartResult,
    ReferenceValues, SolverLog, TOTAL_FORCES, Verdict, WindTunnelResult, assess, assess_series,
    assess_with_flow, cautious_numerics, forces_from, remaining_seconds, y_plus_hint,
};
use foamcraft_sim::{cell_budget, flow_direction, frontal_area, medium_properties};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::SharedRunner;
use crate::mesh::prepare_models;

/// Folder of the result objects in a project.
pub const RESULTS_DIR: &str = "results";
/// The iteration limit is this multiple of the expected iterations of the quality level.
pub const LIMIT_FACTOR: u32 = 3;
/// File in the job folder with the setup a resumed job needs.
pub const SETUP_FILE: &str = "solve.json";
/// Inflow and outflow may differ by this share before the result gets a hint.
pub const MASS_BALANCE_LIMIT: f64 = 0.01;

/// The part of a solve job that is stored with it.
#[derive(Serialize, Deserialize)]
struct Setup {
    volume: String,
    cores: Option<u32>,
    expected: u32,
    settings: ConvergenceSettings,
    mesh: MeshRecord,
    #[serde(flatten)]
    kind: Kind,
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum Kind {
    WindTunnel {
        regions: Vec<String>,
        reference: ReferenceValues,
        wind: WindTunnel,
        /// The scenario has sources of air: the run also watches the mean speed in the domain.
        #[serde(default)]
        sources: bool,
    },
    InternalFlow {
        flow: InternalFlow,
        boundaries: Vec<BoundaryValues>,
        density: f64,
        reynolds: f64,
        regime: FlowRegime,
    },
}

/// Everything a background thread needs to run and evaluate the solve job.
pub struct SolveJob {
    runner: SharedRunner,
    store: JobStore,
    record: JobRecord,
    case: PathBuf,
    results: PathBuf,
    setup: Setup,
}

impl SolveJob {
    pub fn job_id(&self) -> &str {
        &self.record.id
    }

    /// A solve job of the project at `root` from its record and stored setup.
    pub fn load(runner: &SharedRunner, root: &Path, job_id: &str) -> Result<Self, FoamError> {
        let store = JobStore::new(root.join("runs"));
        let record = store.load(job_id)?;
        let text = std::fs::read_to_string(store.dir(job_id).join(SETUP_FILE))
            .map_err(|e| FoamError::new("job.not_found", e.to_string()))?;
        let setup: Setup = serde_json::from_str(&text).map_err(io)?;
        Ok(Self {
            runner: runner.clone(),
            case: store.dir(job_id).join("case"),
            results: root.join(RESULTS_DIR),
            store,
            record,
            setup,
        })
    }
}

fn io(e: impl std::fmt::Display) -> FoamError {
    FoamError::new("job.storage_failed", e.to_string())
}

fn read_plan<T: for<'de> Deserialize<'de>>(
    store: &JobStore,
    mesh: &MeshRecord,
) -> Result<T, FoamError> {
    let text = std::fs::read_to_string(
        store
            .dir(&mesh.job_id)
            .join("case/foamcraft-mesh-plan.json"),
    )
    .map_err(|e| FoamError::new("mesh.none", e.to_string()))?;
    serde_json::from_str(&text).map_err(|e| FoamError::new("mesh.none", e.to_string()))
}

/// Job name of a solve run of this scenario.
pub fn case_name(scenario: &Scenario) -> &'static str {
    match scenario {
        Scenario::WindTunnel(_) => "wind_tunnel_solve",
        Scenario::InternalFlow(_) => "internal_flow_solve",
    }
}

/// Writes the solver setup of the project's scenario next to its mesh, for the job `record`
/// (created by the caller, who also lists it among the runs of the project). The project is only
/// read, so this works on a copy in the background.
pub fn prepare(
    runner: &SharedRunner,
    project: &Project,
    record: JobRecord,
    iterations: Option<u32>,
    cores: Option<u32>,
) -> Result<SolveJob, FoamError> {
    let scenario = project
        .manifest
        .scenario
        .clone()
        .ok_or_else(|| FoamError::new("scenario.none", ""))?;
    let mesh = project
        .manifest
        .mesh
        .clone()
        .ok_or_else(|| FoamError::new("mesh.none", ""))?;
    let store = JobStore::new(project.root.join("runs"));
    let volume = store
        .load(&mesh.job_id)?
        .volume
        .ok_or_else(|| FoamError::new("mesh.none", "the mesh volume is gone"))?;
    let quality = match &scenario {
        Scenario::WindTunnel(w) => w.quality,
        Scenario::InternalFlow(f) => f.quality,
    };
    let expected = cell_budget(u64::MAX, 1, quality).iterations;
    let max_iterations = iterations.unwrap_or(expected * LIMIT_FACTOR).max(1);
    let case = store.dir(&record.id).join("case");
    let kind = match scenario {
        Scenario::WindTunnel(mut wind) => {
            // With sources of air the reference speed is the fastest jet.
            if let Some(v) = project
                .manifest
                .jets
                .iter()
                .map(|j| j.speed_m_s)
                .reduce(f64::max)
            {
                wind.speed_m_s = v;
            }
            let plan: MeshPlan = read_plan(&store, &mesh)?;
            let prepared = prepare_models(project)?;
            let medium = medium_properties(&wind.medium);
            let models: Vec<ModelSurface<'_>> = prepared
                .worlds
                .iter()
                .zip(&prepared.regions)
                .map(|(mesh, regions)| ModelSurface {
                    mesh,
                    regions: regions.clone(),
                })
                .collect();
            write_wind_tunnel_case(&case, &plan, &models, &wind, &medium, max_iterations)?;
            let jets: Vec<foamcraft_sim::jets::JetFlow> = project
                .manifest
                .jets
                .iter()
                .map(|j| foamcraft_sim::jets::to_flow(j, wind.direction_deg))
                .collect();
            foamcraft_sim::jets::apply_jets(&case, &plan, &jets)?;
            let mut regions: Vec<String> = prepared.regions.iter().flatten().cloned().collect();
            regions.sort();
            regions.dedup();
            let meshes: Vec<&foamcraft_geometry::Mesh> = prepared.worlds.iter().collect();
            let area = frontal_area(&meshes, flow_direction(wind.direction_deg));
            let reference = ReferenceValues {
                area_m2: area,
                length_m: plan.reference_length,
                speed_m_s: wind.speed_m_s,
                density_kg_m3: medium.density,
                dynamic_pressure_pa: 0.5 * medium.density * wind.speed_m_s * wind.speed_m_s,
            };
            Kind::WindTunnel {
                regions,
                reference,
                wind,
                sources: !jets.is_empty(),
            }
        }
        Scenario::InternalFlow(flow) => {
            let plan: InternalPlan = read_plan(&store, &mesh)?;
            let info = foamcraft_sim::internal::evaluate(project, &flow)?;
            let surfaces = crate::mesh_internal::fluid_surfaces(project, &flow)?;
            let (case_boundaries, boundaries) = crate::mesh_internal::boundaries(project, &flow)?;
            let medium = medium_properties(&flow.medium);
            let models: Vec<ModelSurface<'_>> = surfaces
                .meshes
                .iter()
                .zip(&surfaces.regions)
                .map(|(mesh, regions)| ModelSurface {
                    mesh,
                    regions: regions.clone(),
                })
                .collect();
            // The turbulence model follows the current scenario.
            let plan = InternalPlan {
                turbulent: info.regime.turbulent(),
                ..plan
            };
            write_internal_case(
                &case,
                &plan,
                &models,
                &case_boundaries,
                &medium,
                max_iterations,
            )?;
            Kind::InternalFlow {
                flow,
                boundaries,
                density: medium.density,
                reynolds: info.reynolds,
                regime: info.regime,
            }
        }
    };
    let results = project.root.join(RESULTS_DIR);
    let setup = Setup {
        volume,
        cores,
        expected,
        settings: ConvergenceSettings::for_iterations(expected, max_iterations),
        mesh,
        kind,
    };
    let json = serde_json::to_string_pretty(&setup).map_err(io)?;
    std::fs::write(store.dir(&record.id).join(SETUP_FILE), json).map_err(io)?;
    Ok(SolveJob {
        runner: runner.clone(),
        store,
        record,
        case,
        results,
        setup,
    })
}

/// Only the solver setup goes into the volume: the mesh there stays.
fn upload_setup(job: &SolveJob, container: &Container) -> Result<(), FoamError> {
    let r = job.runner.as_ref();
    // Old solver output and stage markers of an earlier attempt.
    let clean = "cd /work/case && rm -rf processor* postProcessing stop [1-9]* \
                 system/fvOptions log.decompose log.solve log.reconstruct log.potentialFoam \
                 .exit.decompose .exit.solve .exit.reconstruct";
    let out = r.exec(container, &["sh", "-c", clean], &mut |_| {})?;
    if out.status != 0 {
        return Err(FoamError::new("runtime.command_failed", out.stderr));
    }
    let staging = job.case.parent().unwrap_or(&job.case).join("setup");
    let target = staging.join("case");
    let _ = std::fs::remove_dir_all(&staging);
    for dir in ["system", "constant", "0"] {
        copy_dir(&job.case.join(dir), &target.join(dir), dir == "constant")?;
    }
    // Copied as folder "case" into /work, it merges with the case there.
    r.copy_into(container, &target, foamcraft_runtime::WORK_DIR)?;
    Ok(())
}

/// Copies a folder; with `top_files_only` only its files (constant/: not the mesh or surfaces).
fn copy_dir(from: &Path, to: &Path, top_files_only: bool) -> Result<(), FoamError> {
    std::fs::create_dir_all(to).map_err(io)?;
    for entry in std::fs::read_dir(from).map_err(io)?.flatten() {
        let path = entry.path();
        if path.is_dir() {
            if !top_files_only {
                copy_dir(&path, &to.join(entry.file_name()), false)?;
            }
        } else {
            std::fs::copy(&path, to.join(entry.file_name())).map_err(io)?;
        }
    }
    Ok(())
}

/// Switches the case to first-order convection and stronger relaxation.
fn make_cautious(case: &Path) -> Result<(), FoamError> {
    let schemes_path = case.join("system/fvSchemes");
    let solution_path = case.join("system/fvSolution");
    let schemes = std::fs::read_to_string(&schemes_path).map_err(io)?;
    let solution = std::fs::read_to_string(&solution_path).map_err(io)?;
    let (schemes, solution) = cautious_numerics(&schemes, &solution);
    std::fs::write(schemes_path, schemes).map_err(io)?;
    std::fs::write(solution_path, solution).map_err(io)
}

/// Follows the solver log; decides when the watched values have settled.
struct Watch {
    log: SolverLog,
    /// Flow rates and mean pressures (internal flow).
    fields: Option<FieldValueLog>,
    boundaries: Vec<BoundaryValues>,
    density: f64,
    pressure_loss: Vec<f64>,
    outflow: Vec<f64>,
    /// Mean speed in the domain per iteration (wind tunnel with sources of air).
    flow: Option<FieldValueLog>,
    speed: Vec<f64>,
    /// Number of values when the run was judged settled.
    decided: Option<usize>,
    diverged: bool,
    settings: ConvergenceSettings,
    expected: u32,
}

impl Watch {
    fn new(setup: &Setup) -> Self {
        let (fields, boundaries, density) = match &setup.kind {
            Kind::InternalFlow {
                boundaries,
                density,
                ..
            } => (
                Some(FieldValueLog::new(2 * boundaries.len())),
                boundaries.clone(),
                *density,
            ),
            Kind::WindTunnel { .. } => (None, Vec::new(), 1.0),
        };
        let flow = match &setup.kind {
            Kind::WindTunnel { sources: true, .. } => Some(FieldValueLog::new(1)),
            _ => None,
        };
        Self {
            log: SolverLog::default(),
            fields,
            boundaries,
            density,
            pressure_loss: Vec::new(),
            outflow: Vec::new(),
            flow,
            speed: Vec::new(),
            decided: None,
            diverged: false,
            settings: setup.settings.clone(),
            expected: setup.expected,
        }
    }

    /// Feeds one solver line; returns an event after a new iteration of the watched values.
    fn feed(&mut self, line: &str) -> Option<(&'static str, Value)> {
        let before = self.log.samples.len();
        self.log.feed(line);
        let remaining = remaining_seconds(&self.log.execution_times, &self.settings, self.expected);
        if let Some(fields) = self.fields.as_mut() {
            if !fields.feed(line) {
                return None;
            }
            let sample = fields.samples.last().cloned().unwrap_or_default();
            let e = evaluate_openings(&[sample], &self.boundaries, self.density, 1);
            self.pressure_loss.push(e.pressure_loss_pa);
            self.outflow.push(
                e.openings
                    .iter()
                    .filter(|o| o.share.is_some())
                    .map(|o| o.flow_m3_s)
                    .sum(),
            );
            if self.decided.is_none() && !self.diverged {
                let a = assess_series(&[&self.pressure_loss, &self.outflow], &self.settings);
                self.judge(a.verdict, self.pressure_loss.len());
            }
            return Some((
                "run.pressure_loss",
                json!({
                    "iteration": self.pressure_loss.len(),
                    "pressure_loss_pa": e.pressure_loss_pa,
                    "flow_m3_s": self.outflow.last(),
                    "remaining_s": remaining,
                    "max_iterations": self.settings.max_iterations,
                }),
            ));
        }
        if let Some(flow) = self.flow.as_mut()
            && flow.feed(line)
        {
            let value = flow.samples.last().and_then(|s| s.get(FLOW_OBJECT));
            self.speed.push(value.copied().unwrap_or(f64::NAN));
        }
        let new_total = self.log.samples[before..]
            .iter()
            .any(|s| s.object == TOTAL_FORCES);
        if !new_total {
            return None;
        }
        let history = self.log.history(TOTAL_FORCES);
        if self.decided.is_none() && !self.diverged {
            self.judge(self.assess_forces(&history).verdict, history.len());
        }
        let last = history.last().copied().unwrap_or_default();
        Some((
            "run.coefficients",
            json!({
                "iteration": history.len(),
                "cd": last.cd,
                "cl": last.cl,
                "remaining_s": remaining,
                "max_iterations": self.settings.max_iterations,
            }),
        ))
    }

    /// Verdict on the coefficients; with sources of air the mean speed must have settled too.
    fn assess_forces(&self, history: &[Coefficients]) -> Assessment {
        if self.flow.is_some() {
            let n = history.len().min(self.speed.len());
            assess_with_flow(history, &self.speed[..n], &self.settings)
        } else {
            assess(history, &self.settings)
        }
    }

    fn judge(&mut self, verdict: Verdict, n: usize) {
        match verdict {
            Verdict::Continue => {}
            Verdict::Diverged => self.diverged = true,
            _ => self.decided = Some(n),
        }
    }
}

/// How one attempt ended.
enum Attempt {
    Done {
        watch: Box<Watch>,
        timings: BTreeMap<String, f64>,
    },
    Diverged(FoamError),
    Failed(FoamError),
}

/// Starts a fresh attempt: new container, solver setup uploaded, record reset.
fn begin_attempt(job: &mut SolveJob) -> Result<(Container, u32), FoamError> {
    let r = job.runner.as_ref();
    let container = r.start(&StartOptions {
        mount: Some(Mount::Volume(job.setup.volume.clone())),
        ..Default::default()
    })?;
    job.record.container = Some(container.id.clone());
    if let Err(e) = upload_setup(job, &container) {
        let _ = r.stop(&container);
        return Err(e);
    }
    let limit = job
        .setup
        .cores
        .unwrap_or_else(|| core_limit(r, &job.record))
        .max(1);
    job.record.cores = choose_cores(job.setup.mesh.cells, limit, 0);
    job.record.cells = Some(job.setup.mesh.cells);
    job.record.start_at = Some(if job.record.cores > 1 {
        Stage::Decompose
    } else {
        Stage::Solve
    });
    job.record.state = JobState::Pending;
    job.record.offsets.clear();
    job.store.save(&job.record)?;
    Ok((container, limit))
}

fn run_attempt(
    job: &mut SolveJob,
    cancel: &AtomicBool,
    events: &EventBus,
    resume: bool,
) -> Result<Attempt, FoamError> {
    let runner = job.runner.clone();
    let r = runner.as_ref();
    let mut watch = Watch::new(&job.setup);
    let (container, limit) = if resume {
        let id = job
            .record
            .container
            .clone()
            .ok_or_else(|| FoamError::new("run.lost", "job has no container"))?;
        // What the solver printed so far, from the job log.
        let log = job.store.read_log(&job.record.id)?;
        for line in crate::mesh::stage_log(&log, Stage::Solve).lines() {
            watch.feed(line);
        }
        (Container { id }, job.record.cores.max(1))
    } else {
        begin_attempt(job)?
    };
    let stop_file = format!("{}/stop", case_dir());
    if watch.decided.is_some() {
        let _ = r.exec(&container, &["touch", &stop_file], &mut |_| {});
    }
    let options = RunOptions {
        cores: limit,
        poll_interval: Duration::from_millis(500),
        stop_after: None,
    };
    let id = job.record.id.clone();
    let mut timings = BTreeMap::new();
    let mut started: Option<Instant> = None;
    let state = execute_job(r, &job.store, &mut job.record, &options, cancel, &mut |e| {
        match &e {
            JobEvent::StageStarted(_) => started = Some(Instant::now()),
            JobEvent::StageFinished(stage) => {
                if let Some(t) = started.take() {
                    timings.insert(stage.id().to_string(), t.elapsed().as_secs_f64());
                }
            }
            JobEvent::Log {
                stage: Stage::Solve,
                line,
            } => {
                let undecided = watch.decided.is_none();
                if let Some((name, mut payload)) = watch.feed(line) {
                    payload["job_id"] = json!(id);
                    events.emit(name, payload);
                }
                if watch.diverged {
                    cancel.store(true, Ordering::SeqCst);
                } else if undecided && watch.decided.is_some() {
                    // The abort function object writes the fields and ends the solver.
                    let _ = r.exec(&container, &["touch", &stop_file], &mut |_| {});
                }
            }
            _ => {}
        }
        if let Some((name, payload)) = crate::run::event_payload(&id, &e) {
            events.emit(name, payload);
        }
    });
    let diverged = watch.diverged;
    Ok(match state {
        JobState::Succeeded => Attempt::Done {
            watch: Box::new(watch),
            timings,
        },
        JobState::Cancelled if diverged => Attempt::Diverged(FoamError::new(
            "run.diverged",
            "the watched values grew without bound",
        )),
        JobState::Cancelled => Attempt::Failed(FoamError::new("run.cancelled", "")),
        JobState::Failed { error, .. } if error.code == "run.diverged" => Attempt::Diverged(error),
        JobState::Failed { error, .. } => Attempt::Failed(error),
        other => Attempt::Failed(FoamError::new(
            "run.stage_failed",
            format!("unexpected state {other:?}"),
        )),
    })
}

/// Final verdict when the automation did not decide: residual targets met or limit reached.
fn fallback(verdict: Verdict, residuals_converged: bool) -> Verdict {
    match verdict {
        Verdict::Continue | Verdict::Diverged if residuals_converged => Verdict::Stable,
        Verdict::Continue | Verdict::Diverged => Verdict::LimitReached,
        v => v,
    }
}

fn reason_hint(v: Verdict) -> String {
    match v {
        Verdict::Oscillating => "run.oscillating",
        Verdict::LimitReached => "run.limit_reached",
        _ => "run.stable",
    }
    .to_string()
}

/// Mean coefficients over the last `span` samples of a force object.
fn mean_of_last(log: &SolverLog, object: &str, span: usize) -> Coefficients {
    let h = log.history(object);
    let tail = &h[h.len().saturating_sub(span)..];
    let n = tail.len().max(1) as f64;
    let mut m = Coefficients::default();
    for c in tail {
        m.cd += c.cd / n;
        m.cl += c.cl / n;
        m.cs += c.cs / n;
        m.cm_pitch += c.cm_pitch / n;
        m.cm_roll += c.cm_roll / n;
        m.cm_yaw += c.cm_yaw / n;
    }
    m
}

fn evaluate_wind(
    job: &SolveJob,
    watch: &Watch,
    timings: BTreeMap<String, f64>,
    restarted: bool,
) -> WindTunnelResult {
    let Kind::WindTunnel {
        regions,
        reference: rv,
        wind,
        ..
    } = &job.setup.kind
    else {
        unreachable!("wind tunnel setup")
    };
    let log = &watch.log;
    let history = log.history(TOTAL_FORCES);
    let n = watch.decided.unwrap_or(history.len()).min(history.len());
    let a = watch.assess_forces(&history[..n]);
    let assessment = Assessment {
        verdict: if watch.decided.is_some() {
            a.verdict
        } else {
            fallback(a.verdict, log.residuals_converged)
        },
        ..a
    };
    let span = if assessment.verdict == Verdict::Oscillating {
        3 * job.setup.settings.window
    } else {
        job.setup.settings.window
    };
    let force = |c: &Coefficients| forces_from(c, rv.density_kg_m3, rv.speed_m_s, rv.area_m2);
    let qa = rv.dynamic_pressure_pa * rv.area_m2;
    let split = |object: &str| {
        let (p, v) = log.mean_split(object, n, span);
        Some(foamcraft_sim::solve::DragSplit {
            pressure_n: p * qa,
            viscous_n: v * qa,
        })
    };
    let parts = if regions.len() > 1 {
        regions
            .iter()
            .map(|r| {
                let c = mean_of_last(log, &format!("{PART_PREFIX}{r}"), span);
                PartResult {
                    name: r.clone(),
                    coefficients: c,
                    forces: force(&c),
                    drag_split: split(&format!("{PART_PREFIX}{r}")),
                }
            })
            .collect()
    } else {
        vec![]
    };
    let y_plus: Vec<_> = log
        .y_plus
        .iter()
        .filter(|y| y.patch.starts_with("model"))
        .cloned()
        .collect();
    let mut hints = vec![reason_hint(assessment.verdict)];
    if restarted {
        hints.push("run.restarted".into());
    }
    // Sources of air: the coefficients refer to the fastest jet, not to an oncoming flow.
    if job.case.join("system").join("fvOptions").is_file() {
        hints.push("run.jet_reference".into());
    }
    if !y_plus.is_empty() {
        let average = y_plus.iter().map(|y| y.average).sum::<f64>() / y_plus.len() as f64;
        if let Some(h) = y_plus_hint(average) {
            hints.push(h.code().into());
        }
    }
    WindTunnelResult {
        job_id: job.record.id.clone(),
        reason: assessment.verdict,
        iterations: history.len() as u32,
        decided_at_iteration: n as u32,
        max_iterations: job.setup.settings.max_iterations,
        coefficients: assessment.mean,
        forces: force(&assessment.mean),
        fluctuation_percent: assessment.fluctuation * 100.0,
        drag_split: split(TOTAL_FORCES),
        parts,
        reference: rv.clone(),
        y_plus,
        hints,
        restarted,
        cells: job.setup.mesh.cells,
        cores: job.record.cores,
        timings_s: timings,
        scenario: wind.clone(),
        mesh: Some(job.setup.mesh.clone()),
        history: History {
            iterations: (1..=history.len()).map(|i| i as f64).collect(),
            cd: history.iter().map(|c| c.cd).collect(),
            cl: history.iter().map(|c| c.cl).collect(),
            residual_iterations: log.iterations.clone(),
            residuals: log.residuals.clone(),
        },
    }
}

fn evaluate_internal(
    job: &SolveJob,
    watch: &Watch,
    timings: BTreeMap<String, f64>,
    restarted: bool,
) -> InternalFlowResult {
    let Kind::InternalFlow {
        flow,
        boundaries,
        density,
        reynolds,
        regime,
    } = &job.setup.kind
    else {
        unreachable!("internal flow setup")
    };
    let samples = watch
        .fields
        .as_ref()
        .map(|f| f.samples.clone())
        .unwrap_or_default();
    let total = watch.pressure_loss.len();
    let n = watch.decided.unwrap_or(total).min(total);
    let a = assess_series(
        &[&watch.pressure_loss[..n], &watch.outflow[..n]],
        &job.setup.settings,
    );
    let verdict = if watch.decided.is_some() {
        a.verdict
    } else {
        fallback(a.verdict, watch.log.residuals_converged)
    };
    let evaluation = evaluate_openings(
        &samples[..n.min(samples.len())],
        boundaries,
        *density,
        a.span,
    );
    let mut hints = vec![reason_hint(verdict)];
    if restarted {
        hints.push("run.restarted".into());
    }
    if evaluation.mass_balance_error > MASS_BALANCE_LIMIT {
        hints.push("run.mass_balance".into());
    }
    if *regime == FlowRegime::Transitional {
        hints.push("scenario.transitional".into());
    }
    InternalFlowResult {
        kind: "internal_flow".into(),
        job_id: job.record.id.clone(),
        reason: verdict,
        iterations: total as u32,
        decided_at_iteration: n as u32,
        max_iterations: job.setup.settings.max_iterations,
        evaluation,
        fluctuation_percent: a.fluctuation * 100.0,
        reynolds: *reynolds,
        regime: *regime,
        hints,
        restarted,
        cells: job.setup.mesh.cells,
        cores: job.record.cores,
        timings_s: timings,
        scenario: flow.clone(),
        mesh: Some(job.setup.mesh.clone()),
        history: InternalHistory {
            iterations: (1..=total).map(|i| i as f64).collect(),
            pressure_loss_pa: watch.pressure_loss.clone(),
            flow_m3_s: watch.outflow.clone(),
            residual_iterations: watch.log.iterations.clone(),
            residuals: watch.log.residuals.clone(),
        },
    }
}

/// Grid levels and reference values of the job; runs the result extraction.
fn extract_for(job: &SolveJob, result: &Value) -> Result<crate::results::CacheMeta, FoamError> {
    use foamcraft_sim::results::{GridSpec, POINTS_PER_LEVEL, level_dims};
    let level = |min: [f64; 3], max: [f64; 3]| GridSpec {
        min,
        max,
        dims: level_dims([0, 1, 2].map(|k| max[k] - min[k]), POINTS_PER_LEVEL),
    };
    let root = job.results.parent().unwrap_or(&job.results).to_path_buf();
    let (patches, turbulent, levels, yaw, density, speed) = match &job.setup.kind {
        Kind::WindTunnel { reference, .. } => {
            let plan: MeshPlan = read_plan(&job.store, &job.setup.mesh)?;
            (
                "\"model.*\"".to_string(),
                true,
                vec![
                    level(plan.tunnel_min, plan.tunnel_max),
                    level(plan.wake_box[0], plan.wake_box[1]),
                    level(plan.near_box[0], plan.near_box[1]),
                ],
                plan.yaw_deg,
                reference.density_kg_m3,
                reference.speed_m_s,
            )
        }
        Kind::InternalFlow {
            density, regime, ..
        } => {
            let plan: InternalPlan = read_plan(&job.store, &job.setup.mesh)?;
            let speed = result["openings"][0]["velocity_m_s"]
                .as_f64()
                .unwrap_or(1.0);
            (
                "walls".to_string(),
                regime.turbulent(),
                vec![level(plan.box_min, plan.box_max)],
                0.0,
                *density,
                speed,
            )
        }
    };
    crate::results::extract(
        &job.runner,
        &crate::results::ExtractInput {
            job_id: job.record.id.clone(),
            volume: job.setup.volume.clone(),
            project_root: root,
            patches,
            turbulent,
            levels,
            yaw_deg: yaw,
            density,
            reference_speed: speed,
        },
    )
}

pub fn result_path(results: &Path, job_id: &str) -> PathBuf {
    results.join(format!("{job_id}.json"))
}

/// Runs the job to the end (at most one restart) and stores the result.
/// With `resume`, follows a job that is already running (after a restart of the app).
pub fn run(
    mut job: SolveJob,
    cancel: &AtomicBool,
    events: &EventBus,
    resume: bool,
) -> Result<Value, FoamError> {
    // A job that already runs with cautious numerics was restarted before.
    let mut restarted = resume
        && std::fs::read_to_string(job.case.join("system/fvSchemes"))
            .is_ok_and(|s| s.contains("div(phi,U)      bounded Gauss upwind;"));
    let mut resume = resume;
    let outcome = loop {
        let attempt = run_attempt(&mut job, cancel, events, resume)?;
        resume = false;
        match attempt {
            Attempt::Done { watch, timings } => {
                let value = match job.setup.kind {
                    Kind::WindTunnel { .. } => {
                        let r = evaluate_wind(&job, &watch, timings, restarted);
                        json!({"job_id": r.job_id, "reason": r.reason, "cd": r.coefficients.cd, "cl": r.coefficients.cl})
                            .as_object()
                            .cloned()
                            .map(|summary| (serde_json::to_value(&r), Value::Object(summary)))
                    }
                    Kind::InternalFlow { .. } => {
                        let r = evaluate_internal(&job, &watch, timings, restarted);
                        json!({"job_id": r.job_id, "reason": r.reason, "pressure_loss_pa": r.evaluation.pressure_loss_pa})
                            .as_object()
                            .cloned()
                            .map(|summary| (serde_json::to_value(&r), Value::Object(summary)))
                    }
                };
                break value
                    .ok_or_else(|| io("summary"))
                    .and_then(|(full, summary)| Ok((full.map_err(io)?, summary)));
            }
            Attempt::Diverged(error) if !restarted => {
                restarted = true;
                cancel.store(false, Ordering::SeqCst);
                events.emit(
                    "run.restarted",
                    json!({"job_id": job.record.id, "reason": error.to_string()}),
                );
                make_cautious(&job.case)?;
            }
            Attempt::Diverged(error) | Attempt::Failed(error) => break Err(error),
        }
    };
    let (result, summary) = outcome.inspect_err(|_| {
        job.record.state = match &job.record.state {
            s if s.is_finished() => s.clone(),
            _ => JobState::Cancelled,
        };
        let _ = job.store.save(&job.record);
    })?;
    std::fs::create_dir_all(&job.results).map_err(io)?;
    let text = serde_json::to_string_pretty(&result).map_err(io)?;
    std::fs::write(result_path(&job.results, &job.record.id), text).map_err(io)?;
    // Fields for the 3D view; the numbers of the run stay valid if this fails.
    match extract_for(&job, &result) {
        Ok(_) => events.emit("results.ready", json!({"job_id": job.record.id})),
        Err(e) => events.emit(
            "results.failed",
            json!({"job_id": job.record.id, "code": e.code, "detail": e.detail}),
        ),
    }
    events.emit("run.result", summary);
    Ok(result)
}

/// The stored result of a job, or of the latest job with a result.
pub fn load_result(results: &Path, job_id: Option<&str>) -> Result<Value, FoamError> {
    let path = match job_id {
        Some(id) => result_path(results, id),
        None => std::fs::read_dir(results)
            .ok()
            .into_iter()
            .flatten()
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|x| x == "json"))
            .max_by_key(|p| p.metadata().and_then(|m| m.modified()).ok())
            .ok_or_else(|| FoamError::new("run.no_result", ""))?,
    };
    let text = std::fs::read_to_string(&path)
        .map_err(|e| FoamError::new("run.no_result", e.to_string()))?;
    serde_json::from_str(&text).map_err(io)
}
