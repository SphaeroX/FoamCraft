// SPDX-License-Identifier: GPL-3.0-or-later
// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { Api } from "../../app/src/api";
import { FlowViewSection } from "../../app/src/components/FlowViewSection";
import { LanguageProvider, STORAGE_KEY } from "../../app/src/i18n/i18n";

const levels = [{ min: [-1, -2, 0], max: [3, 2, 2], dims: [40, 40, 20] }];

const state = {
  slices: [
    { id: "slice-1", origin: [1, 0, 1], normal: [0, 0, 1], field: "U_mag", hidden: false },
    { id: "slice-2", origin: [1, 0, 1], normal: [1, 0, 0], field: "p", hidden: true },
  ],
  isosurfaces: [],
  clip: { origin: [1, 0, 1], normal: [0, 0, 1] },
  clip_targets: ["model"],
};

function setup(selected: string | null = null) {
  const call = vi.fn(async (name: string) => (name === "view.volume_fields" ? [] : state));
  const onSelect = vi.fn();
  render(
    <LanguageProvider>
      <FlowViewSection
        api={{ call } as unknown as Api}
        levels={levels}
        selected={selected}
        onSelect={onSelect}
      />
    </LanguageProvider>,
  );
  return { call, onSelect };
}

describe("slice list", () => {
  beforeEach(() => localStorage.setItem(STORAGE_KEY, "en"));
  afterEach(cleanup);

  it("selects a slice from the list", async () => {
    const { onSelect } = setup();
    const item = await screen.findByRole("listitem", { name: "slice-2" });
    fireEvent.click(within(item).getByRole("button", { name: "Select" }));
    expect(onSelect).toHaveBeenCalledWith("slice-2");
  });

  it("hides and shows a slice", async () => {
    const { call } = setup();
    const item = await screen.findByRole("listitem", { name: "slice-1" });
    fireEvent.click(within(item).getByRole("checkbox", { name: "Visible" }));
    await waitFor(() =>
      expect(call).toHaveBeenCalledWith("view.update_slice", { id: "slice-1", hidden: true }),
    );
    const hidden = screen.getByRole("listitem", { name: "slice-2" });
    expect(within(hidden).getByRole("checkbox", { name: "Visible" })).not.toBeChecked();
  });

  it("turns the selected slice to another axis", async () => {
    const { call } = setup("slice-1");
    const item = await screen.findByRole("listitem", { name: "slice-1" });
    expect(within(item).getByRole("button", { name: "Select" })).toHaveAttribute(
      "aria-pressed",
      "true",
    );
    fireEvent.click(within(item).getByRole("button", { name: "Normal along X" }));
    await waitFor(() =>
      expect(call).toHaveBeenCalledWith("view.update_slice", { id: "slice-1", normal: [1, 0, 0] }),
    );
  });

  it("chooses what the cut opens and cuts at the selected slice", async () => {
    const { call } = setup("slice-2");
    fireEvent.click(await screen.findByRole("checkbox", { name: "Smoke and particles" }));
    await waitFor(() =>
      expect(call).toHaveBeenCalledWith("view.set_clip", {
        plane: { origin: [1, 0, 1], normal: [0, 0, 1] },
        targets: ["model", "particles"],
      }),
    );
    fireEvent.click(screen.getByRole("button", { name: "Cut at the selected slice" }));
    await waitFor(() =>
      expect(call).toHaveBeenCalledWith("view.set_clip", {
        plane: { origin: [1, 0, 1], normal: [1, 0, 0] },
      }),
    );
  });
});
