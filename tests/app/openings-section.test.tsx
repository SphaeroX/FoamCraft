// SPDX-License-Identifier: GPL-3.0-or-later
// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { Api } from "../../app/src/api";
import { OpeningsSection } from "../../app/src/components/OpeningsSection";
import { LanguageProvider, STORAGE_KEY } from "../../app/src/i18n/i18n";

const notFlat = {
  code: "geometry.opening_not_flat",
  en: { text: "The edge of this opening is not flat.", fix: "Check the cap." },
  de: { text: "Der Rand dieser Öffnung ist nicht eben.", fix: "Prüfen." },
};

function setup(sealed = true) {
  const call = vi.fn(async (name: string, params?: Record<string, unknown>) => {
    switch (name) {
      case "geometry.list":
        return [{ id: "model-1", name: "pipe.stl" }];
      case "geometry.openings":
        return {
          model: params?.model,
          kind: "open_shell",
          passages: 0,
          openings: [
            {
              centroid_m: [0, 0, 0],
              normal: [1, 0, 0],
              area_m2: 3.1e-4,
              planar: true,
              cap: "model-1:cap-1",
            },
            {
              centroid_m: [0.05, 0, 0],
              normal: [1, 0, 0],
              area_m2: 3.1e-4,
              planar: false,
              cap: null,
            },
          ],
        };
      case "cap.list":
        return [
          {
            key: "model-1:cap-1",
            model: "model-1",
            centroid_m: [0, 0, 0],
            normal: [1, 0, 0],
            area_m2: 3.1e-4,
            role: null,
            warnings: [notFlat],
          },
        ];
      case "geometry.fluid_check":
        return sealed
          ? {
              sealed: true,
              fluid_point_m: [0.02, 0, 0],
              fluid_volume_m3: 1e-5,
              caps: 2,
              warnings: [],
            }
          : {
              sealed: false,
              fluid_point_m: null,
              fluid_volume_m3: 0,
              caps: 1,
              warnings: [
                {
                  code: "geometry.not_sealed",
                  en: { text: "The fluid region is not closed.", fix: "Close every opening." },
                  de: { text: "Nicht geschlossen.", fix: "Alle verschließen." },
                },
              ],
            };
      default:
        return {};
    }
  });
  const onCapMode = vi.fn();
  render(
    <LanguageProvider>
      <OpeningsSection
        api={{ call } as unknown as Api}
        revision={0}
        capMode={false}
        onCapMode={onCapMode}
      />
    </LanguageProvider>,
  );
  return { call, onCapMode };
}

describe("openings section", () => {
  beforeEach(() => localStorage.setItem(STORAGE_KEY, "en"));
  afterEach(cleanup);

  it("shows the kind of model and its openings", async () => {
    setup();
    expect(await screen.findByText(/Open surfaces/)).toBeInTheDocument();
    expect(screen.getByText(/2 openings, 1 closed/)).toBeInTheDocument();
  });

  it("closes an open opening at its centre", async () => {
    const { call } = setup();
    fireEvent.click(await screen.findByRole("button", { name: "Close opening 2" }));
    await waitFor(() =>
      expect(call).toHaveBeenCalledWith("geometry.cap_opening", {
        model: "model-1",
        point: [0.05, 0, 0],
      }),
    );
  });

  it("switches to closing by click in the view", async () => {
    const { onCapMode } = setup();
    fireEvent.click(await screen.findByRole("button", { name: "Close by clicking in the view" }));
    expect(onCapMode).toHaveBeenCalledWith(true);
  });

  it("marks a cap as inlet and removes it", async () => {
    const { call } = setup();
    const caps = await screen.findByRole("list", { name: "Caps" });
    fireEvent.change(within(caps).getByRole("combobox", { name: "Role of model-1:cap-1" }), {
      target: { value: "inlet" },
    });
    await waitFor(() =>
      expect(call).toHaveBeenCalledWith("scenario.set_boundary_role", {
        keys: ["model-1:cap-1"],
        role: "inlet",
      }),
    );
    expect(within(caps).getByText(/not flat/)).toBeInTheDocument();
    fireEvent.click(within(caps).getByRole("button", { name: "Remove" }));
    await waitFor(() => expect(call).toHaveBeenCalledWith("cap.remove", { key: "model-1:cap-1" }));
  });

  it("checks whether the fluid region is sealed", async () => {
    setup(false);
    fireEvent.click(await screen.findByRole("button", { name: "Check seal" }));
    expect(await screen.findByText(/The fluid region is not closed\./)).toBeInTheDocument();
  });

  it("reports a sealed region", async () => {
    setup(true);
    fireEvent.click(await screen.findByRole("button", { name: "Check seal" }));
    expect(await screen.findByText("The fluid region is sealed.")).toBeInTheDocument();
  });
});
