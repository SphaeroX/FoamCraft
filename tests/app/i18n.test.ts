// SPDX-License-Identifier: GPL-3.0-or-later
import { describe, expect, it } from "vitest";
import { dictionaries, languages, translate } from "../../app/src/i18n/i18n";

function keysOf(value: unknown, prefix = ""): string[] {
  if (typeof value !== "object" || value === null) return [prefix];
  return Object.entries(value).flatMap(([k, v]) => keysOf(v, prefix ? `${prefix}.${k}` : k));
}

describe("translations", () => {
  it("offers English and German", () => {
    expect(languages).toEqual(["en", "de"]);
  });

  it("has every key in both languages with non-empty text", () => {
    const en = keysOf(dictionaries.en).sort();
    const de = keysOf(dictionaries.de).sort();
    expect(de).toEqual(en);
    for (const lang of languages) {
      for (const key of en) {
        expect(translate(lang, key), `${lang}:${key}`).not.toBe("");
        expect(translate(lang, key), `${lang}:${key}`).not.toBe(key);
      }
    }
  });

  it("falls back to the key for unknown keys", () => {
    expect(translate("de", "no.such.key")).toBe("no.such.key");
  });

  it("fills placeholders", () => {
    expect(translate("en", "about.version", { version: "1.2.3" })).toContain("1.2.3");
  });
});
