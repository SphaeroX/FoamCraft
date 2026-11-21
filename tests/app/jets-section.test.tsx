// SPDX-License-Identifier: GPL-3.0-or-later
// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { Api } from "../../app/src/api";
import { JetsSection, directionOf, anglesOf } from "../../app/src/components/JetsSection";
import { LanguageProvider, STORAGE_KEY } from "../../app/src/i18n/i18n";

function fakeApi(jets: unknown[] = []) {
  const call = vi.fn(async (name: string) => {
    if (name === "scenario.info") return { jets };
    return { jets: [] };
  });
  return { call } as unknown as Api & { call: typeof call };
}

const ui = (api: Api) =>
  render(
    <LanguageProvider>
      <JetsSection api={api} units="metric" revision={0} />
    </LanguageProvider>,
  );

const setCalls = (api: { call: ReturnType<typeof vi.fn> }) =>
  api.call.mock.calls.filter((c) => c[0] === "scenario.set_jets");

describe("sources of air", () => {
  beforeEach(() => localStorage.setItem(STORAGE_KEY, "en"));
  afterEach(cleanup);

  it("turns angles into a direction and back", () => {
    const d = directionOf(90, 0);
    expect(d[0]).toBeCloseTo(0);
    expect(d[1]).toBeCloseTo(1);
    const [yaw, pitch] = anglesOf([0, 0, 1]);
    expect(pitch).toBeCloseTo(90);
    expect(yaw).toBeCloseTo(0);
  });

  it("switches from the whole wall to a source and sends it in SI units", async () => {
    const api = fakeApi();
    ui(api);
    fireEvent.click(await screen.findByRole("radio", { name: "Nozzles or fans" }));
    await waitFor(() => expect(setCalls(api)).toHaveLength(1));
    const sent = (setCalls(api)[0] as unknown[])[1] as { jets: { size: number[] }[] };
    expect(sent.jets).toHaveLength(1);
    const item = screen.getByRole("group", { name: "Source 1" });
    fireEvent.change(within(item).getByLabelText("Diameter (mm)"), { target: { value: "50" } });
    await waitFor(() => {
      const last = setCalls(api).at(-1) as unknown[];
      expect((last[1] as { jets: { size: number[] }[] }).jets[0].size[0]).toBeCloseTo(0.05);
    });
  });

  it("adds up to four sources and removes one", async () => {
    const one = {
      shape: "circle",
      center: [0, 0, 0],
      direction: [1, 0, 0],
      size: [0.1, 0.1],
      speed_m_s: 10,
    };
    const api = fakeApi([one]);
    ui(api);
    const add = await screen.findByRole("button", { name: "Add source" });
    fireEvent.click(add);
    await waitFor(() =>
      expect(screen.getAllByRole("group", { name: /^Source \d/ })).toHaveLength(2),
    );
    fireEvent.click(screen.getByRole("button", { name: "Remove source 2" }));
    await waitFor(() => {
      const last = setCalls(api).at(-1) as unknown[];
      expect((last[1] as { jets: unknown[] }).jets).toHaveLength(1);
    });
    fireEvent.click(screen.getByRole("radio", { name: "The whole wall (wind tunnel)" }));
    await waitFor(() => {
      const last = setCalls(api).at(-1) as unknown[];
      expect((last[1] as { jets: unknown[] }).jets).toHaveLength(0);
    });
  });
});
