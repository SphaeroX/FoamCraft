// SPDX-License-Identifier: GPL-3.0-or-later
// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { Api } from "../../app/src/api";
import { ParticlesSection } from "../../app/src/components/ParticlesSection";
import { LanguageProvider, STORAGE_KEY } from "../../app/src/i18n/i18n";

const levels = [{ min: [-1, -2, 0], max: [3, 2, 2], dims: [40, 40, 20] }];
const on = {
  particles: {
    style: "comets",
    count: 50000,
    seed: 1,
    color_by: "speed",
    speed: 1,
    paused: false,
    region: { kind: "area", origin: [-0.9, -1, 0.1], u: [0, 2, 0], v: [0, 0, 1] },
  },
};

function setup(state: Record<string, unknown> = { particles: null }) {
  const call = vi.fn(async (name: string) => (name === "view.state" ? state : state));
  render(
    <LanguageProvider>
      <ParticlesSection api={{ call } as unknown as Api} levels={levels} />
    </LanguageProvider>,
  );
  return call;
}

describe("particles section", () => {
  beforeEach(() => localStorage.setItem(STORAGE_KEY, "en"));
  afterEach(cleanup);

  it("starts the presets", async () => {
    const call = setup();
    fireEvent.click(await screen.findByRole("button", { name: "Wind tunnel smoke" }));
    await waitFor(() =>
      expect(call).toHaveBeenCalledWith("view.set_particles", { preset: "wind_tunnel_smoke" }),
    );
    fireEvent.click(screen.getByRole("button", { name: "Internal flow" }));
    await waitFor(() =>
      expect(call).toHaveBeenCalledWith("view.set_particles", { preset: "internal_flow" }),
    );
  });

  it("changes style, colour, density, speed, pause and seed", async () => {
    const call = setup(on);
    fireEvent.change(await screen.findByRole("combobox", { name: "Style" }), {
      target: { value: "streamlines" },
    });
    await waitFor(() =>
      expect(call).toHaveBeenCalledWith("view.set_particles", { style: "streamlines" }),
    );
    fireEvent.change(screen.getByRole("combobox", { name: "Colour by" }), {
      target: { value: "pressure" },
    });
    await waitFor(() =>
      expect(call).toHaveBeenCalledWith("view.set_particles", { color_by: "pressure" }),
    );
    fireEvent.change(screen.getByRole("combobox", { name: "Density" }), {
      target: { value: "200000" },
    });
    await waitFor(() => expect(call).toHaveBeenCalledWith("view.set_particles", { count: 200000 }));
    fireEvent.change(screen.getByRole("slider", { name: "Tempo" }), { target: { value: "2" } });
    await waitFor(() => expect(call).toHaveBeenCalledWith("view.set_particles", { speed: 2 }));
    fireEvent.click(screen.getByRole("button", { name: "Pause" }));
    await waitFor(() => expect(call).toHaveBeenCalledWith("view.set_particles", { paused: true }));
    fireEvent.change(screen.getByRole("spinbutton", { name: "Start value" }), {
      target: { value: "9" },
    });
    await waitFor(() => expect(call).toHaveBeenCalledWith("view.set_particles", { seed: 9 }));
  });

  it("moves the start area along x", async () => {
    const call = setup(on);
    fireEvent.change(await screen.findByRole("slider", { name: "Start area position" }), {
      target: { value: "0.5" },
    });
    await waitFor(() =>
      expect(call).toHaveBeenCalledWith("view.set_particles", {
        region: { kind: "area", origin: [0.5, -1, 0.1], u: [0, 2, 0], v: [0, 0, 1] },
      }),
    );
  });

  it("switches the particles off", async () => {
    const call = setup(on);
    fireEvent.click(await screen.findByRole("button", { name: "Off" }));
    await waitFor(() => expect(call).toHaveBeenCalledWith("view.particles_off", {}));
  });
});
