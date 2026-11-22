// SPDX-License-Identifier: GPL-3.0-or-later
// The state of the run of a project (meshing, solving, result), kept outside the "Run" step so it
// stays complete while the user looks at another step. It follows the events of the command API.
import { useSyncExternalStore } from "react";
import { errorMessage, isApiError, type Api, type ApiEvent } from "./api";
import type { Language } from "./i18n/i18n";

export type RunPhase = "idle" | "running" | "finished" | "failed" | "cancelled";

export interface RunResult {
  kind?: "internal_flow";
  job_id: string;
  reason: string;
  iterations: number;
  coefficients?: { cd: number; cl: number };
  forces?: { drag_n: number; lift_n: number };
  pressure_loss_pa?: number;
  static_pressure_loss_pa?: number;
  mass_balance_error?: number;
  openings?: { key: string; role: string; flow_m3_s: number; share: number | null }[];
  hints: string[];
  timings_s: Record<string, number>;
}

/** Progress of the meshing of the run. */
export interface MeshProgress {
  /** 0 to 1. */
  fraction: number;
  /** snappyHexMesh phase: refine, snap, layers; null outside of it. */
  phase: string | null;
  remaining_s: number | null;
  /** Attempts beyond the first use simpler settings (fewer layers, coarser). */
  attempt: number;
}

export interface MonitorState {
  phase: RunPhase;
  jobId: string | null;
  /** The run only builds the mesh. */
  meshOnly: boolean;
  stage: string | null;
  seen: string[];
  mesh: MeshProgress | null;
  /** Counts finished meshes, so views of the mesh reload. */
  meshRevision: number;
  /** The mesh of this run is ready and the solver is being started. */
  meshReady: boolean;
  residuals: Record<string, [number, number][]>;
  coeffs: { cd: [number, number][]; cl: [number, number][] };
  loss: [number, number][];
  iteration: { n: number; max: number } | null;
  remaining: number | null;
  result: RunResult | null;
  /** Message code of the failure (or a ready text when `failureText` is set). */
  failure: string | null;
  /** The failure is a text, not a code (the start command itself failed). */
  failureIsText: boolean;
}

export const initialMonitor: MonitorState = {
  phase: "idle",
  jobId: null,
  meshOnly: false,
  stage: null,
  seen: [],
  mesh: null,
  meshRevision: 0,
  meshReady: false,
  residuals: {},
  coeffs: { cd: [], cl: [] },
  loss: [],
  iteration: null,
  remaining: null,
  result: null,
  failure: null,
  failureIsText: false,
};

const asRecord = (v: unknown) =>
  typeof v === "object" && v !== null ? (v as Record<string, unknown>) : {};

const withStage = (state: MonitorState, stage: string): MonitorState => ({
  ...state,
  stage,
  seen: state.seen.includes(stage) ? state.seen : [...state.seen, stage],
});

/** The state after an event of the command API (pure; results are loaded by the monitor). */
export function reduceMonitor(state: MonitorState, event: ApiEvent): MonitorState {
  const p = asRecord(event.payload);
  // Events of other runs than the followed one are not ours.
  const run = typeof p.run_id === "string" ? p.run_id : p.job_id;
  if (state.jobId !== null && typeof run === "string" && run !== state.jobId) return state;
  switch (event.name) {
    case "run.stage":
      return { ...withStage(state, String(p.stage)), mesh: null };
    case "mesh.stage": {
      const attempt = typeof p.attempt === "number" ? p.attempt : 1;
      const next = withStage(state, String(p.stage));
      // A further attempt starts the meshing stages again.
      const restarted = attempt > (state.mesh?.attempt ?? 1) || !state.mesh;
      return {
        ...next,
        seen: restarted && p.stage === "background" ? ["background"] : next.seen,
        mesh: restarted
          ? { fraction: 0, phase: null, remaining_s: null, attempt }
          : (state.mesh ?? { fraction: 0, phase: null, remaining_s: null, attempt }),
      };
    }
    case "mesh.progress":
      return {
        ...state,
        mesh: {
          fraction: Number(p.fraction) || 0,
          phase: typeof p.phase === "string" ? p.phase : null,
          remaining_s: typeof p.remaining_s === "number" ? p.remaining_s : null,
          attempt: typeof p.attempt === "number" ? p.attempt : (state.mesh?.attempt ?? 1),
        },
      };
    case "mesh.ready":
      return {
        ...state,
        mesh: null,
        meshRevision: state.meshRevision + 1,
        meshReady: state.phase === "running",
        phase: state.meshOnly && state.phase === "running" ? "finished" : state.phase,
      };
    case "run.progress": {
      const i = Number(p.iteration);
      const residuals = { ...state.residuals };
      for (const [field, value] of Object.entries(asRecord(p.residuals))) {
        residuals[field] = [...(residuals[field] ?? []), [i, Number(value)]];
      }
      return { ...state, residuals };
    }
    case "run.coefficients": {
      const i = Number(p.iteration);
      return {
        ...state,
        coeffs: {
          cd: [...state.coeffs.cd, [i, Number(p.cd)]],
          cl: [...state.coeffs.cl, [i, Number(p.cl)]],
        },
        iteration: { n: i, max: Number(p.max_iterations) },
        remaining: typeof p.remaining_s === "number" ? p.remaining_s : null,
      };
    }
    case "run.pressure_loss": {
      const i = Number(p.iteration);
      return {
        ...state,
        loss: [...state.loss, [i, Number(p.pressure_loss_pa)]],
        iteration: { n: i, max: Number(p.max_iterations) },
        remaining: typeof p.remaining_s === "number" ? p.remaining_s : null,
      };
    }
    case "run.failed": {
      const code = String(p.code);
      if (code === "run.cancelled") return { ...state, phase: "cancelled", mesh: null };
      return { ...state, phase: "failed", failure: code, failureIsText: false, mesh: null };
    }
    default:
      return state;
  }
}

type Subscribe = (handler: (event: ApiEvent) => void) => () => void;

/** Follows the run of one project. Create one per open project and keep it while it is open. */
export class RunMonitor {
  private state: MonitorState = initialMonitor;
  private listeners = new Set<() => void>();
  private resultListeners = new Set<(result: RunResult) => void>();
  private stop: (() => void) | null = null;
  /** Language for the message of a start command that fails. */
  language: Language = "en";

  constructor(
    private api: Api,
    private subscribeEvents: Subscribe,
  ) {}

  getState = (): MonitorState => this.state;

  subscribe = (listener: () => void): (() => void) => {
    this.listeners.add(listener);
    return () => this.listeners.delete(listener);
  };

  /** Called when a run this monitor started or attached to has its result (not a mesh-only run). */
  onResult = (listener: (result: RunResult) => void): (() => void) => {
    this.resultListeners.add(listener);
    return () => this.resultListeners.delete(listener);
  };

  private set(next: MonitorState) {
    if (next === this.state) return;
    this.state = next;
    this.listeners.forEach((l) => l());
  }

  /** Starts following the events; reconnects to a job that still runs (after a restart). */
  connect(): void {
    if (this.stop) return;
    this.stop = this.subscribeEvents((event) => this.onEvent(event));
    this.api
      .call<{ job_id: string; case: string; state: { state: string; stage?: string } }[]>(
        "run.active",
      )
      .then((jobs) => {
        if (this.state.phase === "running") return;
        const job = jobs.find((j) => j.case.endsWith("_solve")) ?? jobs[0];
        if (!job) return;
        this.set({
          ...initialMonitor,
          meshRevision: this.state.meshRevision,
          phase: "running",
          jobId: job.job_id,
          stage: job.state.stage ?? null,
          seen: job.state.stage ? [job.state.stage] : [],
        });
        return this.api.call("run.attach", { job_id: job.job_id }).catch((e: unknown) => {
          // The run cannot be followed any more (its meshing was lost with the app).
          const code = isApiError(e) ? e.code : null;
          this.set({
            ...this.state,
            phase: "failed",
            failure: code ?? errorMessage(e, this.language),
            failureIsText: code === null,
          });
        });
      })
      .catch(() => {});
  }

  dispose(): void {
    this.stop?.();
    this.stop = null;
  }

  private onEvent(event: ApiEvent) {
    if (event.name === "run.result") {
      const id = String(asRecord(event.payload).job_id);
      if (this.state.jobId !== null && id !== this.state.jobId) return;
      // Only a run this monitor follows tells its listeners (not each run of a queue).
      const followed = this.state.phase === "running" && !this.state.meshOnly;
      this.api
        .call<RunResult>("run.result", { job_id: id })
        .then((result) => {
          this.set({ ...this.state, result, phase: "finished", mesh: null });
          if (followed) this.resultListeners.forEach((l) => l(result));
        })
        .catch((e: unknown) =>
          this.set({
            ...this.state,
            phase: "failed",
            failure: errorMessage(e, this.language),
            failureIsText: true,
          }),
        );
      return;
    }
    this.set(reduceMonitor(this.state, event));
  }

  /** Starts a run: meshing when needed, then solving. `meshOnly` stops after the mesh. */
  start(meshOnly = false): void {
    this.set({
      ...initialMonitor,
      meshRevision: this.state.meshRevision,
      phase: "running",
      meshOnly,
    });
    this.api
      .call<{ job_id: string }>("run.start", meshOnly ? { mesh_only: true } : {})
      .then((s) => {
        // Events of the run may have arrived before this answer; they stay.
        if (this.state.phase !== "idle") this.set({ ...this.state, jobId: s.job_id });
      })
      .catch((e: unknown) =>
        this.set({
          ...this.state,
          phase: "failed",
          failure: errorMessage(e, this.language),
          failureIsText: true,
        }),
      );
  }

  cancel(): void {
    const id = this.state.jobId;
    if (id) this.api.call("run.cancel", { job_id: id }).catch(() => {});
  }
}

/** The current state of a monitor; the component renders again when it changes. */
export function useRunMonitor(monitor: RunMonitor): MonitorState {
  return useSyncExternalStore(monitor.subscribe, monitor.getState);
}
