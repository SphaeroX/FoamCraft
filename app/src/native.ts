// SPDX-License-Identifier: GPL-3.0-or-later
// Bridge to the Rust side. Outside Tauri (tests, plain browser) calls are no-ops.
import { invoke, isTauri } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { HandleValue } from "./components/MoveTools";
import type { RideAction, RideClock } from "./ride";
import type { CameraAction, CameraPose } from "./viewport-input";

export interface AppInfo {
  version: string;
}

/** Moves the camera; returns its yaw and pitch afterwards (null without a native view). */
export async function sendCamera(action: CameraAction): Promise<CameraPose | null> {
  if (!isTauri()) return null;
  return invoke<CameraPose | null>("viewport_camera", { action });
}

/** Camera updates during a view change, so the view cube turns along. */
export async function onCameraMoved(handler: (pose: CameraPose) => void): Promise<() => void> {
  if (!isTauri()) return () => {};
  return listen<CameraPose>("viewport://camera", (e) => handler(e.payload));
}

/** Playback of the ride in the window; its clock afterwards (null without a ride). */
export async function rideControl(action: RideAction): Promise<RideClock | null> {
  if (!isTauri()) return null;
  return invoke<RideClock | null>("viewport_ride", { action });
}

/** The clock of the ride, every frame while it plays. */
export function onRideClock(handler: (clock: RideClock) => void): () => void {
  if (!isTauri()) return () => {};
  let stop: (() => void) | null = null;
  let ended = false;
  listen<RideClock>("viewport://ride", (e) => handler(e.payload))
    .then((unlisten) => (ended ? unlisten() : (stop = unlisten)))
    .catch((err: unknown) => console.error(err));
  return () => {
    ended = true;
    stop?.();
  };
}

/** Start point of a ride under a point of the view (CSS pixels): a particle or the model. */
export async function ridePointAt(x: number, y: number): Promise<[number, number, number] | null> {
  if (!isTauri()) return null;
  return invoke("viewport_ride_point", { x: physical(x), y: physical(y) });
}

export async function appInfo(): Promise<AppInfo> {
  if (!isTauri()) return { version: "dev" };
  return invoke<AppInfo>("app_info");
}

/** Path of the command line program next to the app (for the MCP configuration). */
export async function cliPath(): Promise<string> {
  if (!isTauri()) return "foamcraft";
  return invoke<string>("cli_path");
}

/** Shows the models of the open project in the 3D view and fits the camera. */
export async function syncViewport(): Promise<void> {
  if (!isTauri()) return;
  await invoke("viewport_sync");
}

/** CSS pixels to the physical pixels of the native view. */
function physical(v: number): number {
  return v * (window.devicePixelRatio || 1);
}

/** Face key under a point of the window (CSS pixels), or null. */
export async function pickAt(x: number, y: number): Promise<string | null> {
  if (!isTauri()) return null;
  return invoke<string | null>("viewport_pick", { x: physical(x), y: physical(y) });
}

/** Face or cap under a point of the view and the 3D point there (metres). */
export async function pickPointAt(
  x: number,
  y: number,
): Promise<{ key: string; point: [number, number, number] | null } | null> {
  if (!isTauri()) return null;
  return invoke("viewport_pick_point", { x: physical(x), y: physical(y) });
}

/** Highlights the face under the mouse; returns its key. */
export async function hoverAt(x: number, y: number): Promise<string | null> {
  if (!isTauri()) return null;
  return invoke<string | null>("viewport_hover", { x: physical(x), y: physical(y) });
}

/** Widths covered by the side panels (CSS pixels); the split view of two runs uses the rest. */
export async function setViewInsets(left: number, right: number): Promise<void> {
  if (!isTauri()) return;
  await invoke("viewport_insets", { left: physical(left), right: physical(right) });
}

/** Highlights the selected faces in the 3D view. */
export async function showSelection(keys: string[]): Promise<void> {
  if (!isTauri()) return;
  await invoke("viewport_set_selection", { keys });
}

/** Shows the move and rotate handles of a model at a pivot (metres); null hides them. */
export async function setHandles(
  model: string | null,
  pivot: [number, number, number] | null,
): Promise<void> {
  if (!isTauri()) return;
  await invoke("viewport_set_handles", { model, pivot });
}

/** Starts dragging a handle at a point of the window (CSS pixels). */
export async function handleBegin(key: string, x: number, y: number): Promise<boolean> {
  if (!isTauri()) return false;
  return invoke<boolean>("viewport_handle_begin", { key, x: physical(x), y: physical(y) });
}

/** Follows the mouse while dragging; the value to show (metres or degrees). */
export async function handleDrag(x: number, y: number, free: boolean): Promise<HandleValue | null> {
  if (!isTauri()) return null;
  return invoke<HandleValue | null>("viewport_handle_drag", {
    x: physical(x),
    y: physical(y),
    free,
  });
}

/** Ends a drag: the command that applies it (one undo step), or null when nothing moved. */
export async function handleEnd(): Promise<{
  command: string;
  params: Record<string, unknown>;
} | null> {
  if (!isTauri()) return null;
  return invoke("viewport_handle_end");
}

/** Cancels a drag; the model goes back. */
export async function handleCancel(): Promise<void> {
  if (!isTauri()) return;
  await invoke("viewport_handle_cancel");
}

/** Shows or hides the wind arrow handle in the view (Scenario step of a wind tunnel). */
export async function showWindHandle(show: boolean): Promise<void> {
  if (!isTauri()) return;
  await invoke("viewport_show_wind", { show });
}
