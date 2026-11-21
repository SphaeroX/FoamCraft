// SPDX-License-Identifier: GPL-3.0-or-later
// @vitest-environment jsdom
import { act, cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { Api } from "../../app/src/api";
import { PresetGallery } from "../../app/src/components/PresetGallery";
import { DemoPlayer } from "../../app/src/demo";
import { LanguageProvider, STORAGE_KEY } from "../../app/src/i18n/i18n";

const msg = (en: string, de: string) => ({ en: { text: en, fix: "" }, de: { text: de, fix: "" } });

const PRESETS = [
  { id: "smoke_lines", view: "right", explanation: "preset.smoke_lines" },
  { id: "mid_slice", view: "right", explanation: "preset.mid_slice" },
  { id: "surface_pressure", view: "default", explanation: "preset.surface_pressure" },
];

function fakeApi() {
  const call = vi.fn(async (name: string) => {
    switch (name) {
      case "view.presets":
        return PRESETS;
      case "error.describe":
        return {
          "preset.smoke_lines": msg("Smoke lines show the flow.", "Rauchfäden."),
          "preset.mid_slice": msg("Speed in the middle.", "Mitte."),
          "preset.surface_pressure": msg("{share}% of the drag is pressure.", "{share} % Druck."),
        };
      case "results.summary":
        return {
          figures: [
            { id: "drag_pressure_n", value: 8.5 },
            { id: "drag_viscous_n", value: 1.5 },
          ],
        };
      default:
        return {};
    }
  });
  return { call } as unknown as Api & { call: typeof call };
}

describe("demo player", () => {
  afterEach(() => vi.useRealTimers());

  it("runs through all steps on its own, then ends", () => {
    vi.useFakeTimers();
    const shown: string[] = [];
    const onEnd = vi.fn();
    const p = new DemoPlayer(["a", "b", "c"], (s) => shown.push(s), { stepMs: 1000, onEnd });
    p.start();
    expect(shown).toEqual(["a"]);
    vi.advanceTimersByTime(1000);
    expect(shown).toEqual(["a", "b"]);
    vi.advanceTimersByTime(2000);
    expect(shown).toEqual(["a", "b", "c"]);
    expect(onEnd).toHaveBeenCalledTimes(1);
    expect(p.state().playing).toBe(false);
  });

  it("pauses, goes back and forth and stops", () => {
    vi.useFakeTimers();
    const shown: string[] = [];
    const onEnd = vi.fn();
    const p = new DemoPlayer(["a", "b", "c"], (s) => shown.push(s), { stepMs: 1000, onEnd });
    p.start();
    p.pause();
    vi.advanceTimersByTime(5000);
    expect(shown).toEqual(["a"]);
    p.next();
    expect(shown).toEqual(["a", "b"]);
    p.prev();
    expect(shown).toEqual(["a", "b", "a"]);
    p.resume();
    vi.advanceTimersByTime(1000);
    expect(shown.at(-1)).toBe("b");
    p.stop();
    expect(onEnd).toHaveBeenCalledTimes(1);
    vi.advanceTimersByTime(5000);
    expect(shown.at(-1)).toBe("b");
  });
});

describe("preset gallery", () => {
  beforeEach(() => localStorage.setItem(STORAGE_KEY, "en"));
  afterEach(() => {
    vi.useRealTimers();
    cleanup();
  });

  function setup() {
    const api = fakeApi();
    const onView = vi.fn();
    render(
      <LanguageProvider>
        <PresetGallery api={api} onView={onView} />
      </LanguageProvider>,
    );
    return { api, onView };
  }

  it("applies a preset, turns the camera and explains it", async () => {
    const { api, onView } = setup();
    fireEvent.click(await screen.findByRole("button", { name: "Smoke lines" }));
    await waitFor(() =>
      expect(api.call).toHaveBeenCalledWith("view.apply_preset", { id: "smoke_lines" }),
    );
    expect(onView).toHaveBeenCalledWith("right");
    expect(await screen.findByText("Smoke lines show the flow.")).toBeInTheDocument();
  });

  it("fills in numbers from the run", async () => {
    setup();
    fireEvent.click(await screen.findByRole("button", { name: "Pressure on the model" }));
    expect(await screen.findByText("85% of the drag is pressure.")).toBeInTheDocument();
  });

  it("the demo shows every preset and undoes them at the end", async () => {
    const { api } = setup();
    await screen.findByRole("button", { name: "Smoke lines" });
    vi.useFakeTimers({ shouldAdvanceTime: true });
    fireEvent.click(screen.getByRole("button", { name: "Start demo" }));
    expect(screen.getByRole("button", { name: "Pause" })).toBeInTheDocument();
    await act(async () => {
      vi.advanceTimersByTime(30_000);
    });
    const applied = api.call.mock.calls.filter((c) => c[0] === "view.apply_preset");
    expect(applied.map((c) => ((c as unknown[])[1] as { id: string }).id)).toEqual([
      "smoke_lines",
      "mid_slice",
      "surface_pressure",
    ]);
    await waitFor(() =>
      expect(api.call.mock.calls.filter((c) => c[0] === "history.undo")).toHaveLength(3),
    );
    expect(screen.getByRole("button", { name: "Start demo" })).toBeInTheDocument();
  });
});
