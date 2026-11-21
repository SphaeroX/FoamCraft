// SPDX-License-Identifier: GPL-3.0-or-later
// @vitest-environment jsdom
// Light trails, ribbons and tubes (Task_35): the style list, the sliders of each style, the hint
// "trails shortened" and the long exposure of the export.
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { Api } from "../../app/src/api";
import { ExportDialog } from "../../app/src/components/ExportDialog";
import { ParticlesSection } from "../../app/src/components/ParticlesSection";
import { LanguageProvider, STORAGE_KEY } from "../../app/src/i18n/i18n";

const levels = [{ min: [-1, -2, 0], max: [3, 2, 2], dims: [40, 40, 20] }];

function particles(style: string, extra: Record<string, unknown> = {}) {
  return {
    particles: {
      style,
      count: 50000,
      seed: 1,
      color_by: "speed",
      speed: 1,
      paused: false,
      region: { kind: "area", origin: [-0.9, -1, 0.1], u: [0, 2, 0], v: [0, 0, 1] },
      ...extra,
    },
  };
}

function section(state: Record<string, unknown>) {
  const call = vi.fn(async () => state);
  render(
    <LanguageProvider>
      <ParticlesSection api={{ call } as unknown as Api} levels={levels} />
    </LanguageProvider>,
  );
  return call;
}

describe("light trails and ribbons", () => {
  beforeEach(() => localStorage.setItem(STORAGE_KEY, "en"));
  afterEach(cleanup);

  it("offers the new styles", async () => {
    const call = section(particles("comets"));
    const style = await screen.findByRole("combobox", { name: "Style" });
    for (const name of ["Light trails", "Ribbons", "Tubes"]) {
      expect(screen.getByRole("option", { name })).toBeInTheDocument();
    }
    fireEvent.change(style, { target: { value: "trails" } });
    await waitFor(() =>
      expect(call).toHaveBeenCalledWith("view.set_particles", { style: "trails" }),
    );
    // Comets have no trail sliders.
    expect(screen.queryByRole("slider", { name: "Trail length" })).toBeNull();
  });

  it("shows the sliders of the trails and sends them", async () => {
    const call = section(particles("trails", { glow: true, exposure: 1, width: 2.5 }));
    fireEvent.change(await screen.findByRole("slider", { name: "Trail length" }), {
      target: { value: "5" },
    });
    await waitFor(() =>
      expect(call).toHaveBeenCalledWith("view.set_particles", { trail_factor: 5 }),
    );
    fireEvent.change(screen.getByRole("slider", { name: "Width" }), { target: { value: "3.5" } });
    await waitFor(() => expect(call).toHaveBeenCalledWith("view.set_particles", { width: 3.5 }));
    fireEvent.change(screen.getByRole("slider", { name: "Brightness" }), {
      target: { value: "2" },
    });
    await waitFor(() => expect(call).toHaveBeenCalledWith("view.set_particles", { exposure: 2 }));
    fireEvent.click(screen.getByRole("checkbox", { name: "Glow" }));
    await waitFor(() => expect(call).toHaveBeenCalledWith("view.set_particles", { glow: false }));
    fireEvent.click(screen.getByRole("checkbox", { name: "Long exposure" }));
    await waitFor(() =>
      expect(call).toHaveBeenCalledWith("view.set_particles", { long_exposure: 2 }),
    );
  });

  it("changes the half-life and switches the long exposure off", async () => {
    const call = section(particles("trails", { long_exposure: 2 }));
    fireEvent.change(await screen.findByRole("slider", { name: "Half-life (s)" }), {
      target: { value: "6" },
    });
    await waitFor(() =>
      expect(call).toHaveBeenCalledWith("view.set_particles", { long_exposure: 6 }),
    );
    fireEvent.click(screen.getByRole("checkbox", { name: "Long exposure" }));
    await waitFor(() =>
      expect(call).toHaveBeenCalledWith("view.set_particles", { long_exposure: null }),
    );
  });

  it("says when the memory limit shortens the trails", async () => {
    section(particles("trails", { count: 200000 }));
    expect(await screen.findByText(/Trails shortened/)).toBeInTheDocument();
    cleanup();
    section(particles("trails"));
    await screen.findByRole("slider", { name: "Trail length" });
    expect(screen.queryByText(/Trails shortened/)).toBeNull();
  });

  it("colours ribbons by swirl and sets their width", async () => {
    const call = section(particles("ribbons"));
    const colour = await screen.findByRole("combobox", { name: "Colour by" });
    fireEvent.change(colour, { target: { value: "swirl" } });
    await waitFor(() =>
      expect(call).toHaveBeenCalledWith("view.set_particles", { color_by: "swirl" }),
    );
    fireEvent.change(screen.getByRole("slider", { name: "Width" }), {
      target: { value: "0.02" },
    });
    await waitFor(() => expect(call).toHaveBeenCalledWith("view.set_particles", { width: 0.02 }));
    expect(screen.queryByRole("slider", { name: "Trail length" })).toBeNull();
  });
});

describe("long exposure in the export", () => {
  beforeEach(() => localStorage.setItem(STORAGE_KEY, "en"));
  afterEach(cleanup);

  it("passes exposure_s with the image", async () => {
    const call = vi.fn(async (name: string) =>
      name === "export.preview"
        ? { width: 480, height: 270, png_base64: "AAAA" }
        : { path: "C:/out/shot.png", width: 1920, height: 1080 },
    );
    render(
      <LanguageProvider>
        <ExportDialog
          api={{ call } as unknown as Api}
          pickSavePath={async (ext: string) => `C:/out/file.${ext}`}
          onClose={() => {}}
        />
      </LanguageProvider>,
    );
    fireEvent.click(await screen.findByRole("checkbox", { name: "Long exposure" }));
    fireEvent.change(screen.getByRole("spinbutton", { name: "Exposure time (s)" }), {
      target: { value: "3" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Save image" }));
    await waitFor(() =>
      expect(call).toHaveBeenCalledWith("export.image", {
        path: "C:/out/file.png",
        resolution: "full_hd",
        transparent: false,
        legend: true,
        exposure_s: 3,
      }),
    );
  });
});
