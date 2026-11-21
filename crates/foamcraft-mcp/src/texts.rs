// SPDX-License-Identifier: GPL-3.0-or-later
//! Server instructions and the guide resource for agents.

pub const GUIDE_URI: &str = "foamcraft://guide";

pub const INSTRUCTIONS: &str = "FoamCraft sets up, runs and evaluates flow simulations (OpenFOAM in \
a local container). Typical order: project_new or project_open → geometry_import (give the unit) → \
geometry_info (closed? normals?) → scenario_set_wind_tunnel or scenario_set_internal_flow → \
run_start (returns a job id at once) → run_wait (repeat until finished) → results_summary → \
view_render. All values are SI (m, m/s, Pa, N, W). Errors come back as {code, message, fix}. Paths \
must be absolute and inside the folders given to the server with --root. Read the resource \
foamcraft://guide before the first simulation.";

pub const GUIDE: &str = r#"# Setting up and checking a FoamCraft simulation

## 1. Project and geometry
- `project_new` with an absolute folder inside the allowed roots; `project_open` for an existing one.
- `geometry_import` with `unit` (`mm`, `cm`, `m`, `in`). Check the answer: a model for external
  flow must be closed and its normals outward (`geometry_info`; `command_run` with
  `geometry.fix_normals` repairs the normals).
- Size matters: a helmet is about 0.3 m, a car about 4 m. A size off by 1000 means a wrong unit.

## 2. Scenario
- External flow: `scenario_set_wind_tunnel` with speed (m/s), direction (degrees, 0 = flow along
  +x), ground (`none`, `fixed`, `moving`), medium (`{"kind": "air", "temperature_c": 20}` or water)
  and quality (`draft` for a first look, `standard`, `fine`). The answer has the frontal area and
  the Reynolds number.
- Internal flow: close the openings (`geometry_cap_opening`), mark them
  (`scenario_set_boundary_role`), then `scenario_set_internal_flow` with the inlet condition and the
  outlet pressure.

## 3. Running
- `run_start` meshes when needed and starts the solver; it answers at once with `job_id`.
- `run_wait` with `job_id` and `timeout_s` (up to 3600). When `finished` is false, call it again.
  `run_status` shows the stage, `run_log` the end of the log, `run_cancel` stops a job.

## 4. Results
- `results_summary`: forces (N), coefficients, drag area, power, pressure loss, flow per opening,
  each with a short explanation, and the reliability light with its reasons.
- `view_render` answers with a PNG of the 3D view. `view_set_surface_field` (for example `p`)
  colours the model first; `view_add_slice` adds a plane through the flow.
- `view_apply_preset` sets up a ready-made view in one step (smoke lines, mid slice, pressure
  on the model, vortices, wake; for internal flow the path through the part); then `view_render`.
- `view_set_surface_flow` draws lines like an oil film and wool tufts on the model; the surface
  field `attached` (wind tunnel) shows attached flow (+1) and reverse flow (-1), and
  `results_summary` gives `reverse_flow_area` in %.
- Pictures to show: `view_apply_preset` with `light_trails` (glowing trails) or `ribbons`
  (twisting ribbons coloured by swirl); `export_image` with `exposure_s` takes a long exposure.
- Heavy particles: `particles_materials`, then `view_set_particles` with `inertia`
  (`{"material": "rain"}`); `results_impact` computes where they hit and which share hits
  (`results_impact_status` until done; then `view_set_surface_field` with `impact`).
- `ride_suggest` lists rides along one particle; `ride_start` (a suggestion id or a world point
  `from`) answers with the readings along the path: speed, Cp, pressure, wall distance and the
  events (stagnation, suction, reverse flow). `view_render` with `ride_t` shows the rider's view,
  `export_video` with camera preset `ride` the whole ride; `ride_stop` ends it.
- `results_compare` compares two runs; `runs_list` lists them.

## 5. Checking plausibility
- The light must be green or yellow; read the reasons when it is not green.
- Wind tunnel: drag coefficients of everyday shapes lie between about 0.2 (streamlined) and 1.3
  (flat plate across the flow). A sphere is near 0.4 to 0.5 at moderate speed, a cube near 1.05.
- Drag grows with the square of the speed; power with its cube. Two runs at different speeds must
  show this within a few per cent.
- Average y+ between 30 and 300 suits the wall model; far outside, friction forces are unreliable.
- Internal flow: the mass balance error should be below 1 %; the pressure loss must be positive in
  the flow direction.
- When the run stopped at the iteration limit, run again with more iterations
  (`run_start` with `iterations`) or a finer quality.

## 6. Generic access
`commands_list`, `command_describe` and `command_run` reach every command of the app, with the
same path rules.
"#;
