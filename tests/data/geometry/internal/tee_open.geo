// SPDX-License-Identifier: CC0-1.0
// Open T-piece: main pipe r = 8 mm along x (0..60), branch r = 6 mm along z up to z = 30.
SetFactory("OpenCASCADE");
Cylinder(1) = {0, 0, 0, 60, 0, 0, 8};
Cylinder(2) = {30, 0, 0, 0, 0, 30, 6};
BooleanUnion{ Volume{1}; Delete; }{ Volume{2}; Delete; }
ends() = Surface In BoundingBox{-1, -9, -9, 1, 9, 9};
ends() += Surface In BoundingBox{59, -9, -9, 61, 9, 9};
ends() += Surface In BoundingBox{23, -7, 29, 37, 7, 31};
shell() = Surface{:};
shell() -= ends();
Physical Surface(1) = {shell()};
Mesh.MeshSizeMax = 2;
Mesh.MeshSizeFromCurvature = 36;
