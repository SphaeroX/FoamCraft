// SPDX-License-Identifier: GPL-3.0-or-later
// @vitest-environment jsdom
import { act, cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { Api } from "../../app/src/api";
import { ScenarioPanel } from "../../app/src/components/ScenarioPanel";
import { WindRose, angleFromPoint } from "../../app/src/components/WindRose";
import { LanguageProvider, STORAGE_KEY } from "../../app/src/i18n/i18n";

const message = (en: string) => ({ en: { text: en, fix: "" }, de: { text: en, fix: "" } });

const saved = {
  scenario: {
    kind: "wind_tunnel",
    speed_m_s: 20,
    direction_deg: 0,
    ground: "fixed",
    medium: { kind: "air", temperature_c: 20 },
    quality: "standard",
  },
  frontal_area_m2: 0.0004,
  reference_length_m: 0.02,
  reynolds: 26385,
  mach: 0.06,
  flow: { code: "scenario.flow_laminar", ...message("Moderate.") },
  budget: { cells: 2000000, iterations: 500, estimate_s: 600, limited_by_memory: false },
  warnings: [],
};

function fakeApi() {
  const call = vi.fn(async (name: string, params?: Record<string, unknown>) => {
    if (name === "scenario.info") return saved;
    if (name === "scenario.set_wind_tunnel")
      return { ...saved, scenario: { ...saved.scenario, ...params } };
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

const setCalls = (api: { call: ReturnType<typeof vi.fn> }) =>
  api.call.mock.calls.filter((c) => c[0] === "scenario.set_wind_tunnel");

describe("wind rose", () => {
  beforeEach(() => localStorage.setItem(STORAGE_KEY, "en"));
  afterEach(() => {
    vi.useRealTimers();
    cleanup();
  });

  it("turns a point into the direction the wind blows towards, +Y up", () => {
    expect(angleFromPoint(110, 60, 60, 60)).toBeCloseTo(0);
    expect(angleFromPoint(60, 10, 60, 60)).toBeCloseTo(90);
    expect(angleFromPoint(10, 60, 60, 60)).toBeCloseTo(180);
    expect(angleFromPoint(60, 110, 60, 60)).toBeCloseTo(-90);
  });

  it("is a slider that the arrow keys turn by 5°", () => {
    const onChange = vi.fn();
    render(
      <LanguageProvider>
        <WindRose value={30} onChange={onChange} />
      </LanguageProvider>,
    );
    const slider = screen.getByRole("slider", { name: "Wind direction" });
    expect(slider).toHaveAttribute("aria-valuenow", "30");
    fireEvent.keyDown(slider, { key: "ArrowRight" });
    expect(onChange).toHaveBeenLastCalledWith(25);
    fireEvent.keyDown(slider, { key: "ArrowLeft" });
    expect(onChange).toHaveBeenLastCalledWith(35);
  });

  it("a click in the rose sets the direction and applies the scenario", async () => {
    const api = fakeApi();
    renderPanel(api);
    const slider = await screen.findByRole("slider", { name: "Wind direction" });
    await waitFor(() => expect(api.call).toHaveBeenCalledWith("scenario.info"));
    // jsdom has no layout: the rose maps client pixels 1:1 to its 120 px drawing.
    fireEvent.mouseDown(slider, { clientX: 60, clientY: 10 });
    fireEvent.mouseUp(window);
    await waitFor(() =>
      expect(api.call).toHaveBeenCalledWith(
        "scenario.set_wind_tunnel",
        expect.objectContaining({ direction_deg: 90 }),
      ),
    );
    expect(screen.getByLabelText("Flow direction (°)")).toHaveValue(90);
  });

  it("typing quickly applies once, after a short pause", async () => {
    const api = fakeApi();
    renderPanel(api);
    const speed = await screen.findByLabelText("Speed (km/h)");
    await waitFor(() => expect(speed).toHaveValue(72));
    vi.useFakeTimers();
    for (const v of ["1", "10", "100"]) fireEvent.change(speed, { target: { value: v } });
    expect(setCalls(api)).toHaveLength(0);
    await act(async () => {
      vi.advanceTimersByTime(500);
    });
    expect(setCalls(api)).toHaveLength(1);
    expect(setCalls(api)[0][1]).toMatchObject({ speed_m_s: Number((100 / 3.6).toPrecision(12)) });
  });

  it("loading a saved scenario does not apply it again", async () => {
    const api = fakeApi();
    renderPanel(api);
    await screen.findByLabelText("Speed (km/h)");
    await new Promise((r) => setTimeout(r, 600));
    expect(setCalls(api)).toHaveLength(0);
  });

  it("offers typical speeds as buttons and the ground as picture buttons", async () => {
    const api = fakeApi();
    renderPanel(api);
    fireEvent.click(await screen.findByRole("button", { name: "50 km/h" }));
    expect(screen.getByLabelText("Speed (km/h)")).toHaveValue(50);
    fireEvent.click(screen.getByRole("radio", { name: /No ground/ }));
    expect(screen.getByLabelText("Ground")).toHaveValue("none");
    expect(screen.getByRole("radio", { name: /Moving floor/ })).toBeDisabled();
    expect(screen.queryByRole("button", { name: "Apply" })).toBeNull();
  });
});
