// SPDX-License-Identifier: GPL-3.0-or-later
// @vitest-environment jsdom
import { act, cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { Api, GeometryInfo } from "../../app/src/api";
import { GeometryPanel } from "../../app/src/components/GeometryPanel";
import { LanguageProvider, STORAGE_KEY } from "../../app/src/i18n/i18n";

const open: GeometryInfo = {
  id: "model-1",
  name: "box.stl",
  file: "geometry/model-1.stl",
  unit: "mm",
  scale: 1,
  rotation_deg: [0, 0, 0],
  translation: [0, 0, 0],
  fix_normals: false,
  suggested_unit: "mm",
  size_m: [0.02, 0.03, 0.04],
  bounds_m: { min: [0, 0, 0], max: [0.02, 0.03, 0.04] },
  diagnosis: { triangles: 11, watertight: false, boundary_loops: 1 },
  messages: [
    {
      code: "geometry.open",
      severity: "warning",
      count: 1,
      en: { text: "The model has holes.", fix: "Close them." },
      de: { text: "Das Modell hat Löcher.", fix: "Schließen." },
    },
  ],
};

function fakeApi() {
  const call = vi.fn(async (name: string, params: Record<string, unknown> = {}) => {
    switch (name) {
      case "geometry.list":
        return [{ id: "model-1" }];
      case "geometry.info":
      case "geometry.set_unit":
      case "geometry.set_transform":
      case "geometry.place_on_ground":
      case "geometry.fix_normals":
        return { ...open, unit: (params.unit as string) ?? open.unit };
      case "geometry.import":
        return open;
      default:
        throw new Error(`unexpected ${name}`);
    }
  });
  return { call } as unknown as Api & { call: typeof call };
}

function renderPanel(api: Api, pickModel = async () => "C:/Models/box.stl" as string | null) {
  return render(
    <LanguageProvider>
      <GeometryPanel api={api} pickModel={pickModel} units="metric" />
    </LanguageProvider>,
  );
}

describe("geometry panel", () => {
  beforeEach(() => localStorage.setItem(STORAGE_KEY, "en"));
  afterEach(cleanup);

  it("lists models with size and translated diagnosis", async () => {
    renderPanel(fakeApi());
    expect(await screen.findByText("box.stl")).toBeInTheDocument();
    expect(screen.getByText(/The model has holes/)).toBeInTheDocument();
    expect(screen.getByText(/20 × 30 × 40 mm/)).toBeInTheDocument();
  });

  it("shows the diagnosis in German after switching the language", async () => {
    localStorage.setItem(STORAGE_KEY, "de");
    renderPanel(fakeApi());
    expect(await screen.findByText(/Das Modell hat Löcher/)).toBeInTheDocument();
  });

  it("imports the picked file through geometry.import", async () => {
    const api = fakeApi();
    renderPanel(api);
    fireEvent.click(await screen.findByRole("button", { name: "Import model" }));
    await waitFor(() =>
      expect(api.call).toHaveBeenCalledWith("geometry.import", { path: "C:/Models/box.stl" }),
    );
  });

  it("changes the unit, rotation and position through commands", async () => {
    const api = fakeApi();
    renderPanel(api);
    const unit = await screen.findByLabelText("Unit");
    act(() => {
      fireEvent.change(unit, { target: { value: "cm" } });
    });
    await waitFor(() =>
      expect(api.call).toHaveBeenCalledWith("geometry.set_unit", { id: "model-1", unit: "cm" }),
    );
    const pitch = screen.getByLabelText("Pitch (°)");
    fireEvent.change(pitch, { target: { value: "90" } });
    fireEvent.blur(pitch);
    await waitFor(() =>
      expect(api.call).toHaveBeenCalledWith("geometry.set_transform", {
        id: "model-1",
        rotation_deg: [0, 90, 0],
      }),
    );
    const z = screen.getByLabelText("Z (mm)");
    fireEvent.change(z, { target: { value: "15" } });
    fireEvent.blur(z);
    await waitFor(() =>
      expect(api.call).toHaveBeenCalledWith("geometry.set_transform", {
        id: "model-1",
        translation: [0, 0, 0.015],
      }),
    );
    fireEvent.click(screen.getByRole("button", { name: "Place on ground" }));
    await waitFor(() =>
      expect(api.call).toHaveBeenCalledWith("geometry.place_on_ground", { id: "model-1" }),
    );
  });
});
