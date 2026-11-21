// SPDX-License-Identifier: GPL-3.0-or-later
// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { Api } from "../../app/src/api";
import { FlowViewSection } from "../../app/src/components/FlowViewSection";
import { LanguageProvider, STORAGE_KEY } from "../../app/src/i18n/i18n";

const levels = [{ min: [-1, -2, 0], max: [3, 2, 2], dims: [40, 40, 20] }];

function setup(state: Record<string, unknown> = { slices: [], isosurfaces: [], clip: null }) {
  const call = vi.fn(async (name: string) => {
    switch (name) {
      case "view.state":
        return state;
      case "view.volume_fields":
        return [
          { name: "U_mag", unit: "m/s" },
          { name: "p_total", unit: "Pa" },
          { name: "Q", unit: "1/s²" },
          { name: "p", unit: "Pa" },
        ];
      default:
        // Changes answer with the (here unchanged) view state.
        return state;
    }
  });
  render(
    <LanguageProvider>
      <FlowViewSection api={{ call } as unknown as Api} levels={levels} />
    </LanguageProvider>,
  );
  return call;
}

describe("flow view section", () => {
  beforeEach(() => localStorage.setItem(STORAGE_KEY, "en"));
  afterEach(cleanup);

  it("adds a slice through the middle of the domain along the chosen axis", async () => {
    const call = setup();
    fireEvent.change(await screen.findByRole("combobox", { name: "Slice normal" }), {
      target: { value: "z" },
    });
    fireEvent.change(screen.getByRole("combobox", { name: "Slice field" }), {
      target: { value: "p_total" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Add slice" }));
    await waitFor(() =>
      expect(call).toHaveBeenCalledWith("view.add_slice", {
        origin: [1, 0, 1],
        normal: [0, 0, 1],
        field: "p_total",
      }),
    );
  });

  it("moves a slice by dragging its slider", async () => {
    const call = setup({
      slices: [{ id: "slice-1", origin: [1, 0, 1], normal: [0, 0, 1], field: "U_mag" }],
      isosurfaces: [],
      clip: null,
    });
    const item = await screen.findByRole("listitem", { name: "slice-1" });
    fireEvent.change(within(item).getByRole("slider"), { target: { value: "1.5" } });
    await waitFor(() =>
      expect(call).toHaveBeenCalledWith("view.update_slice", {
        id: "slice-1",
        origin: [1, 0, 1.5],
      }),
    );
    fireEvent.click(within(item).getByRole("button", { name: "Remove" }));
    await waitFor(() => expect(call).toHaveBeenCalledWith("view.remove", { id: "slice-1" }));
  });

  it("adds an isosurface of the Q criterion", async () => {
    const call = setup();
    fireEvent.change(await screen.findByRole("combobox", { name: "Isosurface field" }), {
      target: { value: "Q" },
    });
    fireEvent.change(screen.getByRole("spinbutton", { name: "Isosurface value" }), {
      target: { value: "500" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Add isosurface" }));
    await waitFor(() =>
      expect(call).toHaveBeenCalledWith("view.add_isosurface", { field: "Q", value: 500 }),
    );
  });

  it("cuts the model open along the first slice", async () => {
    const call = setup({
      slices: [{ id: "slice-1", origin: [1, 0, 1], normal: [0, 1, 0], field: "U_mag" }],
      isosurfaces: [],
      clip: null,
    });
    fireEvent.click(
      await screen.findByRole("checkbox", { name: "Cut the model open at the first slice" }),
    );
    await waitFor(() =>
      expect(call).toHaveBeenCalledWith("view.set_clip", {
        plane: { origin: [1, 0, 1], normal: [0, 1, 0] },
      }),
    );
  });
});

describe("flow view section clip side", () => {
  beforeEach(() => localStorage.setItem(STORAGE_KEY, "en"));
  afterEach(cleanup);

  it("flips the hidden side of the cut", async () => {
    const call = setup({
      slices: [{ id: "slice-1", origin: [1, 0, 1], normal: [0, 1, 0], field: "U_mag" }],
      isosurfaces: [],
      clip: { origin: [1, 0, 1], normal: [0, 1, 0] },
    });
    fireEvent.click(await screen.findByRole("button", { name: "Other side" }));
    await waitFor(() =>
      expect(call).toHaveBeenCalledWith("view.set_clip", {
        plane: { origin: [1, 0, 1], normal: [-0, -1, -0] },
      }),
    );
  });
});

describe("flow view section without slices", () => {
  beforeEach(() => localStorage.setItem(STORAGE_KEY, "en"));
  afterEach(cleanup);

  it("can still switch an active cut off", async () => {
    const call = setup({
      slices: [],
      isosurfaces: [],
      clip: { origin: [1, 0, 1], normal: [0, 1, 0] },
    });
    fireEvent.click(
      await screen.findByRole("checkbox", { name: "Cut the model open at the first slice" }),
    );
    await waitFor(() => expect(call).toHaveBeenCalledWith("view.set_clip", { plane: null }));
  });
});
