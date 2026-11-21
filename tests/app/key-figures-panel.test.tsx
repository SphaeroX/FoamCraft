// SPDX-License-Identifier: GPL-3.0-or-later
// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { Api } from "../../app/src/api";
import { KeyFiguresPanel } from "../../app/src/components/KeyFiguresPanel";
import { LanguageProvider, STORAGE_KEY } from "../../app/src/i18n/i18n";

const text = (en: string, de: string) => ({ en: { text: en, fix: "" }, de: { text: de, fix: "" } });

const runs = [
  {
    job_id: "job-b",
    name: "Spoiler flat",
    kind: "wind_tunnel",
    created: 2,
    key_value: 43.2,
    key_unit: "N",
    reliability: "yellow",
    bytes: 2_500_000,
  },
  {
    job_id: "job-a",
    name: "job-a",
    kind: "wind_tunnel",
    created: 1,
    key_value: 48,
    key_unit: "N",
    reliability: "green",
    bytes: 1_000_000,
  },
];

const summary = {
  schema: 1,
  job_id: "job-b",
  name: "Spoiler flat",
  kind: "wind_tunnel",
  figures: [
    {
      id: "drag_n",
      value: 43.2,
      unit: "N",
      part: null,
      ...text("Drag: force along the flow.", "Widerstand: Kraft in Strömungsrichtung."),
    },
    {
      id: "drag_area_m2",
      value: 0.135,
      unit: "m²",
      part: null,
      ...text("Drag area.", "Widerstandsfläche."),
    },
    {
      id: "drag_n",
      value: 20,
      unit: "N",
      part: "wing",
      ...text("Drag: force along the flow.", "Widerstand."),
    },
  ],
  reliability: {
    level: "yellow",
    checks: [
      {
        code: "reliability.converged",
        level: "green",
        ...text("The values settled.", "Eingependelt."),
      },
      {
        code: "reliability.mesh_acceptable",
        level: "yellow",
        ...text("The mesh quality is acceptable.", "Annehmbar."),
      },
    ],
  },
};

const comparison = {
  a: "job-a",
  b: "job-b",
  name_a: "job-a",
  name_b: "Spoiler flat",
  rows: [
    { id: "drag_n", part: null, unit: "N", a: 48, b: 43.2, difference: -4.8, relative: -0.1 },
    {
      id: "drag_area_m2",
      part: null,
      unit: "m²",
      a: 0.15,
      b: 0.135,
      difference: -0.015,
      relative: -0.1,
    },
  ],
};

function setup(stored: { a: string; b: string; mode: string } | null = null) {
  const call = vi.fn(async (name: string) => {
    switch (name) {
      case "view.state":
        return { compare: stored };
      case "runs.list":
        return runs;
      case "results.summary":
        return summary;
      case "results.compare":
        return comparison;
      case "view.set_compare":
        return { difference: { min: -5, max: 12 } };
      case "runs.rename":
      case "results.export":
        return {};
      case "runs.delete":
        return { freed_bytes: 2_500_000 };
      case "runs.duplicate":
        return { path: "C:/p/Variant" };
    }
    throw new Error(name);
  });
  const pickSavePath = vi.fn(async (ext: string) => `C:/Temp/figures.${ext}`);
  const confirm = vi.fn(() => true);
  render(
    <LanguageProvider>
      <KeyFiguresPanel
        api={{ call } as unknown as Api}
        revision={0}
        pickSavePath={pickSavePath}
        confirm={confirm}
      />
    </LanguageProvider>,
  );
  return { call, pickSavePath, confirm };
}

describe("key figures panel", () => {
  beforeEach(() => localStorage.setItem(STORAGE_KEY, "en"));
  afterEach(cleanup);

  it("shows the figures of the newest run with unit, explanation and the light", async () => {
    const { call } = setup();
    await waitFor(() => expect(call).toHaveBeenCalledWith("results.summary", { job_id: "job-b" }));
    const table = await screen.findByRole("table", { name: "Key figures" });
    const drag = within(table)
      .getAllByRole("row")
      .find((r) => r.textContent?.includes("Drag force"));
    expect(drag).toHaveTextContent("43.2");
    expect(drag).toHaveTextContent("N");
    expect(within(table).getAllByTitle("Drag: force along the flow.").length).toBe(2);
    expect(table).toHaveTextContent("wing");
    const light = screen.getByRole("status", { name: /Reliability/ });
    expect(light).toHaveTextContent("Use with care");
    expect(light).toHaveTextContent("The mesh quality is acceptable.");
  });

  it("compares two runs side by side with difference", async () => {
    const { call } = setup();
    fireEvent.change(await screen.findByRole("combobox", { name: "Compare with" }), {
      target: { value: "job-a" },
    });
    await waitFor(() =>
      expect(call).toHaveBeenCalledWith("results.compare", { a: "job-a", b: "job-b" }),
    );
    const table = await screen.findByRole("table", { name: "Comparison" });
    const row = within(table)
      .getAllByRole("row")
      .find((r) => r.textContent?.includes("Drag force"));
    expect(row).toHaveTextContent("48");
    expect(row).toHaveTextContent("43.2");
    expect(row).toHaveTextContent("-4.8");
    expect(row).toHaveTextContent("-10");
    fireEvent.click(screen.getByRole("button", { name: "Side by side" }));
    await waitFor(() =>
      expect(call).toHaveBeenCalledWith("view.set_compare", {
        a: "job-a",
        b: "job-b",
        mode: "split",
      }),
    );
    fireEvent.click(screen.getByRole("button", { name: "Difference on the model" }));
    await waitFor(() =>
      expect(call).toHaveBeenCalledWith("view.set_compare", {
        a: "job-a",
        b: "job-b",
        mode: "difference",
      }),
    );
    // The colours of the difference are explained: symmetric around zero.
    expect(await screen.findByText(/−12 … \+12 Pa/)).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "End comparison" }));
    await waitFor(() => expect(call).toHaveBeenCalledWith("view.set_compare", { a: null }));
  });

  it("exports, renames, duplicates and deletes runs", async () => {
    const { call, pickSavePath, confirm } = setup();
    fireEvent.click(await screen.findByRole("button", { name: "Export CSV" }));
    await waitFor(() =>
      expect(call).toHaveBeenCalledWith("results.export", {
        job_id: "job-b",
        path: "C:/Temp/figures.csv",
        format: "csv",
      }),
    );
    expect(pickSavePath).toHaveBeenCalledWith("csv");
    fireEvent.change(screen.getByRole("textbox", { name: "Run name" }), {
      target: { value: "Spoiler steep" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Rename" }));
    await waitFor(() =>
      expect(call).toHaveBeenCalledWith("runs.rename", { job_id: "job-b", name: "Spoiler steep" }),
    );
    fireEvent.change(screen.getByRole("textbox", { name: "Variant name" }), {
      target: { value: "Variant" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Duplicate as variant" }));
    await waitFor(() => expect(call).toHaveBeenCalledWith("runs.duplicate", { name: "Variant" }));
    expect(await screen.findByText(/C:\/p\/Variant/)).toBeInTheDocument();
    // Disk space of the run is shown; deleting asks first.
    expect(screen.getByText(/2\.5 MB/)).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Delete run" }));
    expect(confirm).toHaveBeenCalled();
    await waitFor(() => expect(call).toHaveBeenCalledWith("runs.delete", { job_id: "job-b" }));
  });

  it("picks up the comparison stored in the project", async () => {
    const { call } = setup({ a: "job-a", b: "job-b", mode: "split" });
    expect(await screen.findByRole("table", { name: "Comparison" })).toBeInTheDocument();
    expect(screen.getByRole("combobox", { name: "Compare with" })).toHaveValue("job-a");
    expect(call).toHaveBeenCalledWith("results.compare", { a: "job-a", b: "job-b" });
  });

  it("shows German labels", async () => {
    localStorage.setItem(STORAGE_KEY, "de");
    setup();
    const table = await screen.findByRole("table", { name: "Kennzahlen" });
    expect(table).toHaveTextContent("Widerstandskraft");
    expect(
      within(table).getByTitle("Widerstand: Kraft in Strömungsrichtung.", { exact: false }),
    ).toBeTruthy();
  });
});
