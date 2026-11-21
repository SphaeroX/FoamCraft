// SPDX-License-Identifier: GPL-3.0-or-later
// Display conversion only: the command API always uses SI units (metres).
export type DisplayUnits = "metric" | "imperial";

export function lengthUnit(units: DisplayUnits): string {
  return units === "metric" ? "mm" : "in";
}

export function toDisplayLength(metres: number, units: DisplayUnits): number {
  return units === "metric" ? metres * 1000 : metres / 0.0254;
}

export function fromDisplayLength(value: number, units: DisplayUnits): number {
  return units === "metric" ? value / 1000 : value * 0.0254;
}

/** Rounds for display without trailing noise (for example 20.000000001 -> 20). */
export function formatNumber(value: number): string {
  return String(Number(value.toPrecision(6)));
}

export function speedUnit(units: DisplayUnits): string {
  return units === "metric" ? "km/h" : "mph";
}

export function toDisplaySpeed(metresPerSecond: number, units: DisplayUnits): number {
  return units === "metric" ? metresPerSecond * 3.6 : (metresPerSecond * 3600) / 1609.344;
}

export function fromDisplaySpeed(value: number, units: DisplayUnits): number {
  return units === "metric" ? value / 3.6 : (value * 1609.344) / 3600;
}
