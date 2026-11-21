// SPDX-License-Identifier: GPL-3.0-or-later
// Two levels of detail: "simple" shows each step's main task, "expert" everything. Without a
// provider the level is "expert", so components rendered on their own show all their controls.
import { createContext, useCallback, useContext, useMemo, useState, type ReactNode } from "react";
import { useTranslation } from "./i18n/i18n";

export type Mode = "simple" | "expert";
export const MODE_KEY = "foamcraft.mode";

interface ModeContext {
  mode: Mode;
  setMode: (mode: Mode) => void;
}

const Context = createContext<ModeContext>({ mode: "expert", setMode: () => {} });

function readStored(): Mode {
  try {
    return localStorage.getItem(MODE_KEY) === "expert" ? "expert" : "simple";
  } catch {
    return "simple";
  }
}

/** The app's provider: "simple" unless the user chose "expert" on this computer before. */
export function ModeProvider({ children }: { children: ReactNode }) {
  const [mode, setState] = useState<Mode>(readStored);
  const setMode = useCallback((m: Mode) => {
    setState(m);
    try {
      localStorage.setItem(MODE_KEY, m);
    } catch {
      // Not remembered; the choice still applies now.
    }
  }, []);
  const value = useMemo(() => ({ mode, setMode }), [mode, setMode]);
  return <Context.Provider value={value}>{children}</Context.Provider>;
}

export function useMode(): ModeContext {
  return useContext(Context);
}

/** Children only in expert mode. */
export function ExpertOnly({ children }: { children: ReactNode }) {
  return useMode().mode === "expert" ? <>{children}</> : null;
}

/** Switch between simple and expert. */
export function ModeSwitch() {
  const { t } = useTranslation();
  const { mode, setMode } = useMode();
  return (
    <label className="mode-switch" title={t("mode.hint")}>
      <input
        type="checkbox"
        role="checkbox"
        aria-label={t("mode.expert")}
        checked={mode === "expert"}
        onChange={(e) => setMode(e.target.checked ? "expert" : "simple")}
      />
      <span>{t("mode.expert")}</span>
    </label>
  );
}
