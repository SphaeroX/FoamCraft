# Sample projects

Self-made sample projects of FoamCraft, released as **CC0-1.0** (geometry, scripts and project
files). The app creates a copy of them with the command `samples.create`.

- `wind_tunnel/`: "Rounded body", a 400 × 180 × 150 mm block with rounded front and side edges
  (`body.geo`) in a wind tunnel at 20 m/s, free flow, draft quality.
- `manifold/`: "Pipe manifold", a header pipe (Ø 40 mm) with three branches (Ø 20 mm)
  (`manifold.geo`); inlet and three outlets are capped, water at 0.5 m/s, draft quality.

The STL files were made with Gmsh 4.14 from the `.geo` scripts in the runtime image
(`gmsh -2 <name>.geo -format stl -bin`) and imported with `geometry.import` (unit mm); the project
files come from the command API (`project.new`, `geometry.import`, `geometry.cap_opening`,
`scenario.*`).
