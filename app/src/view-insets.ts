// SPDX-License-Identifier: GPL-3.0-or-later
// The part of the 3D view not covered by the side panels; the split view of two runs uses it.

export interface PanelBox {
  left: number;
  right: number;
  side: "left" | "right";
}

/** Widths covered on the left and on the right (CSS pixels); none when too little stays free. */
export function viewInsets(panels: PanelBox[], width: number): { left: number; right: number } {
  const left = Math.max(0, ...panels.filter((p) => p.side === "left").map((p) => p.right));
  const right = Math.max(0, ...panels.filter((p) => p.side === "right").map((p) => width - p.left));
  if (width - left - right < width / 3) return { left: 0, right: 0 };
  return { left, right };
}

/** The panels shown now, measured in the document. */
export function measurePanels(): PanelBox[] {
  return Array.from(document.querySelectorAll<HTMLElement>(".side-panel")).map((el) => {
    const r = el.getBoundingClientRect();
    return {
      left: r.left,
      right: r.right,
      side: el.classList.contains("right") ? "right" : "left",
    };
  });
}
