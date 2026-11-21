// SPDX-License-Identifier: GPL-3.0-or-later
// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { Api, ProjectInfo } from "../../app/src/api";
import { StartPage } from "../../app/src/components/StartPage";
import { LanguageProvider, STORAGE_KEY } from "../../app/src/i18n/i18n";

const wing: ProjectInfo = {
  path: "C:/Projects/Wing",
  name: "Wing",
  schema_version: 1,
  display_units: "metric",
  runs: [],
};

function fakeApi(overrides: Partial<Record<string, unknown>> = {}) {
  const call = vi.fn(async (name: string, params: Record<string, unknown> = {}) => {
    if (name in overrides) return overrides[name];
    if (name === "project.recent") return [{ path: "C:/Projects/Old", name: "Old car" }];
    if (name === "project.new") return { ...wing, path: params.path, name: params.name };
    if (name === "project.open") return { ...wing, path: params.path };
    throw new Error(`unexpected ${name}`);
  });
  return { call } as unknown as Api & { call: typeof call };
}

function renderPage(api: Api, pickFolder: () => Promise<string | null>, onOpened = vi.fn()) {
  render(
    <LanguageProvider>
      <StartPage api={api} pickFolder={pickFolder} onOpened={onOpened} />
    </LanguageProvider>,
  );
  return onOpened;
}

describe("start page", () => {
  beforeEach(() => localStorage.setItem(STORAGE_KEY, "en"));
  afterEach(cleanup);

  it("shows recently opened projects and opens one on click", async () => {
    const api = fakeApi();
    const onOpened = renderPage(api, async () => null);
    const item = await screen.findByRole("button", { name: /Old car/ });
    fireEvent.click(item);
    await waitFor(() => expect(onOpened).toHaveBeenCalled());
    expect(api.call).toHaveBeenCalledWith("project.open", { path: "C:/Projects/Old" });
  });

  it("creates a project with project.new in the chosen folder", async () => {
    const api = fakeApi();
    const onOpened = renderPage(api, async () => "C:/Projects");
    fireEvent.change(screen.getByLabelText("Project name"), { target: { value: "Wing" } });
    fireEvent.click(screen.getByRole("button", { name: "Create project" }));
    await waitFor(() => expect(onOpened).toHaveBeenCalled());
    expect(api.call).toHaveBeenCalledWith("project.new", {
      path: "C:/Projects/Wing",
      name: "Wing",
    });
  });

  it("does nothing when the folder dialog is cancelled", async () => {
    const api = fakeApi();
    const onOpened = renderPage(api, async () => null);
    fireEvent.change(screen.getByLabelText("Project name"), { target: { value: "Wing" } });
    fireEvent.click(screen.getByRole("button", { name: "Create project" }));
    await new Promise((r) => setTimeout(r, 10));
    expect(onOpened).not.toHaveBeenCalled();
    expect(api.call).not.toHaveBeenCalledWith("project.new", expect.anything());
  });

  it("shows a translated error message", async () => {
    const api = fakeApi();
    api.call.mockImplementation(async (name: string) => {
      if (name === "project.recent") return [];
      throw {
        code: "project.not_found",
        detail: "C:/x",
        en: { text: "There is no FoamCraft project in this folder.", fix: "Choose another." },
        de: { text: "Kein Projekt.", fix: "Anderen wählen." },
      };
    });
    renderPage(api, async () => "C:/x");
    fireEvent.click(screen.getByRole("button", { name: "Open project" }));
    expect(await screen.findByRole("alert")).toHaveTextContent(/no FoamCraft project/);
  });
});
