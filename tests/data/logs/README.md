# Recorded OpenFOAM logs

Logs recorded with OpenFOAM v2606 (OpenCFD, image `foamcraft-runtime:0.1.0`) on 2026-11-21 from the
FoamCraft reference case (`crates/foamcraft-core/templates/reference_cube`) and the cavity case
(`runtime/cases/cavity`). Used by the log parser tests. OpenFOAM output, GPL-3.0-or-later.

- `simpleFoam_ok.log`: iterations 1-20 and the last iteration (200) of an 8-process run; the
  middle was cut to keep the file small.
- `snappyHexMesh_fatal_io_error.log`: missing `errorReduction` in `meshQualityControls`.
- `icoFoam_sigfpe.log`: cavity with deltaT 0.5 and lid speed 100 m/s; floating point exception.
- `icoFoam_nan.log`: the same with `FOAM_SIGFPE=false`; residuals become `nan`.
- `snappyHexMesh_layers.log`: last 42 lines of a wind tunnel cube run (Task_09) with the layer table.
- `checkMesh_wind_tunnel.log`: checkMesh of the same mesh (188 962 cells, 4 faces above 70° non-orthogonality).
- `simpleFoam_forces.log`: iterations 1-2 of the 20 mm wind tunnel cube (Task_10) with the force
  coefficient output (`forceCoeffs`, `log true`) and the `yPlus` output of the same case.
