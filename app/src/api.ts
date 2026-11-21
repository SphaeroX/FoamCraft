// SPDX-License-Identifier: GPL-3.0-or-later
// Client of the FoamCraft command API. The interface never implements actions itself; it calls
// commands by name and listens to state events.
import { invoke, isTauri } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { Language } from "./i18n/i18n";

export interface ProjectInfo {
  path: string;
  name: string;
  schema_version: number;
  display_units: "metric" | "imperial";
  runs: string[];
}

export interface RecentProject {
  path: string;
  name: string;
}

export interface LocalizedMessage {
  text: string;
  fix: string;
}

/** Error of a command: stable code plus English and German message. */
export interface ApiError {
  code: string;
  detail: string;
  en: LocalizedMessage;
  de: LocalizedMessage;
}

export interface ApiEvent {
  name: string;
  payload: unknown;
}

export interface Api {
  call<T = unknown>(name: string, params?: Record<string, unknown>): Promise<T>;
}

export function isApiError(value: unknown): value is ApiError {
  return (
    typeof value === "object" && value !== null && "code" in value && "en" in value && "de" in value
  );
}

/** Message for any thrown value, in the interface language. */
export function errorMessage(err: unknown, lang: Language): string {
  if (isApiError(err)) {
    const m = err[lang];
    return m.fix ? `${m.text} ${m.fix}` : m.text;
  }
  return String(err);
}

export const tauriApi: Api = {
  call<T>(name: string, params: Record<string, unknown> = {}) {
    if (!isTauri()) return Promise.reject(new Error("FoamCraft API needs the desktop app"));
    return invoke<T>("api_call", { name, params });
  },
};

/** Subscribes to state events; returns an unsubscribe function. */
export async function onApiEvent(handler: (event: ApiEvent) => void): Promise<() => void> {
  if (!isTauri()) return () => {};
  return listen<ApiEvent>("foamcraft://event", (e) => handler(e.payload));
}

/** Synchronous form of onApiEvent for effects: returns the unsubscribe function at once. */
export function subscribeApiEvents(handler: (event: ApiEvent) => void): () => void {
  let stop: (() => void) | undefined;
  let ended = false;
  onApiEvent(handler)
    .then((unlisten) => (ended ? unlisten() : (stop = unlisten)))
    .catch((err: unknown) => console.error(err));
  return () => {
    ended = true;
    stop?.();
  };
}

export interface LocalizedFinding {
  code: string;
  severity: "info" | "warning";
  count: number;
  en: LocalizedMessage;
  de: LocalizedMessage;
}

/** Result of geometry.info and of the commands that change a model. */
export interface GeometryInfo {
  id: string;
  name: string;
  file: string;
  unit: string;
  scale: number;
  rotation_deg: [number, number, number];
  translation: [number, number, number];
  fix_normals: boolean;
  suggested_unit: string;
  size_m: [number, number, number];
  bounds_m: { min: [number, number, number]; max: [number, number, number] };
  diagnosis: { triangles: number; watertight: boolean; boundary_loops: number };
  messages: LocalizedFinding[];
}

export interface SurfaceInfo {
  key: string;
  model: string;
  model_name: string;
  part: string;
  name: string | null;
  groups: string[];
  triangles: number;
  area_m2: number;
  normal: number[];
  centroid_m: number[];
}

export interface SurfaceGroup {
  name: string;
  keys: string[];
}
