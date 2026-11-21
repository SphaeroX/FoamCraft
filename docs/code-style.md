# Code style and formatting

> When to read: before writing, changing or formatting code.

## Tools

- Rust formatter: `rustfmt`. Command: `cargo fmt --all`. Configuration: `rustfmt.toml`.
- Rust linter: `clippy`. Command: `cargo clippy --workspace --all-targets -- -D warnings`.
- TypeScript formatter: Prettier. Command: `npm run format`. Configuration: `.prettierrc.json`.
- TypeScript linter: ESLint. Command: `npm run lint`. Configuration: `eslint.config.js`.

The configuration files are authoritative; their rules are not repeated here.

## Binding

- Every source file starts with the SPDX line, for example
  `// SPDX-License-Identifier: GPL-3.0-or-later` (`#` in TOML, YAML, shell and Dockerfiles).
- Rust: no `unwrap()` or `expect()` outside tests, except where failure is impossible and a
  comment says why. Errors are typed and carry the stable error code from the command API.
- Rust: no `unsafe` without a comment that states the invariant.
- TypeScript: strict mode, no `any`.
- No hard-coded user-visible text. Every string goes through the translation layer and has an
  English and a German entry.
- Physical quantities are stored and passed in SI units. Unit conversion happens only where a
  value is shown or entered.
- The interface never builds OpenFOAM dictionaries. It sends commands; the core generates files.

## Interface design system (Task_29)

- Colours, spacing, radii and text sizes are CSS variables on `:root` in `app/src/styles.css`
  (`--ink`, `--panel-bg`, `--panel-border`, `--raised`, `--text`, `--muted`, `--accent`,
  `--accent-ink`, `--ok`, `--warn`, `--bad`, `--space-1..4`, `--radius-s/m`, `--text-s..xxl`), redefined
  in the `prefers-color-scheme: light` block. Rules use variables, not literal colours.
- One accent (dye cyan) marks the main action of a step (`button.primary`); secondary actions are
  `button.quiet`. State colours (`--ok`, `--warn`, `--bad`) only mark states.
- Text contrast is at least 4.5:1 in both themes, measured against the panel over the fixed dark
  3D background (`BACKGROUND` in `foamcraft-render`), not against the panel colour alone.
- System fonts only (Segoe UI Variable on Windows); no fonts or icons loaded from the network;
  icons are inline SVG drawn for FoamCraft.
- Simple and expert: wrap controls that need CFD knowledge in `<ExpertOnly>` (`app/src/mode.tsx`).
  Without a `ModeProvider` the level is expert, so components rendered alone in tests show all
  controls; only the app sets the provider (default simple, stored as `foamcraft.mode`).

## Recommendation

- Prefer small modules with one responsibility over large files.
- Comment the reason for a non-obvious choice (a solver setting, a tolerance), not the mechanics.
- Keep OpenFOAM settings in templates and named presets instead of scattering literal values.

---

Maintenance: update this file immediately when style or tool rules change.
