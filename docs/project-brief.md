# Project brief

> When to read: when the goal, scope or boundaries of the project are unclear.

## Goal and audience

FoamCraft is a free, independent, open-source desktop app that lets people without CFD training run
flow simulations with OpenFOAM. The workflow is: load a file, click openings or choose the wind
tunnel, run, look at the result, export.

- **Audience:** makers, students and engineers without CFD experience; AI agents through an MCP
  server.
- **Success means:** a first-time user gets from a 3D file to a plausible result and a report
  without touching an OpenFOAM dictionary, on both base cases (wind tunnel and internal flow).

## Core functions

- Import STL, OBJ, 3MF, STEP, IGES; diagnose geometry problems in plain language.
- External flow in a virtual wind tunnel: orientation, speed, ground, medium, quality level.
- Internal flow: click an opening to close it and mark it as inlet or outlet.
- Automatic meshing, solver setup, convergence detection and stop.
- 3D results: surface fields, slices, iso-surfaces, streamlines, animated particles.
- Key figures (forces, coefficients, pressure drop, flow rates) and comparison of runs.
- Image and video export, report export as PDF and HTML.
- MCP server with the same capabilities as the user interface.
- Physics extensions: transient flow, heat and buoyancy, rotating and porous parts, free water
  surface.
- Performance: all CPU cores, optional GPU linear solver, later an own GPU solver for instant
  previews.

## Non-goals

- CAD editing. Geometry is only loaded, oriented, scaled and capped at openings.

## Wishes for later (not planned yet)

- Cloud service with accounts.
- Automatic shape optimisation (adjoint, morphing).
- Supersonic flow and combustion.

## Constraints

- **Local only:** everything runs on the user's machine. No server, no account, no telemetry.
- **Platforms:** Windows first, Linux and macOS supported.
- **License:** GPL-3.0-or-later; see [licensing.md](licensing.md).
- **Independence:** own user interface and wording. No code, text, graphics or branding from
  commercial CFD products. Trademark notice for OpenFOAM in app and README.
- **Languages:** user interface in English and German. Code, comments, `docs/` and `AGENTS.md` in
  English. The files under `Roadmap/` are German.
- **Team:** one maintainer working with AI coding agents (Claude Code).
- **Reference machine:** Windows 11, Intel i9-13900HX (8 P-cores, 16 E-cores), 64 GB RAM,
  NVIDIA RTX 4080 Laptop (12 GB), Docker Desktop.

## Open decisions

- Contribution policy (DCO sign-off or none): decide before the first public release.
- Technical open decisions are listed in [architecture.md](architecture.md).

---

Maintenance: update this file when goals or scope change.
