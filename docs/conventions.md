# Naming conventions and practices

> When to read: when naming things, organising files, committing or adding a component.

## Naming

- Rust crates: `foamcraft-<area>`. Modules and functions `snake_case`, types `PascalCase`.
- TypeScript: components `PascalCase.tsx`, all other files `kebab-case.ts`.
- Commands: `<domain>.<verb>` in lower snake case, for example `geometry.import`,
  `scenario.set_wind_tunnel`, `run.start`.
- Error codes: `<domain>.<problem>`, for example `mesh.layers_failed`. A code never changes
  once it is released.
- Translation keys: `<area>.<element>.<purpose>`, for example `tunnel.speed.label`.
- Tests: see [testing.md](testing.md).

## Structure and organisation

- Logic that the CLI or MCP server also needs belongs in the core library, never in `app/`.
- A new user-visible action is added as a command first, then wired into the interface.
- Code that links against OpenFOAM (function objects, boundary conditions) lives in `runtime/`.
- OpenFOAM case templates live with the core library as data files, not as string literals.

## Practices

- No network access except pulling the runtime image and a user-triggered update check.
  No telemetry.
- Write only inside the project folder, the app data folder and locations the user chose. The MCP
  server accepts paths only inside the roots it was started with.
- Never commit simulation cases, meshes or results. Only small self-made test data is versioned.
- Before adding a dependency, check its license against [licensing.md](licensing.md). Ask the
  maintainer if the license is not on the allowed list.
- Do not describe FoamCraft as OpenFOAM or as approved by its owners; see
  [licensing.md](licensing.md) for wording.

## Git

- Commit messages follow Conventional Commits (`feat:`, `fix:`, `docs:`, `test:`, `chore:`,
  `refactor:`).
- Work happens on `main` unless the maintainer asks for a branch.

---

Maintenance: update this file immediately when conventions or components are added.
