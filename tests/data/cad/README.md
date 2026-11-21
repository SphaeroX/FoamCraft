# CAD test files

Self-made with Gmsh 4.14 (Open CASCADE kernel) on 2026-11-21, released as CC0:

- `cube.step`, `cube.iges`: box from (0,0,0) to (20,20,20) mm.
- `pipe_flange.step`: pipe, outer radius 10 mm, inner radius 8 mm, length 50 mm, with a flange of
  radius 25 mm and 5 mm thickness at z = 0. Six faces, one volume.
- `*_step.msh`, `cube_iges.msh`: the Gmsh 4.1 meshes FoamCraft gets from these files
  (`gmsh <file> -2 -format msh41 -clcurv 24`, STEP with `-setstring Geometry.OCCTargetUnit M`).
  STEP meshes are in metres, the IGES mesh in millimetres.
