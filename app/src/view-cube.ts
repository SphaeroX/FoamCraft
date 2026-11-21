// SPDX-License-Identifier: GPL-3.0-or-later
// Maths of the view cube: the same orbit camera as the native view (world Z up, metres), mapped
// to CSS screen axes (x right, y down, z towards the viewer).

export type Vec3 = [number, number, number];

/** Faces, edges and corners of the cube, as the view names of `view.render`. */
export const CUBE_VIEWS = [
  "front",
  "back",
  "left",
  "right",
  "top",
  "bottom",
  "front_left",
  "front_right",
  "back_left",
  "back_right",
  "top_front",
  "top_back",
  "top_left",
  "top_right",
  "bottom_front",
  "bottom_back",
  "bottom_left",
  "bottom_right",
  "top_front_left",
  "top_front_right",
  "top_back_left",
  "top_back_right",
  "bottom_front_left",
  "bottom_front_right",
  "bottom_back_left",
  "bottom_back_right",
] as const;

export type CubeView = (typeof CUBE_VIEWS)[number];

const AXES: Record<string, Vec3> = {
  front: [-1, 0, 0],
  back: [1, 0, 0],
  left: [0, 1, 0],
  right: [0, -1, 0],
  top: [0, 0, 1],
  bottom: [0, 0, -1],
};

/** Direction from the model to the eye, not normalised (a face, edge midpoint or corner). */
export function viewNormal(name: CubeView): Vec3 {
  const n: Vec3 = [0, 0, 0];
  for (const part of name.split("_")) {
    const a = AXES[part];
    n[0] += a[0];
    n[1] += a[1];
    n[2] += a[2];
  }
  return n;
}

/** Same as the camera's maximum pitch: never straight up or down. */
const MAX_PITCH = Math.PI / 2 - 1e-3;

/** Yaw and pitch (radians) of a cube view, as the native camera computes them. */
export function namedView(name: CubeView): [number, number] {
  const [x, y, z] = viewNormal(name);
  const h = Math.hypot(x, y);
  const yaw = h < 1e-9 ? Math.PI : Math.atan2(y, x);
  const pitch = Math.max(-MAX_PITCH, Math.min(MAX_PITCH, Math.atan2(z, h)));
  return [yaw, pitch];
}

function cross(a: Vec3, b: Vec3): Vec3 {
  return [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]];
}

function normalize(a: Vec3): Vec3 {
  const l = Math.hypot(...a) || 1;
  return [a[0] / l, a[1] / l, a[2] / l];
}

/** Rows: screen right, screen down, towards the viewer, each as a world vector. */
export type Basis = [Vec3, Vec3, Vec3];

/** World to CSS screen axes for the orbit camera with this yaw and pitch. */
export function cubeMatrix(yaw: number, pitch: number): Basis {
  const offset: Vec3 = [
    Math.cos(pitch) * Math.cos(yaw),
    Math.cos(pitch) * Math.sin(yaw),
    Math.sin(pitch),
  ];
  const forward: Vec3 = [-offset[0], -offset[1], -offset[2]];
  const right = normalize(cross(forward, [0, 0, 1]));
  const up = cross(right, forward);
  return [right, [-up[0], -up[1], -up[2]], offset];
}

/** A world vector in CSS screen axes. */
export function toScreen(m: Basis, v: Vec3): Vec3 {
  const dot = (a: Vec3) => a[0] * v[0] + a[1] * v[1] + a[2] * v[2];
  return [dot(m[0]), dot(m[1]), dot(m[2])];
}

/** CSS `matrix3d` of a basis (rows are the images of world x, y, z in columns). */
export function basisToCss(m: Basis, translate: Vec3 = [0, 0, 0]): string {
  // Column j of the 4x4 matrix is the screen image of world axis j.
  const col = (j: number) => [m[0][j], m[1][j], m[2][j], 0];
  const values = [...col(0), ...col(1), ...col(2), ...translate, 1];
  return `matrix3d(${values.map((v) => +v.toFixed(6)).join(",")})`;
}

/**
 * Placement of a cube element in world space: its own x (text right), y (text down) and z (out of
 * the cube) as world vectors, so labels read upright when the element faces the viewer.
 */
export function elementBasis(name: CubeView): Basis {
  const n = normalize(viewNormal(name));
  const horizontal = Math.hypot(n[0], n[1]) > 1e-6;
  const right = horizontal ? normalize(cross([0, 0, 1], n)) : ([0, -1, 0] as Vec3);
  const down = cross(right, n);
  return [right, down, n];
}

/** CSS transform placing an element of the cube (world -> element local is the transpose). */
export function elementCss(name: CubeView, half: number): string {
  const [r, d, n] = elementBasis(name);
  const c = viewNormal(name);
  const offset: Vec3 = [c[0] * half, c[1] * half, c[2] * half];
  // Columns are the element axes in world space; the cube body maps world to screen.
  const values = [...r, 0, ...d, 0, ...n, 0, ...offset, 1];
  return `matrix3d(${values.map((v) => +v.toFixed(6)).join(",")})`;
}

/** Kind of an element: a face, an edge or a corner. */
export function elementKind(name: CubeView): "face" | "edge" | "corner" {
  const parts = name.split("_").length;
  return parts === 1 ? "face" : parts === 2 ? "edge" : "corner";
}

/** For an edge: true when it runs vertically (along world Z). */
export function isVerticalEdge(name: CubeView): boolean {
  return elementKind(name) === "edge" && viewNormal(name)[2] === 0;
}
