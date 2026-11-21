# Test geometry

Self-made test files, released as CC0. Generated on 2026-11-21 for Task_05:

- `cube_ascii.stl`, `cube_binary.stl`, `cube.obj`, `cube.3mf`: cube from (0,0,0) to (20,20,20) in
  millimetres, 12 triangles (the OBJ uses 6 quads), outward normals.
- `cube_open.stl`: the cube without its last triangle (one boundary loop).
- `cube_flipped_face.stl`: the cube with the two triangles of the top face reversed.
- `broken.stl`: a truncated ASCII STL with an invalid number.

`internal/` (Task_12): bodies for internal flow made with Gmsh 4.14 from the `.geo` scripts next to
them (`gmsh -2 <name>.geo -format stl -bin`), CC0, in millimetres: open pipe, open T-piece, housing
with 5 mm walls and two bores, open annular gap, pipe with a saddle-shaped (not flat) end, thick-walled
tube.

- `pipe_d10_l300.stl`, `pipe_d50_l2000.stl` (Task_13): open validation pipes (D = 10 mm, 300 mm; D = 50 mm,
  2000 mm) from the `.geo` scripts of the same name.
