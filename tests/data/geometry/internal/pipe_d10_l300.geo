// SPDX-License-Identifier: CC0-1.0
// Validation pipe (laminar): open lateral surface, D = 10 mm, length 300 mm along x.
SetFactory("OpenCASCADE");
Cylinder(1) = {0, 0, 0, 300, 0, 0, 5};
ends() = Surface In BoundingBox{-1, -6, -6, 1, 6, 6};
ends() += Surface In BoundingBox{299, -6, -6, 301, 6, 6};
shell() = Surface{:};
shell() -= ends();
Physical Surface(1) = {shell()};
Mesh.MeshSizeMax = 2;
Mesh.MeshSizeFromCurvature = 36;
