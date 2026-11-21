// SPDX-License-Identifier: GPL-3.0-or-later
//! Steady wind tunnel solve: inlet turbulence, force and y+ output of the solver, convergence
//! automation, cautious numerics for a restart and the result object.

use std::collections::BTreeMap;

use foamcraft_core::log::{LogEvent, LogParser};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Turbulence intensity at the inlet (a low-turbulence wind tunnel).
pub const INLET_INTENSITY: f64 = 0.01;
/// Turbulent length scale at the inlet as a share of the reference length.
pub const LENGTH_SCALE_SHARE: f64 = 0.1;
/// k-omega SST model constant beta* (C_mu).
const C_MU: f64 = 0.09;

/// Coefficients beyond this magnitude mean the run has diverged.
pub const DIVERGED_COEFFICIENT: f64 = 100.0;
/// Wall functions work for an average y+ in this range.
pub const Y_PLUS_RANGE: (f64, f64) = (30.0, 300.0);

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct InletTurbulence {
    /// Turbulent kinetic energy in m²/s².
    pub k: f64,
    /// Specific dissipation rate in 1/s.
    pub omega: f64,
}

/// k = 3/2 (I U)², omega = sqrt(k) / (C_mu^0.25 l).
pub fn inlet_turbulence(speed: f64, intensity: f64, length_scale: f64) -> InletTurbulence {
    let k = 1.5 * (intensity * speed).powi(2);
    let omega = k.sqrt() / (C_MU.powf(0.25) * length_scale.max(1e-9));
    InletTurbulence { k, omega }
}

/// Force and moment coefficients (flow frame: drag along x, lift along z, side along y).
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Coefficients {
    pub cd: f64,
    pub cl: f64,
    pub cs: f64,
    pub cm_pitch: f64,
    pub cm_roll: f64,
    pub cm_yaw: f64,
}

impl Coefficients {
    fn values(&self) -> [f64; 6] {
        [
            self.cd,
            self.cl,
            self.cs,
            self.cm_pitch,
            self.cm_roll,
            self.cm_yaw,
        ]
    }

    fn mean(samples: &[Coefficients]) -> Coefficients {
        let n = samples.len().max(1) as f64;
        let mut sum = [0.0; 6];
        for s in samples {
            for (a, v) in sum.iter_mut().zip(s.values()) {
                *a += v;
            }
        }
        let [cd, cl, cs, cm_pitch, cm_roll, cm_yaw] = sum.map(|v| v / n);
        Coefficients {
            cd,
            cl,
            cs,
            cm_pitch,
            cm_roll,
            cm_yaw,
        }
    }
}

/// Forces in newtons: F = C q A.
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Forces {
    pub drag_n: f64,
    pub lift_n: f64,
    pub side_n: f64,
}

pub fn forces_from(c: &Coefficients, density: f64, speed: f64, area: f64) -> Forces {
    let qa = 0.5 * density * speed * speed * area;
    Forces {
        drag_n: c.cd * qa,
        lift_n: c.cl * qa,
        side_n: c.cs * qa,
    }
}

/// Coefficients of one force object at one iteration.
#[derive(Debug, Clone, PartialEq)]
pub struct CoeffSample {
    pub object: String,
    pub iteration: f64,
    pub coefficients: Coefficients,
    /// Share of the pressure forces.
    pub pressure: Coefficients,
    /// Share of the friction (viscous) forces.
    pub viscous: Coefficients,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct YPlus {
    pub patch: String,
    pub min: f64,
    pub max: f64,
    pub average: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum YPlusHint {
    /// Cells at the wall are finer than wall functions expect.
    Low,
    /// Cells at the wall are too coarse.
    High,
}

impl YPlusHint {
    pub fn code(self) -> &'static str {
        match self {
            YPlusHint::Low => "run.y_plus_low",
            YPlusHint::High => "run.y_plus_high",
        }
    }
}

pub fn y_plus_hint(average: f64) -> Option<YPlusHint> {
    if average < Y_PLUS_RANGE.0 {
        Some(YPlusHint::Low)
    } else if average > Y_PLUS_RANGE.1 {
        Some(YPlusHint::High)
    } else {
        None
    }
}

/// Reads the solver log: residuals, force coefficients (`forceCoeffs` with `log true`) and y+.
#[derive(Default)]
pub struct SolverLog {
    parser: LogParser,
    iteration: f64,
    force: Option<(String, [Coefficients; 3], bool)>,
    in_y_plus: bool,
    pub samples: Vec<CoeffSample>,
    pub y_plus: Vec<YPlus>,
    /// Iterations with residuals, and the initial residual of every field per iteration.
    pub iterations: Vec<f64>,
    pub residuals: BTreeMap<String, Vec<f64>>,
    /// Solver run time in seconds (last `ExecutionTime`).
    pub execution_s: f64,
    /// Cumulative execution time at the end of every iteration.
    pub execution_times: Vec<f64>,
    /// The solver met its residual targets.
    pub residuals_converged: bool,
}

fn first_number(s: &str) -> Option<f64> {
    s.split_whitespace().next()?.parse().ok()
}

fn number_after(s: &str, key: &str) -> Option<f64> {
    let rest = &s[s.find(key)? + key.len()..];
    rest.split(|c: char| c == ',' || c.is_whitespace())
        .find(|t| !t.is_empty())?
        .parse()
        .ok()
}

impl SolverLog {
    fn flush_force(&mut self) {
        if let Some((object, [coefficients, pressure, viscous], any)) = self.force.take()
            && any
        {
            self.samples.push(CoeffSample {
                object,
                iteration: self.iteration,
                coefficients,
                pressure,
                viscous,
            });
        }
    }

    pub fn feed(&mut self, raw: &str) {
        let line = raw.trim_end();
        let trimmed = line.trim_start();
        if self.force.is_some() {
            if trimmed.is_empty() || !line.starts_with(char::is_whitespace) {
                self.flush_force();
            } else if let Some((key, rest)) = trimmed.split_once(':')
                && let Some(v) = first_number(rest)
                && let Some((_, cs, any)) = self.force.as_mut()
            {
                // Columns: total, pressure, viscous (, internal).
                let numbers: Vec<f64> = rest
                    .split_whitespace()
                    .filter_map(|t| t.parse().ok())
                    .collect();
                for (n, c) in cs.iter_mut().enumerate() {
                    let value = if n == 0 {
                        v
                    } else {
                        numbers.get(n).copied().unwrap_or(0.0)
                    };
                    let slot = match key {
                        "Cd" => Some(&mut c.cd),
                        "Cl" => Some(&mut c.cl),
                        "Cs" => Some(&mut c.cs),
                        "CmPitch" => Some(&mut c.cm_pitch),
                        "CmRoll" => Some(&mut c.cm_roll),
                        "CmYaw" => Some(&mut c.cm_yaw),
                        _ => None,
                    };
                    if let Some(slot) = slot {
                        *slot = value;
                        *any = true;
                    }
                }
                return;
            } else {
                return;
            }
        }
        if self.in_y_plus {
            if let Some(rest) = trimmed.strip_prefix("patch ")
                && let Some((patch, values)) = rest.split_once(" y+ :")
            {
                self.y_plus.retain(|y| y.patch != patch.trim());
                self.y_plus.push(YPlus {
                    patch: patch.trim().to_string(),
                    min: number_after(values, "min = ").unwrap_or(f64::NAN),
                    max: number_after(values, "max = ").unwrap_or(f64::NAN),
                    average: number_after(values, "average = ").unwrap_or(f64::NAN),
                });
                return;
            }
            if trimmed.is_empty() || !line.starts_with(char::is_whitespace) {
                self.in_y_plus = false;
            } else {
                return;
            }
        }
        if let Some(rest) = trimmed.strip_prefix("forceCoeffs ")
            && let Some(name) = rest.strip_suffix(" write:")
        {
            self.force = Some((name.trim().to_string(), [Coefficients::default(); 3], false));
            return;
        }
        if trimmed.starts_with("yPlus ") && trimmed.ends_with(" write:") {
            self.in_y_plus = true;
            return;
        }
        for event in self.parser.feed(line) {
            match event {
                LogEvent::Iteration { time } => self.iteration = time,
                LogEvent::Residual { field, initial } => {
                    if self.iterations.last() != Some(&self.iteration) {
                        self.iterations.push(self.iteration);
                    }
                    // The first solve of a field in an iteration counts.
                    let n = self.iterations.len();
                    let values = self.residuals.entry(field).or_default();
                    if values.len() < n {
                        values.resize(n - 1, f64::NAN);
                        values.push(initial);
                    }
                }
                LogEvent::ExecutionTime { seconds } => {
                    self.execution_s = seconds;
                    self.execution_times.push(seconds);
                }
                LogEvent::Converged => self.residuals_converged = true,
                _ => {}
            }
        }
    }

    /// Mean drag split (pressure, viscous coefficient) of a force object over the last `span`
    /// samples up to sample `n`.
    pub fn mean_split(&self, object: &str, n: usize, span: usize) -> (f64, f64) {
        let all: Vec<&CoeffSample> = self.samples.iter().filter(|s| s.object == object).collect();
        let end = n.min(all.len());
        let tail = &all[end.saturating_sub(span)..end];
        let k = tail.len().max(1) as f64;
        (
            tail.iter().map(|s| s.pressure.cd).sum::<f64>() / k,
            tail.iter().map(|s| s.viscous.cd).sum::<f64>() / k,
        )
    }

    /// Coefficients of one force object, one entry per iteration.
    pub fn history(&self, object: &str) -> Vec<Coefficients> {
        self.samples
            .iter()
            .filter(|s| s.object == object)
            .map(|s| s.coefficients)
            .collect()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Verdict {
    Continue,
    /// Coefficients settled within the tolerance over the window.
    Stable,
    /// Coefficients swing around a steady mean; the result is the mean.
    Oscillating,
    /// The iteration limit was reached before the coefficients settled.
    LimitReached,
    Diverged,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConvergenceSettings {
    /// Iterations in the moving window.
    pub window: usize,
    /// Allowed band of the coefficients in the window, relative to the drag coefficient.
    pub tolerance: f64,
    pub max_iterations: u32,
}

impl ConvergenceSettings {
    /// Window a fifth of the expected iterations (30 to 200), tolerance 1 %.
    pub fn for_iterations(expected: u32, max_iterations: u32) -> Self {
        Self {
            window: (expected as usize / 5).clamp(30, 200),
            tolerance: 0.01,
            max_iterations,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Assessment {
    pub verdict: Verdict,
    /// Mean over the deciding window(s).
    pub mean: Coefficients,
    /// Band (max - min) of the drag coefficient in the last window, relative to its mean.
    pub fluctuation: f64,
}

fn band(values: impl Iterator<Item = f64>) -> f64 {
    let (lo, hi) = values.fold((f64::MAX, f64::MIN), |(lo, hi), v| (lo.min(v), hi.max(v)));
    hi - lo
}

/// Judges the coefficient history (one entry per iteration).
pub fn assess(history: &[Coefficients], s: &ConvergenceSettings) -> Assessment {
    let n = history.len();
    let w = s.window.max(1);
    let last = &history[n.saturating_sub(w)..];
    let mean = Coefficients::mean(last);
    let scale = mean.cd.abs().max(0.1);
    let band_cd = band(last.iter().map(|c| c.cd));
    let band_cl = band(last.iter().map(|c| c.cl));
    let fluctuation = band_cd / scale;
    let result = |verdict, mean| Assessment {
        verdict,
        mean,
        fluctuation,
    };
    let diverged = history.last().is_some_and(|c| {
        c.values()
            .iter()
            .any(|v| !v.is_finite() || v.abs() > DIVERGED_COEFFICIENT)
    });
    if diverged {
        return result(Verdict::Diverged, mean);
    }
    if n >= 2 * w {
        let tol = s.tolerance * scale;
        if band_cd <= tol && band_cl <= tol {
            return result(Verdict::Stable, mean);
        }
        if n >= 3 * w {
            // The means of the last three windows agree although the values swing: a periodic
            // swing moves a window mean by at most a small share of its band, a drift by all of it.
            let means: Vec<Coefficients> = (0..3)
                .map(|k| Coefficients::mean(&history[n - (k + 1) * w..n - k * w]))
                .collect();
            let spread_cd = band(means.iter().map(|m| m.cd));
            let spread_cl = band(means.iter().map(|m| m.cl));
            if spread_cd <= (2.0 * tol).max(0.25 * band_cd)
                && spread_cl <= (2.0 * tol).max(0.25 * band_cl)
            {
                return result(
                    Verdict::Oscillating,
                    Coefficients::mean(&history[n - 3 * w..]),
                );
            }
        }
    }
    if n >= s.max_iterations as usize {
        return result(Verdict::LimitReached, mean);
    }
    result(Verdict::Continue, mean)
}

/// Name of the force object over all model patches.
pub const TOTAL_FORCES: &str = "forces";
/// Prefix of the force object of one part (model patch region).
pub const PART_PREFIX: &str = "part_";

/// `functions` entries of the controlDict: force coefficients in total and per region (when
/// there is more than one), y+ at write time and a stop file for the convergence automation.
pub fn solver_functions(
    regions: &[String],
    area: f64,
    length: f64,
    speed: f64,
    density: f64,
) -> String {
    let force = |name: &str, patches: &str| {
        format!(
            "    {name}
    {{
        type forceCoeffs;
        libs (forces);
        writeControl timeStep;
        writeInterval 1;
        log true;
        patches ({patches});
        rho rhoInf;
        rhoInf {density:.6e};
        CofR (0 0 0);
        dragDir (1 0 0);
        liftDir (0 0 1);
        pitchAxis (0 1 0);
        magUInf {speed:.6e};
        lRef {length:.6e};
        Aref {area:.6e};
    }}
"
        )
    };
    let mut out = force(TOTAL_FORCES, "model");
    if regions.len() > 1 {
        for r in regions {
            out.push_str(&force(&format!("{PART_PREFIX}{r}"), &format!("model_{r}")));
        }
    }
    out.push_str(
        "    yPlus
    {
        type yPlus;
        libs (fieldFunctionObjects);
        writeControl writeTime;
        log true;
    }
",
    );
    // FoamCraft creates the file when the coefficients are stable; the solver then writes and ends.
    out.push_str(
        "    stop
    {
        type abort;
        libs (utilityFunctionObjects);
        file \"<case>/stop\";
        action writeNow;
    }
",
    );
    out
}

/// First-order convection and stronger under-relaxation for a restart after divergence.
pub fn cautious_numerics(fv_schemes: &str, fv_solution: &str) -> (String, String) {
    let schemes = fv_schemes.replace(
        "bounded Gauss linearUpwindV grad(U)",
        "bounded Gauss upwind",
    );
    let solution = fv_solution
        .replace("consistent      true;", "consistent      false;")
        .replace(
            "equations { U 0.9; \".*\" 0.9; }",
            "equations { U 0.5; \".*\" 0.5; }",
        )
        .replace("fields { p 1; }", "fields { p 0.3; }");
    (schemes, solution)
}

/// Drag split into the share of the pressure and of the friction, N.
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct DragSplit {
    pub pressure_n: f64,
    pub viscous_n: f64,
}

/// Coefficients and forces of one part (model patch region), referred to the total frontal area.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct PartResult {
    pub name: String,
    pub coefficients: Coefficients,
    pub forces: Forces,
    /// Since Task_17.
    #[serde(default)]
    pub drag_split: Option<DragSplit>,
}

/// Reference values of the coefficients.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ReferenceValues {
    pub area_m2: f64,
    pub length_m: f64,
    pub speed_m_s: f64,
    pub density_kg_m3: f64,
    pub dynamic_pressure_pa: f64,
}

/// Coefficients and initial residuals per iteration.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct History {
    pub iterations: Vec<f64>,
    pub cd: Vec<f64>,
    pub cl: Vec<f64>,
    pub residual_iterations: Vec<f64>,
    pub residuals: BTreeMap<String, Vec<f64>>,
}

/// Result of a wind tunnel run, stored in `<project>/results/<job id>.json`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct WindTunnelResult {
    pub job_id: String,
    /// Why the run ended: `stable`, `oscillating` or `limit_reached`.
    pub reason: Verdict,
    /// Iterations the solver ran (a few more than the decision while it writes and ends).
    pub iterations: u32,
    /// Iteration at which the convergence automation decided.
    pub decided_at_iteration: u32,
    pub max_iterations: u32,
    /// Mean over the deciding window (flow frame: drag along the flow, lift upwards).
    pub coefficients: Coefficients,
    pub forces: Forces,
    /// Band of the drag coefficient in the last window, in percent of its mean.
    pub fluctuation_percent: f64,
    /// Since Task_17.
    #[serde(default)]
    pub drag_split: Option<DragSplit>,
    pub parts: Vec<PartResult>,
    pub reference: ReferenceValues,
    /// y+ of the model patches at the end of the run.
    pub y_plus: Vec<YPlus>,
    /// Message codes that explain the result (`run.oscillating`, `run.y_plus_low`, ...).
    pub hints: Vec<String>,
    pub restarted: bool,
    pub cells: u64,
    pub cores: u32,
    /// Wall-clock seconds per stage.
    pub timings_s: BTreeMap<String, f64>,
    pub scenario: foamcraft_core::project::WindTunnel,
    pub mesh: Option<foamcraft_core::project::MeshRecord>,
    pub history: History,
}

/// Seconds until the run probably ends, from the solver's cumulative execution time per
/// iteration: up to the expected iterations, after that one more window (at most the limit).
pub fn remaining_seconds(
    execution_times: &[f64],
    settings: &ConvergenceSettings,
    expected: u32,
) -> Option<f64> {
    let n = execution_times.len();
    if n < 3 {
        return None;
    }
    let mut steps: Vec<f64> = execution_times[n.saturating_sub(21)..]
        .windows(2)
        .map(|w| w[1] - w[0])
        .collect();
    steps.sort_by(f64::total_cmp);
    let rate = steps[steps.len() / 2];
    let max = settings.max_iterations as usize;
    let target = if n < expected as usize {
        expected as usize
    } else {
        (n + settings.window).min(max)
    };
    Some(target.saturating_sub(n) as f64 * rate)
}

/// Assessment of several series at once (for example pressure loss and flow).
#[derive(Debug, Clone, PartialEq)]
pub struct SeriesAssessment {
    pub verdict: Verdict,
    /// Mean of every series over the deciding window(s).
    pub means: Vec<f64>,
    /// Largest band of a series in the last window relative to its mean.
    pub fluctuation: f64,
    /// Values that make up the means (window or three windows).
    pub span: usize,
}

fn mean_of(v: &[f64]) -> f64 {
    v.iter().sum::<f64>() / v.len().max(1) as f64
}

/// Like [`assess`], for several series of equal length, each judged relative to its own mean.
pub fn assess_series(series: &[&[f64]], s: &ConvergenceSettings) -> SeriesAssessment {
    let n = series.iter().map(|x| x.len()).min().unwrap_or(0);
    let w = s.window.max(1);
    let last = |x: &[f64]| x[n.saturating_sub(w)..n].to_vec();
    let means: Vec<f64> = series.iter().map(|x| mean_of(&last(x))).collect();
    let scale = |m: f64| m.abs().max(1e-30);
    let bands: Vec<f64> = series.iter().map(|x| band(last(x).into_iter())).collect();
    let fluctuation = bands
        .iter()
        .zip(&means)
        .map(|(b, m)| b / scale(*m))
        .fold(0.0, f64::max);
    let out = |verdict, means: Vec<f64>, span| SeriesAssessment {
        verdict,
        means,
        fluctuation,
        span,
    };
    if series
        .iter()
        .any(|x| x[..n].last().is_some_and(|v| !v.is_finite()))
    {
        return out(Verdict::Diverged, means, w);
    }
    if n >= 2 * w {
        let settled = bands
            .iter()
            .zip(&means)
            .all(|(b, m)| *b <= s.tolerance * scale(*m));
        if settled {
            return out(Verdict::Stable, means, w);
        }
        if n >= 3 * w {
            let steady = series.iter().zip(&bands).all(|(x, b)| {
                let ms: Vec<f64> = (0..3)
                    .map(|k| mean_of(&x[n - (k + 1) * w..n - k * w]))
                    .collect();
                let spread = band(ms.iter().copied());
                spread <= (2.0 * s.tolerance * scale(mean_of(&ms))).max(0.25 * b)
            });
            if steady {
                let means = series.iter().map(|x| mean_of(&x[n - 3 * w..n])).collect();
                return out(Verdict::Oscillating, means, 3 * w);
            }
        }
    }
    if n >= s.max_iterations as usize {
        return out(Verdict::LimitReached, means, w);
    }
    out(Verdict::Continue, means, w)
}
