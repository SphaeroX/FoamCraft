// SPDX-License-Identifier: CC0-1.0
// Thick-walled tube: outer r = 10 mm, inner r = 7 mm, length 40 mm along x (closed solid).
SetFactory("OpenCASCADE");
Cylinder(1) = {0, 0, 0, 40, 0, 0, 10};
Cylinder(2) = {-1, 0, 0, 42, 0, 0, 7};
BooleanDifference{ Volume{1}; Delete; }{ Volume{2}; Delete; }
Physical Surface(1) = Surface{:};
Mesh.MeshSizeMax = 2;
Mesh.MeshSizeFromCurvature = 36;
