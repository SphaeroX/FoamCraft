// SPDX-License-Identifier: GPL-3.0-or-later
// @vitest-environment jsdom
// The ride (Task_32): suggestion cards, flight bar, HUD readings and explaining sentences.
import { act, cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { Api } from "../../app/src/api";
import { RideOverlay } from "../../app/src/components/RideOverlay";
import { RideSection } from "../../app/src/components/RideSection";
import { LanguageProvider, STORAGE_KEY } from "../../app/src/i18n/i18n";
import { eventAt, readingsAt, type RideClock, type RideInfo } from "../../app/src/ride";

const msg = (en: string, de: string, fix = "") => ({
  en: { text: en, fix },
  de: { text: de, fix },
});

const INFO: RideInfo = {
  ride: { start: [-1, 0, 0], mode: "calm", view: "first_person", speed: 1, guide: true },
  job: "job-1",
  duration_s: 10,
  start_t: 0,
  length_m: 2,
  flow_time_s: 0.2,
  reference_speed_m_s: 20,
  density_kg_m3: 1.2,
  model_length_m: 0.4,
  ends: ["outside", "stalled"],
  columns: ["t", "s", "speed", "cp", "p", "d", "accel", "time_factor"],
  rows: [
    [0, 0, 20, 0, 0, 0.1, 0, 0.5],
    [4, 1, 10, 0.75, 180, 0.01, 98.1, 0.1],
    [10, 2, 2, 0.99, 238, 0.002, 9.81, 0.02],
  ],
  events: [
    { id: "approach", code: "ride.event.approach", s: 0.5, t: 2, value: 0.6 },
    { id: "stagnation", code: "ride.event.stagnation", s: 1.8, t: 8, value: 0.99 },
  ],
};

const TEXTS = {
  "ride.event.approach": msg("The flow piles up.", "Die Strömung staut sich."),
  "ride.event.stagnation": msg("Almost at a standstill.", "Fast Stillstand."),
  "ride.suggestion.stagnation": msg("Towards the stagnation point", "Auf den Staupunkt zu"),
  "ride.suggestion.hugging": msg("Close along the surface", "Dicht an der Oberfläche"),
};

function fakeApi(info: RideInfo | null = INFO) {
  const call = vi.fn(async (name: string) => {
    switch (name) {
      case "ride.telemetry":
        if (!info) throw { code: "ride.none", detail: "", en: {}, de: {} };
        return info;
      case "ride.suggest":
        return [
          {
            id: "stagnation",
            title: "ride.suggestion.stagnation",
            start: [0, 0, 0],
            duration_s: 9,
          },
          { id: "hugging", title: "ride.suggestion.hugging", start: [0, 1, 0], duration_s: 14 },
        ];
      case "error.describe":
        return TEXTS;
      case "ride.start":
        return info;
      default:
        return {};
    }
  });
  return { call } as unknown as Api & { call: typeof call };
}

describe("ride helpers", () => {
  it("interpolate the readings at a screen time", () => {
    const r = readingsAt(INFO, 2);
    expect(r?.speed).toBeCloseTo(15);
    expect(r?.cp).toBeCloseTo(0.375);
    expect(r?.d).toBeCloseTo(0.055);
    expect(readingsAt(INFO, 99)?.speed).toBeCloseTo(2);
    expect(readingsAt(INFO, -1)?.speed).toBeCloseTo(20);
  });

  it("show an event for 2.5 s after it starts", () => {
    expect(eventAt(INFO, 1.9)).toBeNull();
    expect(eventAt(INFO, 2.1)?.id).toBe("approach");
    expect(eventAt(INFO, 4.6)).toBeNull();
    expect(eventAt(INFO, 8.5)?.id).toBe("stagnation");
  });
});

describe("ride section", () => {
  beforeEach(() => localStorage.setItem(STORAGE_KEY, "en"));
  afterEach(cleanup);

  it("a suggestion card starts the ride", async () => {
    const api = fakeApi();
    render(
      <LanguageProvider>
        <RideSection api={api} onPick={() => {}} />
      </LanguageProvider>,
    );
    fireEvent.click(await screen.findByRole("button", { name: /Towards the stagnation point/ }));
    await waitFor(() =>
      expect(api.call).toHaveBeenCalledWith(
        "ride.start",
        expect.objectContaining({ suggestion: "stagnation" }),
      ),
    );
  });

  it("the pick button asks for a click in the view", async () => {
    const onPick = vi.fn();
    render(
      <LanguageProvider>
        <RideSection api={fakeApi()} onPick={onPick} />
      </LanguageProvider>,
    );
    fireEvent.click(await screen.findByRole("button", { name: "Pick a particle in the view" }));
    expect(onPick).toHaveBeenCalled();
  });
});

function overlay(api: Api, control = vi.fn(async () => null)) {
  let push: (c: RideClock) => void = () => {};
  const subscribe = (h: (c: RideClock) => void) => {
    push = h;
    return () => {};
  };
  render(
    <LanguageProvider>
      <RideOverlay api={api} revision={0} units="metric" control={control} onClock={subscribe} />
    </LanguageProvider>,
  );
  return { control, push: (c: RideClock) => act(() => push(c)) };
}

describe("ride overlay", () => {
  beforeEach(() => localStorage.setItem(STORAGE_KEY, "en"));
  afterEach(cleanup);

  it("shows nothing without a ride", async () => {
    const api = fakeApi(null);
    overlay(api);
    await waitFor(() => expect(api.call).toHaveBeenCalledWith("ride.telemetry", {}));
    expect(screen.queryByRole("toolbar", { name: "Ride" })).toBeNull();
  });

  it("the bar pauses, jumps, changes speed, mode and view, and Esc ends the ride", async () => {
    const api = fakeApi();
    const { control, push } = overlay(api);
    await screen.findByRole("toolbar", { name: "Ride" });
    push({ t: 1, duration: 10, playing: true });
    fireEvent.click(screen.getByRole("button", { name: "Pause" }));
    expect(control).toHaveBeenCalledWith({ kind: "pause" });
    fireEvent.change(screen.getByRole("slider", { name: "Position" }), { target: { value: "5" } });
    expect(control).toHaveBeenCalledWith({ kind: "seek", t: 5 });
    fireEvent.change(screen.getByRole("slider", { name: "Speed" }), { target: { value: "2" } });
    await waitFor(() => expect(api.call).toHaveBeenCalledWith("ride.start", { speed: 2 }));
    fireEvent.click(screen.getByRole("button", { name: "Roller coaster" }));
    await waitFor(() => expect(api.call).toHaveBeenCalledWith("ride.start", { mode: "coaster" }));
    fireEvent.click(screen.getByRole("button", { name: "Chase" }));
    await waitFor(() => expect(api.call).toHaveBeenCalledWith("ride.start", { view: "chase" }));
    // Keys: space pauses (the clock says it plays again), arrows jump 5 %, 3 picks the overview.
    push({ t: 1, duration: 10, playing: true });
    fireEvent.keyDown(window, { key: " " });
    expect(control).toHaveBeenLastCalledWith({ kind: "pause" });
    fireEvent.keyDown(window, { key: "ArrowRight" });
    expect(control).toHaveBeenLastCalledWith({ kind: "seek", t: 1.5 });
    fireEvent.keyDown(window, { key: "3" });
    await waitFor(() => expect(api.call).toHaveBeenCalledWith("ride.start", { view: "overview" }));
    fireEvent.keyDown(window, { key: "Escape" });
    await waitFor(() => expect(api.call).toHaveBeenCalledWith("ride.stop", {}));
  });

  it("the HUD shows the readings at the sent time and the sentence at an event", async () => {
    const api = fakeApi();
    const { push } = overlay(api);
    await screen.findByRole("toolbar", { name: "Ride" });
    push({ t: 2.1, duration: 10, playing: true });
    const hud = await screen.findByRole("region", { name: "Readings" });
    // 14.8 m/s ≈ 53 km/h, 74 % of the oncoming flow.
    expect(hud.textContent).toContain("53");
    expect(hud.textContent).toContain("74 %");
    expect(hud.textContent).toMatch(/Cp\s*0\.39/);
    expect(await screen.findByText("The flow piles up.")).toBeTruthy();
    push({ t: 5, duration: 10, playing: true });
    await waitFor(() => expect(screen.queryByText("The flow piles up.")).toBeNull());
    // Event markers sit on the timeline.
    expect(screen.getAllByRole("listitem", { name: /event/i }).length).toBe(2);
  });
});
