#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-3.0-or-later
#
# Assembles the landing page for GitHub Pages: the page from site/, the README pictures from
# docs/screenshots/, the display font from assets/fonts/ and the app icon. Used by the workflow
# `pages.yml` and for a local preview.
#
# Usage: tools/site/assemble.sh <output folder>
set -euo pipefail

OUT="$1"
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"

rm -rf "$OUT"
mkdir -p "$OUT/screenshots" "$OUT/fonts"
cp -R "$ROOT/site/." "$OUT/"
cp "$ROOT"/docs/screenshots/*.png "$OUT/screenshots/"
cp "$ROOT"/assets/fonts/LibertinusSerif-Regular.otf "$ROOT"/assets/fonts/LibertinusSerif-Bold.otf \
  "$ROOT"/assets/fonts/OFL.txt "$OUT/fonts/"
cp "$ROOT/app/src-tauri/icons/128x128.png" "$OUT/icon.png"
echo "Site assembled in $OUT"
