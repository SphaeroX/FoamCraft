// SPDX-License-Identifier: GPL-3.0-or-later
// @vitest-environment jsdom
// The demo of the ready-made views ends with a ride (Task_32).
import { act, cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { Api } from "../../app/src/api";
import { PresetGallery } from "../../app/src/components/PresetGallery";
import { DemoPlayer } from "../../app/src/demo";
import { LanguageProvider, STORAGE_KEY } from "../../app/src/i18n/i18n";

describe("demo player durations", () => {
  afterEach(() => vi.useRealTimers());

  it("a step can last longer than the others", () => {
    vi.useFakeTimers();
    const shown: string[] = [];
    const p = new DemoPlayer(["a", "long", "b"], (s) => shown.push(s), {
      stepMs: 1000,
      durationOf: (s) => (s === "long" ? 5000 : 1000),
    });
    p.start();
    vi.advanceTimersByTime(1000);
    expect(shown).toEqual(["a", "long"]);
    vi.advanceTimersByTime(4000);
    expect(shown).toEqual(["a", "long"]);
    vi.advanceTimersByTime(1000);
    expect(shown).toEqual(["a", "long", "b"]);
  });
});

describe("demo with a ride", () => {
  beforeEach(() => localStorage.setItem(STORAGE_KEY, "en"));
  afterEach(() => {
    vi.useRealTimers();
    cleanup();
  });

  it("rides towards the stagnation point for 12 s after the views and ends the ride", async () => {
    const call = vi.fn(async (name: string) => {
      switch (name) {
        case "view.presets":
          return [{ id: "smoke_lines", view: "right", explanation: "preset.smoke_lines" }];
        case "ride.suggest":
          return [{ id: "hugging" }, { id: "stagnation" }];
        default:
          return {};
      }
    });
    render(
      <LanguageProvider>
        <PresetGallery api={{ call } as unknown as Api} onView={() => {}} />
      </LanguageProvider>,
    );
    await screen.findByRole("button", { name: "Smoke lines" });
    await waitFor(() => expect(call).toHaveBeenCalledWith("ride.suggest", {}));
    vi.useFakeTimers({ shouldAdvanceTime: true });
    fireEvent.click(screen.getByRole("button", { name: "Start demo" }));
    await act(async () => {
      vi.advanceTimersByTime(8_000);
    });
    expect(call).toHaveBeenCalledWith("ride.start", { suggestion: "stagnation" });
    await act(async () => {
      vi.advanceTimersByTime(11_000);
    });
    expect(call).not.toHaveBeenCalledWith("ride.stop", {});
    await act(async () => {
      vi.advanceTimersByTime(1_500);
    });
    await waitFor(() => expect(call).toHaveBeenCalledWith("ride.stop", {}));
    expect(screen.getByRole("button", { name: "Start demo" })).toBeInTheDocument();
  });
});
