// SPDX-License-Identifier: GPL-3.0-or-later
// @vitest-environment jsdom
import { act, cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { MoveTools, formatHandleValue } from "../../app/src/components/MoveTools";
import { ViewportInput } from "../../app/src/components/ViewportInput";
import type { Api } from "../../app/src/api";
import { LanguageProvider, STORAGE_KEY } from "../../app/src/i18n/i18n";

const PIVOTS = {
  center: [0.01, 0.01, 0.01],
  centroid: [0.012, 0.01, 0.009],
  origin: [0, 0, 0],
};

function fakeApi() {
  const api = {
    call: vi.fn(async (name: string) => {
      if (name === "geometry.pivot") return PIVOTS;
      return {};
    }),
  };
  return api as typeof api & Api;
}

describe("MoveTools", () => {
  beforeEach(() => localStorage.setItem(STORAGE_KEY, "en"));
  afterEach(cleanup);

  it("asks for the pivots and reports the chosen one", async () => {
    const api = fakeApi();
    const onPivot = vi.fn();
    render(
      <LanguageProvider>
        <MoveTools api={api} model="model-1" revision={0} value={null} onPivot={onPivot} />
      </LanguageProvider>,
    );
    await act(async () => {});
    expect(api.call).toHaveBeenCalledWith("geometry.pivot", { id: "model-1" });
    expect(onPivot).toHaveBeenLastCalledWith("model-1", PIVOTS.center);
    await act(async () => {
      fireEvent.change(screen.getByRole("combobox", { name: "Handle position" }), {
        target: { value: "centroid" },
      });
    });
    expect(onPivot).toHaveBeenLastCalledWith("model-1", PIVOTS.centroid);
  });

  it("shows the value while dragging", () => {
    render(
      <LanguageProvider>
        <MoveTools
          api={fakeApi()}
          model="model-1"
          revision={0}
          value={{ kind: "move", axis: "x", value: 0.012 }}
          onPivot={() => {}}
        />
      </LanguageProvider>,
    );
    expect(screen.getByRole("status")).toHaveTextContent("X +12 mm");
  });

  it("formats moves in millimetres and turns in degrees", () => {
    expect(formatHandleValue({ kind: "move", axis: "z", value: -0.0005 })).toBe("Z −0.5 mm");
    expect(formatHandleValue({ kind: "rotate", axis: "y", value: 15 })).toBe("Y +15°");
  });
});

describe("ViewportInput with handles", () => {
  afterEach(cleanup);

  it("drags a handle instead of the camera when the mouse is over one", () => {
    const send = vi.fn();
    const handles = {
      keyUnderMouse: () => "handle:move-x",
      begin: vi.fn(),
      drag: vi.fn(),
      end: vi.fn(),
      cancel: vi.fn(),
    };
    render(<ViewportInput send={send} handles={handles} />);
    const area = screen.getByTestId("viewport-input");
    fireEvent.mouseDown(area, { button: 0, clientX: 100, clientY: 100 });
    expect(handles.begin).toHaveBeenCalledWith("handle:move-x", 100, 100);
    fireEvent.mouseMove(window, { clientX: 120, clientY: 100, shiftKey: true });
    expect(handles.drag).toHaveBeenLastCalledWith(120, 100, true);
    fireEvent.mouseUp(window, { clientX: 120, clientY: 100 });
    expect(handles.end).toHaveBeenCalled();
    expect(send).not.toHaveBeenCalled();
  });

  it("cancels a handle drag with Escape", () => {
    const handles = {
      keyUnderMouse: () => "handle:rotate-z",
      begin: vi.fn(),
      drag: vi.fn(),
      end: vi.fn(),
      cancel: vi.fn(),
    };
    render(<ViewportInput send={vi.fn()} handles={handles} />);
    fireEvent.mouseDown(screen.getByTestId("viewport-input"), {
      button: 0,
      clientX: 10,
      clientY: 10,
    });
    fireEvent.keyDown(window, { key: "Escape" });
    expect(handles.cancel).toHaveBeenCalled();
    fireEvent.mouseUp(window, { clientX: 10, clientY: 10 });
    expect(handles.end).not.toHaveBeenCalled();
  });

  it("orbits as before when the mouse is not over a handle", () => {
    const send = vi.fn();
    const handles = {
      keyUnderMouse: () => "model-1:face-2",
      begin: vi.fn(),
      drag: vi.fn(),
      end: vi.fn(),
      cancel: vi.fn(),
    };
    render(<ViewportInput send={send} handles={handles} />);
    const area = screen.getByTestId("viewport-input");
    fireEvent.mouseDown(area, { button: 0, clientX: 100, clientY: 100 });
    fireEvent.mouseMove(window, { clientX: 110, clientY: 100 });
    expect(handles.begin).not.toHaveBeenCalled();
    expect(send).toHaveBeenCalled();
  });
});

describe("ViewportInput with slices", () => {
  afterEach(cleanup);

  function handles(key: string) {
    return {
      keyUnderMouse: () => key,
      begin: vi.fn(),
      drag: vi.fn(),
      end: vi.fn(),
      cancel: vi.fn(),
    };
  }

  it("drags a slice like a handle", () => {
    const h = handles("slice:slice-1");
    render(<ViewportInput send={vi.fn()} handles={h} />);
    fireEvent.mouseDown(screen.getByTestId("viewport-input"), {
      button: 0,
      clientX: 50,
      clientY: 50,
    });
    expect(h.begin).toHaveBeenCalledWith("slice:slice-1", 50, 50);
    fireEvent.mouseMove(window, { clientX: 50, clientY: 80 });
    fireEvent.mouseUp(window, { clientX: 50, clientY: 80 });
    expect(h.end).toHaveBeenCalled();
  });

  it("a click on a slice without moving picks it instead", () => {
    const h = handles("slice:slice-1");
    const onPick = vi.fn();
    render(<ViewportInput send={vi.fn()} handles={h} onPick={onPick} />);
    fireEvent.mouseDown(screen.getByTestId("viewport-input"), {
      button: 0,
      clientX: 50,
      clientY: 50,
    });
    fireEvent.mouseUp(window, { clientX: 50, clientY: 50 });
    expect(h.cancel).toHaveBeenCalled();
    expect(h.end).not.toHaveBeenCalled();
    expect(onPick).toHaveBeenCalledWith(50, 50, "replace");
  });
});
