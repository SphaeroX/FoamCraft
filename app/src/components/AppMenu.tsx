// SPDX-License-Identifier: GPL-3.0-or-later
// One menu for everything that is not part of the current task: language, setup check, agents
// and about.
import { useEffect, useRef, useState } from "react";
import { useTranslation } from "../i18n/i18n";
import { ModeSwitch } from "../mode";
import { LanguageSwitcher } from "./LanguageSwitcher";

interface Props {
  onSystem: () => void;
  onAgents: () => void;
  onAbout: () => void;
}

export function AppMenu({ onSystem, onAgents, onAbout }: Props) {
  const { t } = useTranslation();
  const [open, setOpen] = useState(false);
  const root = useRef<HTMLDivElement>(null);

  // A click outside closes the menu.
  useEffect(() => {
    if (!open) return;
    const away = (e: MouseEvent) => {
      if (!root.current?.contains(e.target as Node)) setOpen(false);
    };
    window.addEventListener("mousedown", away);
    return () => window.removeEventListener("mousedown", away);
  }, [open]);

  const item = (label: string, action: () => void) => (
    <button
      type="button"
      role="menuitem"
      onClick={() => {
        setOpen(false);
        action();
      }}
    >
      {label}
    </button>
  );

  return (
    <div className="app-menu" ref={root}>
      <button
        type="button"
        className="quiet menu-toggle"
        aria-haspopup="menu"
        aria-expanded={open}
        aria-label={t("menu.label")}
        onClick={() => setOpen((o) => !o)}
      >
        <svg width="18" height="18" viewBox="0 0 18 18" aria-hidden="true">
          <path d="M3 5h12M3 9h12M3 13h12" />
        </svg>
      </button>
      {open && (
        <div
          className="app-menu-list panel"
          role="menu"
          onKeyDown={(e) => {
            if (e.key === "Escape") setOpen(false);
          }}
        >
          <LanguageSwitcher />
          <ModeSwitch />
          {item(t("setup.button"), onSystem)}
          {item(t("mcp.button"), onAgents)}
          {item(t("toolbar.about"), onAbout)}
        </div>
      )}
    </div>
  );
}
