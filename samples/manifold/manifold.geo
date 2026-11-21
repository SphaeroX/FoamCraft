// SPDX-License-Identifier: CC0-1.0
// FoamCraft sample "Pipe manifold": a header pipe r = 20 mm along x (0..300 mm, closed at the
// far end) with three branches r = 10 mm along +y (to y = 120 mm) at x = 75, 150 and 225 mm.
// Open at the inlet (x = 0) and at the three branch ends. Own design for the internal flow sample.
SetFactory("OpenCASCADE");
Cylinder(1) = {0, 0, 0, 300, 0, 0, 20};
Cylinder(2) = {75, 0, 0, 0, 120, 0, 10};
Cylinder(3) = {150, 0, 0, 0, 120, 0, 10};
Cylinder(4) = {225, 0, 0, 0, 120, 0, 10};
BooleanUnion{ Volume{1}; Delete; }{ Volume{2, 3, 4}; Delete; }
ends() = Surface In BoundingBox{-1, -21, -21, 1, 21, 21};
ends() += Surface In BoundingBox{64, 119, -11, 86, 121, 11};
ends() += Surface In BoundingBox{139, 119, -11, 161, 121, 11};
ends() += Surface In BoundingBox{214, 119, -11, 236, 121, 11};
shell() = Surface{:};
shell() -= ends();
Physical Surface(1) = {shell()};
Mesh.MeshSizeMax = 6;
Mesh.MeshSizeFromCurvature = 20;
