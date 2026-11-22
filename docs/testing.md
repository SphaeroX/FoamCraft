# Testing strategy

> When to read: before you write tests or change the test configuration.

## Test suites

- **Rust:** the built-in test harness (`cargo test`). It is the ecosystem standard, fast for TDD
  and needs no extra tooling.
- **Frontend:** Vitest. It shares the Vite toolchain of the app and has a fast single-file run.

## TDD cycle (binding)

1. Red: write the test, run it, confirm that it fails for the expected reason.
2. Green: write the minimal code that makes it pass.
3. Refactor: clean up while the tests stay green.

## Commands

```bash
cargo test --workspace                            # all fast Rust tests
cargo test -p <crate> <test_name>                 # a single Rust test
cargo test --workspace -- --ignored runtime_      # needs Docker and the runtime image
cargo test --workspace -- --ignored validation_   # slow CFD reference cases
npm test                                          # all frontend tests
npx vitest run tests/app/<file>.test.ts           # one frontend test file
npx vitest run -t "<test name>"                   # one frontend test
cargo xtask license-check                         # dependency license check (Rust + npm)
cargo test -p foamcraft-xtask --test release      # license bundle complete, versions agree
npm run typecheck                                 # TypeScript type check
```

`.github/workflows/ci.yml` runs format, clippy, the fast Rust and frontend tests, lint, type check
and the license check on Windows, Linux and macOS for every push to `main` and every pull request.

## Which tests to run when

- Change in one crate: `cargo test -p <crate>`.
- Change in the core library or the command API: `cargo test --workspace` and `npm test`.
- Change in case generation, the runtime layer or the runtime image: also the `runtime_` tests.
- Change in solver settings, meshing rules or numerics: also the `validation_` tests.
- Change in `app/`: `npm test`.

## Done criteria

A piece of work is done when all of this holds:

1. The tests of the affected area pass (commands above).
2. `cargo fmt --all --check` and `cargo clippy --workspace --all-targets -- -D warnings` pass.
3. `npm run lint` and `npm run format:check` pass.
4. The dependency license check passes: `cargo xtask license-check` (policy in
   `license-policy.json`, rules in [licensing.md](licensing.md)).
5. No existing test was changed, deleted or weakened, unless that was explicitly requested.

## Hard rules

- A failing test means the implementation is wrong. Fix the implementation, not the test.
- At most 3 repair attempts per failing test, then stop and report what failed, what was tried
  and the suspected cause.
- If an existing test must change, give the reason and wait for approval.
- Never widen the tolerance of a validation or golden-image test to make it pass.
- A test that is skipped because Docker or a GPU is missing counts as skipped, not as passed.
  Say so in the report.

## Test structure

- **Location:** all tests live in `tests/` at the repository root. The only exception is Rust
  unit tests that need private access; they sit in a `#[cfg(test)]` module of the source file.
- **Rust integration tests:** files under `tests/<area>/`, wired into the owning crate with
  `[[test]]` entries that set `path`.
- **Frontend tests:** `tests/app/**/*.test.ts` and `*.test.tsx`.
- **Runtime tests:** need Docker and the runtime image (`foamcraft runtime build`, or
  `cargo run -p foamcraft-cli -- runtime build`). Mark them `#[ignore]` and prefix the function
  name with `runtime_`. They stop their containers through a drop guard. Runner logic without
  Docker is tested against `foamcraft_runtime::fake::FakeEngine`.
- **Validation tests:** compare a full simulation with a literature or analytic value inside a
  stated tolerance. Mark them `#[ignore]` and prefix the name with `validation_`. Record source
  and tolerance in the test.
- **Snapshot tests:** generated OpenFOAM dictionaries and the JSON schema of `results.summary` are
  compared with files in `tests/data/snapshots/`; update them only with a stated reason
  (`FOAMCRAFT_UPDATE_SNAPSHOTS=1`). Prettier skips that folder so formatting never changes them.
- **Recorded OpenFOAM output:** `tests/data/fields` (field files ASCII/binary, foam surface
  format, raw sets) for the reader; record new data with the runtime image and describe it in the
  folder's README.
- **Golden-image tests:** render offscreen and compare with images in `tests/golden/` using a
  tolerance. Update a golden image only with a stated reason, by running the test with
  `FOAMCRAFT_UPDATE_GOLDEN=1`. Without a GPU adapter they print `SKIPPED` and return (that is a
  skip, not a pass); `FOAMCRAFT_REQUIRE_GPU=1` turns a missing adapter into a failure.
- **Test data:** small, self-made geometry files under `tests/data/` (CC0). No large files, no
  third-party models without a clear license. Recorded OpenFOAM logs for the log parser are in
  `tests/data/logs/` (see its README); record new samples when the OpenFOAM release changes.
- **Naming:** a test name states the behaviour, for example `imports_binary_stl_in_millimetres`.

---

Maintenance: update this file immediately when the test suites or the strategy change.
