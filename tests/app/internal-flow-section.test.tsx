// SPDX-License-Identifier: GPL-3.0-or-later
// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { Api } from "../../app/src/api";
import { InternalFlowSection } from "../../app/src/components/InternalFlowSection";
import { LanguageProvider, STORAGE_KEY } from "../../app/src/i18n/i18n";

const info = {
  scenario: { kind: "internal_flow" },
  reynolds: 9876,
  regime: "turbulent",
  flow: {
    code: "scenario.internal_turbulent",
    en: { text: "Turbulent flow: computed with the k-omega SST model.", fix: "" },
    de: { text: "Turbulente Strömung.", fix: "" },
  },
  openings: [
    {
      key: "model-1:cap-1",
      role: "inlet",
      area_m2: 3.1e-4,
      hydraulic_diameter_m: 0.02,
      velocity_m_s: 0.5,
    },
    {
      key: "model-1:cap-2",
      role: "outlet",
      area_m2: 3.1e-4,
      hydraulic_diameter_m: 0.02,
      velocity_m_s: null,
    },
  ],
  budget: { cells: 500000, iterations: 300, estimate_s: 120, limited_by_memory: false },
  warnings: [],
};

function setup(saved: boolean) {
  const call = vi.fn(async (name: string) => {
    switch (name) {
      case "scenario.boundary_roles":
        return { "model-1:cap-1": "inlet", "model-1:cap-2": "outlet", "model-1:cap-3": "wall" };
      case "scenario.info":
        if (!saved) throw { code: "scenario.none", detail: "", en: {}, de: {} };
        return {
          ...info,
          scenario: {
            kind: "internal_flow",
            medium: { kind: "water", temperature_c: 20 },
            quality: "draft",
            inlets: [{ key: "model-1:cap-1", condition: { type: "volume_flow", m3_s: 0.0002 } }],
            outlets: [{ key: "model-1:cap-2", pressure_pa: 50 }],
          },
        };
      case "scenario.set_internal_flow":
        return info;
      default:
        return {};
    }
  });
  render(
    <LanguageProvider>
      <InternalFlowSection api={{ call } as unknown as Api} />
    </LanguageProvider>,
  );
  return call;
}

describe("internal flow section", () => {
  beforeEach(() => localStorage.setItem(STORAGE_KEY, "en"));
  afterEach(cleanup);

  it("lists inlets and outlets from the boundary roles and applies the scenario", async () => {
    const call = setup(false);
    const inlets = await screen.findByRole("list", { name: "Inlets" });
    expect(within(inlets).getByText("model-1:cap-1")).toBeInTheDocument();
    const outlets = screen.getByRole("list", { name: "Outlets" });
    expect(within(outlets).getByText("model-1:cap-2")).toBeInTheDocument();
    expect(screen.queryByText("model-1:cap-3")).not.toBeInTheDocument();

    fireEvent.change(within(inlets).getByRole("combobox", { name: "Given at model-1:cap-1" }), {
      target: { value: "velocity" },
    });
    fireEvent.change(within(inlets).getByRole("spinbutton", { name: "Value at model-1:cap-1" }), {
      target: { value: "0.5" },
    });
    fireEvent.change(
      within(outlets).getByRole("spinbutton", { name: "Pressure at model-1:cap-2 (Pa)" }),
      {
        target: { value: "0" },
      },
    );
    fireEvent.click(screen.getByRole("button", { name: "Apply" }));
    await waitFor(() =>
      expect(call).toHaveBeenCalledWith("scenario.set_internal_flow", {
        medium: { kind: "water", temperature_c: 20 },
        quality: "standard",
        inlets: [{ key: "model-1:cap-1", condition: { type: "velocity", m_s: 0.5 } }],
        outlets: [{ key: "model-1:cap-2", pressure_pa: 0 }],
      }),
    );
    expect(await screen.findByText(/9[,.]?876/)).toBeInTheDocument();
    expect(
      screen.getByText("Turbulent flow: computed with the k-omega SST model."),
    ).toBeInTheDocument();
    expect(screen.getByText(/20 mm/)).toBeInTheDocument();
  });

  it("shows a saved scenario", async () => {
    setup(true);
    const inlets = await screen.findByRole("list", { name: "Inlets" });
    await waitFor(() =>
      expect(within(inlets).getByRole("combobox", { name: "Given at model-1:cap-1" })).toHaveValue(
        "volume_flow",
      ),
    );
    expect(within(inlets).getByRole("spinbutton", { name: "Value at model-1:cap-1" })).toHaveValue(
      0.0002,
    );
    expect(screen.getByRole("spinbutton", { name: "Pressure at model-1:cap-2 (Pa)" })).toHaveValue(
      50,
    );
  });

  it("asks for marked openings when there are none", async () => {
    const call = vi.fn(async (name: string) => {
      if (name === "scenario.boundary_roles") return {};
      throw { code: "scenario.none", detail: "", en: {}, de: {} };
    });
    render(
      <LanguageProvider>
        <InternalFlowSection api={{ call } as unknown as Api} />
      </LanguageProvider>,
    );
    expect(await screen.findByText(/Mark openings as inlet and outlet/)).toBeInTheDocument();
  });
});
