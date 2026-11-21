// SPDX-License-Identifier: GPL-3.0-or-later
// Transparent layer over the native 3D view. Only input that starts here moves the camera or picks
// faces; interface controls sit above it and keep their own input.
import { useEffect, useRef } from "react";
import { dragToCamera, wheelToZoom, type CameraAction } from "../viewport-input";

export type PickMode = "replace" | "add" | "toggle";

/** Handles and slices in the view: a press on one drags it instead of the camera. */
export interface HandleInput {
  /** Key of what is under the mouse now (from the last hover), e.g. `handle:move-x`. */
  keyUnderMouse: () => string | null;
  begin: (key: string, x: number, y: number) => void;
  /** `free` (Shift held) turns snapping off. */
  drag: (x: number, y: number, free: boolean) => void;
  end: () => void;
  cancel: () => void;
}

interface Props {
  send: (action: CameraAction) => void;
  /** Click without dragging, in CSS pixels; Shift adds, Ctrl toggles. */
  onPick?: (x: number, y: number, mode: PickMode) => void;
  /** Mouse moved over the free area without a button pressed. */
  onHover?: (x: number, y: number) => void;
  handles?: HandleInput;
}

/** Movement (pixels) below which a press counts as a click. */
const CLICK_TOLERANCE = 3;

interface Drag {
  button: number;
  x: number;
  y: number;
  moved: number;
  /** Set when the press started on a handle. */
  handle?: string;
}

export function ViewportInput({ send, onPick, onHover, handles }: Props) {
  const drag = useRef<Drag | null>(null);
  const area = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const onMove = (e: MouseEvent) => {
      const d = drag.current;
      if (!d) return;
      const dx = e.clientX - d.x;
      const dy = e.clientY - d.y;
      if (dx === 0 && dy === 0) return;
      const moved = d.moved + Math.abs(dx) + Math.abs(dy);
      drag.current = { ...d, x: e.clientX, y: e.clientY, moved };
      if (d.handle) {
        handles?.drag(e.clientX, e.clientY, e.shiftKey);
        return;
      }
      if (moved > CLICK_TOLERANCE) {
        send(dragToCamera(d.button, dx, dy, area.current?.clientHeight || window.innerHeight));
      }
    };
    const onUp = (e: MouseEvent) => {
      const d = drag.current;
      drag.current = null;
      if (d?.handle) {
        // Pressed and released in place: a click (selects a slice) rather than a drag.
        if (d.moved <= CLICK_TOLERANCE && onPick) {
          handles?.cancel();
          const mode: PickMode = e.shiftKey ? "add" : e.ctrlKey ? "toggle" : "replace";
          onPick(d.x, d.y, mode);
          return;
        }
        handles?.end();
        return;
      }
      if (d && d.button === 0 && d.moved <= CLICK_TOLERANCE && onPick) {
        const mode: PickMode = e.shiftKey ? "add" : e.ctrlKey ? "toggle" : "replace";
        onPick(d.x, d.y, mode);
      }
    };
    const onKey = (e: KeyboardEvent) => {
      if (e.key !== "Escape" || !drag.current?.handle) return;
      drag.current = null;
      handles?.cancel();
    };
    window.addEventListener("mousemove", onMove);
    window.addEventListener("mouseup", onUp);
    window.addEventListener("keydown", onKey);
    return () => {
      window.removeEventListener("mousemove", onMove);
      window.removeEventListener("mouseup", onUp);
      window.removeEventListener("keydown", onKey);
    };
  }, [send, onPick, handles]);

  return (
    <div
      ref={area}
      className="viewport-input"
      data-testid="viewport-input"
      onMouseDown={(e) => {
        if (e.target !== e.currentTarget) return;
        const key = e.button === 0 ? handles?.keyUnderMouse() : null;
        // Handles and slices are dragged instead of turning the camera.
        const handle = key?.startsWith("handle:") || key?.startsWith("slice:") ? key : undefined;
        drag.current = { button: e.button, x: e.clientX, y: e.clientY, moved: 0, handle };
        if (handle) handles?.begin(handle, e.clientX, e.clientY);
        e.preventDefault();
      }}
      onMouseMove={(e) => {
        if (!drag.current && e.target === e.currentTarget) onHover?.(e.clientX, e.clientY);
      }}
      onWheel={(e) => {
        if (e.target !== e.currentTarget) return;
        send({ kind: "zoom", factor: wheelToZoom(e.deltaY) });
      }}
      onContextMenu={(e) => e.preventDefault()}
    />
  );
}
