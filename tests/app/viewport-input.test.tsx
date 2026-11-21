// SPDX-License-Identifier: GPL-3.0-or-later
// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { ViewportInput } from "../../app/src/components/ViewportInput";
import { dragToCamera, wheelToZoom } from "../../app/src/viewport-input";

describe("viewport input mapping", () => {
  it("left drag orbits, right and middle drag pan", () => {
    expect(dragToCamera(0, 10, -5, 500)).toEqual({ kind: "orbit", yaw: -0.1, pitch: 0.05 });
    expect(dragToCamera(2, 50, 25, 500)).toEqual({ kind: "pan", dx: 0.1, dy: 0.05 });
    expect(dragToCamera(1, 50, 25, 500)).toEqual({ kind: "pan", dx: 0.1, dy: 0.05 });
  });

  it("wheel down zooms out, wheel up zooms in", () => {
    expect(wheelToZoom(100)).toBeGreaterThan(1);
    expect(wheelToZoom(-100)).toBeLessThan(1);
    expect(wheelToZoom(0)).toBe(1);
  });
});

describe("ViewportInput", () => {
  afterEach(cleanup);

  function setup() {
    const send = vi.fn();
    render(
      <div>
        <ViewportInput send={send} />
        <button type="button">control</button>
      </div>,
    );
    return send;
  }

  it("sends camera actions for drags over the free area", () => {
    const send = setup();
    const area = screen.getByTestId("viewport-input");
    fireEvent.mouseDown(area, { button: 0, clientX: 100, clientY: 100 });
    fireEvent.mouseMove(window, { clientX: 110, clientY: 100 });
    fireEvent.mouseUp(window, { button: 0 });
    fireEvent.mouseMove(window, { clientX: 150, clientY: 100 });
    expect(send).toHaveBeenCalledTimes(1);
    expect(send.mock.calls[0][0].kind).toBe("orbit");
    fireEvent.wheel(area, { deltaY: 100 });
    expect(send.mock.calls[1][0]).toEqual({ kind: "zoom", factor: wheelToZoom(100) });
  });

  it("ignores drags that start on interface controls", () => {
    const send = setup();
    const button = screen.getByRole("button");
    fireEvent.mouseDown(button, { button: 0, clientX: 100, clientY: 100 });
    fireEvent.mouseMove(window, { clientX: 140, clientY: 100 });
    fireEvent.wheel(button, { deltaY: 100 });
    expect(send).not.toHaveBeenCalled();
  });
});

describe("ViewportInput clicks and hover", () => {
  afterEach(cleanup);

  it("reports a click without movement with its modifier mode", () => {
    const send = vi.fn();
    const onPick = vi.fn();
    render(<ViewportInput send={send} onPick={onPick} />);
    const area = screen.getByTestId("viewport-input");
    fireEvent.mouseDown(area, { button: 0, clientX: 40, clientY: 50 });
    fireEvent.mouseUp(window, { button: 0, clientX: 41, clientY: 50 });
    expect(onPick).toHaveBeenCalledWith(40, 50, "replace");
    fireEvent.mouseDown(area, { button: 0, clientX: 10, clientY: 10, shiftKey: true });
    fireEvent.mouseUp(window, { button: 0, clientX: 10, clientY: 10, shiftKey: true });
    expect(onPick).toHaveBeenLastCalledWith(10, 10, "add");
    fireEvent.mouseDown(area, { button: 0, clientX: 10, clientY: 10, ctrlKey: true });
    fireEvent.mouseUp(window, { button: 0, clientX: 10, clientY: 10, ctrlKey: true });
    expect(onPick).toHaveBeenLastCalledWith(10, 10, "toggle");
    expect(send).not.toHaveBeenCalled();
  });

  it("a drag is not a click", () => {
    const onPick = vi.fn();
    render(<ViewportInput send={vi.fn()} onPick={onPick} />);
    const area = screen.getByTestId("viewport-input");
    fireEvent.mouseDown(area, { button: 0, clientX: 40, clientY: 50 });
    fireEvent.mouseMove(window, { clientX: 80, clientY: 50 });
    fireEvent.mouseUp(window, { button: 0, clientX: 80, clientY: 50 });
    expect(onPick).not.toHaveBeenCalled();
  });

  it("reports hover over the free area", () => {
    const onHover = vi.fn();
    render(<ViewportInput send={vi.fn()} onHover={onHover} />);
    fireEvent.mouseMove(screen.getByTestId("viewport-input"), { clientX: 5, clientY: 6 });
    expect(onHover).toHaveBeenCalledWith(5, 6);
  });
});
