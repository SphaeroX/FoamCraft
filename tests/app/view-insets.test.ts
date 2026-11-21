// SPDX-License-Identifier: GPL-3.0-or-later
import { describe, expect, it } from "vitest";
import { viewInsets } from "../../app/src/view-insets";

describe("free area of the 3D view", () => {
  it("is the width between the left and the right panels", () => {
    const panels = [
      { left: 8, right: 333, side: "left" as const },
      { left: 996, right: 1288, side: "right" as const },
    ];
    expect(viewInsets(panels, 1296)).toEqual({ left: 333, right: 1296 - 996 });
  });

  it("is the whole window without panels", () => {
    expect(viewInsets([], 1200)).toEqual({ left: 0, right: 0 });
  });

  it("keeps at least a third of the window free", () => {
    const panels = [
      { left: 0, right: 500, side: "left" as const },
      { left: 600, right: 900, side: "right" as const },
    ];
    expect(viewInsets(panels, 900)).toEqual({ left: 0, right: 0 });
  });
});
