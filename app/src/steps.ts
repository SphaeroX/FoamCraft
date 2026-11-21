// SPDX-License-Identifier: GPL-3.0-or-later
// The four steps of a simulation and their state, from what the project already has.

export const STEPS = ["geometry", "scenario", "run", "results"] as const;
export type Step = (typeof STEPS)[number];

/** open: not yet possible; ready: can be done now; done; attention: done but worth a look. */
export type StepState = "open" | "ready" | "done" | "attention";

export interface StepInfo {
  state: StepState;
  /** Translation key of what is missing before the step is done. */
  missing?: string;
}

export interface ProjectFacts {
  models: number;
  /** Closed models; null when not known yet. */
  watertight: boolean | null;
  scenario: boolean;
  running: boolean;
  hasResult: boolean;
}

export function stepStates(f: ProjectFacts): Record<Step, StepInfo> {
  const geometry: StepInfo =
    f.models === 0
      ? { state: "open", missing: "missing.model" }
      : { state: f.watertight === false ? "attention" : "done" };
  const hasModel = f.models > 0;
  const scenario: StepInfo = !hasModel
    ? { state: "open", missing: "missing.model" }
    : f.scenario
      ? { state: "done" }
      : { state: "ready", missing: "missing.scenario" };
  const run: StepInfo = f.hasResult
    ? { state: "done" }
    : !hasModel || !f.scenario
      ? { state: "open", missing: hasModel ? "missing.scenario" : "missing.model" }
      : { state: "ready", missing: f.running ? "missing.running" : "missing.run" };
  const results: StepInfo = f.hasResult
    ? { state: "ready" }
    : { state: "open", missing: "missing.run" };
  return { geometry, scenario, run, results };
}

/** True when "Next" may leave `step`. */
export function canAdvance(states: Record<Step, StepInfo>, step: Step): boolean {
  const s = states[step].state;
  return (s === "done" || s === "attention") && STEPS.indexOf(step) < STEPS.length - 1;
}
