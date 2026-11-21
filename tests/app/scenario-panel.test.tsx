// SPDX-License-Identifier: GPL-3.0-or-later
// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { Api } from "../../app/src/api";
import { ScenarioPanel } from "../../app/src/components/ScenarioPanel";
import { LanguageProvider, STORAGE_KEY } from "../../app/src/i18n/i18n";

const message = (en: string, de: string) => ({
  en: { text: en, fix: "" },
  de: { text: de, fix: "" },
});

const result = {
  scenario: {
    kind: "wind_tunnel",
    speed_m_s: 50,
    direction_deg: 0,
    ground: "fixed",
    medium: { kind: "air", temperature_c: 20 },
    quality: "standard",
  },
  medium: { density: 1.204, kinematic_viscosity: 1.516e-5, speed_of_sound: 343 },
  frontal_area_m2: 0.0004,
  reference_length_m: 0.02,
  reynolds: 65963,
  mach: 0.146,
  flow: {
    code: "scenario.flow_laminar",
    ...message("Moderate Reynolds number.", "Mittlere Reynolds-Zahl."),
  },
  tunnel: { center: [0, 0, 0], length: 0.42, width: 0.22, height: 0.12, yaw_deg: 0, floor_z: 0 },
  budget: { cells: 2000000, iterations: 500, estimate_s: 600, limited_by_memory: false },
  memory_bytes: 68719476736,
  cores: 24,
  warnings: [{ code: "scenario.mach_high", ...message("Too fast.", "Zu schnell.") }],
};

function fakeApi() {
  const call = vi.fn(async (name: string) => {
    if (name === "scenario.info") throw { code: "scenario.none", detail: "", ...message("x", "x") };
    if (name === "scenario.set_wind_tunnel") return result;
    throw new Error(name);
  });
  return { call } as unknown as Api & { call: typeof call };
}

function renderPanel(api: Api) {
  render(
    <LanguageProvider>
      <ScenarioPanel api={api} units="metric" />
    </LanguageProvider>,
  );
}

describe("scenario panel", () => {
  beforeEach(() => localStorage.setItem(STORAGE_KEY, "en"));
  afterEach(cleanup);

  it("rejects a speed of zero without calling the command", async () => {
    const api = fakeApi();
    renderPanel(api);
    fireEvent.change(await screen.findByLabelText("Speed (km/h)"), { target: { value: "0" } });
    expect(await screen.findByRole("alert")).toHaveTextContent(/greater than zero/);
    // Longer than the delay of the automatic apply.
    await new Promise((r) => setTimeout(r, 600));
    expect(api.call).not.toHaveBeenCalledWith("scenario.set_wind_tunnel", expect.anything());
  });

  it("sends SI values and shows reference values and warnings", async () => {
    const api = fakeApi();
    renderPanel(api);
    fireEvent.change(await screen.findByLabelText("Speed (km/h)"), { target: { value: "180" } });
    fireEvent.change(screen.getByLabelText("Ground"), { target: { value: "fixed" } });
    await waitFor(() =>
      expect(api.call).toHaveBeenCalledWith("scenario.set_wind_tunnel", {
        speed_m_s: 50,
        direction_deg: 0,
        ground: "fixed",
        medium: { kind: "air", temperature_c: 20 },
        quality: "standard",
      }),
    );
    expect(await screen.findByText(/Too fast/)).toBeInTheDocument();
    expect(screen.getByText(/65[,.]?963/)).toBeInTheDocument();
    expect(screen.getByText(/Moderate Reynolds number/)).toBeInTheDocument();
    expect(screen.getByText(/2[,.]?000[,.]?000 cells/)).toBeInTheDocument();
  });

  it("asks for density and viscosity of a custom medium", async () => {
    const api = fakeApi();
    renderPanel(api);
    fireEvent.change(await screen.findByLabelText("Medium"), { target: { value: "custom" } });
    expect(screen.getByLabelText("Density (kg/m³)")).toBeInTheDocument();
    expect(screen.getByLabelText("Kinematic viscosity (m²/s)")).toBeInTheDocument();
  });
});
