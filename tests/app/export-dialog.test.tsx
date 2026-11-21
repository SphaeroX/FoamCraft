// SPDX-License-Identifier: GPL-3.0-or-later
// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { Api } from "../../app/src/api";
import { ExportDialog } from "../../app/src/components/ExportDialog";
import { LanguageProvider, STORAGE_KEY } from "../../app/src/i18n/i18n";

function setup() {
  let polls = 0;
  const call = vi.fn(async (name: string) => {
    switch (name) {
      case "export.preview":
        return { width: 480, height: 270, png_base64: "AAAA" };
      case "export.image":
        return { path: "C:/out/shot.png", width: 3840, height: 2160 };
      case "export.video":
        return { export_id: "export-1", path: "C:/out/clip.avi", frames: 240 };
      case "export.status":
        polls += 1;
        return polls < 2
          ? {
              export_id: "export-1",
              state: "running",
              frame: 12,
              frames: 240,
              path: "C:/out/clip.avi",
            }
          : {
              export_id: "export-1",
              state: "done",
              frame: 240,
              frames: 240,
              path: "C:/out/clip.avi",
            };
      case "export.cancel":
        return {};
    }
    throw new Error(name);
  });
  const pickSavePath = vi.fn(async (ext: string) => `C:/out/file.${ext}`);
  render(
    <LanguageProvider>
      <ExportDialog
        api={{ call } as unknown as Api}
        pickSavePath={pickSavePath}
        onClose={() => {}}
        pollMs={10}
      />
    </LanguageProvider>,
  );
  return { call, pickSavePath };
}

describe("export dialog", () => {
  beforeEach(() => localStorage.setItem(STORAGE_KEY, "en"));
  afterEach(cleanup);

  it("shows a preview of the timeline and follows the slider", async () => {
    const { call } = setup();
    await waitFor(() =>
      expect(call).toHaveBeenCalledWith("export.preview", {
        width: 480,
        height: 270,
        time_s: 0,
        duration_s: 8,
        camera: { preset: "turntable" },
        legend: true,
      }),
    );
    const img = await screen.findByRole("img", { name: "Preview" });
    expect(img).toHaveAttribute("src", "data:image/png;base64,AAAA");
    fireEvent.change(screen.getByRole("slider", { name: "Time" }), { target: { value: "4" } });
    await waitFor(() =>
      expect(call).toHaveBeenCalledWith("export.preview", expect.objectContaining({ time_s: 4 })),
    );
  });

  it("saves a 4K image with transparent background", async () => {
    const { call, pickSavePath } = setup();
    fireEvent.change(await screen.findByRole("combobox", { name: "Resolution" }), {
      target: { value: "4k" },
    });
    fireEvent.click(screen.getByRole("checkbox", { name: "Transparent background" }));
    fireEvent.click(screen.getByRole("button", { name: "Save image" }));
    await waitFor(() =>
      expect(call).toHaveBeenCalledWith("export.image", {
        path: "C:/out/file.png",
        resolution: "4k",
        transparent: true,
        legend: true,
      }),
    );
    expect(pickSavePath).toHaveBeenCalledWith("png");
    expect(await screen.findByText(/C:\/out\/shot\.png/)).toBeInTheDocument();
  });

  it("renders a video with progress until it is done", async () => {
    const { call } = setup();
    fireEvent.change(await screen.findByRole("combobox", { name: "Camera path" }), {
      target: { value: "swing" },
    });
    fireEvent.change(screen.getByRole("combobox", { name: "Frame rate" }), {
      target: { value: "60" },
    });
    fireEvent.change(screen.getByRole("spinbutton", { name: "Duration (s)" }), {
      target: { value: "4" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Save video" }));
    await waitFor(() =>
      expect(call).toHaveBeenCalledWith("export.video", {
        path: "C:/out/file.avi",
        resolution: "full_hd",
        fps: 60,
        duration_s: 4,
        camera: { preset: "swing" },
        legend: true,
      }),
    );
    expect(await screen.findByText(/Saved: C:\/out\/clip\.avi/)).toBeInTheDocument();
    expect(call).toHaveBeenCalledWith("export.status", { export_id: "export-1" });
  });

  it("can cancel a running video", async () => {
    const { call } = setup();
    fireEvent.click(await screen.findByRole("button", { name: "Save video" }));
    fireEvent.click(await screen.findByRole("button", { name: "Cancel export" }));
    await waitFor(() =>
      expect(call).toHaveBeenCalledWith("export.cancel", { export_id: "export-1" }),
    );
  });
});
