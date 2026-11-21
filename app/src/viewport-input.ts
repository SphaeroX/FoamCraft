// SPDX-License-Identifier: GPL-3.0-or-later
// Maps mouse input over the free viewport area to camera actions for the native renderer.

export type CameraAction =
  | { kind: "orbit"; yaw: number; pitch: number }
  | { kind: "pan"; dx: number; dy: number }
  | { kind: "zoom"; factor: number }
  | { kind: "fit" }
  | { kind: "resize" }
  /** Turns to a standard view (`front`, `top_front_right`, `default`, ...) with a short move. */
  | { kind: "view"; name: string };

/** Yaw and pitch of the camera in radians, as reported by the native view. */
export interface CameraPose {
  yaw: number;
  pitch: number;
}

/** Radians of rotation per pixel of drag. */
const ORBIT_SPEED = 0.01;

/** `button` follows MouseEvent.button: 0 left, 1 middle, 2 right. Pan is in view heights. */
export function dragToCamera(
  button: number,
  dx: number,
  dy: number,
  viewHeight: number,
): CameraAction {
  if (button === 0) {
    return { kind: "orbit", yaw: -dx * ORBIT_SPEED, pitch: -dy * ORBIT_SPEED };
  }
  const h = Math.max(viewHeight, 1);
  return { kind: "pan", dx: dx / h, dy: dy / h };
}

/** One wheel notch (100) changes the distance by 15 %. */
export function wheelToZoom(deltaY: number): number {
  return Math.pow(1.15, deltaY / 100);
}
