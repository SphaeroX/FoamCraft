# FoamCraft

FoamCraft is a free, independent desktop app that makes CFD simulation with OpenFOAM approachable
for non-experts. Stack: a Rust workspace (core, wgpu renderer, CLI with MCP server), a Tauri 2
shell with a TypeScript and React interface in `app/`, and OpenFOAM in a container image built
from `runtime/`.

## Top rules

1. **Test-driven development is mandatory.** Write a failing test first (`cargo test` for Rust,
   Vitest for the interface), then the minimal code, then refactor. No production code without a
   test that failed before. A bug fix starts with a test that reproduces the bug.
   - Never change, delete or weaken an existing test to get green. If a test fails, fix the
     implementation. If the test itself is wrong, give the reason and wait for approval.
   - Repair budget: at most 3 attempts per failing test, then stop and report which tests fail,
     what you tried and the suspected cause.
   - Run the tests that belong to your change first (see `docs/testing.md`); run everything only
     when shared code changed.
   - Never widen a tolerance of a validation or golden-image test.
2. **Keep the documentation current.** When stack, architecture, conventions, code style, testing
   or licensing change, update the matching file in `docs/` in the same step. Work is not done
   until `docs/` is correct.
3. **Use the pinned tools only.** Rust toolchain from `rust-toolchain.toml`. Node 22 or newer
   with `npm`; no other package manager. No Python in this project.

## Commands

The Rust commands work once the Cargo workspace exists (see `docs/architecture.md`, first
implementation step).

```bash
npm ci                                                   # install the JS tooling
cargo test --workspace                                   # all fast Rust tests
cargo test -p <crate> <test_name>                        # a single Rust test
npm test                                                 # all frontend tests
npx vitest run tests/app/<file>.test.ts                  # one frontend test file
cargo fmt --all --check                                  # Rust format check
cargo clippy --workspace --all-targets -- -D warnings    # Rust lint
npm run lint                                             # TypeScript lint
npm run format                                           # TypeScript and Markdown format
```

## Documentation (load on demand)

Read only what your current work needs:

- [docs/architecture.md](docs/architecture.md): read before structural changes, new components
  or new dependencies. Tech stack, architecture rules, open technical decisions.
- [docs/testing.md](docs/testing.md): read before writing tests or changing test configuration.
  Test groups, commands, done criteria.
- [docs/code-style.md](docs/code-style.md): read before writing or formatting code.
- [docs/conventions.md](docs/conventions.md): read when naming things, organising files or
  committing.
- [docs/licensing.md](docs/licensing.md): read before adding a dependency, asset or sample file,
  before naming another product, and before touching the runtime image or a release.
- [docs/project-brief.md](docs/project-brief.md): read when goal, scope or non-goals are unclear.

## Limits

- **Licenses:** only dependencies on the allowed list in `docs/licensing.md`. Ask before adding
  anything else. Never add code or assets under a non-commercial or proprietary license.
- **Independence:** do not copy names, wording, layouts, graphics or sample models from other CFD
  products. Never present FoamCraft as OpenFOAM or as endorsed by OpenCFD Limited.
- **Everything is a command:** user-visible actions go through the command API so that UI, CLI
  and MCP stay equal. Do not put logic into `app/` that the CLI would also need.
- **No hard-coded interface text:** every string has an English and a German translation.
- **Local only:** no telemetry and no network calls beyond pulling the runtime image.
- **No simulation data in Git:** never commit cases, meshes or results.
- **Tests live in `tests/`** at the repository root (exception: Rust unit tests that need private
  access).
