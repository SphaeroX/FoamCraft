// SPDX-License-Identifier: GPL-3.0-or-later
// @vitest-environment jsdom
import { act, cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { Api, ApiEvent } from "../../app/src/api";
import { RunPanel } from "../../app/src/components/RunPanel";
import { LanguageProvider, STORAGE_KEY } from "../../app/src/i18n/i18n";

const texts = {
  "run.diverged": {
    en: { text: "The simulation diverged.", fix: "Use a finer mesh." },
    de: { text: "Die Simulation ist divergiert.", fix: "Feineres Netz." },
  },
  "run.cancelled": {
    en: { text: "The simulation was cancelled.", fix: "Start it again." },
    de: { text: "Abgebrochen.", fix: "Neu starten." },
  },
  "run.stable": {
    en: { text: "The forces settled.", fix: "Use as is." },
    de: { text: "Eingependelt.", fix: "So verwenden." },
  },
};

const result = {
  job_id: "job-1",
  reason: "stable",
  iterations: 290,
  decided_at_iteration: 288,
  max_iterations: 1500,
  coefficients: { cd: 0.312, cl: -0.05, cs: 0, cm_pitch: 0, cm_roll: 0, cm_yaw: 0 },
  forces: { drag_n: 41.2, lift_n: -6.6, side_n: 0 },
  fluctuation_percent: 0.6,
  parts: [],
  reference: {
    area_m2: 0.5,
    length_m: 1,
    speed_m_s: 15,
    density_kg_m3: 1.2,
    dynamic_pressure_pa: 135,
  },
  y_plus: [],
  hints: ["run.stable"],
  restarted: false,
  cells: 700000,
  cores: 8,
  timings_s: { decompose: 2, solve: 170, reconstruct: 2 },
};

function setup(active: unknown[] = []) {
  let handler: ((e: ApiEvent) => void) | null = null;
  const subscribe = (h: (e: ApiEvent) => void) => {
    handler = h;
    return () => (handler = null);
  };
  const call = vi.fn(async (name: string, params?: Record<string, unknown>) => {
    switch (name) {
      case "scenario.info":
        return { budget: { cells: 700000, estimate_s: 600, iterations: 500 } };
      case "mesh.info":
        return { cells: 690000 };
      case "run.active":
        return active;
      case "run.start":
        return { job_id: "job-1" };
      case "run.attach":
        return { job_id: params?.job_id };
      case "run.cancel":
        return {};
      case "run.result":
        return result;
      case "run.log":
        return { lines: ["[solve] Time = 3", "[solve] End"] };
      case "error.describe":
        return Object.fromEntries(
          ((params?.codes as string[]) ?? []).map((c) => [c, texts[c as keyof typeof texts]]),
        );
      default:
        throw new Error(name);
    }
  });
  render(
    <LanguageProvider>
      <RunPanel api={{ call } as unknown as Api} subscribe={subscribe} />
    </LanguageProvider>,
  );
  const emit = (name: string, payload: unknown) =>
    act(() => {
      handler?.({ name, payload });
    });
  return { call, emit };
}

describe("run panel", () => {
  beforeEach(() => localStorage.setItem(STORAGE_KEY, "en"));
  afterEach(cleanup);

  it("summarises the run before the start", async () => {
    setup();
    expect(await screen.findByText(/690[,.]?000 cells/)).toBeInTheDocument();
    expect(screen.getByText(/about 10 min/)).toBeInTheDocument();
  });

  it("follows stages, residuals, coefficients and the remaining time", async () => {
    const { call, emit } = setup();
    fireEvent.click(await screen.findByRole("button", { name: "Start run" }));
    await waitFor(() => expect(call).toHaveBeenCalledWith("run.start", {}));
    await screen.findByRole("button", { name: "Cancel" });

    emit("run.stage", { job_id: "job-1", stage: "decompose" });
    emit("run.stage", { job_id: "job-1", stage: "solve" });
    const timeline = screen.getByRole("list", { name: "Stages" });
    expect(within(timeline).getByText("Solve")).toHaveAttribute("aria-current", "step");
    expect(within(timeline).getByText("Prepare parallel run")).toHaveClass("done");

    for (let i = 1; i <= 3; i++) {
      emit("run.progress", {
        job_id: "job-1",
        stage: "solve",
        iteration: i,
        residuals: { p: 1 / i, Ux: 0.5 / i },
      });
      emit("run.coefficients", {
        job_id: "job-1",
        iteration: i,
        cd: 0.3 + 0.01 * i,
        cl: -0.05,
        remaining_s: 125,
        max_iterations: 1500,
      });
    }
    const residuals = screen.getByRole("img", { name: /Residuals/ });
    expect(residuals.querySelectorAll("polyline")).toHaveLength(2);
    expect(residuals.querySelector("polyline")?.getAttribute("points")?.split(" ")).toHaveLength(3);
    expect(screen.getByRole("img", { name: /Cd 0[.,]33/ })).toBeInTheDocument();
    expect(screen.getByText(/about 2 min left/)).toBeInTheDocument();
    expect(screen.getByText(/Iteration 3 of at most 1[,.]?500/)).toBeInTheDocument();
  });

  it("cancels through run.cancel and shows the cancelled state", async () => {
    const { call, emit } = setup();
    fireEvent.click(await screen.findByRole("button", { name: "Start run" }));
    fireEvent.click(await screen.findByRole("button", { name: "Cancel" }));
    await waitFor(() => expect(call).toHaveBeenCalledWith("run.cancel", { job_id: "job-1" }));
    emit("run.failed", { job_id: "job-1", code: "run.cancelled", detail: "" });
    expect(await screen.findByText("Cancelled")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Start run" })).toBeEnabled();
  });

  it("shows the result card when the run is done", async () => {
    const { emit } = setup();
    fireEvent.click(await screen.findByRole("button", { name: "Start run" }));
    await screen.findByRole("button", { name: "Cancel" });
    emit("run.result", { job_id: "job-1", reason: "stable", cd: 0.312, cl: -0.05 });
    const card = await screen.findByRole("region", { name: "Result" });
    expect(within(card).getByText("0.312")).toBeInTheDocument();
    expect(within(card).getByText(/41[.,]2 N/)).toBeInTheDocument();
    expect(within(card).getByText("The forces settled.")).toBeInTheDocument();
    expect(screen.getByText("Finished")).toBeInTheDocument();
  });

  it("explains a failure in plain words with a suggestion", async () => {
    const { emit } = setup();
    fireEvent.click(await screen.findByRole("button", { name: "Start run" }));
    await screen.findByRole("button", { name: "Cancel" });
    emit("run.failed", { job_id: "job-1", code: "run.diverged", detail: "nan" });
    const alert = await screen.findByRole("alert");
    expect(alert).toHaveTextContent("The simulation diverged.");
    expect(alert).toHaveTextContent("Use a finer mesh.");
    expect(screen.getByText("Failed")).toBeInTheDocument();
  });

  it("shows the log on demand", async () => {
    const { call } = setup();
    fireEvent.click(await screen.findByRole("button", { name: "Start run" }));
    await screen.findByRole("button", { name: "Cancel" });
    fireEvent.click(screen.getByText("Log"));
    await waitFor(() =>
      expect(call).toHaveBeenCalledWith("run.log", { job_id: "job-1", lines: 200 }),
    );
    expect(await screen.findByText(/\[solve\] End/)).toBeInTheDocument();
  });

  it("reconnects to a running job after a restart", async () => {
    const { call } = setup([
      { job_id: "job-9", case: "wind_tunnel_solve", state: { state: "running", stage: "solve" } },
    ]);
    await waitFor(() => expect(call).toHaveBeenCalledWith("run.attach", { job_id: "job-9" }));
    expect(await screen.findByText("Running")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Cancel" })).toBeInTheDocument();
  });
});

describe("run panel for internal flow", () => {
  beforeEach(() => localStorage.setItem(STORAGE_KEY, "en"));
  afterEach(cleanup);

  it("draws the pressure loss and shows flows per opening", async () => {
    const internal = {
      kind: "internal_flow",
      job_id: "job-1",
      reason: "stable",
      iterations: 130,
      pressure_loss_pa: 24.26,
      static_pressure_loss_pa: 24.26,
      mass_balance_error: 0.0004,
      openings: [
        { key: "model-1:cap-1", role: "inlet", flow_m3_s: 1.5628e-4, share: null },
        { key: "model-1:cap-2", role: "outlet", flow_m3_s: 1.2e-4, share: 0.768 },
        { key: "model-1:cap-3", role: "outlet", flow_m3_s: 3.6e-5, share: 0.232 },
      ],
      hints: ["run.stable"],
      timings_s: { solve: 20 },
    };
    let handler: ((e: { name: string; payload: unknown }) => void) | null = null;
    const call = vi.fn(async (name: string) => {
      if (name === "run.start") return { job_id: "job-1" };
      if (name === "run.result") return internal;
      if (name === "run.active") return [];
      if (name === "error.describe") return {};
      throw new Error(name);
    });
    render(
      <LanguageProvider>
        <RunPanel api={{ call } as unknown as Api} subscribe={(h) => ((handler = h), () => {})} />
      </LanguageProvider>,
    );
    fireEvent.click(await screen.findByRole("button", { name: "Start run" }));
    await screen.findByRole("button", { name: "Cancel" });
    for (let i = 1; i <= 3; i++) {
      act(() =>
        handler?.({
          name: "run.pressure_loss",
          payload: {
            job_id: "job-1",
            iteration: i,
            pressure_loss_pa: 20 + i,
            remaining_s: 60,
            max_iterations: 900,
          },
        }),
      );
    }
    expect(screen.getByRole("img", { name: /Pressure loss: 23 Pa/ })).toBeInTheDocument();
    act(() => handler?.({ name: "run.result", payload: { job_id: "job-1" } }));
    const card = await screen.findByRole("region", { name: "Result" });
    expect(within(card).getByText(/24[.,]3 Pa/)).toBeInTheDocument();
    expect(within(card).getByText("model-1:cap-2")).toBeInTheDocument();
    expect(within(card).getByText(/76[.,]8 %/)).toBeInTheDocument();
    expect(within(card).getByText(/0[.,]04 %/)).toBeInTheDocument();
  });
});
