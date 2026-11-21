// SPDX-License-Identifier: GPL-3.0-or-later
// @vitest-environment jsdom
// Samples and new projects need no folder dialog when the default projects folder is known.
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { Api, ProjectInfo } from "../../app/src/api";
import { SamplesSection } from "../../app/src/components/SamplesSection";
import { StartPage } from "../../app/src/components/StartPage";
import { LanguageProvider, STORAGE_KEY } from "../../app/src/i18n/i18n";

const FOLDER = "C:/Users/me/FoamCraft";

const project = (path: string, name: string): ProjectInfo => ({
  path,
  name,
  schema_version: 11,
  display_units: "metric",
  runs: [],
});

function fakeApi() {
  const call = vi.fn(async (name: string, params: Record<string, unknown> = {}) => {
    switch (name) {
      case "project.default_location":
        return {
          folder: FOLDER,
          path: typeof params.name === "string" ? `${FOLDER}/${params.name}` : null,
        };
      case "project.recent":
        return [];
      case "samples.list":
        return [
          {
            id: "wind_tunnel",
            en: { title: "Rounded body", description: "A block in a wind tunnel." },
            de: { title: "Abgerundeter Körper", description: "Ein Block." },
          },
        ];
      case "samples.create":
        return { path: `${FOLDER}/Rounded body`, name: "Rounded body" };
      case "project.info":
        return project(`${FOLDER}/Rounded body`, "Rounded body");
      case "project.new":
        return project(String(params.path ?? `${FOLDER}/${String(params.name)}`), "Wing");
      default:
        throw new Error(`unexpected ${name}`);
    }
  });
  return { call } as unknown as Api & { call: typeof call };
}

describe("start without a folder dialog", () => {
  beforeEach(() => localStorage.setItem(STORAGE_KEY, "en"));
  afterEach(cleanup);

  it("opens a sample with one click", async () => {
    const api = fakeApi();
    const pickFolder = vi.fn(async () => "C:/Elsewhere");
    const onOpened = vi.fn();
    render(
      <LanguageProvider>
        <SamplesSection api={api} pickFolder={pickFolder} onOpened={onOpened} />
      </LanguageProvider>,
    );
    await waitFor(() =>
      expect(api.call).toHaveBeenCalledWith("project.default_location", expect.anything()),
    );
    fireEvent.click(await screen.findByRole("button", { name: /Rounded body/ }));
    await waitFor(() =>
      expect(onOpened).toHaveBeenCalledWith(expect.objectContaining({ name: "Rounded body" })),
    );
    expect(api.call).toHaveBeenCalledWith("samples.create", {
      sample: "wind_tunnel",
      language: "en",
    });
    expect(pickFolder).not.toHaveBeenCalled();
  });

  it("creates a project from a name alone and shows where it goes", async () => {
    const api = fakeApi();
    const pickFolder = vi.fn(async () => "C:/Elsewhere");
    const onOpened = vi.fn();
    render(
      <LanguageProvider>
        <StartPage api={api} pickFolder={pickFolder} onOpened={onOpened} />
      </LanguageProvider>,
    );
    fireEvent.change(screen.getByLabelText("Project name"), { target: { value: "Wing" } });
    expect(await screen.findByText(`${FOLDER}/Wing`, { exact: false })).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Create project" }));
    await waitFor(() => expect(onOpened).toHaveBeenCalled());
    expect(api.call).toHaveBeenCalledWith("project.new", { name: "Wing" });
    expect(pickFolder).not.toHaveBeenCalled();
  });

  it("still lets the user choose another folder", async () => {
    const api = fakeApi();
    const pickFolder = vi.fn(async () => "C:/Elsewhere");
    const onOpened = vi.fn();
    render(
      <LanguageProvider>
        <StartPage api={api} pickFolder={pickFolder} onOpened={onOpened} />
      </LanguageProvider>,
    );
    fireEvent.change(screen.getByLabelText("Project name"), { target: { value: "Wing" } });
    fireEvent.click(await screen.findByRole("button", { name: "Other folder …" }));
    await waitFor(() => expect(onOpened).toHaveBeenCalled());
    expect(pickFolder).toHaveBeenCalled();
    expect(api.call).toHaveBeenCalledWith("project.new", {
      path: "C:/Elsewhere/Wing",
      name: "Wing",
    });
  });
});

describe("a sample's scenario is ready", () => {
  beforeEach(() => localStorage.setItem(STORAGE_KEY, "en"));
  afterEach(cleanup);

  it("shows the kind of flow of a saved internal flow scenario as selected", async () => {
    const { ScenarioPanel } = await import("../../app/src/components/ScenarioPanel");
    const call = vi.fn(async (name: string) => {
      if (name === "scenario.info")
        return {
          scenario: {
            kind: "internal_flow",
            medium: { kind: "water", temperature_c: 20 },
            quality: "draft",
            inlets: [],
            outlets: [],
          },
          reynolds: 1,
          regime: "laminar",
          flow: { code: "x", en: { text: "", fix: "" }, de: { text: "", fix: "" } },
          openings: [],
          warnings: [],
        };
      if (name === "scenario.boundary_roles") return {};
      throw { code: "mesh.none", detail: "", en: { text: "", fix: "" }, de: { text: "", fix: "" } };
    });
    render(
      <LanguageProvider>
        <ScenarioPanel api={{ call } as unknown as Api} units="metric" />
      </LanguageProvider>,
    );
    await waitFor(() => expect(screen.getByLabelText("Kind of flow")).toHaveValue("internal_flow"));
  });
});
