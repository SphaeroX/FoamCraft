// SPDX-License-Identifier: CC0-1.0
// Housing 60 x 40 x 30 mm with 5 mm walls and two bores r = 4 mm through the walls at x = 0 and
// x = 60 (a closed solid with two passages into the cavity).
SetFactory("OpenCASCADE");
Box(1) = {0, 0, 0, 60, 40, 30};
Box(2) = {5, 5, 5, 50, 30, 20};
Cylinder(3) = {-1, 20, 15, 7, 0, 0, 4};
Cylinder(4) = {54, 20, 15, 7, 0, 0, 4};
BooleanDifference{ Volume{1}; Delete; }{ Volume{2, 3, 4}; Delete; }
Physical Surface(1) = Surface{:};
Mesh.MeshSizeMax = 2.5;
Mesh.MeshSizeFromCurvature = 36;
