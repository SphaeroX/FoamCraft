// SPDX-License-Identifier: CC0-1.0
// FoamCraft sample "Rounded body": a block 400 × 180 × 150 mm, front and side edges rounded
// (r = 40 mm), rear edges sharp (a clean separation line). Own design for the wind tunnel sample.
SetFactory("OpenCASCADE");
Box(1) = {0, -90, 0, 400, 180, 150};
front() = Curve In BoundingBox{-1, -91, -1, 1, 91, 151};
rear() = Curve In BoundingBox{399, -91, -1, 401, 91, 151};
all() = Curve{:};
side() = all();
side() -= front();
side() -= rear();
Fillet{1}{front(), side()}{40}
Physical Surface(1) = Surface{:};
Mesh.MeshSizeMax = 12;
Mesh.MeshSizeFromCurvature = 24;
