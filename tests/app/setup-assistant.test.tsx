// SPDX-License-Identifier: GPL-3.0-or-later
// @vitest-environment jsdom
import { act, cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { Api } from "../../app/src/api";
import { SetupAssistant } from "../../app/src/components/SetupAssistant";
import { GuidedTour } from "../../app/src/components/GuidedTour";
import { SamplesSection } from "../../app/src/components/SamplesSection";
import { LanguageProvider, STORAGE_KEY } from "../../app/src/i18n/i18n";

const text = (en: string, fix = "", help: string[] = []) => ({
  en: { text: en, fix, help },
  de: { text: en, fix, help },
});

const step = (
  id: string,
  state: string,
  code: string,
  en: string,
  fix = "",
  help: string[] = [],
) => ({
  id,
  state,
  code,
  detail: "",
  ...text(en, fix, help),
});

const profile = {
  physical_cores: 8,
  host_memory_bytes: 32e9,
  container_cpus: 8,
  container_memory_bytes: 16e9,
  default_cores: 8,
  max_cells: 6400000,
  quality: "standard",
};

const stopped = {
  ready: false,
  profile,
  steps: [
    step("docker", "ok", "system.docker_installed", "Docker is installed."),
    step(
      "service",
      "action",
      "runtime.daemon_not_running",
      "The Docker service is not running.",
      "Start Docker Desktop.",
      [
        "Start Docker Desktop from the Start menu.",
        "Run wsl --install if there is no WSL distribution.",
      ],
    ),
    step("image", "skipped", "system.skipped", "Checked after the step above is done."),
  ],
};

const noImage = {
  ready: false,
  profile,
  steps: [
    step("docker", "ok", "system.docker_installed", "Docker is installed."),
    step("service", "ok", "system.service_running", "The Docker service is running."),
    step(
      "image",
      "action",
      "runtime.image_missing",
      "The runtime image is missing.",
      "Provide it.",
      ["Press Provide runtime.", "Wait."],
    ),
  ],
};

const ready = {
  ready: true,
  profile,
  steps: [
    step("docker", "ok", "system.docker_installed", "Docker is installed."),
    step("service", "ok", "system.service_running", "The Docker service is running."),
    step("image", "ok", "system.image_present", "The OpenFOAM runtime is ready."),
  ],
};

type Listener = (e: { name: string; payload: unknown }) => void;

function setup(checks: unknown[]) {
  let n = 0;
  const listeners: Listener[] = [];
  const call = vi.fn(async (name: string) => {
    switch (name) {
      case "system.check":
        return checks[Math.min(n++, checks.length - 1)];
      case "system.diagnostics":
        return { text: "FoamCraft 0.1.0\nChecks: ..." };
      case "system.finish_setup":
        return { done: true };
      case "system.provide_image":
        return { started: true };
    }
    throw new Error(name);
  });
  const copy = vi.fn(async () => {});
  const onClose = vi.fn();
  const subscribe = (l: Listener) => {
    listeners.push(l);
    return () => {};
  };
  render(
    <LanguageProvider>
      <SetupAssistant
        api={{ call } as unknown as Api}
        copy={copy}
        onClose={onClose}
        subscribe={subscribe}
      />
    </LanguageProvider>,
  );
  const emit = (name: string, payload: unknown) =>
    act(() => listeners.forEach((l) => l({ name, payload })));
  return { call, copy, onClose, emit };
}

describe("setup assistant", () => {
  beforeEach(() => localStorage.setItem(STORAGE_KEY, "en"));
  afterEach(cleanup);

  it("explains a stopped Docker service and notices when it runs", async () => {
    const { call } = setup([stopped, ready]);
    const list = await screen.findByRole("list", { name: "System check" });
    expect(list).toHaveTextContent("The Docker service is not running.");
    expect(list).toHaveTextContent("Run wsl --install");
    expect(screen.getByText("Action needed")).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Check again" }));
    expect(await screen.findByText("Ready for simulations.")).toBeInTheDocument();
    expect(call).toHaveBeenCalledTimes(2);
    // The hardware profile with its defaults.
    const p = screen.getByRole("region", { name: "Hardware profile" });
    expect(within(p).getByText(/8 cores/)).toBeInTheDocument();
    expect(within(p).getByText(/6\.4 million cells/)).toBeInTheDocument();
  });

  it("provides the runtime image with progress and checks again when it is ready", async () => {
    const { call, emit } = setup([noImage, ready]);
    fireEvent.click(await screen.findByRole("button", { name: "Provide runtime" }));
    await waitFor(() => expect(call).toHaveBeenCalledWith("system.provide_image", {}));
    emit("system.image_progress", { line: "Step 3/12 : RUN apt-get install", lines: 40 });
    expect(await screen.findByText(/Step 3\/12/)).toBeInTheDocument();
    emit("system.image_ready", null);
    expect(await screen.findByText("Ready for simulations.")).toBeInTheDocument();
  });

  it("copies the diagnostic report and finishes the setup", async () => {
    const { call, copy, onClose } = setup([ready]);
    fireEvent.click(await screen.findByRole("button", { name: "Copy diagnostic report" }));
    await waitFor(() => expect(copy).toHaveBeenCalledWith("FoamCraft 0.1.0\nChecks: ..."));
    expect(await screen.findByText("Copied.")).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Finish" }));
    await waitFor(() => expect(call).toHaveBeenCalledWith("system.finish_setup", {}));
    await waitFor(() => expect(onClose).toHaveBeenCalled());
  });
});

describe("guided tour", () => {
  beforeEach(() => localStorage.setItem(STORAGE_KEY, "en"));
  afterEach(cleanup);

  it("walks through the four steps and can be closed with Escape", () => {
    const onClose = vi.fn();
    render(
      <LanguageProvider>
        <GuidedTour onClose={onClose} />
      </LanguageProvider>,
    );
    const dialog = screen.getByRole("dialog", { name: /Geometry/ });
    expect(dialog).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Next" }));
    expect(screen.getByRole("dialog", { name: /Scenario/ })).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Back" }));
    expect(screen.getByRole("dialog", { name: /Geometry/ })).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Next" }));
    fireEvent.click(screen.getByRole("button", { name: "Next" }));
    fireEvent.click(screen.getByRole("button", { name: "Next" }));
    expect(screen.getByRole("dialog", { name: /Results/ })).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Done" }));
    expect(onClose).toHaveBeenCalledTimes(1);
    cleanup();
    const onClose2 = vi.fn();
    render(
      <LanguageProvider>
        <GuidedTour onClose={onClose2} />
      </LanguageProvider>,
    );
    fireEvent.keyDown(screen.getByRole("dialog"), { key: "Escape" });
    expect(onClose2).toHaveBeenCalled();
  });
});

describe("sample projects on the start page", () => {
  beforeEach(() => localStorage.setItem(STORAGE_KEY, "en"));
  afterEach(cleanup);

  it("creates a sample in a chosen folder and opens it", async () => {
    const call = vi.fn(async (name: string) => {
      if (name === "samples.list")
        return [
          {
            id: "wind_tunnel",
            en: { title: "Rounded body", description: "A block in a wind tunnel." },
            de: { title: "Abgerundeter Körper", description: "Ein Block." },
          },
          {
            id: "manifold",
            en: { title: "Pipe manifold", description: "A header pipe." },
            de: { title: "Rohrverteiler", description: "Ein Sammelrohr." },
          },
        ];
      if (name === "samples.create") return { path: "C:/CFD/Rounded body", name: "Rounded body" };
      if (name === "project.info")
        return {
          path: "C:/CFD/Rounded body",
          name: "Rounded body",
          schema_version: 11,
          display_units: "metric",
          runs: [],
        };
      throw new Error(name);
    });
    const onOpened = vi.fn();
    render(
      <LanguageProvider>
        <SamplesSection
          api={{ call } as unknown as Api}
          pickFolder={async () => "C:/CFD"}
          onOpened={onOpened}
        />
      </LanguageProvider>,
    );
    fireEvent.click(await screen.findByRole("button", { name: /Rounded body/ }));
    await waitFor(() =>
      expect(call).toHaveBeenCalledWith("samples.create", {
        sample: "wind_tunnel",
        path: "C:/CFD/Rounded body",
        language: "en",
      }),
    );
    await waitFor(() =>
      expect(onOpened).toHaveBeenCalledWith(expect.objectContaining({ name: "Rounded body" })),
    );
  });

  it("stays hidden when the samples cannot be listed", async () => {
    const call = vi.fn(async () => {
      throw new Error("no");
    });
    const { container } = render(
      <LanguageProvider>
        <SamplesSection
          api={{ call } as unknown as Api}
          pickFolder={async () => null}
          onOpened={() => {}}
        />
      </LanguageProvider>,
    );
    await waitFor(() => expect(call).toHaveBeenCalled());
    expect(container).toBeEmptyDOMElement();
  });
});
