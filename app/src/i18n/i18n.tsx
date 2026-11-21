// SPDX-License-Identifier: GPL-3.0-or-later
// Translation layer: every user-visible string comes from en.json and de.json.
import { createContext, useCallback, useContext, useMemo, useState, type ReactNode } from "react";
import de from "./de.json";
import en from "./en.json";

export const languages = ["en", "de"] as const;
export type Language = (typeof languages)[number];
export const dictionaries: Record<Language, unknown> = { en, de };
export const STORAGE_KEY = "foamcraft.language";

export function isLanguage(value: unknown): value is Language {
  return typeof value === "string" && (languages as readonly string[]).includes(value);
}

/** Looks up a dotted key; unknown keys return the key itself so gaps stay visible. */
export function translate(
  lang: Language,
  key: string,
  params: Record<string, string> = {},
): string {
  let node: unknown = dictionaries[lang];
  for (const part of key.split(".")) {
    if (typeof node !== "object" || node === null) return key;
    node = (node as Record<string, unknown>)[part];
  }
  if (typeof node !== "string") return key;
  return node.replace(/\{(\w+)\}/g, (match, name: string) => params[name] ?? match);
}

function readStoredLanguage(): Language {
  try {
    const stored = localStorage.getItem(STORAGE_KEY);
    if (isLanguage(stored)) return stored;
  } catch {
    // Storage can be unavailable; fall back to the system language.
  }
  return typeof navigator !== "undefined" && navigator.language.toLowerCase().startsWith("de")
    ? "de"
    : "en";
}

interface TranslationContext {
  language: Language;
  setLanguage: (lang: Language) => void;
  t: (key: string, params?: Record<string, string>) => string;
}

const Context = createContext<TranslationContext | null>(null);

export function LanguageProvider({ children }: { children: ReactNode }) {
  const [language, setLanguageState] = useState<Language>(readStoredLanguage);
  const setLanguage = useCallback((lang: Language) => {
    setLanguageState(lang);
    try {
      localStorage.setItem(STORAGE_KEY, lang);
    } catch {
      // Not persisting is acceptable; the choice still applies to this session.
    }
  }, []);
  const value = useMemo(
    () => ({
      language,
      setLanguage,
      t: (key: string, params?: Record<string, string>) => translate(language, key, params),
    }),
    [language, setLanguage],
  );
  return <Context.Provider value={value}>{children}</Context.Provider>;
}

export function useTranslation(): TranslationContext {
  const ctx = useContext(Context);
  if (!ctx) throw new Error("useTranslation outside LanguageProvider");
  return ctx;
}
