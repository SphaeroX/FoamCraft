# Changelog

All notable changes to FoamCraft. Versions follow [semantic versioning](https://semver.org);
`0.x` releases may still change the project format and the command interface.

## 0.1.0 – 2026-11-22

First public version.

- Load models (STL, OBJ, 3MF, STEP, IGES), check and place them in the view.
- Wind tunnel and internal flow with clicked openings; several scenarios per project and a queue
  that runs them one after another (all, or only the ones without a result).
- Meshing and solving with OpenFOAM in a managed runtime container in one step, with progress,
  remaining time, cancel and a plain-language diagnosis; the results open when a run ends.
- Results in a GPU-rendered 3D view: surface fields, slices, iso-surfaces, streamlines,
  particles, ride-along on a particle, surface flow, heavy particles, trails and ribbons.
- Key figures with reliability light, run comparison, image and video export, reports as PDF and
  HTML.
- MCP server (`foamcraft mcp`) with the same commands as the app.
- English and German interface; setup assistant and sample projects.
- Installers for Windows, Linux and macOS with the license bundle; the app downloads the runtime
  image of its version.
