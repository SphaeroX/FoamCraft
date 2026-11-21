// SPDX-License-Identifier: GPL-3.0-or-later
// Bridge to the Rust side. Outside Tauri (tests, plain browser) calls are no-ops.
import { invoke, isTauri } from "@tauri-apps/api/core";
import type { CameraAction } from "./viewport-input";

export interface AppInfo {
  version: string;
}

export async function sendCamera(action: CameraAction): Promise<void> {
  if (!isTauri()) return;
  await invoke("viewport_camera", { action });
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
