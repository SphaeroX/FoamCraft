// SPDX-License-Identifier: CC0-1.0
// Open annular gap: outer wall r = 12 mm and inner wall r = 6 mm, length 40 mm along x.
SetFactory("OpenCASCADE");
Cylinder(1) = {0, 0, 0, 40, 0, 0, 12};
Cylinder(2) = {0, 0, 0, 40, 0, 0, 6};
ends() = Surface In BoundingBox{-1, -13, -13, 1, 13, 13};
ends() += Surface In BoundingBox{39, -13, -13, 41, 13, 13};
shell() = Surface{:};
shell() -= ends();
Physical Surface(1) = {shell()};
Mesh.MeshSizeMax = 2;
Mesh.MeshSizeFromCurvature = 36;
