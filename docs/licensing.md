# Licensing, trademarks and third-party material

> When to read: before adding a dependency, an asset, sample data or any text that names another
> product; before changing the runtime image or preparing a release.

This is engineering guidance for the project, not legal advice.

## Project license

FoamCraft is licensed under **GPL-3.0-or-later**. Contributions are accepted under the same
license. Every source file carries the SPDX line (see [code-style.md](code-style.md)).

## Dependencies

**Allowed:** MIT, Apache-2.0, BSD-2-Clause, BSD-3-Clause, ISC, Zlib, BSL-1.0, Unicode-3.0,
MPL-2.0, LGPL-2.1-or-later, LGPL-3.0, GPL-2.0-or-later, GPL-3.0-only, GPL-3.0-or-later, 0BSD,
BlueOak-1.0.0 (the last two approved by the maintainer on 2026-11-21).

**Not allowed:**

- GPL-2.0-only (incompatible with GPL-3.0).
- Licenses with a non-commercial, research-only or field-of-use restriction (example: FluidX3D).
- Proprietary, freeware or source-available licenses (SSPL, BUSL, Elastic).
- AGPL-3.0 without the maintainer's approval, because it adds a network clause to the whole app.

Anything not listed: ask the maintainer before adding it.

A dependency license check runs over the Rust and the npm dependency trees and must pass before
work is handed back: `cargo xtask license-check`. The allowed list is mirrored in
`license-policy.json`; per-package exceptions go there with a reason and need the maintainer's
approval.

## OpenFOAM

- OpenFOAM is GPL-3.0-or-later. FoamCraft drives it as a separate program and ships it inside the
  runtime image.
- Because we distribute OpenFOAM binaries in the image, the corresponding source must be
  available: the image records the exact upstream release, and our Dockerfile and patches are in
  this repository. Releases state where to get the source.
- Code that links against OpenFOAM libraries is GPL-3.0-or-later and lives in `runtime/`.
- OpenFOAM tutorial cases may be adapted (same license); keep the attribution.

## Trademarks and other products

- OPENFOAM® and OpenCFD® are trade marks of OpenCFD Limited. Use the name only to describe what
  the software works with ("uses OpenFOAM"). Never use it in the product name, logo, domain or in
  a way that suggests endorsement, and never call the project or its contributors "OpenFOAM
  developers".
- README, the About dialog, the report footer and release pages carry this notice: "This offering
  is not approved or endorsed by OpenCFD Limited, producer and distributor of the OpenFOAM
  software via www.openfoam.com, and owner of the OPENFOAM® and OpenCFD® trade marks."
- Other CFD products may inspire functionality, nothing more. Do not copy or imitate their names,
  logos, interface layout, wording, screenshots, report layout or sample models. Do not use their
  names in the app, the report or the repository.
- Do not transcribe code or algorithms from sources whose license is not on the allowed list.

## Assets, fonts and sample data

- Icons and fonts: MIT, Apache-2.0, OFL-1.1 or CC0.
- Report font: Libertinus Serif (Regular, Bold, Italic) under the SIL OFL 1.1, unmodified from the
  typst-assets 0.15.1 package, in `assets/fonts/` with its licence text (`OFL.txt`). It is embedded
  in every PDF report. Releases ship `OFL.txt` with the other notices. The landing page (`site/`)
  serves the Regular and Bold files from the same folder, together with `OFL.txt`; it loads no
  fonts or scripts from third-party servers.
- Demo video (`site/media/demo.mp4`) and its poster: self-made from FoamCraft exports of the CC0
  sample geometry, with self-synthesised music; released under the project licence.
- Sample geometry and test data: self-made and released as CC0 (sample projects in `samples/`,
  made from own Gmsh scripts). Model reference bodies from their
  published dimensions instead of downloading third-party files. Check the license of any
  third-party model before it enters the repository.

## Gmsh and Open CASCADE in the runtime image

The runtime image installs the Ubuntu package `gmsh` (Gmsh, GPL-2.0-or-later, with Open CASCADE
Technology, LGPL-2.1 with exception) to convert STEP and IGES files. FoamCraft runs it as a separate
program and does not link against it. The corresponding sources are the Ubuntu source packages of
the base image's release.

## Video encoder and container (decided in Task_19)

Videos are **Motion JPEG in an AVI container**, both written by FoamCraft itself
(`crates/foamcraft-render/src/jpeg.rs`, `video.rs`): a baseline JPEG encoder after ITU-T T.81 with
the example tables of its Annex K and 4:2:0 chroma, and a plain RIFF/AVI writer with an `idx1`
index. Reasons:

- **Licence and patents:** baseline JPEG and AVI carry no patent licences; no codec library is
  bundled. H.264/H.265 need patent licences (OpenH264 covers only Cisco's own binaries); VP9 and
  AV1 are royalty-free but need C libraries (libvpx) or a large encoder (rav1e) and play in the
  Windows player only with Store extensions.
- **Playback:** the built-in player of Windows (and Media Foundation) plays MJPEG AVI without
  extensions. The Media Foundation decoder needs subsampled chroma; 4:4:4 frames show wrong
  colours there. That is why the encoder of the `image` crate (4:4:4 only) is not used for video
  (it stays a test-only dependency to decode frames), and `jpeg-encoder` (IJG licence, not on the
  allowed list) was not added.
- **Cost:** files are large (about 15 MB per second at Full HD with particles); a plain AVI stays
  below 1 GiB, so long 4K videos stop with an error. A compressed codec can be added later as an
  option without changing the commands.

## GPU runtime variant

A runtime image with NVIDIA CUDA libraries stays a separate, optional image based on NVIDIA's
official base image. Whether and how it may be redistributed together with GPL solver plugins is
an open question that must be answered before such an image is published.

## Releases

Each release bundles the license texts and notices of all shipped dependencies, the OpenFOAM
source reference and the trademark notice.

---

Maintenance: update this file immediately when a license rule, the allowed list or the
distribution model changes.
