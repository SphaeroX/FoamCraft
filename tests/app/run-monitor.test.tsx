// SPDX-License-Identifier: GPL-3.0-or-later
// @vitest-environment jsdom
// One run monitor per project: meshing progress, a state that survives leaving the "Run" step,
// mesh-only runs and the console.
import { act, cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { Api, ApiEvent } from "../../app/src/api";
import { ConsoleDock } from "../../app/src/components/ConsoleDock";
import { RunPanel } from "../../app/src/components/RunPanel";
import { ScenarioPanel } from "../../app/src/components/ScenarioPanel";
import { LanguageProvider, STORAGE_KEY } from "../../app/src/i18n/i18n";
import {
  RunMonitor,
  initialMonitor,
  reduceMonitor,
  type MonitorState,
} from "../../app/src/run-monitor";

function setup(overrides: Record<string, unknown> = {}) {
  let handler: ((e: ApiEvent) => void) | null = null;
  const subscribe = (h: (e: ApiEvent) => void) => {
    handler = h;
    return () => (handler = null);
  };
  let meshed = false;
  const call = vi.fn(async (name: string, params?: Record<string, unknown>) => {
    if (name in overrides) return overrides[name];
    switch (name) {
      case "scenario.info":
        return { budget: { cells: 500000, estimate_s: 600, iterations: 300 } };
      case "mesh.info":
        if (!meshed) throw { code: "mesh.none", detail: "", en: {}, de: {} };
        return { cells: 487123, verdict: "good", layers_target: 3, layers_achieved: 2.8 };
      case "run.active":
        return [];
      case "run.start":
        return { job_id: "run-1" };
      case "run.cancel":
        return {};
      case "run.result":
        return {
          job_id: "run-1",
          reason: "stable",
          iterations: 200,
          coefficients: { cd: 0.42, cl: 0.01 },
          forces: { drag_n: 3.1, lift_n: 0.1 },
          hints: [],
          timings_s: { solve: 120 },
        };
      case "run.log":
        return { lines: ["[mesh] Refinement phase", `[mesh] line for ${String(params?.job_id)}`] };
      case "error.describe":
        return {};
      default:
        throw new Error(name);
    }
  });
  const api = { call } as unknown as Api;
  const monitor = new RunMonitor(api, subscribe);
  monitor.connect();
  const emit = (name: string, payload: unknown) =>
    act(() => {
      if (name === "mesh.ready") meshed = true;
      handler?.({ name, payload });
    });
  return { api, call, monitor, emit };
}

const panel = (api: Api, monitor: RunMonitor) => (
  <LanguageProvider>
    <RunPanel api={api} subscribe={() => () => {}} monitor={monitor} />
  </LanguageProvider>
);

describe("run monitor", () => {
  beforeEach(() => localStorage.setItem(STORAGE_KEY, "en"));
  afterEach(cleanup);

  it("shows the meshing with phase, progress and remaining time, and can cancel it", async () => {
    const { api, call, monitor, emit } = setup();
    render(panel(api, monitor));
    fireEvent.click(await screen.findByRole("button", { name: "Start run" }));
    await waitFor(() => expect(call).toHaveBeenCalledWith("run.start", {}));
    // Until the first stage reports, the panel says what happens.
    expect(await screen.findByText("Preparing the run …")).toBeInTheDocument();
    emit("mesh.stage", { job_id: "mesh-1", run_id: "run-1", stage: "background", attempt: 1 });
    expect(screen.queryByText("Preparing the run …")).toBeNull();
    emit("mesh.stage", { job_id: "mesh-1", run_id: "run-1", stage: "mesh", attempt: 1 });
    emit("mesh.progress", {
      job_id: "mesh-1",
      run_id: "run-1",
      attempt: 1,
      stage: "mesh",
      phase: "snap",
      fraction: 0.62,
      remaining_s: 170,
    });
    const bar = screen.getByRole("progressbar", { name: "Meshing" });
    expect(bar).toHaveAttribute("aria-valuenow", "62");
    expect(screen.getByText(/Fitting the mesh to the surface/)).toBeInTheDocument();
    expect(screen.getByText(/about 3 min left/)).toBeInTheDocument();
    const timeline = screen.getByRole("list", { name: "Stages" });
    expect(within(timeline).getByText("Body-fitted mesh")).toHaveAttribute("aria-current", "step");
    fireEvent.click(screen.getByRole("button", { name: "Cancel" }));
    await waitFor(() => expect(call).toHaveBeenCalledWith("run.cancel", { job_id: "run-1" }));
    emit("run.failed", { job_id: "run-1", code: "run.cancelled", detail: "" });
    expect(await screen.findByText("Cancelled")).toBeInTheDocument();
    expect(screen.queryByRole("progressbar")).toBeNull();
  });

  it("keeps stages, curves and the result while another step is shown", async () => {
    const { api, monitor, emit } = setup();
    const view = render(panel(api, monitor));
    fireEvent.click(await screen.findByRole("button", { name: "Start run" }));
    await screen.findByRole("button", { name: "Cancel" });
    emit("mesh.stage", { job_id: "mesh-1", run_id: "run-1", stage: "mesh", attempt: 1 });
    emit("mesh.ready", { run_id: "run-1", job_id: "mesh-1", cells: 487123, verdict: "good" });
    emit("run.stage", { job_id: "run-1", stage: "solve" });
    emit("run.coefficients", {
      job_id: "run-1",
      iteration: 1,
      cd: 0.5,
      cl: 0,
      remaining_s: 100,
      max_iterations: 900,
    });
    // The user looks at the results step: the panel is gone, the run goes on.
    view.unmount();
    for (let i = 2; i <= 4; i++) {
      emit("run.coefficients", {
        job_id: "run-1",
        iteration: i,
        cd: 0.5 - 0.01 * i,
        cl: 0,
        remaining_s: 100 - 10 * i,
        max_iterations: 900,
      });
    }
    render(panel(api, monitor));
    const chart = await screen.findByRole("img", { name: /Cd 0[.,]46/ });
    expect(chart.querySelector("polyline")?.getAttribute("points")?.split(" ")).toHaveLength(4);
    const timeline = screen.getByRole("list", { name: "Stages" });
    expect(within(timeline).getByText("Body-fitted mesh")).toHaveClass("done");
    expect(within(timeline).getByText("Solve")).toHaveAttribute("aria-current", "step");
    expect(screen.getByText(/Iteration 4 of at most 900/)).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Cancel" })).toBeInTheDocument();
    // The mesh of the run is described once it is ready.
    expect(await screen.findByText(/487[,.]?123 cells/)).toBeInTheDocument();
    cleanup();
    emit("run.result", { job_id: "run-1" });
    render(panel(api, monitor));
    const card = await screen.findByRole("region", { name: "Result" });
    expect(within(card).getByText("0.42")).toBeInTheDocument();
    expect(screen.getByText("Finished")).toBeInTheDocument();
  });

  it("shows that the run goes on by itself after the mesh is ready", async () => {
    const { api, monitor, emit } = setup();
    render(panel(api, monitor));
    fireEvent.click(await screen.findByRole("button", { name: "Start run" }));
    // While it runs, the start button says so instead of inviting a click.
    expect(await screen.findByRole("button", { name: "Running …" })).toBeDisabled();
    emit("mesh.stage", { job_id: "mesh-1", run_id: "run-1", stage: "mesh", attempt: 1 });
    emit("mesh.ready", { run_id: "run-1", job_id: "mesh-1", cells: 487123, verdict: "good" });
    expect(
      screen.getByText("The mesh is ready; the solver starts by itself …"),
    ).toBeInTheDocument();
    emit("run.stage", { job_id: "run-1", stage: "solve" });
    emit("run.coefficients", {
      job_id: "run-1",
      iteration: 1,
      cd: 0.5,
      cl: 0,
      remaining_s: 100,
      max_iterations: 900,
    });
    expect(screen.queryByText(/the solver starts by itself/)).toBeNull();
  });

  it("builds only the mesh on request", async () => {
    const { api, call, monitor, emit } = setup();
    render(panel(api, monitor));
    fireEvent.click(await screen.findByText("Mesh"));
    fireEvent.click(await screen.findByRole("button", { name: "Mesh only" }));
    await waitFor(() => expect(call).toHaveBeenCalledWith("run.start", { mesh_only: true }));
    expect(screen.getByRole("button", { name: "Start run" })).toBeDisabled();
    emit("mesh.ready", { run_id: "run-1", job_id: "mesh-1", cells: 487123, verdict: "good" });
    expect(await screen.findByText("Finished")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Start run" })).toBeEnabled();
    expect(await screen.findByText(/Good/)).toBeInTheDocument();
  });

  it("tells when a started run has its result, but not for a mesh-only run", async () => {
    const { monitor, emit } = setup();
    const done = vi.fn();
    monitor.onResult(done);
    monitor.start(true);
    await waitFor(() => expect(monitor.getState().jobId).toBe("run-1"));
    emit("mesh.ready", { run_id: "run-1", job_id: "mesh-1", cells: 1, verdict: "good" });
    expect(done).not.toHaveBeenCalled();
    monitor.start();
    await waitFor(() => expect(monitor.getState().jobId).toBe("run-1"));
    emit("run.result", { job_id: "run-1" });
    await waitFor(() => expect(done).toHaveBeenCalledTimes(1));
    expect(done.mock.calls[0][0]).toMatchObject({ job_id: "run-1" });
  });

  it("names the active scenario on the start button when there are several", async () => {
    const { api, monitor } = setup({
      "scenario.list": {
        active: "scenario-2",
        scenarios: [
          { id: "scenario-1", name: "", runs: [] },
          { id: "scenario-2", name: "Fast", runs: [] },
        ],
      },
    });
    render(panel(api, monitor));
    expect(await screen.findByRole("button", { name: "Run “Fast”" })).toBeInTheDocument();
  });

  it("ignores events of other runs and starts the stages again for a further attempt", () => {
    let s: MonitorState = { ...initialMonitor, phase: "running", jobId: "run-1" };
    s = reduceMonitor(s, {
      name: "mesh.stage",
      payload: { job_id: "m1", run_id: "run-1", stage: "background", attempt: 1 },
    });
    s = reduceMonitor(s, {
      name: "mesh.stage",
      payload: { job_id: "m1", run_id: "run-1", stage: "mesh", attempt: 1 },
    });
    const other = reduceMonitor(s, {
      name: "mesh.stage",
      payload: { job_id: "m9", run_id: "run-9", stage: "check", attempt: 1 },
    });
    expect(other).toBe(s);
    s = reduceMonitor(s, {
      name: "mesh.stage",
      payload: { job_id: "m2", run_id: "run-1", stage: "background", attempt: 2 },
    });
    expect(s.seen).toEqual(["background"]);
    expect(s.mesh?.attempt).toBe(2);
    expect(s.mesh?.fraction).toBe(0);
  });
});

describe("console", () => {
  beforeEach(() => localStorage.setItem(STORAGE_KEY, "en"));
  afterEach(cleanup);

  it("stays out of the way until a run exists, then shows its status and log on demand", async () => {
    const { api, call, monitor, emit } = setup();
    const { container } = render(
      <LanguageProvider>
        <ConsoleDock api={api} monitor={monitor} />
      </LanguageProvider>,
    );
    expect(container).toBeEmptyDOMElement();
    act(() => monitor.start());
    await waitFor(() => expect(monitor.getState().jobId).toBe("run-1"));
    emit("mesh.stage", { job_id: "mesh-1", run_id: "run-1", stage: "mesh", attempt: 1 });
    emit("mesh.progress", {
      job_id: "mesh-1",
      run_id: "run-1",
      attempt: 1,
      stage: "mesh",
      phase: "refine",
      fraction: 0.2,
      remaining_s: null,
    });
    const toggle = screen.getByRole("button", { name: /Console/ });
    expect(toggle).toHaveAttribute("aria-expanded", "false");
    expect(screen.getByText(/Body-fitted mesh · 20 %/)).toBeInTheDocument();
    expect(call).not.toHaveBeenCalledWith("run.log", expect.anything());
    fireEvent.click(toggle);
    await waitFor(() =>
      expect(call).toHaveBeenCalledWith("run.log", { job_id: "run-1", lines: 400 }),
    );
    expect(await screen.findByText(/line for run-1/)).toBeInTheDocument();
    expect(toggle).toHaveAttribute("aria-expanded", "true");
    fireEvent.click(toggle);
    expect(screen.queryByText(/line for run-1/)).toBeNull();
  });
});

describe("scenario step", () => {
  beforeEach(() => localStorage.setItem(STORAGE_KEY, "en"));
  afterEach(cleanup);

  it("only defines the scenario: there is no mesh button", async () => {
    const call = vi.fn(async (name: string) => {
      if (name === "scenario.info")
        return {
          scenario: {
            kind: "wind_tunnel",
            speed_m_s: 20,
            direction_deg: 0,
            ground: "none",
            medium: { kind: "air", temperature_c: 20 },
            quality: "draft",
          },
          frontal_area_m2: 0.01,
          reference_length_m: 0.2,
          reynolds: 260000,
          mach: 0.06,
          flow: { code: "x", en: { text: "Flow.", fix: "" }, de: { text: "Strömung.", fix: "" } },
          budget: { cells: 500000, iterations: 300, estimate_s: 600, limited_by_memory: false },
          warnings: [],
        };
      throw new Error(name);
    });
    render(
      <LanguageProvider>
        <ScenarioPanel api={{ call } as unknown as Api} units="metric" />
      </LanguageProvider>,
    );
    expect(await screen.findByText("Flow.")).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "Generate mesh" })).toBeNull();
    expect(call).not.toHaveBeenCalledWith("mesh.info");
  });
});
