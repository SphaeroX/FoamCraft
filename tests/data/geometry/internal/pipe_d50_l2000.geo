// SPDX-License-Identifier: CC0-1.0
// Validation pipe (turbulent): open lateral surface, D = 50 mm, length 2000 mm along x.
SetFactory("OpenCASCADE");
Cylinder(1) = {0, 0, 0, 2000, 0, 0, 25};
ends() = Surface In BoundingBox{-1, -26, -26, 1, 26, 26};
ends() += Surface In BoundingBox{1999, -26, -26, 2001, 26, 26};
shell() = Surface{:};
shell() -= ends();
Physical Surface(1) = {shell()};
Mesh.MeshSizeMax = 10;
Mesh.MeshSizeFromCurvature = 36;
