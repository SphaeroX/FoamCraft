# Tech stack and architecture

> When to read: before structural changes, new components or new dependencies.

## Tech stack

- **Core:** Rust (stable, edition 2024) in one Cargo workspace. No Python anywhere.
- **Desktop shell:** Tauri 2. User interface in TypeScript, React and Vite, located in `app/`.
- **3D rendering:** own renderer in Rust on `wgpu`. It draws natively beneath the transparent web
  view and can render offscreen. Reason: one renderer for the viewport, MCP screenshots, video
  export and the later GPU preview solver, independent of WebGPU support in system web views.
- **Solver:** OpenFOAM from the OpenCFD line (openfoam.com), pinned to one release, shipped as a
  container image built from `runtime/`.
- **Report:** Typst for PDF, a template for HTML.
- **MCP:** the official Rust SDK (`rmcp`), stdio transport.
- **Data:** a project is a folder with a versioned JSON manifest plus assets. No database, no
  network services.

## Architecture

Planned components and their responsibilities (crate names are fixed when they are created):

- **Core library:** project model, command API, geometry processing, case generation, log parsing,
  result data.
- **Runtime layer:** the `Runner` abstraction that starts and controls OpenFOAM. Docker is the
  first implementation; a managed WSL2 distribution and native Linux follow.
- **In-runtime agent:** a Linux build of the core that runs inside the runtime next to OpenFOAM.
  It is the only component that reads and writes case files, so OpenFOAM never works across the
  slow Windows-to-Linux file bridge.
- **Renderer:** wgpu scene, picking, colour maps, particles, offscreen frames.
- **CLI:** `foamcraft`, headless access to every command, including the MCP server.
- **App:** Tauri shell and React interface; a thin client of the command API.

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

## First implementation step

Not executed during setup:

- Create the root `Cargo.toml` workspace and the first crates.
- Scaffold the Tauri app into `app/` (for example `npm create tauri-app@latest`), add it to the
  npm workspaces and add a `tsconfig.json`.
- Set up the dependency license check described in [licensing.md](licensing.md).

## Open decisions

- **Web view over wgpu:** compositing a transparent web view above a native wgpu surface must be
  proven on Windows first. Fallback: render with three.js inside the web view.
- **OpenFOAM release:** exact version, chosen for compatibility with the GPU solver plugins.
- **Case file I/O:** in-runtime agent on a container volume (planned) versus a bind mount; decide
  by measurement.
- **STEP and IGES import:** Open CASCADE linked natively or loaded as WebAssembly; must also work
  headless.
- **Video encoder:** a license-clean encoder and container format.
- **GPU linear solver:** Ginkgo (OGL) or AmgX (petsc4Foam).

---

Maintenance: update this file immediately when the stack or architecture changes.
