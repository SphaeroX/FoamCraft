# Recorded OpenFOAM field data

Recorded with OpenFOAM v2606 (image `foamcraft-runtime:0.1.0`) on 2026-11-21 from the cavity case in
`runtime/cases/cavity` after one icoFoam step (t = 0.005). OpenFOAM output, GPL-3.0-or-later. Used by
`tests/sim/foam_io.rs`.

- `ascii/p`, `ascii/U`, `binary/p`, `binary/U`: the fields written with `writeFormat ascii` and
  `binary` (1600 cells, non-uniform).
- `surface/`: the `movingWall` patch written by the `surfaces` function object in the `foam` surface
  format (triangulated, face values); `vectorField/U` is the uniform list form `80{(1 0 0)}`.
- `grid_level0_p_U.xy`: a 4 × 4 × 1 `array` sample set in the `raw` format (`x y z p Ux Uy Uz`);
  the points lie at origin + (i + 1) / (n + 1) · span.
