// SPDX-License-Identifier: GPL-3.0-or-later
// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { Api } from "../../app/src/api";
import { SurfacePanel } from "../../app/src/components/SurfacePanel";
import { LanguageProvider, STORAGE_KEY } from "../../app/src/i18n/i18n";

const faces = [1, 2, 3].map((i) => ({
  key: `model-1:face-${i}`,
  model: "model-1",
  model_name: "box.stl",
  part: "model-1:part-1",
  name: i === 1 ? "Front" : null,
  groups: i === 2 ? ["Inlet"] : [],
  triangles: 2,
  area_m2: 0.0004,
  normal: [0, 0, 1],
  centroid_m: [0, 0, 0],
}));

function fakeApi() {
  const call = vi.fn(async (name: string) => {
    switch (name) {
      case "surface.list":
        return faces;
      case "group.list":
        return [{ name: "Inlet", keys: ["model-1:face-2"] }];
      default:
        return null;
    }
  });
  return { call } as unknown as Api & { call: typeof call };
}

function renderPanel(api: Api, selection: string[] = []) {
  return render(
    <LanguageProvider>
      <SurfacePanel api={api} selection={selection} revision={0} />
    </LanguageProvider>,
  );
}

describe("surface panel", () => {
  beforeEach(() => localStorage.setItem(STORAGE_KEY, "en"));
  afterEach(cleanup);

  it("lists parts and groups", async () => {
    renderPanel(fakeApi());
    expect(await screen.findByText(/Part 1/)).toBeInTheDocument();
    expect(screen.getByText("Inlet")).toBeInTheDocument();
    expect(screen.getByText(/1 face/)).toBeInTheDocument();
  });

  it("creates a group from the selection", async () => {
    const api = fakeApi();
    renderPanel(api, ["model-1:face-1", "model-1:face-3"]);
    expect(await screen.findByText(/2 faces selected/)).toBeInTheDocument();
    fireEvent.change(screen.getByLabelText("Group name"), { target: { value: "Outlet" } });
    fireEvent.click(screen.getByRole("button", { name: "Create group" }));
    await waitFor(() => expect(api.call).toHaveBeenCalledWith("group.create", { name: "Outlet" }));
  });

  it("selects the faces of a group and of a part", async () => {
    const api = fakeApi();
    renderPanel(api);
    fireEvent.click(await screen.findByRole("button", { name: "Select Inlet" }));
    await waitFor(() =>
      expect(api.call).toHaveBeenCalledWith("surface.select", { keys: ["model-1:face-2"] }),
    );
    fireEvent.click(screen.getByRole("button", { name: /Part 1/ }));
    await waitFor(() =>
      expect(api.call).toHaveBeenCalledWith("surface.select", {
        keys: ["model-1:face-1", "model-1:face-2", "model-1:face-3"],
      }),
    );
  });

  it("renames a single selected face", async () => {
    const api = fakeApi();
    renderPanel(api, ["model-1:face-3"]);
    const field = await screen.findByLabelText("Face name");
    fireEvent.change(field, { target: { value: "Back" } });
    fireEvent.blur(field);
    await waitFor(() =>
      expect(api.call).toHaveBeenCalledWith("surface.rename", {
        key: "model-1:face-3",
        name: "Back",
      }),
    );
  });
});
