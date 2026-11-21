// SPDX-License-Identifier: GPL-3.0-or-later
// @vitest-environment jsdom
import { act, cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { ViewCube } from "../../app/src/components/ViewCube";
import { LanguageProvider, STORAGE_KEY } from "../../app/src/i18n/i18n";
import de from "../../app/src/i18n/de.json";
import en from "../../app/src/i18n/en.json";
import { CUBE_VIEWS, cubeMatrix, namedView, toScreen, viewNormal } from "../../app/src/view-cube";

const DEG = Math.PI / 180;

describe("view cube maths", () => {
  it("knows the 26 cube targets", () => {
    expect(CUBE_VIEWS).toHaveLength(26);
    expect(new Set(CUBE_VIEWS).size).toBe(26);
  });

  it("turns the front face towards the viewer in the front view", () => {
    const [yaw, pitch] = namedView("front");
    const n = toScreen(cubeMatrix(yaw, pitch), [-1, 0, 0]);
    expect(n[2]).toBeCloseTo(1, 4);
  });

  it("turns the top face towards the viewer in the top view, front at the bottom", () => {
    const [yaw, pitch] = namedView("top");
    const m = cubeMatrix(yaw, pitch);
    expect(toScreen(m, [0, 0, 1])[2]).toBeCloseTo(1, 2);
    // CSS y grows downwards: the front (-X) lies below the centre.
    expect(toScreen(m, [-1, 0, 0])[1]).toBeGreaterThan(0.9);
  });

  it("keeps world up pointing up on the screen for a side view", () => {
    const m = cubeMatrix(-60 * DEG, 25 * DEG);
    expect(toScreen(m, [0, 0, 1])[1]).toBeLessThan(0);
  });

  it("gives the eye direction of every target", () => {
    expect(viewNormal("top_front_right")).toEqual([-1, -1, 1]);
    expect(viewNormal("back_left")).toEqual([1, 1, 0]);
  });
});

describe("ViewCube", () => {
  beforeEach(() => localStorage.setItem(STORAGE_KEY, "en"));
  afterEach(cleanup);

  function setup(yaw = -60 * DEG, pitch = 25 * DEG) {
    const onView = vi.fn();
    const result = render(
      <LanguageProvider>
        <ViewCube yaw={yaw} pitch={pitch} onView={onView} />
      </LanguageProvider>,
    );
    return { onView, ...result };
  }

  it("sends the view of a clicked face, edge or corner", () => {
    const { onView } = setup();
    fireEvent.click(screen.getByRole("button", { name: "View from the front" }));
    expect(onView).toHaveBeenLastCalledWith("front");
    fireEvent.click(screen.getByRole("button", { name: "View from top and front" }));
    expect(onView).toHaveBeenLastCalledWith("top_front");
    fireEvent.click(screen.getByRole("button", { name: "View from top, front and right" }));
    expect(onView).toHaveBeenLastCalledWith("top_front_right");
  });

  it("has a button for the default view", () => {
    const { onView } = setup();
    fireEvent.click(screen.getByRole("button", { name: "Default view" }));
    expect(onView).toHaveBeenLastCalledWith("default");
  });

  it("offers all 27 targets as named buttons", () => {
    setup();
    const names = screen.getAllByRole("button").map((b) => b.getAttribute("aria-label"));
    expect(names).toHaveLength(27);
    expect(names.every((n) => n && n.length > 0)).toBe(true);
  });

  it("follows the reported camera", () => {
    const { rerender } = setup(0, 0);
    const cube = screen.getByTestId("view-cube-body");
    const before = cube.style.transform;
    act(() => {
      rerender(
        <LanguageProvider>
          <ViewCube yaw={1} pitch={0.3} onView={() => {}} />
        </LanguageProvider>,
      );
    });
    expect(cube.style.transform).toMatch(/^matrix3d\(/);
    expect(cube.style.transform).not.toBe(before);
  });

  it("has names in both languages for every target", () => {
    for (const lang of [en, de] as const) {
      const cube = (lang as unknown as { viewCube: Record<string, string> }).viewCube;
      for (const name of [...CUBE_VIEWS, "default"]) {
        expect(cube[name], name).toBeTruthy();
      }
    }
  });
});
