// SPDX-License-Identifier: GPL-3.0-or-later
// @vitest-environment jsdom
// Flow on the surface (Task_33): switches and sliders call view.set_surface_flow; the notes on
// flutter and approximation appear.
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { Api } from "../../app/src/api";
import { SurfaceFlowSection } from "../../app/src/components/SurfaceFlowSection";
import { LanguageProvider, STORAGE_KEY } from "../../app/src/i18n/i18n";

function setup(info: Record<string, unknown>) {
  const call = vi.fn(async (name: string, params?: Record<string, unknown>) => {
    if (name === "view.surface_flow_info") return info;
    if (name === "view.set_surface_flow") {
      const lines = (params?.lines as { on?: boolean } | undefined)?.on ? {} : null;
      const tufts = (params?.tufts as { on?: boolean } | undefined)?.on ? {} : null;
      return { ...info, surface_flow: { lines, tufts } };
    }
    return {};
  });
  render(
    <LanguageProvider>
      <SurfaceFlowSection api={{ call } as unknown as Api} />
    </LanguageProvider>,
  );
  return call;
}

describe("surface flow section", () => {
  beforeEach(() => localStorage.setItem(STORAGE_KEY, "en"));
  afterEach(cleanup);

  it("switches the lines and the tufts on", async () => {
    const call = setup({ surface_flow: null, approximate: false });
    fireEvent.click(await screen.findByRole("checkbox", { name: "Lines (oil film)" }));
    await waitFor(() =>
      expect(call).toHaveBeenCalledWith("view.set_surface_flow", { lines: { on: true } }),
    );
    fireEvent.click(screen.getByRole("checkbox", { name: "Wool tufts" }));
    await waitFor(() =>
      expect(call).toHaveBeenCalledWith("view.set_surface_flow", { tufts: { on: true } }),
    );
  });

  it("changes spacing and colour of the lines", async () => {
    const call = setup({
      surface_flow: {
        lines: { density: 1 / 60, color_by: "plain", animate: true, seed: 1 },
        tufts: null,
        speed: 1,
        paused: false,
      },
      approximate: false,
    });
    fireEvent.change(await screen.findByRole("slider", { name: "Line spacing" }), {
      target: { value: "40" },
    });
    await waitFor(() =>
      expect(call).toHaveBeenCalledWith("view.set_surface_flow", {
        lines: { on: true, density: 1 / 40 },
      }),
    );
    fireEvent.change(screen.getByRole("combobox", { name: "Line colour" }), {
      target: { value: "wall_shear_stress" },
    });
    await waitFor(() =>
      expect(call).toHaveBeenCalledWith("view.set_surface_flow", {
        lines: { on: true, color_by: "wall_shear_stress" },
      }),
    );
  });

  it("shows the flutter note with tufts and the approximation note for old runs", async () => {
    setup({
      surface_flow: {
        lines: null,
        tufts: { spacing: 1 / 25, color_by: "plain", animate: true, seed: 1 },
        speed: 1,
        paused: false,
      },
      approximate: true,
    });
    expect(await screen.findByText(/It is an illustration, not a time-accurate/)).toBeTruthy();
    expect(screen.getByText(/Approximate: this run has no wall shear stress vector/)).toBeTruthy();
  });
});
