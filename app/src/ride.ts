// SPDX-License-Identifier: GPL-3.0-or-later
// The ride along one particle (Task_32): what `ride.start` and `ride.telemetry` answer, and the
// readings at a screen time. The app sends only the screen time each frame; the interface
// interpolates the readings itself.

export interface RideSpec {
  start: [number, number, number];
  /** Counts the starts of rides. */
  run?: number;
  mode: "calm" | "coaster";
  view: "first_person" | "chase" | "overview";
  speed: number;
  guide: boolean;
}

export interface RideEvent {
  id: string;
  code: string;
  s: number;
  t: number;
  value: number;
  /** A second number: the angle of an impact, degrees. */
  extra?: number;
}

export interface RideInfo {
  ride: RideSpec;
  job: string;
  duration_s: number;
  start_t: number;
  length_m: number;
  flow_time_s: number;
  reference_speed_m_s: number;
  density_kg_m3: number;
  model_length_m: number;
  ends: [string, string];
  /** t, s, speed, cp, p, d, accel, time_factor */
  columns: string[];
  rows: number[][];
  events: RideEvent[];
}

/** Clock of the ride, sent by the app with every frame. */
export interface RideClock {
  t: number;
  duration: number;
  playing: boolean;
}

/** Playback actions of the window. */
export type RideAction =
  { kind: "play" } | { kind: "pause" } | { kind: "seek"; t: number } | { kind: "state" };

export interface Readings {
  speed: number;
  cp: number;
  p: number;
  d: number;
  accel: number;
  factor: number;
  /** Heavy particles: speed against the flow, m/s. */
  slip: number;
}

/** Seconds an explaining sentence stays. */
export const SENTENCE_SECONDS = 2.5;

/** Readings at screen time `t`, linear between the rows. */
export function readingsAt(info: RideInfo, t: number): Readings | null {
  const rows = info.rows;
  if (rows.length === 0) return null;
  let i = 0;
  while (i + 1 < rows.length && rows[i + 1][0] <= t) i++;
  const a = rows[i];
  const b = rows[Math.min(i + 1, rows.length - 1)];
  const span = b[0] - a[0];
  const f = span > 0 ? Math.min(1, Math.max(0, (t - a[0]) / span)) : 0;
  const at = (k: number) => (a[k] ?? 0) + ((b[k] ?? 0) - (a[k] ?? 0)) * f;
  return {
    speed: at(2),
    cp: at(3),
    p: at(4),
    d: at(5),
    accel: at(6),
    factor: at(7),
    slip: at(8),
  };
}

/** The event whose sentence shows at `t` (the latest that started less than 2.5 s ago). */
export function eventAt(info: RideInfo, t: number): RideEvent | null {
  let found: RideEvent | null = null;
  for (const e of info.events) {
    if (e.t <= t && t < e.t + SENTENCE_SECONDS) found = e;
  }
  return found;
}

/** A column of the readings over the whole ride, scaled to 0..1 (for the small curves). */
export function curve(info: RideInfo, column: number): { t: number; v: number }[] {
  const values = info.rows.map((r) => r[column]);
  const lo = Math.min(...values);
  const hi = Math.max(...values);
  const span = hi - lo || 1;
  return info.rows.map((r) => ({ t: r[0], v: (r[column] - lo) / span }));
}
