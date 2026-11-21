// SPDX-License-Identifier: CC0-1.0
// Open pipe r = 10 mm along x whose far end is cut by a cylinder r = 12 mm along z at x = 60:
// that opening is a saddle curve, not flat.
SetFactory("OpenCASCADE");
Cylinder(1) = {0, 0, 0, 60, 0, 0, 10};
Cylinder(2) = {60, 0, -20, 0, 0, 40, 12};
BooleanDifference{ Volume{1}; Delete; }{ Volume{2}; Delete; }
ends() = Surface In BoundingBox{-1, -11, -11, 1, 11, 11};
ends() += Surface In BoundingBox{47, -11, -11, 61, 11, 11};
shell() = Surface{:};
shell() -= ends();
Physical Surface(1) = {shell()};
Mesh.MeshSizeMax = 2;
Mesh.MeshSizeFromCurvature = 36;
