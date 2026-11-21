// SPDX-License-Identifier: GPL-3.0-or-later
// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { Api } from "../../app/src/api";
import { ResultsPanel } from "../../app/src/components/ResultsPanel";
import { LanguageProvider, STORAGE_KEY } from "../../app/src/i18n/i18n";

const meta = {
  job_id: "job-7",
  fields: [
    { name: "p", unit: "Pa", location: "surface", min: -250, max: 240 },
    { name: "Cp", unit: "-", location: "surface", min: -1.04, max: 1.0 },
    { name: "wall_shear_stress", unit: "Pa", location: "surface", min: 0, max: 3.2 },
    { name: "Ux", unit: "m/s", location: "volume", min: -4, max: 24 },
  ],
  levels: [{ min: [0, 0, 0], max: [1, 1, 1], dims: [64, 64, 64] }],
  yaw_deg: 0,
  bytes: 1234567,
  colormap: ["#1b2a5c", "#2f6f8f", "#f2d33a"],
};

function setup(none = false) {
  const call = vi.fn(async (name: string) => {
    if (name === "results.list_fields") {
      if (none) throw { code: "results.none", detail: "", en: {}, de: {} };
      return meta;
    }
    if (name === "view.set_surface_field") return {};
    throw new Error(name);
  });
  const view = render(
    <LanguageProvider>
      <ResultsPanel api={{ call } as unknown as Api} probe={null} />
    </LanguageProvider>,
  );
  return { call, view };
}

describe("results panel", () => {
  beforeEach(() => localStorage.setItem(STORAGE_KEY, "en"));
  afterEach(cleanup);

  it("colours the model by pressure at first and shows the legend", async () => {
    const { call } = setup();
    await waitFor(() =>
      expect(call).toHaveBeenCalledWith("view.set_surface_field", { field: "p", range: null }),
    );
    expect(screen.getByRole("combobox", { name: "Surface field" })).toHaveValue("p");
    const legend = screen.getByRole("img", { name: /Colour scale/ });
    expect(legend).toHaveTextContent("-250");
    expect(legend).toHaveTextContent("240");
    expect(legend).toHaveTextContent("Pa");
    // Only surface fields can colour the model.
    expect(screen.queryByRole("option", { name: /Ux/ })).not.toBeInTheDocument();
  });

  it("switches the field and sets a range by hand", async () => {
    const { call } = setup();
    fireEvent.change(await screen.findByRole("combobox", { name: "Surface field" }), {
      target: { value: "Cp" },
    });
    await waitFor(() =>
      expect(call).toHaveBeenCalledWith("view.set_surface_field", { field: "Cp", range: null }),
    );
    fireEvent.click(screen.getByRole("checkbox", { name: "Set range by hand" }));
    fireEvent.change(screen.getByRole("spinbutton", { name: "From" }), { target: { value: "-1" } });
    fireEvent.change(screen.getByRole("spinbutton", { name: "To" }), { target: { value: "1" } });
    fireEvent.click(screen.getByRole("button", { name: "Apply range" }));
    await waitFor(() =>
      expect(call).toHaveBeenCalledWith("view.set_surface_field", { field: "Cp", range: [-1, 1] }),
    );
  });

  it("shows the value under the mouse with its unit", async () => {
    const call = vi.fn(async (name: string) => (name === "results.list_fields" ? meta : {}));
    render(
      <LanguageProvider>
        <ResultsPanel
          api={{ call } as unknown as Api}
          probe={{ field: "p", value: 123.456, unit: "Pa", location: "surface" }}
        />
      </LanguageProvider>,
    );
    expect(await screen.findByText(/123[.,]5 Pa/)).toBeInTheDocument();
  });

  it("explains that there are no results yet", async () => {
    setup(true);
    expect(await screen.findByText(/Run the simulation first/)).toBeInTheDocument();
  });
});
