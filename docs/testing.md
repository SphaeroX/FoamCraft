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

The Rust commands work once the Cargo workspace exists (first implementation step).

```bash
cargo test --workspace                            # all fast Rust tests
cargo test -p <crate> <test_name>                 # a single Rust test
cargo test --workspace -- --ignored runtime_      # needs Docker and the runtime image
cargo test --workspace -- --ignored validation_   # slow CFD reference cases
npm test                                          # all frontend tests
npx vitest run tests/app/<file>.test.ts           # one frontend test file
npx vitest run -t "<test name>"                   # one frontend test
```

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
4. The dependency license check passes (see [licensing.md](licensing.md); its command is added
   here when the check is set up).
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
- **Runtime tests:** need a running container. Mark them `#[ignore]` and prefix the function name
  with `runtime_`.
- **Validation tests:** compare a full simulation with a literature or analytic value inside a
  stated tolerance. Mark them `#[ignore]` and prefix the name with `validation_`. Record source
  and tolerance in the test.
- **Golden-image tests:** render offscreen and compare with images in `tests/golden/` using a
  tolerance. Update a golden image only with a stated reason.
- **Test data:** small, self-made geometry files under `tests/data/` (CC0). No large files, no
  third-party models without a clear license.
- **Naming:** a test name states the behaviour, for example `imports_binary_stl_in_millimetres`.

---

Maintenance: update this file immediately when the test suites or the strategy change.
