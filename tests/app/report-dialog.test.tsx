// SPDX-License-Identifier: GPL-3.0-or-later
// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { Api } from "../../app/src/api";
import { ReportDialog } from "../../app/src/components/ReportDialog";
import { LanguageProvider, STORAGE_KEY } from "../../app/src/i18n/i18n";

const runs = [
  { job_id: "job-b", name: "30 m/s" },
  { job_id: "job-a", name: "20 m/s" },
];

function setup() {
  const call = vi.fn(async (name: string) => {
    if (name === "runs.list") return runs;
    if (name === "export.report")
      return { pdf: "C:/out/report.pdf", html: "C:/out/report.html", images: 3 };
    throw new Error(name);
  });
  const pickSavePath = vi.fn(async () => "C:/out/report.pdf");
  const pickLogo = vi.fn(async () => "C:/logos/company.png");
  render(
    <LanguageProvider>
      <ReportDialog
        api={{ call } as unknown as Api}
        projectName="Helmet"
        pickSavePath={pickSavePath}
        pickLogo={pickLogo}
        onClose={() => {}}
      />
    </LanguageProvider>,
  );
  return { call, pickSavePath, pickLogo };
}

describe("report dialog", () => {
  beforeEach(() => localStorage.setItem(STORAGE_KEY, "en"));
  afterEach(cleanup);

  it("writes PDF and HTML of the newest run with title and author", async () => {
    const { call } = setup();
    expect(await screen.findByRole("combobox", { name: "Run" })).toHaveValue("job-b");
    expect(screen.getByRole("textbox", { name: "Title" })).toHaveValue("Helmet");
    fireEvent.change(screen.getByRole("textbox", { name: "Author" }), {
      target: { value: "A. Tester" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Save report" }));
    await waitFor(() =>
      expect(call).toHaveBeenCalledWith("export.report", {
        path: "C:/out/report.pdf",
        job_id: "job-b",
        language: "en",
        formats: ["pdf", "html"],
        title: "Helmet",
        author: "A. Tester",
      }),
    );
    expect(await screen.findByText(/C:\/out\/report\.pdf/)).toBeInTheDocument();
  });

  it("chooses language, formats, sections, a logo and a comparison", async () => {
    const { call, pickLogo } = setup();
    fireEvent.change(await screen.findByRole("combobox", { name: "Language of the report" }), {
      target: { value: "de" },
    });
    fireEvent.click(screen.getByRole("checkbox", { name: "HTML" }));
    fireEvent.click(screen.getByRole("checkbox", { name: "Views" }));
    fireEvent.change(screen.getByRole("combobox", { name: "Compare with" }), {
      target: { value: "job-a" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Choose logo" }));
    await waitFor(() => expect(pickLogo).toHaveBeenCalled());
    expect(await screen.findByText(/company\.png/)).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Save report" }));
    await waitFor(() =>
      expect(call).toHaveBeenCalledWith(
        "export.report",
        expect.objectContaining({
          language: "de",
          formats: ["pdf"],
          compare_with: "job-a",
          logo_path: "C:/logos/company.png",
          sections: [
            "summary",
            "setup",
            "mesh",
            "convergence",
            "figures",
            "reliability",
            "comparison",
            "limits",
            "versions",
          ],
        }),
      ),
    );
  });
});
