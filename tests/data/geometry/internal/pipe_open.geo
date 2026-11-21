// SPDX-License-Identifier: CC0-1.0
// Open pipe: lateral surface of a cylinder, r = 10 mm, length 50 mm along x, both ends open.
SetFactory("OpenCASCADE");
Cylinder(1) = {0, 0, 0, 50, 0, 0, 10};
ends() = Surface In BoundingBox{-1, -11, -11, 1, 11, 11};
ends() += Surface In BoundingBox{49, -11, -11, 51, 11, 11};
shell() = Surface{:};
shell() -= ends();
Physical Surface(1) = {shell()};
Mesh.MeshSizeMax = 2;
Mesh.MeshSizeFromCurvature = 36;
