# Tech stack and architecture

> When to read: before structural changes, new components or new dependencies.

## Tech stack

- **Core:** Rust (stable, edition 2024) in one Cargo workspace. No Python anywhere.
- **Desktop shell:** Tauri 2. User interface in TypeScript, React and Vite, located in `app/`.
- **3D rendering:** own renderer in Rust on `wgpu`. It draws natively beneath the transparent web
  view and can render offscreen. Reason: one renderer for the viewport, MCP screenshots, video
  export and the later GPU preview solver, independent of WebGPU support in system web views.
- **Solver:** OpenFOAM v2606 from the OpenCFD line (openfoam.com), shipped as the container image
  `foamcraft-runtime:<version>` built from `runtime/Dockerfile`.
- **Report:** Typst 0.15 as a library for PDF, own HTML writer; see the section "Reports".
- **MCP:** the official Rust SDK (`rmcp` 3.11), stdio transport; see the section "MCP server".
- **Data:** a project is a folder with a versioned JSON manifest plus assets. No database, no
  network services.

## Architecture

Components and their responsibilities. Existing crates are named; the others are planned.

- **Core library** (`crates/foamcraft-core`): project model, command API, geometry processing, case generation, log parsing,
  result data.
- **Runtime layer** (`crates/foamcraft-runtime`): the `Runner` trait that starts and controls
  OpenFOAM. `DockerRunner` is the first implementation (a managed WSL2 distribution and native
  Linux follow); `fake::FakeEngine` scripts the engine for tests.
- **In-runtime agent** (`crates/foamcraft-agent`, binary `foamcraft-agent`): a Linux build of the
  core that runs inside the runtime next to OpenFOAM. It is the only component that reads and
  writes case files, so OpenFOAM never works across the slow Windows-to-Linux file bridge.
- **Geometry** (`crates/foamcraft-geometry`): mesh readers (STL, OBJ, 3MF), diagnosis,
  placement (unit, rotation, translation), normal repair and the `geometry.*` commands.
- **Simulation setup** (`crates/foamcraft-sim`): scenarios, media, wind tunnel rules, cell
  budget and the `scenario.*` commands; case generation follows in Task_09.
- **Renderer** (`crates/foamcraft-render`): wgpu scene, orbit camera, picking, colour maps, particles, offscreen frames.
- **CLI** (`crates/foamcraft-cli`, binary `foamcraft`): headless access to every command, including the MCP server.
- **App** (`app/`, Rust crate `foamcraft-app` in `app/src-tauri`): Tauri shell and React
  interface; a thin client of the command API.
- **Command API** (`crates/foamcraft-api`): assembles every command (core project and history
  commands plus the run commands that need the runtime) into one `Registry`; used by the app,
  the CLI and later the MCP server.
- **Dev tasks** (`crates/foamcraft-xtask`, alias `cargo xtask`): dependency license check.

Rules that shape the code:

1. **Everything is a command.** Each user-visible action is a command with a JSON schema. UI, CLI
   and MCP are equal clients of the same command API. Undo and redo work on the command history.
2. **Headless first.** Every feature works without a window, so tests and agents can verify it.
3. **OpenFOAM does the heavy post-processing.** Function objects write surfaces and fields; Rust
   reads them and resamples the volume into 3D textures. Slices, iso-surfaces and particles are
   computed on the GPU. We do not depend on VTK or ParaView.
4. **Physical inputs, generated dictionaries.** Users enter speed, medium and a quality level.
   OpenFOAM dictionaries are generated from templates and are never edited in the interface.
5. **SI units internally.** Conversion to display units happens only at the interface boundary.
6. **Long work is a job** with progress events, cancellation and a persistent log.
7. **Failures have a stable error code** with an English and German message and a suggested fix.

## Environment

- Rust toolchain from `rust-toolchain.toml` (stable with `rustfmt` and `clippy`).
- Node 22 or newer. Use `npm` only, no other package manager. The root `package.json` holds the
  shared tooling; the frontend in `app/` becomes an npm workspace.
- Runtime tests need Docker Desktop with the WSL2 backend (Windows) or Docker Engine (Linux).

## Workspace layout

```text
Cargo.toml                 Cargo workspace (members crates/* and app/src-tauri)
crates/foamcraft-core      core library
crates/foamcraft-render    wgpu renderer (viewport and offscreen)
crates/foamcraft-geometry  mesh readers, diagnosis, placement, geometry commands
crates/foamcraft-sim       scenarios, media, wind tunnel rules, cell budget
crates/foamcraft-cli       `foamcraft` binary
crates/foamcraft-api       complete command registry (core + runtime commands), app bridge
crates/foamcraft-mcp       MCP server: curated tools, path sandbox, rmcp adapter
crates/foamcraft-report    report model, PDF (Typst template) and self-contained HTML
assets/fonts/              fonts embedded in reports (Libertinus Serif, OFL-1.1)
samples/                   sample projects (CC0): geometry scripts, STL and project files
tools/windows/             PowerShell helpers to screenshot and click the app window
                           (app-print.ps1 captures it without focus or mouse)
tools/i18n-merge.mjs       adds interface texts to en.json and de.json in one step
crates/foamcraft-runtime   runner trait, Docker runner, fake engine
crates/foamcraft-agent     agent inside the runtime container (Linux)
runtime/Dockerfile         runtime image: OpenFOAM plus agent (multi-stage build)
runtime/cases/             OpenFOAM cases adapted from tutorials (GPL-3.0-or-later)
crates/foamcraft-core/templates/  case templates (data files)
crates/foamcraft-xtask     development tasks (`cargo xtask license-check`)
app/                       npm workspace: React + Vite frontend (src/), Tauri crate (src-tauri/)
license-policy.json        allowed licenses for the license check
tests/                     all tests (Rust integration tests, Vitest, golden images, test data)
```

Start the app in development mode with `npm run app:dev` (Vite on port 1420 plus the Tauri
shell). `npm run app:build` builds a release binary.

## Viewport compositing (decided in Task_01)

The Tauri window is created with `transparent: true`; the page background is transparent. The
Rust side creates a wgpu surface on the same window and draws the 3D view into it; the WebView2
layer is composited above it by Windows, so interface panels float over the native view. This was
proven on Windows 11 (DX12/Vulkan). Mouse input always reaches the web view first: a full-window
`ViewportInput` layer underneath the panels turns drags and wheel events that start on the free
area into camera actions and sends them to Rust (`viewport_camera` command), which redraws.
Linux and macOS are checked in Task_32. Colours are written to non-sRGB targets so the viewport
and offscreen frames show identical pixel values.

## Runtime (decided in Task_02)

- **OpenFOAM release:** v2606 (OpenCFD, released 2026-06-26, build `_481094f-20260618`), base
  image `opencfd/openfoam-run:2606` pinned by digest in `runtime/Dockerfile`. Reason: the current
  OpenCFD release; the small `-run` image holds every solver and mesher we need (no tutorials,
  no compilers). GPU plugins (Task_34) need a development image or their own build stage; at the
  time of writing OpenCFD had not yet published `openfoam-dev:2606` (only up to 2512), so Task_34
  checks plugin compatibility and may add a build stage. Image labels
  `org.foamcraft.openfoam.version`, `.source` and `.build` name the exact upstream source.
- **Docker access:** through the `docker` command line, not a client library: no extra dependency,
  the same behaviour a user sees, and stable error texts that map to error codes
  (`runtime.docker_missing`, `runtime.daemon_not_running`, `runtime.no_wsl2_backend`,
  `runtime.insufficient_memory`, `runtime.image_missing`, `runtime.command_failed`,
  `runtime.agent_failed`). Messages live in `crates/foamcraft-core/data/errors.json`.
- **Host-agent protocol:** the container's main process is `foamcraft-agent idle`; the host runs
  `docker exec <container> foamcraft-agent <command>` and reads one JSON document from standard
  output (types in `foamcraft_core::runtime`). No network port, no extra service; log lines stream
  over the same pipe.
- **Case file I/O:** case files live on a named Docker volume mounted at `/work`; the host copies
  files in and out (`docker cp`, later through the agent). Measured on the reference machine
  (Docker Desktop 29.9.2, Hyper-V backend, cavity case adapted from the OpenFOAM tutorial, 40x40
  cells, 100 time steps each written to disk, three runs each):

  | Variant                        | blockMesh + icoFoam incl. copy               |
  | ------------------------------ | -------------------------------------------- |
  | Named volume                   | 1.06 s, 1.07 s, 1.14 s (copy in 0.15-0.23 s) |
  | Bind mount of a Windows folder | 1.17 s, 1.20 s, 1.22 s                       |

  The volume is about 10 % faster here, and a bind mount additionally needs a Docker Desktop
  sharing confirmation for every new host folder, which blocks unattended runs. With the WSL 2
  backend, bind mounts of Windows folders go through a slower file bridge (not measured here).
  Decision: volume plus agent. Tests: `runtime_measure_case_io_on_volume` and
  `runtime_measure_case_io_on_bind_mount` in `tests/runtime/docker.rs`.

- **Commands:** `foamcraft runtime check` (Docker, image, OpenFOAM version, cores, memory, GPU),
  `foamcraft runtime build` (builds the image from the repository root) and
  `foamcraft runtime stop` (removes every container labelled `org.foamcraft.managed=true`).
  `--lang de` switches messages to German.

## Jobs (decided in Task_03)

- **Model** (`foamcraft_core::job`): stages `background` (blockMesh), `mesh` (snappyHexMesh),
  `check` (checkMesh), `decompose`, `solve`, `reconstruct`; states `pending`,
  `running(stage)`, `succeeded`, `failed(stage, error)`, `cancelled`. `run_job` drives the stages
  through a `StageExecutor` and emits `JobEvent`s (stage started, log line, phase, progress with
  residuals, finished).
- **Execution:** the host starts each stage detached with
  `docker exec -d <c> foamcraft-agent run-stage /work/case <stage> <script>`; the agent writes
  `log.<stage>` and `.exit.<stage>` in the case folder. The host polls
  `foamcraft-agent tail /work/case <stage> <offset>` (JSON, complete lines only). Stages keep
  running when the host process ends; `foamcraft job resume <id>` continues from the stored offset.
- **Persistence:** `<data dir>/jobs/<id>/job.json` (record with state, offsets, container, volume,
  cells, cores), `job.log` (every line prefixed with `[stage]`), `case/` (the case as written on
  the host). Data dir: `%LOCALAPPDATA%FoamCraft` on Windows, `FOAMCRAFT_DATA_DIR` overrides.
- **Parallel runs:** after `check` the core count is one per 10 000 cells, at most the smaller of
  the physical host cores and the container's CPUs (`choose_cores`). One core skips decompose and
  reconstruct.
- **Cancel and failures:** cancel removes the job's container, which ends every process at once;
  the volume with the case stays until `foamcraft job clean <id>`. A FATAL ERROR, NaN residuals or a
  floating point exception end the job with `run.foam_fatal_error` or `run.diverged`; a diverging
  solver is stopped. A non-zero exit without a message is `run.stage_failed`.
- **Templates:** case templates are data files under `crates/foamcraft-core/templates/` with
  `{{NAME}}` placeholders (`foamcraft_core::case`). `reference_cube` is the pipeline test case
  (51 680 cells, 5 cores, about 15 s for 200 iterations on the reference machine).
- **Commands:** `foamcraft run [--iterations N] [--cores N]`, `foamcraft job list|log|resume|clean`.

## Projects and the command API (decided in Task_04)

- **Project format:** a folder with `foamcraft.json` (manifest: `schema_version`, `name`, `created`,
  `display_units`, `runs`) and the folders `geometry/`, `runs/` (job records of the project),
  `cache/`. `SCHEMA_VERSION` is 1; `Project::open` migrates older manifests step by step
  (`migrate` in `foamcraft_core::project`) and rejects newer ones with `project.schema_too_new`.
  The manifest is written atomically (temporary file, then rename).
- **Commands:** `foamcraft_core::command::Registry` maps a name to a `CommandSpec` (summary, JSON
  schemas of parameters and result generated with `schemars`, error codes, `undoable`, example
  parameters) and a handler. Parameters are deserialized with serde; a mismatch is
  `command.invalid_params`. Undoable commands are recorded with the manifest before and after, so
  `history.undo` and `history.redo` restore the state exactly; the project is saved after every
  change. Run commands are not undoable.
- **Session and events:** a `Session` holds the open project, the history and an `EventBus`
  (`project.changed`, `history.changed`, `run.stage`, `run.phase`, `run.progress`,
  `run.finished`). Recently opened projects are kept in `<data dir>/recent.json`.
- **Run commands:** `run.start` prepares a job in `<project>/runs/` and follows it on a background
  thread (events above); `run.status` reads the record; `run.cancel` sets the job's cancel flag or,
  for jobs not followed by this process, removes the container.
- **Changes from background work:** a command holds the session only while it runs. Work that
  outlives the command (a run) takes a copy of the project and a `Session::updates()` handle and
  pushes its changes to the manifest there; `Registry::execute` applies them before every command
  (and after one that waited), `Session::apply_updates` does it on request (the app's viewport).
  Such a change is no undo step. Never hold the session while a container works.
- **App bridge:** the Tauri command `api_call(name, params)` runs any command off the main thread
  and returns the result or `{code, detail, en, de}`; every bus event is emitted to the web view as
  `foamcraft://event`. The interface (`app/src/api.ts`) only calls commands. The start page offers
  new project (a name is enough), open, and recent projects.
- **Default projects folder (Task_22):** `project.new` and `samples.create` without `path` create
  a free folder (`Name`, `Name 2`, …; characters a file system rejects become `_`) in the
  session's projects folder: `~/FoamCraft`, overridden by `FOAMCRAFT_PROJECTS_DIR` or
  `Session::set_projects_dir`. The home folder was chosen over the documents folder because that
  one is often synchronised to a cloud drive, which would upload result caches.
  `project.default_location` reports the folder and the free path for a name; the start page
  shows it and offers "Other folder …" (folder dialog via `tauri-plugin-dialog`). The MCP server
  replaces a missing path by a free folder in its first root, so an agent stays inside its roots.
- **Units:** `foamcraft_core::units` has SI newtypes (`Length`, `Velocity`, `Pressure`, `Density`,
  `KinematicViscosity`, `Temperature`) that serialize as plain SI numbers; `display` converts for
  metric or imperial display.
- **CLI:** `foamcraft project new|open|info`, `foamcraft command list|describe|run`
  (`command run <name> '<json>' --project <dir>`).

## Geometry (decided in Task_05)

- **Readers:** own parsers for STL (binary detected by exact file size, otherwise ASCII), OBJ
  (polygons fan-triangulated, negative indices) and 3MF (`zip` + `quick-xml`; the model unit is
  read, build transforms and components are ignored). Equal vertices are merged so topology is
  explicit. Unsupported files: `geometry.unsupported_format`; broken files: `geometry.read_failed`.
- **Diagnosis:** boundary loops, non-manifold edges, inconsistent or inverted orientation,
  duplicate and degenerate triangles, bodies, size and signed volume. Findings are catalog codes
  (`geometry.ok`, `geometry.open`, `geometry.flipped_normals`, ...) with English and German text;
  commands return them already localized (`messages[].en/de`).
- **Units and placement:** the file stays as imported (copied to `geometry/model-N.<ext>`); the
  manifest (schema version 2, `geometry` list) stores unit, scale, yaw/pitch/roll in degrees
  (applied roll, pitch, yaw) and translation in metres. The unit is taken from the command, else from
  the file (3MF), else suggested from the size (largest extent above 5 file units: mm, else m).
  "Fix normals" is a flag applied when the model is loaded (non-destructive, undoable).
- **Commands:** `geometry.import`, `geometry.list`, `geometry.info`, `geometry.set_unit`,
  `geometry.set_transform`, `geometry.place_on_ground`, `geometry.fix_normals`, `geometry.remove`.
  Undo keeps copied files in `geometry/`.
- **Display:** the renderer uploads a scene once (`Renderer::upload` returns a `GpuScene`) and only
  redraws on camera moves; models are flat-shaded, coordinate axes (X red, Y green, Z blue) are drawn
  at the origin. The app calls `viewport_sync` after every `project.changed` event.
- **Performance:** one million triangles are read and diagnosed in about 0.5 s on the reference
  machine; the geometry crate is compiled with `opt-level = 3` even in dev and test builds.

## Faces, parts and selection (decided in Task_06)

- **Segmentation** (`foamcraft_geometry::segment`): region growing over manifold edges; an edge
  sharper than the model's crease angle (default 30°, `geometry.set_crease_angle`) separates faces.
  Faces smaller than 0.5 % of their part's area are merged into their largest neighbour, so scans of
  organic shapes do not break into hundreds of tiny faces. Parts are connected bodies.
- **Stable keys:** faces and parts are numbered by their lowest triangle index; keys are
  `model-1:face-3` and `model-1:part-1`. They stay the same while the file and the crease angle stay
  the same. Names (`surface_names`) and groups (`groups`) are stored in the manifest (schema
  version 3).
- **Selection** is session state (`Session::selection`, event `selection.changed`), not saved and not
  undoable. Commands: `surface.list`, `surface.select` (replace, add, toggle),
  `surface.select_at` (nearest face to a point in metres; works without a window),
  `surface.select_adjacent`, `surface.selection`, `surface.clear_selection`, `surface.rename`,
  `group.create|add|remove|delete|list`. Name and group changes are undoable.
- **Picking:** every vertex carries a pick id (0 = not pickable). The renderer draws a second pass
  into an `R32Uint` texture and reads one texel (`Renderer::pick`); hovered and selected ids are
  tinted in the colour pass (`Renderer::set_highlight`, a bit set in a storage buffer). The app maps
  pick ids to face keys when it builds the scene (`viewport_sync`) and offers `viewport_pick`,
  `viewport_hover` and `viewport_set_selection`; a click selects through `surface.select`, so the
  selection logic stays in the command API.
- **CLI:** `foamcraft command batch '<json list>' --project <dir>` runs several commands in one
  session, so a selection made with `surface.select_at` can be grouped in the same call.

## STEP and IGES (decided in Task_07)

Three ways were compared (2026-11-21):

| Way                                                          | Windows build                                                          | Headless                     | License                              | Size                            |
| ------------------------------------------------------------ | ---------------------------------------------------------------------- | ---------------------------- | ------------------------------------ | ------------------------------- |
| Open CASCADE linked natively                                 | own C++/CMake build of OCCT plus Rust bindings, hours of work, fragile | yes                          | LGPL-2.1 with exception              | about 100 MB of DLLs in the app |
| Open CASCADE as WebAssembly (opencascade.js)                 | prebuilt, but needs an Emscripten/JS runtime inside the app            | only with a JS runtime       | LGPL-2.1                             | 30-60 MB wasm                   |
| Conversion in the runtime container (Gmsh with Open CASCADE) | nothing to build: Ubuntu package `gmsh` 4.14 with OCCT 7.9             | yes (CLI and MCP, no window) | Gmsh GPL-2.0-or-later, OCCT LGPL-2.1 | +~250 MB in the runtime image   |

Decision: **conversion in the container**. The runtime (Docker) is needed for every simulation anyway,
the app stays small, and nothing has to be compiled on Windows. Flow: `geometry.import` copies the
STEP/IGES file to `geometry/model-N.<ext>` and calls a `CadConverter` (`foamcraft_api::DockerCad`):
`gmsh model.step -2 -format msh41 -clcurv 24` in a throwaway container, STEP with
`Geometry.OCCTargetUnit M` (coordinates in metres); IGES keeps its file unit, which is read from the
IGES global section. The resulting Gmsh mesh (`geometry/model-N.msh`, read by
`foamcraft_geometry`'s msh reader) keeps the CAD surface of every triangle, so each CAD face is one
clickable face (no crease splitting, no merging), and CAD volumes become parts. The manifest stores
`source` (the original CAD file) next to `file` (the mesh); schema version 4. Without a runtime,
STEP and IGES fail with `geometry.cad_unavailable`. Tessellation: 24 elements per full circle on
curved faces. Names and colours from the files are not read yet.

## Wind tunnel (decided in Task_08)

- **Scenario** (manifest schema 5, `scenario`): `wind_tunnel` with speed (m/s), flow direction (yaw in
  degrees in the ground plane, 0 = +X), ground (`none`, `fixed`, `moving` – moving is stored but
  computed only from Task_39 on), medium and quality level. Commands `scenario.set_wind_tunnel`
  (undoable) and `scenario.info` return the derived values.
- **Media:** air at 101 325 Pa (ideal gas, R = 287.05 J/(kg K), Sutherland viscosity, speed of sound
  √(1.4 R T)); water (Thiesen density, Vogel viscosity, 0.5-99 °C); custom density and kinematic
  viscosity.
- **Reference values:** frontal area = shadow of all models on a plane normal to the flow,
  rasterised with 2048 cells on the longer side (within 1 % of analytic values); reference length =
  extent along the flow; Re = U L / ν. Classes: below 10³ viscous, up to 5·10⁵ laminar boundary
  layers, above turbulent.
- **Tunnel size rule:** L = largest model dimension. Upstream 5 L, downstream 15 L, 5 L to each
  side and above; floor at the lowest model point with a ground, else 5 L below. The tunnel is
  aligned with the flow (`foamcraft_sim::tunnel_box`). Task_10 checks the rule on a reference case.
- **Cell budget rule:** quality targets Draft 0.5 M cells / 300 iterations, Standard 2 M / 500,
  Fine 8 M / 800, limited to 60 % of the memory at 1.5 GB per million cells (rounded down to
  100 000 cells; never below 100 000). Time estimate: meshing 20 000 cells per core and second plus
  solving 100 000 cell iterations per core and second (Task_03 measured about 140 000). The
  budget uses the computer's memory and physical cores; the memory of the container runtime is
  checked when meshing (Task_09). Re-measure in Task_09 and Task_33.
- **Warnings:** Mach above 0.3 (`scenario.mach_high`, the solvers are incompressible); largest
  model dimension outside 1 mm to 200 m (`scenario.unit_suspect`).
- **Display:** tunnel edges, ground plate and a flow arrow (`foamcraft_render::scene::tunnel`); the
  app draws them whenever the project has a wind tunnel. The interface has steps "Geometry" and
  "Scenario".

## Meshing the wind tunnel (decided in Task_09)

- **Flow frame:** the case is written with the models turned by -yaw, so the flow runs along +X and
  the tunnel is an axis-aligned blockMesh box (patches `inlet`, `outlet`, `sides`, `ground` with a
  floor). Results must be turned back by +yaw for display (Task_14). The plan is stored in the case
  as `foamcraft-mesh-plan.json`.
- **Plan** (`foamcraft_sim::mesh::plan_wind_tunnel`): surface cell size h_s is found by bisection so
  the estimated cell count is 95 % of the budget. Estimate = tunnel volume / h0³ + wake box + near box
  (nested refinement) + surface band (4 cells across, area / h_s²). Levels: surface s, near s-1, wake
  s-2 (at least 1); s chosen so the background cell h0 = h_s · 2^s is about half the largest model
  dimension (2 ≤ s ≤ 10). Near box: model box + 0.5 L; wake box: 0.3 L before to 4 L behind,
  ±0.6 L sideways, 0.8 L above.
- **Layers:** first cell height for y+ = 50 from a flat plate estimate (Cf = 0.0576 Re^-0.2,
  u_tau = U √(Cf/2)); expansion 1.2; layer count until half the surface cell (1-10, Draft at most 3).
- **Point in the flow:** 1.5 background cells behind the inlet, off cell faces; tested to lie outside
  every body (ray parity).
- **Patches from groups:** each face group (Task_06) becomes an STL region and the patch
  `model_<group>` (OpenFOAM word); faces without a group form `model_body`. All model patches are in
  the patch group `model`, which the field templates use.
- **Budget:** `mesh.generate` uses the quality budget with min(computer memory, container memory),
  or `max_cells`. Measured: cube 140 286 and sphere 174 244 cells for a budget of 150 000 (within
  30 %, the accepted tolerance), Demo project 519 195 for 500 000.
- **Quality and layers:** checkMesh → verdict good (non-orthogonality ≤ 65°, skewness ≤ 4),
  acceptable (no failed checks) or poor (failed checks); the snappyHexMesh layer table → mean layers
  built. **Fallback ladder:** if the mesh is poor or less than 50 % of the planned layers are built:
  half the layers (`mesh.fallback_fewer_layers`), no layers (`mesh.fallback_no_layers`), half the
  budget (`mesh.fallback_coarser`), else `mesh.failed`. A model that is not watertight is rejected
  before meshing with `mesh.leak`.
- **Result:** `mesh.generate` (blocking) returns cells, quality, layers, fallbacks and the plan; the
  manifest (schema 6) keeps `mesh` (job id, cells, verdict, layers, fallbacks). Each attempt is a job
  in `<project>/runs/` stopped after the check stage (`RunOptions.stop_after`); the volume of the
  accepted mesh stays for the solver (Task_10).
- **Preview:** the check stage also runs `surfaceMeshExtract` on the patches `"model.*"`; the surface
  is copied to `<project>/cache/mesh_preview.obj` and drawn as edges (line pipeline in the renderer)
  over the models.

## Wind tunnel solve (decided in Task_10)

- **Model:** steady incompressible RANS, `simpleFoam` with k-omega SST and wall functions
  (`kqRWallFunction`, `omegaWallFunction`, `nutkWallFunction`). Inlet: fixed speed, k and omega
  from 1 % intensity and a length scale of 0.1 × reference length (k = 3/2 (I U)²,
  omega = √k / (C_mu^0.25 l)). Outlet: p = 0, `inletOutlet` for the rest. Sides and top: slip.
  Ground and model: no slip. Numerics: `linearUpwindV` for U, upwind for k and omega, consistent
  SIMPLE with relaxation 0.9.
- **Initialisation:** the solve stage first runs `potentialFoam -writep -initialiseUBCs` (also in
  parallel); its log is shown only when it fails.
- **Reusing the mesh:** `run.start` on a project with a scenario meshes first if the project has no
  mesh for its current geometry and scenario (see "Run in one step"), then prepares the solve job whose record names the mesh volume and `start_at`
  (`Decompose` or `Solve`; `JobRecord.start_at`). Only `system/` and `0/` are copied into the
  volume (a folder `case` copied into `/work` merges with the case there); old time folders,
  `processor*`, `postProcessing` and stage markers are removed first. The solve job and the mesh
  job share the volume: removing the mesh (re-meshing) also removes the fields of older solves; the
  result object stays.
- **Forces:** `forceCoeffs` objects with `log true` for all model patches (`forces`) and, with more
  than one region, one per region (`part_<region>`), all referred to the frontal area and reference
  length from Task_08, flow frame (drag +x, lift +z, side +y). The host reads them from the solver log
  (`foamcraft_sim::solve::SolverLog`); forces are C · q · A.
- **Convergence automation** (`foamcraft_sim::solve::assess`, applied after every iteration):
  window = a fifth of the expected iterations of the quality level (30 to 200); limit = 3 × expected
  (or `iterations`). Diverged: a coefficient is not finite or above 100. Nothing stops before two
  windows. Stable: band (max - min) of Cd and Cl in the last window ≤ 1 % of |Cd| (at least 0.1).
  Oscillating: the means of the last three windows agree within max(2 %, a quarter of the band); the
  result is the mean of the three windows. Limit reached: the mean of the last window. When the
  solver ends by itself (residual targets met) the run counts as stable. To stop, FoamCraft creates
  `<case>/stop`; the `abort` function object then writes the fields and ends the solver.
- **Divergence:** a NaN residual, a floating point exception or a diverged coefficient ends the
  attempt; the run is repeated once with first-order convection for U, non-consistent SIMPLE and
  relaxation 0.5 (fields 0.3) (hint `run.restarted`). A second divergence ends with `run.diverged`.
- **y+:** the `yPlus` function object at write time; the average over the model patches outside
  30-300 gives `run.y_plus_low` or `run.y_plus_high`.
- **Result object** (`WindTunnelResult`, `<project>/results/<job id>.json`, `run.result`): reason
  (`stable`, `oscillating`, `limit_reached`), iteration of the decision and of the end, mean
  coefficients and forces in total and per part, fluctuation of Cd in percent, reference values,
  y+, hint codes, restart flag, cells, cores, wall-clock seconds per stage, scenario, mesh record and
  the histories of Cd, Cl and the initial residuals. `run.start` returns at once (events
  `run.coefficients`, `run.result`, `run.failed`, `run.restarted` besides the job events) or, with
  `wait`, the result.
- **Validation:** sharp-edged cube facing the flow in free stream, C_D = 1.05 (Hoerner, Fluid-Dynamic
  Drag, 1965), tolerance 15 % at standard quality (`validation_free_cube_drag_matches_the_literature`).
  The Ahmed body proposed in the task was not chosen: its slant separation at 25° is a known weak
  spot of steady RANS, its stilts and ground need extra geometry, and a converged run needs more
  cells than the 1.9 GiB container allows. The edges of the cube fix the separation, so the
  reference value does not depend on the Reynolds number above about 10⁴.

### Limits of the method for blunt bodies

FoamCraft computes a steady (time-averaged) flow. Behind blunt bodies such as cubes, cylinders,
vehicles without a tapered tail or buildings, the real flow sheds vortices and is unsteady. The
steady solver then often does not settle: the forces keep swinging. FoamCraft reports such runs as
"oscillating" with the mean over the last iterations; the mean is a usable estimate, but it can be
off by 10-20 % and the swinging itself has no physical frequency. Separation from smooth curved
surfaces (spheres, rounded rear ends) is hard to predict with RANS models: drag can be too high or
too low by more than 20 %. Wall functions assume an average y+ between 30 and 300; outside, forces
from friction are less reliable (hint in the result). For exact values of blunt bodies a
time-dependent simulation is needed (planned: Task_37).

## Run in one step (decided in Task_23)

- **One pipeline:** `run.start` creates the record of the run, lists it in `manifest.runs` and
  answers at once with its id. A background thread (`Pipeline` in `crates/foamcraft-api/src/run.rs`)
  then meshes if needed, prepares the solve job (`solve::prepare`, same record) and solves. With
  `wait` the same pipeline runs inside the command. `mesh_only` builds a new mesh and stops (job
  name `mesh_only`, not a run of the project). `mesh.generate` stays as the blocking command for
  the CLI and agents; both use `mesh::mesh_project`, which only reads the project.
- **When a mesh is reused:** a mesh job stores the inputs it was built from
  (`runs/<mesh job>/mesh_inputs.json`: models with placement, groups, caps, boundary roles,
  scenario). `mesh::mesh_is_current` compares them with the project; any difference, or a mesh
  without the file, means a new mesh at the next run. The volume of the replaced mesh is removed.
  Until Task_30 a change of the speed alone also re-meshes (the layer thickness depends on it).
- **Events of the meshing:** `mesh.stage {job_id, run_id, stage, attempt}`,
  `mesh.progress {job_id, run_id, attempt, stage, phase, fraction, remaining_s}` and
  `mesh.ready {run_id, job_id, cells, verdict}`. `job_id` is the job of the meshing attempt,
  `run_id` the run it belongs to (null for `mesh.generate`). A further attempt of the fallback
  ladder has a higher `attempt` and starts its progress at zero.
- **Progress estimate** (`foamcraft_sim::mesh_progress::MeshProgress`): fixed shares for background
  mesh, refinement, snapping, layers (only when planned) and check; inside a phase every step line
  of the snappyHexMesh log covers a quarter of what is left. The remaining time is the time spent
  scaled by the fraction; the interface shows it as "about".
- **Cancel and log:** `run.cancel` with the id of the run also ends the meshing (container and
  volume of the attempt are removed, the run is `cancelled`, `run.failed` carries
  `run.cancelled`). `runs/<run>/mesh_jobs` lists the meshing attempts; `run.log` returns their
  logs followed by the solver log.
- **After a restart of the app:** a run that was still meshing cannot be continued. `run.attach`
  on it stops its meshing jobs, marks the run failed and answers `run.lost`.
- **Interface:** `app/src/run-monitor.ts` holds the state of the run of the open project
  (`RunMonitor`, one per project, created in `App`); `reduceMonitor` is the pure reducer over the
  events. `RunPanel` only renders it, so leaving the step loses nothing. The "Scenario" step has no
  mesh section; mesh statistics and "Mesh only" sit in a collapsed section of the "Run" step.
  `ConsoleDock` shows one status line of the run in every step and the log when opened.

## Run monitor (decided in Task_11)

- **Step "Run"** (`app/src/components/RunPanel.tsx`): summary (cells from `mesh.info`, estimated time
  from `scenario.info`), start (`run.start` without `wait`), stage timeline from `run.stage` and
  `mesh.stage`, live charts (own SVG `LineChart`, no chart library) for the initial residuals
  (`run.progress`, logarithmic) and Cd/Cl (`run.coefficients`), remaining time, cancel
  (`run.cancel`), log on demand (`run.log`, polled every 2 s while open), result card after
  `run.result` (data from `run.result`), plain-language diagnosis after `run.failed`.
- **Texts for codes:** `error.describe {codes}` returns English and German text and suggestion
  from the catalog; the interface never holds its own copies. `tests/api/error_catalog.rs` checks
  that every code raised in the sources (`FoamError::new`), every code a command declares and every
  result hint has both languages and a suggestion.
- **Remaining time** (`foamcraft_sim::solve::remaining_seconds`): median of the last 20 iteration
  times (from `ExecutionTime`) times the iterations left: up to the expected iterations of the
  quality level, after that one more convergence window (at most the limit). Sent with
  `run.coefficients` as `remaining_s`.
- **Reconnect:** a solve job stores its setup in `runs/<job>/solve.json`. `run.active` lists the
  unfinished jobs; `run.attach` follows one again: it replays the solver lines of `job.log` into the
  convergence automation (and creates the stop file if the run has settled meanwhile) and resumes
  `run_job` from the stored log offsets. The run panel calls both when it opens. Curves after a
  reconnect start at the reconnect.
- **Keep awake and notification:** the Tauri bridge resets the system idle timer
  (`SetThreadExecutionState(ES_SYSTEM_REQUIRED)`) on every progress event and flashes the taskbar
  entry (`request_user_attention`) on `run.result` and `run.failed`. A system notification plugin
  was not added (one more dependency for little gain).

## Openings and caps (decided in Task_12)

- **Kind of model** (`foamcraft_geometry::openings::classify`): boundary loops → open shell; closed
  and genus g ≥ 1 (Euler characteristic per body) → thick-walled body with g passages; closed and
  genus 0 → closed fluid volume (its faces are marked directly).
- **Openings:** open shell: boundary loops; a coplanar loop inside another one is its hole (ring
  gaps). Thick-walled: holes in flat faces whose outward normal leaves the body (seen from outside)
  are the mouths. Not flat: points deviate more than 2 % of the diameter from the best-fit (Newell)
  plane → warning `geometry.opening_not_flat`; the cap is still made.
- **Caps:** ear clipping in the best-fit plane, holes bridged into the outer loop. Stored in the
  manifest (schema 7, `caps`) as the loops in the model's file coordinates, so they follow
  placement changes; keys `model-1:cap-N` never change. Caps can be selected like faces.
- **Clicking:** `geometry.cap_opening {model, point, face?}`: for open shells the boundary opening
  nearest to the point; for closed bodies the loop of the clicked face nearest to the point (click
  the inner wall of a passage near its mouth). The app finds the point with
  `viewport_pick_point` (GPU pick for the key, CPU ray cast on that object for the point;
  `OrbitCamera::ray`).
- **Roles:** `scenario.set_boundary_role {keys, role}` with inlet, outlet or wall for face or cap
  keys (`boundary_roles` in the manifest). `cap.list` and `cap.remove` manage caps.
- **Flood test** (`geometry.fluid_check`): voxel grid with 96 cells along the largest extent;
  triangles of models and caps are sampled at a third of a cell; the outside is flooded from the
  border (6-neighbourhood); cells inside the material of thick-walled bodies (column parity) are
  removed. Remaining cells are fluid; none means a leak (`geometry.not_sealed`). The fluid point is
  the cell farthest from walls (for `locationInMesh` in Task_13).
- **View:** open edges as yellow lines, caps translucent (screen-door: every other pixel, no
  blending or sorting; `scene::Mesh::opacity`), both sides drawn, coloured by role; arrows into
  the body at inlets and out of it at outlets.
- **Test bodies:** `tests/data/geometry/internal/*.geo` (Gmsh scripts, at least 36 segments per
  circle so polygon areas stay within 1 % of the circle).

## Internal flow (decided in Task_13)

- **Scenario** (`Scenario::InternalFlow`, `scenario.set_internal_flow`): medium, quality, inlets
  (cap or face key with the role inlet; velocity, volume flow, mass flow or total pressure) and
  outlets (static pressure). Pressures are relative to a reference (gauge). Checks: at least one
  inlet and outlet, roles match, no key twice, positive values, total pressure above the highest
  outlet pressure (`scenario.no_driving_pressure`). `scenario.boundary_roles` lists the marked
  keys for the interface.
- **Regime:** hydraulic diameter D_h = 4 A / P of every inlet (cap loops or face edges);
  Reynolds number from the mean inlet velocity (total pressure: √(2 Δp / ρ)); laminar below 2300,
  turbulent above 4000, in between computed as turbulent with `scenario.transitional`.
- **Mesh** (`mesh.generate` dispatches on the scenario; `mesh_internal.rs`): flood test first
  (`geometry.not_sealed` otherwise) for the fluid point and volume; one surface with the regions
  `wall`, `in_N` and `out_N` (patches `model_<region>`, walls in the patch group `walls`);
  uniform background cells (`plan_internal`: fluid volume / cell³ = 95 % of the budget, the box
  at most three times the budget), no refinement levels and no boundary layers in this version;
  hint `mesh.coarse_openings` below ten cells across the smallest opening; one retry with half
  the cells. The flood test resolves passages with voxels of an eighth of the smallest cap.
- **Boundary conditions** (generated in Rust, `internal_case::fields`): velocity and volume flow
  inlets `flowRateInletVelocity` (volumetric), mass flow `flowRateInletVelocity` (mass,
  `rhoInlet`), total pressure `totalPressure` with `pressureInletOutletVelocity`; outlets
  fixed (kinematic) pressure with `inletOutlet` velocity; walls no slip; turbulence: 5 %
  intensity, length 0.07 D_h; laminar flows run `simulationType laminar` without k and omega.
- **Evaluation:** `surfaceFieldValue` objects per opening (sum of phi, area average of p),
  read from the solver log (`FieldValueLog`). Per opening: volume and mass flow, share of the
  outflow, mean static pressure, total pressure = static + ½ ρ (Q/A)² (mean velocity, an
  approximation); pressure loss = flow-weighted total pressure of the inlets minus that of the
  outlets; mass balance error |in − out| / in (hint `run.mass_balance` above 1 %).
- **Convergence:** window over pressure loss and total outflow together (`assess_series`, each
  relative to its own mean). The run machinery is shared with the wind tunnel (`solve.rs`:
  `Setup` with `kind`, a `Watch` per attempt, `run.pressure_loss` events instead of
  `run.coefficients`); the result (`InternalFlowResult`, `kind: internal_flow`) is stored like
  the wind tunnel result; `run.result` returns either as JSON.
- **Validation:** laminar pipe D = 10 mm, Re ≈ 100, pressure drop between two planes in the
  developed part against Hagen-Poiseuille (5 %); turbulent pipe D = 50 mm, Re ≈ 50 000, against
  Blasius (15 %). The plane averages come from `postProcess` in the test, not from the product.

## Result fields (decided in Task_14)

- **Extraction path:** function objects in a `postProcess` run after each solve
  (`simpleFoam -postProcess -dict system/foamcraftResults -latestTime` in the runtime, generated by
  `foamcraft_sim::results::extraction_dict`): `wallShearStress`, `yPlus` (turbulent only), the
  `surfaces` object writes the wall patches (`"model.*"` or the group `walls`) triangulated in the
  "foam" surface format with face values, and the `sets` object samples U, p and k on `array`
  sets (raw format). Chosen over reading the time folders directly: OpenFOAM interpolates
  (`cellPoint`), finds the cells of the grid points and leaves out points in solids, so FoamCraft
  needs neither a mesh reader nor a point search. FoamCraft reads the output with its own reader
  (`foamcraft_sim::foam_io`: ASCII and binary lists, `N{value}`, uniform and non-uniform fields).
- **Array sets:** `axis xyz; origin (...); rotation { type none; }; pointsDensity (nx ny nz);
spanBox (...)`; points lie at origin + (i + 1) / (n + 1) · span. Without `rotation` the
  coordinate system reads `axis` as a vector and fails.
- **Grid levels:** wind tunnel: tunnel box, wake box, near box (flow frame; turn by the yaw for
  the world); internal flow: one level, the box around the fluid region. At most 64³ = 262 144
  points per level (`POINTS_PER_LEVEL`), cells as cubic as possible (`level_dims`). Points without
  a value (solid, outside) form the mask; probes next to a masked point return
  `results.outside`.
- **Fields:** surface `p` (Pa), `Cp` = p / (½ U_ref²) (wind speed, or the mean velocity of the
  first inlet), `wall_shear_stress` (Pa, magnitude), `y_plus`; volume `p`, `p_total` =
  p + ½ ρ |U|² (Pa), `Ux`, `Uy`, `Uz` (m/s), `k`. Wind tunnel surfaces are turned back to the
  world; grids stay in the flow frame (`yaw_deg` in the metadata).
- **Cache and budget:** `<project>/cache/results/<job>/`: `surface.bin`, `grid_<n>.bin` (own
  compact format: JSON header plus little-endian f32 arrays and a byte mask), `meta.json`.
  Memory budget: about 6.5 MB per grid level (U, p, total pressure, k as f32 plus mask), one result
  below 40 MB (checked by `runtime_cube_results_are_cached_and_probed`); all result caches of a
  project together at most 256 MB (`CACHE_LIMIT_BYTES`), the oldest are removed first.
- **Display:** `view.set_surface_field {field, range}` stores the choice in the manifest
  (schema 8, `view`); the viewport then draws the result surface instead of the plain models,
  one colour per triangle (`scene::Mesh::vertex_colors`, a second vertex buffer). Colour scale
  (`results::colormap`): OKLab lightness rising evenly from 0.45 to 0.95, hue from blue to yellow
  (readable with red-green colour vision deficiency and in grey). Legends get the scale as sRGB
  samples from `results.list_fields`. The value under the mouse: `viewport_pick_point` plus
  `results.probe`. Leaving the results step switches the colouring off.

## Slices and isosurfaces (decided in Task_15)

- **Scene in the project:** `view.slices`, `view.isosurfaces` and `view.clip` in the manifest
  (schema 9) with stable ids (`slice-N`, `iso-N`); commands `view.add_slice`,
  `view.update_slice`, `view.add_isosurface`, `view.remove`, `view.set_clip`, `view.state`,
  `view.volume_fields`. Opening the project shows the same scene.
- **Scene building** moved from the app into `foamcraft_api::scene::project_scene`, so the app,
  and `view.render {path, width, height}` (headless renderer, PNG) draw the same picture.
- **Slices** (`foamcraft_sim::visual::slice`): 160 × 160 points across the box of the coarsest
  grid, value from the finest grid that has one (`sample_levels`), quads with four values become
  triangles; coloured per vertex with the FoamCraft scale (range of the slice or by hand).
- **Isosurfaces:** marching tetrahedra (six tetrahedra per cell, Kuhn split) on the CPU, drawn on
  the GPU translucent (screen-door) and double-sided; a grid leaves cells inside a finer grid's box
  to that grid. Chosen over GPU marching cubes: at most 3 × 64³ points take a few tens of
  milliseconds on the CPU, need no lookup tables or compute pipelines, and have no ambiguous cases.
- **Derived fields** (`add_derived`): `U_mag` and the Q criterion (|Ω|² − |S|²)/2 from central
  differences (one-sided at edges and next to masked points).
- **Cutting the model open:** triangles of the models (or the result surface) whose centre lies on
  the side the clip normal points to are hidden; "Other side" flips the normal.
- **Moving a slice** in the app: the slider in the panel (the scene updates while dragging);
  dragging inside the 3D view itself is not part of this version.

## Particles and streamlines (decided in Task_16)

- **GPU advection** (`foamcraft-render/src/particles.wgsl`, `Renderer::create_particles`,
  `step_particles`): compute shader, midpoint rule in the velocity grids (finest grid containing
  the point; a masked corner blocks); a particle that is blocked, leaves the grids or is older than
  its lifetime starts again at a seed point. The shader writes two line vertices per particle
  (tail and head) straight into vertex buffers, which the scene's line pipeline draws
  (`GpuScene::particles`); no copy to the CPU.
- **CPU reference** (`foamcraft_sim::particles`): the same sampling, integrator, PCG random
  numbers and respawning; tests compare GPU and CPU positions.
- **Repeatable:** all random numbers come from the PCG hash of particle index, frame and the seed
  (`view.particles.seed`); the same seed gives the same image.
- **Start regions:** a list of 4096 seed points made on the CPU: `area` (origin + s u + t v, a
  line when v is zero), `inlets` (squares on the inlet caps, slightly inside), `surface` (result
  surface triangles, three quarters of a grid cell into the fluid). Presets: `wind_tunnel_smoke`
  (60 000 translucent light particles from an area upstream in the finest grid) and
  `internal_flow` (50 000 particles with tails from the inlets).
- **Styles and colours:** particles with a tail (tail length in flow time), smoke (constant light
  colour, translucent), streamlines (CPU, `foamcraft_sim::particles::streamline`, drawn as lines
  coloured by speed); colour by speed, pressure (sampled on the GPU) or age. The grids are in the
  flow frame; the shader turns the drawing by the yaw.
- **Time:** at speed 1 a particle needs about 4 s to cross the finest grid; the app advances the
  particles about 60 times a second on a clock thread and redraws.
- **Measured** (RTX 4080 Laptop): 200 000 particles in three 64³ grids with drawing at 1280 × 800
  and read-back: 2.2 ms per frame (ignored test `performance_of_200000_particles`, release); in
  the app 2.0–2.1 ms per animation frame for 50 000 to 60 000 particles.
- **Settings** in the manifest (schema 10, `view.particles`); commands `view.set_particles`
  (preset and/or single settings) and `view.particles_off`.

## Key figures and run comparison (decided in Task_17)

- **Summary** (`foamcraft_sim::summary`, command `results.summary`): a stable schema (version 1,
  snapshot `tests/data/snapshots/results_summary_schema.json`) with `figures` (stable `id`,
  `value`, `unit`, optional `part` for a face group or opening) and `reliability` (worst `level`
  green/yellow/red and the `checks` with catalog codes `reliability.*`). The command adds the
  English and German explanation of every figure (catalog entry `figure.<id>`) and check. The MCP
  server (Task_18) and the report (Task_20) read this schema; extend it only by new figure ids.
- **Derived figures:** drag area Cd·A, power drag × speed, loss coefficient Δp_total / (½ ρ v²)
  with the inlet velocity. The drag is split into pressure and friction parts from the
  `forceCoeffs` columns (`drag_split` in the stored result, older results have none).
- **Reliability light:** convergence (stable green; oscillating < 5 % yellow, else red; limit
  reached < 2 % yellow, else red), mesh verdict, wind tunnel: average y+ of the model (30–300
  green, 5–1000 yellow, else red), internal flow: mass balance (< 0.5 % green, < 1 % yellow).
- **Runs:** `runs.list` (name, kind, key value, light, disk space of job folder, result and
  cache), `runs.rename` (manifest `run_names`, schema 11), `runs.duplicate` (a variant: copy of
  the project folder with geometry and settings, without runs, mesh and view, next to it),
  `runs.delete` (job folder, result, cache and a job volume; refused for the job that holds the
  current mesh).
- **Comparison:** `results.compare` (rows with a, b, b − a and (b − a)/|a|), `results.export`
  (CSV `id,part,value,unit` or the summary as JSON). `view.set_compare` stores `view.compare`
  (schema 11): `split` draws run a and run b side by side with one camera and one colour range
  (`Renderer::render_split_gpu`, two passes with a viewport each; picking is off),
  `difference` colours the result surface by b − a (symmetric range around zero) and is refused
  with `results.geometry_differs` unless both surfaces have the same points and triangles.

## MCP server (decided in Task_18)

- **Crate `foamcraft-mcp`:** `Server` holds the curated tools (each maps to one command and
  takes its parameter schema; `run_start` drops `wait`) plus `run_wait`, `view_render` and the
  generic `commands_list`, `command_describe`, `command_run`. It is synchronous and tested
  without the protocol; `McpHandler` adapts it to `rmcp` 3.11 (tools, one resource
  `foamcraft://guide`, server instructions) and runs every call with `spawn_blocking`. The CLI
  serves it on stdio: `foamcraft mcp --root <dir>... [--app]`. Setup: `docs/mcp.md`.
- **Waiting** is done by the MCP server (polling `run.status` every 0.5 s up to the time limit),
  not by a command: a command that blocks for minutes would hold the session lock of the app and
  freeze its interface.
- **Path sandbox:** every string under a path-like key (`path`, `folder`, `file`, `dir`,
  `directory`, `source`, `root`, `*_path`, `*_dir`) must be absolute and, after resolving `..`
  lexically, lie in a root given at start (case-insensitive on Windows). `view_render` without a
  path writes a temporary PNG into the data folder and returns the bytes.
- **Bridge to the running app** (`foamcraft_api::bridge`): loopback TCP on 127.0.0.1 with a port
  chosen by the system and a random 128-bit token, both in `<data dir>/app-bridge.json` (removed
  when the app ends); one JSON line per request and answer. Reason: it works the same on Windows,
  Linux and macOS with the standard library only (named pipes and Unix sockets would need two code
  paths or a new dependency), and nothing leaves the computer. The app runs bridged commands in
  its own session, so its interface follows them through the usual events.

## Image and video export (decided in Task_19)

- **Commands** (`foamcraft_api::export`): `export.image` (PNG up to 8K; presets `hd`, `full_hd`,
  `qhd`, `4k`; transparent background = clear alpha 0, geometry is opaque; legend on/off),
  `export.video` (frame by frame offscreen in a background thread, `export.progress` events,
  `wait` for blocking use), `export.status`, `export.cancel` (removes the incomplete file) and
  `export.preview` (one frame of the timeline as base64 PNG for the dialog).
- **Timeline:** frame i shows time i / fps. The camera follows a path of key frames
  (`foamcraft_render::camera_path`: cubic Hermite with Catmull-Rom tangents, keys met exactly,
  continuous speed; presets `turntable`, `swing`, `rise`, `approach`, `still` start at the fitted
  standard view). Particles advance `setup.dt · 60 / fps` per frame, the speed of the app's 60 Hz
  clock. Everything depends only on the settings: two exports give identical files.
- **Legend** (`foamcraft_render::overlay`): drawn on the finished frame at the bottom right with a
  built-in 5 × 7 pixel font (digits, sign, units); the range comes from
  `foamcraft_api::scene::legend` (the same as the colours of the model).
- **Encoding:** Motion JPEG (own baseline encoder, 4:2:0) in AVI; reasons in
  `docs/licensing.md`. Check a file with Windows itself:
  `tools/windows/video-check.ps1 -Video <avi> -Frame <bmp>` (duration, size, codec, decoded frame).

## Reports (decided in Task_20)

- **Crate `foamcraft-report`:** `build(ReportInput) → ReportModel` (pure; texts in
  `crates/foamcraft-report/data/texts.json`, en/de; figure labels shared with the interface from
  `app/src/i18n/*.json`, `figures.label.<id>`; key figures and reliability from `results.summary`
  schema 1). Sections: cover, summary in plain words, setup, mesh, convergence (SVG line chart of
  the drag coefficient or pressure loss), key figures with explanation, standard views,
  reliability light, comparison (with `compare_with`), limits of the results, versions,
  trademark notice (in the language of the report plus the official English sentence; the PDF
  footer carries the English one).
- **PDF:** Typst as a library with an in-memory `World` (template `data/report.typ`, data as
  `/data.json`, images `/view-*.png`, `/chart.svg`, `/logo.png`); only Libertinus Serif is
  available, embedded from `assets/fonts`. Content is passed as data, never as Typst markup.
  `pdf_text` returns the laid-out text for tests.
- **HTML:** one file, inline CSS, images as data URIs, chart as inline SVG, no scripts and no
  external references.
- **Command** `export.report` (`foamcraft_api::report`): run (default newest), `compare_with`,
  language, formats (`pdf`, `html`), sections, title, author, logo PNG. Views (front, side, top,
  1600 × 1000, coloured by the surface field of the view or `p`) are rendered offscreen only when
  the result fields are in the cache; otherwise the report carries a hint instead of images.
  MCP tool `export_report`; dialog "Report" in the toolbar.
- **Look at a PDF:** `tools/windows/pdf-page.ps1 -Pdf <file> -Out <prefix>` renders its pages with
  the PDF engine of Windows.

## Setup assistant and samples (decided in Task_21)

- **System check** (`foamcraft_api::system`): steps `docker`, `service`, `backend`, `memory`,
  `image`, `disk`, `gpu`, each `ok`, `warning`, `action`, `info` or `skipped` with a catalog
  code, a detail and English/German text, suggestion and step-by-step help
  (`crates/foamcraft-api/data/system_help.json` for every `action` code). It uses the runner
  (`Runner::check`, `image_present`) plus a `Probe` (free disk space of the data folder via
  `sysinfo`, GPU name via `nvidia-smi`), so tests run it against the fake engine. Commands:
  `system.check`, `system.diagnostics` (plain text, home folder replaced by `~`),
  `system.setup_state` / `system.finish_setup` (`<data dir>/setup.json`), `system.provide_image`
  (builds the image from the repository in the background with `system.image_progress` events;
  from Task_32 on it downloads the release image instead).
- **Hardware profile:** default cores = min(physical cores, cores of the container runtime); the
  largest mesh = 60 % of the container memory at 1.5 GiB per million cells (the cell budget rule
  of Task_08); recommended quality = the highest level whose cell count fits.
- **Docker wording:** a stopped service shows "cannot connect to the Docker daemon", "error
  during connect" or (Docker 28+) "failed to connect to the docker API … if the daemon is
  running"; all map to `runtime.daemon_not_running`.
- **Samples** (`samples/`, CC0, own Gmsh scripts): `wind_tunnel` (rounded block) and `manifold`
  (header pipe with three branches, capped and with roles). `samples.list`, `samples.create`
  (copies the embedded project into a new folder, by default in the projects folder, and opens
  it). Both run through in draft quality
  (`runtime_` tests).
- **Interface:** the assistant opens at the first start and from "System"; afterwards a four-step
  tour (also from "Tour"); the start page lists the samples. Light appearance follows
  `prefers-color-scheme` (colour tokens on `:root`), every control has a visible focus ring, and
  dialogs close with Escape.

## Open decisions

- **GPU linear solver:** Ginkgo (OGL) or AmgX (petsc4Foam).

---

Maintenance: update this file immediately when the stack or architecture changes.
