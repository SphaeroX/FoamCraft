// SPDX-License-Identifier: GPL-3.0-or-later
// @vitest-environment jsdom
// Heavy particles (Task_34): material cards set the inertia, the Stokes number and its sentence
// follow the choice, the impact map shows its progress and the share, the rough note appears.
import { act, cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { Api } from "../../app/src/api";
import { HeavySection } from "../../app/src/components/HeavySection";
import { LanguageProvider, STORAGE_KEY } from "../../app/src/i18n/i18n";

const msg = (en: string, de: string) => ({ en: { text: en, fix: "" }, de: { text: de, fix: "" } });

const MATERIALS = [
  {
    id: "fine_dust",
    diameter_m: 1e-5,
    density_kg_m3: 1500,
    on_impact: "stick",
    restitution: 0.3,
    stokes: 0.01,
    sentence: "particles.stokes.low",
    rough: false,
  },
  {
    id: "rain",
    diameter_m: 1e-3,
    density_kg_m3: 1000,
    on_impact: "stick",
    restitution: 0.3,
    stokes: 40,
    sentence: "particles.stokes.high",
    rough: false,
  },
  {
    id: "snow",
    diameter_m: 3e-3,
    density_kg_m3: 100,
    on_impact: "stick",
    restitution: 0.3,
    stokes: 5,
    sentence: "particles.stokes.high",
    rough: true,
  },
];

const TEXTS = {
  "particles.stokes.low": msg(
    "Follows the flow almost like smoke.",
    "Folgt der Strömung fast wie Rauch.",
  ),
  "particles.stokes.high": msg("Flies almost straight.", "Fliegt fast geradeaus."),
  "particles.rough": msg("Rough approximation.", "Grobe Näherung."),
  "impact.estimate": msg("Estimate.", "Abschätzung."),
};

function setup(inertia: Record<string, unknown> | null = null) {
  let polls = 0;
  const call = vi.fn(async (name: string) => {
    switch (name) {
      case "particles.materials":
        return MATERIALS;
      case "error.describe":
        return TEXTS;
      case "view.state":
        return { particles: inertia ? { inertia } : null };
      case "view.set_particles":
        return { particles: { inertia: {} } };
      case "results.impact":
        return { impact_id: "impact-1", cached: false };
      case "results.impact_status":
        polls += 1;
        return polls < 2
          ? { impact_id: "impact-1", state: "running", done: 500, total: 2000 }
          : {
              impact_id: "impact-1",
              state: "done",
              done: 2000,
              total: 2000,
              map: { share_percent: 87.4, uncertainty_percent: 2.1, hits: 1700 },
            };
      default:
        return {};
    }
  });
  render(
    <LanguageProvider>
      <HeavySection api={{ call } as unknown as Api} revision={0} pollMs={10} />
    </LanguageProvider>,
  );
  return call;
}

describe("heavy particles section", () => {
  beforeEach(() => localStorage.setItem(STORAGE_KEY, "en"));
  afterEach(cleanup);

  it("a material card makes the particles heavy and shows its Stokes number", async () => {
    const call = setup();
    fireEvent.click(await screen.findByRole("button", { name: /Raindrops/ }));
    await waitFor(() =>
      expect(call).toHaveBeenCalledWith("view.set_particles", { inertia: { material: "rain" } }),
    );
    expect(await screen.findByText(/Stokes number 40/)).toBeTruthy();
    expect(screen.getByText("Flies almost straight.")).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: /Fine dust/ }));
    expect(await screen.findByText("Follows the flow almost like smoke.")).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: "Massless" }));
    await waitFor(() => expect(call).toHaveBeenCalledWith("view.set_particles", { inertia: null }));
  });

  it("warns that light particles are a rough approximation", async () => {
    setup();
    fireEvent.click(await screen.findByRole("button", { name: /Snowflakes/ }));
    expect(await screen.findByText("Rough approximation.")).toBeTruthy();
  });

  it("computes the impact map with progress and shows the share that hits", async () => {
    const call = setup({ material: "rain" });
    fireEvent.click(await screen.findByRole("button", { name: "Compute impact map" }));
    await waitFor(() => expect(call).toHaveBeenCalledWith("results.impact", {}));
    await act(async () => {
      await new Promise((r) => setTimeout(r, 60));
    });
    expect(await screen.findByText(/87\.4 % hit ± 2\.1 %/)).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: "Show on the model" }));
    await waitFor(() =>
      expect(call).toHaveBeenCalledWith("view.set_surface_field", { field: "impact" }),
    );
  });
});
