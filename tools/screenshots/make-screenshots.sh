#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-3.0-or-later
#
# Renders the README pictures from the sample projects: creates both samples, runs them in the
# OpenFOAM runtime and exports views through the command API. The same script runs locally and in
# the GitHub workflow `screenshots.yml`.
#
# Usage: tools/screenshots/make-screenshots.sh <foamcraft binary> <output folder>
# Needs Docker with the runtime image (`foamcraft runtime build`) and a GPU adapter (a software
# Vulkan driver such as Mesa lavapipe is enough).
set -euo pipefail

FC="$1"
OUT="$2"
WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT
mkdir -p "$OUT"

# Paths as the CLI expects them (forward slashes on Windows too).
native() {
  if command -v cygpath >/dev/null 2>&1; then cygpath -m "$1"; else printf '%s' "$1"; fi
}
W="$(native "$WORK")"
O="$(native "$(cd "$OUT" && pwd)")"
SIZE='"width": 1280, "height": 720'

batch() {
  local project="$1"
  shift
  if [ -n "$project" ]; then
    "$FC" command batch --project "$project" "$1" >/dev/null
  else
    "$FC" command batch "$1" >/dev/null
  fi
}

for sample in wind_tunnel manifold; do
  echo "== $sample: create and run"
  batch "" "[{\"name\": \"samples.create\", \"params\": {\"sample\": \"$sample\", \"path\": \"$W/$sample\"}}]"
  batch "$W/$sample" '[{"name": "run.start", "params": {"wait": true}}]'
done

echo "== wind tunnel views"
batch "$W/wind_tunnel" "[
  {\"name\": \"view.apply_preset\", \"params\": {\"id\": \"light_trails\"}},
  {\"name\": \"export.image\", \"params\": {\"path\": \"$O/wind-tunnel-light-trails.png\", $SIZE, \"exposure_s\": 2.0, \"legend\": false}},
  {\"name\": \"view.apply_preset\", \"params\": {\"id\": \"smoke_lines\"}},
  {\"name\": \"export.image\", \"params\": {\"path\": \"$O/wind-tunnel-smoke-lines.png\", $SIZE, \"exposure_s\": 3.0, \"legend\": false}},
  {\"name\": \"view.apply_preset\", \"params\": {\"id\": \"ribbons\"}},
  {\"name\": \"export.image\", \"params\": {\"path\": \"$O/wind-tunnel-ribbons.png\", $SIZE, \"legend\": false}},
  {\"name\": \"view.apply_preset\", \"params\": {\"id\": \"surface_pressure\"}},
  {\"name\": \"export.image\", \"params\": {\"path\": \"$O/wind-tunnel-pressure.png\", $SIZE}},
  {\"name\": \"view.apply_preset\", \"params\": {\"id\": \"oil_film\"}},
  {\"name\": \"export.image\", \"params\": {\"path\": \"$O/wind-tunnel-oil-film.png\", $SIZE, \"legend\": false}}
]"

echo "== manifold views"
batch "$W/manifold" "[
  {\"name\": \"view.apply_preset\", \"params\": {\"id\": \"light_trails\"}},
  {\"name\": \"export.image\", \"params\": {\"path\": \"$O/manifold-light-trails.png\", $SIZE, \"exposure_s\": 3.0, \"legend\": false}},
  {\"name\": \"view.apply_preset\", \"params\": {\"id\": \"internal_slice\"}},
  {\"name\": \"export.image\", \"params\": {\"path\": \"$O/manifold-slice.png\", $SIZE}}
]"

echo "== done: $(ls "$OUT" | wc -l) pictures in $OUT"
