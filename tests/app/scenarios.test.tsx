// SPDX-License-Identifier: GPL-3.0-or-later
// @vitest-environment jsdom
import { act, cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { Api, ApiEvent } from "../../app/src/api";
import { CompareManySection } from "../../app/src/components/CompareManySection";
import { QueueSection } from "../../app/src/components/QueueSection";
import { ScenarioTabs } from "../../app/src/components/ScenarioTabs";
import { LanguageProvider, STORAGE_KEY } from "../../app/src/i18n/i18n";

const tunnel = (speed: number) => ({
  kind: "wind_tunnel",
  speed_m_s: speed,
  direction_deg: 0,
  ground: "fixed",
  medium: { kind: "air", temperature_c: 20 },
  quality: "draft",
});

const LIST = {
  active: "scenario-1",
  scenarios: [
    { id: "scenario-1", name: "", scenario: tunnel(10), mesh: null, runs: [] },
    { id: "scenario-2", name: "Fast", scenario: tunnel(30), mesh: null, runs: ["job-2"] },
  ],
};

function fakeApi(answers: Record<string, unknown> = {}) {
  const call = vi.fn(async (name: string) => {
    if (name in answers) return answers[name];
    if (name.startsWith("scenario.")) return LIST;
    return {};
  });
  return { call } as unknown as Api & { call: typeof call };
}

const ui = (node: React.ReactNode) => render(<LanguageProvider>{node}</LanguageProvider>);

describe("scenario tabs", () => {
  beforeEach(() => localStorage.setItem(STORAGE_KEY, "en"));
  afterEach(cleanup);

  it("lists the scenarios, unnamed ones with a number, and activates one", async () => {
    const api = fakeApi();
    ui(<ScenarioTabs api={api} revision={0} />);
    const first = await screen.findByRole("tab", { name: "Scenario 1" });
    expect(first).toHaveAttribute("aria-selected", "true");
    fireEvent.click(screen.getByRole("tab", { name: "Fast" }));
    await waitFor(() =>
      expect(api.call).toHaveBeenCalledWith("scenario.activate", { id: "scenario-2" }),
    );
  });

  it("adds a copy, renames and removes", async () => {
    const api = fakeApi();
    ui(<ScenarioTabs api={api} revision={0} />);
    await screen.findByRole("tab", { name: "Scenario 1" });
    fireEvent.click(screen.getByRole("button", { name: "Add scenario" }));
    await waitFor(() =>
      expect(api.call).toHaveBeenCalledWith("scenario.add", { name: "", copy: true }),
    );
    fireEvent.change(screen.getByRole("textbox", { name: "Scenario name" }), {
      target: { value: "Slow" },
    });
    fireEvent.blur(screen.getByRole("textbox", { name: "Scenario name" }));
    await waitFor(() =>
      expect(api.call).toHaveBeenCalledWith("scenario.rename", { id: "scenario-1", name: "Slow" }),
    );
    fireEvent.click(screen.getByRole("button", { name: "Remove Fast" }));
    await waitFor(() =>
      expect(api.call).toHaveBeenCalledWith("scenario.remove", { id: "scenario-2" }),
    );
  });
});

describe("queue", () => {
  beforeEach(() => localStorage.setItem(STORAGE_KEY, "en"));
  afterEach(cleanup);

  it("runs all scenarios, follows the queue and can stop it", async () => {
    const api = fakeApi({
      "run.start_all": {
        runs: [
          { job_id: "job-1", scenario: "scenario-1", error: null },
          { job_id: "job-2", scenario: "scenario-2", error: null },
        ],
      },
    });
    let emit: (e: ApiEvent) => void = () => {};
    const subscribe = (h: (e: ApiEvent) => void) => {
      emit = h;
      return () => {};
    };
    ui(<QueueSection api={api} subscribe={subscribe} />);
    fireEvent.click(await screen.findByRole("button", { name: "Run all scenarios" }));
    await waitFor(() => expect(api.call).toHaveBeenCalledWith("run.start_all", {}));
    act(() =>
      emit({
        name: "run.queue",
        payload: { index: 1, total: 2, run_id: "job-2", scenario: "scenario-2" },
      }),
    );
    expect(screen.getByRole("status")).toHaveTextContent("2 of 2: Fast");
    fireEvent.click(screen.getByRole("button", { name: "Stop queue" }));
    await waitFor(() => expect(api.call).toHaveBeenCalledWith("run.cancel_queue", {}));
    act(() => emit({ name: "run.queue_finished", payload: { cancelled: true, runs: [] } }));
    expect(screen.getByRole("status")).toHaveTextContent("Queue stopped");
  });
});

describe("comparing several runs", () => {
  beforeEach(() => localStorage.setItem(STORAGE_KEY, "en"));
  afterEach(cleanup);

  it("shows the chosen runs side by side with the change against the first", async () => {
    const api = fakeApi({
      "runs.list": [
        { job_id: "job-a", name: "Slow" },
        { job_id: "job-b", name: "Medium" },
        { job_id: "job-c", name: "Fast" },
      ],
      "results.compare_many": {
        jobs: ["job-a", "job-c"],
        names: ["Slow", "Fast"],
        reference: "job-a",
        rows: [{ id: "drag_n", part: null, unit: "N", values: [48, 52.8], relative: [0, 0.1] }],
      },
    });
    ui(<CompareManySection api={api} revision={0} />);
    fireEvent.click(await screen.findByRole("checkbox", { name: "Slow" }));
    fireEvent.click(screen.getByRole("checkbox", { name: "Fast" }));
    await waitFor(() =>
      expect(api.call).toHaveBeenCalledWith("results.compare_many", {
        jobs: ["job-a", "job-c"],
        reference: "job-a",
      }),
    );
    const table = await screen.findByRole("table", { name: "Runs side by side" });
    expect(within(table).getByText("+10 %")).toBeInTheDocument();
    expect(within(table).getByText("Fast")).toBeInTheDocument();
  });
});

describe("report with several runs", () => {
  beforeEach(() => localStorage.setItem(STORAGE_KEY, "en"));
  afterEach(cleanup);

  it("sends the ticked runs as compare_runs", async () => {
    const { ReportDialog } = await import("../../app/src/components/ReportDialog");
    const api = fakeApi({
      "runs.list": [
        { job_id: "job-a", name: "Slow" },
        { job_id: "job-b", name: "Medium" },
        { job_id: "job-c", name: "Fast" },
      ],
      "view.state": {},
      "export.report": { pdf: "x.pdf", html: null },
    });
    ui(
      <ReportDialog
        api={api}
        projectName="P"
        pickSavePath={async () => "C:/r.pdf"}
        pickLogo={async () => null}
        onClose={() => {}}
      />,
    );
    const group = await screen.findByRole("group", { name: "Compare with several runs" });
    await waitFor(() => expect(within(group).getAllByRole("checkbox").length).toBe(2));
    fireEvent.click(within(group).getByRole("checkbox", { name: "Medium" }));
    fireEvent.click(within(group).getByRole("checkbox", { name: "Fast" }));
    fireEvent.click(screen.getByRole("button", { name: /Save/ }));
    await waitFor(() =>
      expect(api.call).toHaveBeenCalledWith(
        "export.report",
        expect.objectContaining({ job_id: "job-a", compare_runs: ["job-b", "job-c"] }),
      ),
    );
  });
});
