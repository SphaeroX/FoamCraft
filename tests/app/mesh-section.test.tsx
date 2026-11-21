// SPDX-License-Identifier: GPL-3.0-or-later
// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { Api } from "../../app/src/api";
import { MeshSection } from "../../app/src/components/MeshSection";
import { LanguageProvider, STORAGE_KEY } from "../../app/src/i18n/i18n";

const result = {
  job_id: "job-1",
  budget: 500000,
  cells: 487123,
  quality: {
    cells: 487123,
    failed_checks: 0,
    max_non_orthogonality: 60,
    max_skewness: 2,
    verdict: "good",
  },
  layers: { target: 4, achieved: 3.6, thickness_percent: 85 },
  fallbacks: [
    {
      code: "mesh.fallback_fewer_layers",
      en: { text: "Fewer boundary layers were used.", fix: "" },
      de: { text: "Es wurden weniger Grenzschichten verwendet.", fix: "" },
    },
  ],
  plan: {},
};

describe("mesh section", () => {
  beforeEach(() => localStorage.setItem(STORAGE_KEY, "en"));
  afterEach(cleanup);

  it("generates the mesh and shows cells, verdict, layers and fallbacks", async () => {
    const call = vi.fn(async (name: string) => {
      if (name === "mesh.info") throw { code: "mesh.none", detail: "", en: {}, de: {} };
      if (name === "mesh.generate") return result;
      throw new Error(name);
    });
    render(
      <LanguageProvider>
        <MeshSection api={{ call } as unknown as Api} />
      </LanguageProvider>,
    );
    fireEvent.click(await screen.findByRole("button", { name: "Generate mesh" }));
    await waitFor(() => expect(call).toHaveBeenCalledWith("mesh.generate", {}));
    expect(await screen.findByText(/487[,.]?123 cells/)).toBeInTheDocument();
    expect(screen.getByText(/Good/)).toBeInTheDocument();
    expect(screen.getByText(/3[.,]6 of 4/)).toBeInTheDocument();
    expect(screen.getByText(/Fewer boundary layers/)).toBeInTheDocument();
  });
});
