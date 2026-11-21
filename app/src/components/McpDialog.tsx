// SPDX-License-Identifier: GPL-3.0-or-later
// Settings for AI agents: the MCP configuration to paste into an MCP client.
import { useEffect, useState } from "react";
import { useTranslation } from "../i18n/i18n";

/** The usual `mcpServers` entry that starts `foamcraft mcp`. */
export function mcpConfig(binary: string, root: string, app: boolean): string {
  const args = ["mcp", "--root", root, ...(app ? ["--app"] : [])];
  return JSON.stringify({ mcpServers: { foamcraft: { command: binary, args } } }, null, 2);
}

interface Props {
  cliPath: () => Promise<string>;
  defaultRoot: string;
  copy: (text: string) => Promise<void>;
  onClose: () => void;
}

export function McpDialog({ cliPath, defaultRoot, copy, onClose }: Props) {
  const { t } = useTranslation();
  const [binary, setBinary] = useState("foamcraft");
  const [root, setRoot] = useState(defaultRoot);
  const [app, setApp] = useState(true);
  const [copied, setCopied] = useState(false);
  useEffect(() => {
    cliPath()
      .then(setBinary)
      .catch(() => setBinary("foamcraft"));
  }, [cliPath]);
  const config = mcpConfig(binary, root, app);
  return (
    <div className="dialog-backdrop" role="presentation" onClick={onClose}>
      <div
        className="dialog panel mcp-dialog"
        role="dialog"
        aria-modal="true"
        aria-labelledby="mcp-title"
        onClick={(e) => e.stopPropagation()}
      >
        <h2 id="mcp-title">{t("mcp.title")}</h2>
        <p>{t("mcp.intro")}</p>
        <label className="unit-field">
          <span>{t("mcp.root")}</span>
          <input
            type="text"
            aria-label={t("mcp.root")}
            value={root}
            onChange={(e) => {
              setRoot(e.target.value);
              setCopied(false);
            }}
          />
        </label>
        <label className="check">
          <input
            type="checkbox"
            checked={app}
            onChange={(e) => {
              setApp(e.target.checked);
              setCopied(false);
            }}
          />
          {t("mcp.app")}
        </label>
        <pre aria-label={t("mcp.config")} className="code">
          {config}
        </pre>
        <p className="small muted">{t("mcp.hint")}</p>
        {copied && <p className="small">{t("mcp.copied")}</p>}
        <div className="dialog-actions">
          <button
            type="button"
            onClick={() =>
              copy(config)
                .then(() => setCopied(true))
                .catch(() => setCopied(false))
            }
          >
            {t("mcp.copy")}
          </button>
          <button type="button" onClick={onClose} autoFocus>
            {t("about.close")}
          </button>
        </div>
      </div>
    </div>
  );
}
