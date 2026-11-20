# Licensing, trademarks and third-party material

> When to read: before adding a dependency, an asset, sample data or any text that names another
> product; before changing the runtime image or preparing a release.

This is engineering guidance for the project, not legal advice.

## Project license

FoamCraft is licensed under **GPL-3.0-or-later**. Contributions are accepted under the same
license. Every source file carries the SPDX line (see [code-style.md](code-style.md)).

## Dependencies

**Allowed:** MIT, Apache-2.0, BSD-2-Clause, BSD-3-Clause, ISC, Zlib, BSL-1.0, Unicode-3.0,
MPL-2.0, LGPL-2.1-or-later, LGPL-3.0, GPL-2.0-or-later, GPL-3.0-only, GPL-3.0-or-later.

**Not allowed:**

- GPL-2.0-only (incompatible with GPL-3.0).
- Licenses with a non-commercial, research-only or field-of-use restriction (example: FluidX3D).
- Proprietary, freeware or source-available licenses (SSPL, BUSL, Elastic).
- AGPL-3.0 without the maintainer's approval, because it adds a network clause to the whole app.

Anything not listed: ask the maintainer before adding it.

A dependency license check runs over the Rust and the npm dependency trees and must pass before
work is handed back. It is set up in the first implementation step; its commands are then listed
in [testing.md](testing.md).

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
- Sample geometry and test data: self-made and released as CC0. Model reference bodies from their
  published dimensions instead of downloading third-party files. Check the license of any
  third-party model before it enters the repository.

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
