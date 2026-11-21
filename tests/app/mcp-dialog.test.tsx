// SPDX-License-Identifier: GPL-3.0-or-later
// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { McpDialog, mcpConfig } from "../../app/src/components/McpDialog";
import { LanguageProvider, STORAGE_KEY } from "../../app/src/i18n/i18n";

describe("MCP configuration", () => {
  it("is the usual mcpServers entry with the program, the root and the app switch", () => {
    const config = JSON.parse(
      mcpConfig("C:\\FoamCraft\\foamcraft.exe", "C:\\Users\\me\\CFD", true),
    );
    expect(config).toEqual({
      mcpServers: {
        foamcraft: {
          command: "C:\\FoamCraft\\foamcraft.exe",
          args: ["mcp", "--root", "C:\\Users\\me\\CFD", "--app"],
        },
      },
    });
    const headless = JSON.parse(mcpConfig("/opt/foamcraft", "/home/me/cfd", false));
    expect(headless.mcpServers.foamcraft.args).toEqual(["mcp", "--root", "/home/me/cfd"]);
  });
});

describe("MCP dialog", () => {
  beforeEach(() => localStorage.setItem(STORAGE_KEY, "en"));
  afterEach(cleanup);

  it("shows the configuration to copy and updates it with the folder", async () => {
    const copy = vi.fn(async () => {});
    render(
      <LanguageProvider>
        <McpDialog
          cliPath={async () => "C:\\FoamCraft\\foamcraft.exe"}
          defaultRoot="C:\\Users\\me\\CFD"
          copy={copy}
          onClose={() => {}}
        />
      </LanguageProvider>,
    );
    const code = await screen.findByLabelText("Configuration");
    await waitFor(() => expect(code).toHaveTextContent("foamcraft.exe"));
    expect(code).toHaveTextContent("--app");
    fireEvent.change(screen.getByRole("textbox", { name: "Folder for the agent" }), {
      target: { value: "D:\\Sim" },
    });
    expect(code).toHaveTextContent("D:\\\\Sim");
    fireEvent.click(screen.getByRole("checkbox", { name: /Run in this app/ }));
    expect(code).not.toHaveTextContent("--app");
    fireEvent.click(screen.getByRole("button", { name: "Copy" }));
    await waitFor(() => expect(copy).toHaveBeenCalledWith(code.textContent));
    expect(await screen.findByText("Copied.")).toBeInTheDocument();
  });
});
