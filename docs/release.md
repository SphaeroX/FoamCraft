# Releases

> When to read: before preparing or publishing a release, or before changing the release workflow,
> the version or the runtime image tag.

## Versions

- One version for everything: `[workspace.package] version` in `Cargo.toml`. `app/package.json`
  and `app/src-tauri/tauri.conf.json` carry the same version (checked by `tests/xtask/release.rs`).
  The root `package.json` is the tooling root and stays at `0.0.0`.
- Semantic versioning; `0.x` until the project commits to a stable interface.
- The runtime image tag follows the version: the app pulls
  `ghcr.io/sphaerox/foamcraft-runtime:<version>` (`foamcraft_runtime::PUBLISHED_IMAGE`) and tags it
  locally as `foamcraft-runtime:<version>` (`IMAGE_TAG`). A development build, which finds
  `runtime/Dockerfile` next to its sources, builds the image instead
  (`foamcraft_api::system::fetch_image`).
- `cargo xtask version` prints the version.

## Release workflow

`.github/workflows/release.yml` runs on a pushed tag `v<version>` or by hand (Actions tab, "Run
workflow" on `main`, which creates the tag). It

1. checks that the tag matches the version in `Cargo.toml`,
2. builds `runtime/Dockerfile` and pushes `ghcr.io/sphaerox/foamcraft-runtime:<version>`,
3. builds the installers on Windows (NSIS `.exe`, `.msi`), Linux (`.deb`, `.rpm`, `.AppImage`) and
   macOS (Apple silicon `.dmg`) with `tauri-action` and the overlay configuration
   `app/src-tauri/tauri.release.json` (bundling on, license page, the command line program
   `foamcraft` next to the app as `externalBin`, the license bundle as resource `notices/`),
4. puts them into a draft release and publishes it when every job succeeded.

The license bundle comes from `cargo xtask notices <dir>`: `THIRD-PARTY-NOTICES.txt` (every Rust
crate and npm package of both lockfiles with its license and license files), `NOTICE.txt`
(source links, OpenFOAM source reference, trademark notice), `LICENSE.txt` and `OFL.txt`.

## Checklist

1. All tests green (`docs/testing.md`), `cargo xtask license-check` passes.
2. Raise the version in `Cargo.toml`, `app/package.json` and `app/src-tauri/tauri.conf.json`;
   `cargo test -p foamcraft-xtask --test release` passes.
3. Add the release to `CHANGELOG.md`.
4. Commit, push, then tag: `git tag v<version>` and `git push origin v<version>`. Publishing is
   an outward action: only with the maintainer's explicit consent.
5. After the workflow: the release is published, the runtime image can be pulled without login
   (`docker pull ghcr.io/sphaerox/foamcraft-runtime:<version>`; a new package on GHCR may start
   private and must then be set to public once in the package settings).
6. Smoke test on Windows: install, start, `foamcraft --version` from the install folder, the
   setup assistant provides the image.

## Not yet

- Signing (Windows certificate, Apple developer account): the installers are unsigned, so
  SmartScreen and Gatekeeper warn.
- macOS on Intel and Linux on ARM; the runtime image is built for `linux/amd64` only (Apple
  silicon runs it through Docker's emulation).
- Automatic updates and app stores are out of scope.
