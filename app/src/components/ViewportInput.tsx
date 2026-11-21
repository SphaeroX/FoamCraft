// SPDX-License-Identifier: GPL-3.0-or-later
// Transparent layer over the native 3D view. Only input that starts here moves the camera or picks
// faces; interface controls sit above it and keep their own input.
import { useEffect, useRef } from "react";
import { dragToCamera, wheelToZoom, type CameraAction } from "../viewport-input";

export type PickMode = "replace" | "add" | "toggle";

interface Props {
  send: (action: CameraAction) => void;
  /** Click without dragging, in CSS pixels; Shift adds, Ctrl toggles. */
  onPick?: (x: number, y: number, mode: PickMode) => void;
  /** Mouse moved over the free area without a button pressed. */
  onHover?: (x: number, y: number) => void;
}

/** Movement (pixels) below which a press counts as a click. */
const CLICK_TOLERANCE = 3;

export function ViewportInput({ send, onPick, onHover }: Props) {
  const drag = useRef<{ button: number; x: number; y: number; moved: number } | null>(null);
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
      if (moved > CLICK_TOLERANCE) {
        send(dragToCamera(d.button, dx, dy, area.current?.clientHeight || window.innerHeight));
      }
    };
    const onUp = (e: MouseEvent) => {
      const d = drag.current;
      drag.current = null;
      if (d && d.button === 0 && d.moved <= CLICK_TOLERANCE && onPick) {
        const mode: PickMode = e.shiftKey ? "add" : e.ctrlKey ? "toggle" : "replace";
        onPick(d.x, d.y, mode);
      }
    };
    window.addEventListener("mousemove", onMove);
    window.addEventListener("mouseup", onUp);
    return () => {
      window.removeEventListener("mousemove", onMove);
      window.removeEventListener("mouseup", onUp);
    };
  }, [send, onPick]);

  return (
    <div
      ref={area}
      className="viewport-input"
      data-testid="viewport-input"
      onMouseDown={(e) => {
        if (e.target !== e.currentTarget) return;
        drag.current = { button: e.button, x: e.clientX, y: e.clientY, moved: 0 };
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
