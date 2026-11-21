// SPDX-License-Identifier: GPL-3.0-or-later
// Ride along one particle: suggested rides as cards, or pick a particle or a spot on the model
// in the view.
import { useEffect, useState } from "react";
import type { Api, LocalizedMessage } from "../api";
import { useTranslation } from "../i18n/i18n";

interface Suggestion {
  id: string;
  title: string;
  start: [number, number, number];
  duration_s: number;
}

type Texts = Record<string, { en: LocalizedMessage; de: LocalizedMessage }>;

interface Props {
  api: Api;
  /** The next click in the view picks the start (a particle or a spot on the model). */
  onPick: () => void;
  /** A pick is waiting for the click. */
  picking?: boolean;
}

/** Start options that respect "reduce motion": chase view, calm. */
export function gentleStart(): Record<string, unknown> {
  const reduce =
    typeof window !== "undefined" &&
    typeof window.matchMedia === "function" &&
    window.matchMedia("(prefers-reduced-motion: reduce)").matches;
  return reduce ? { view: "chase", mode: "calm" } : {};
}

export function RideSection({ api, onPick, picking = false }: Props) {
  const { t, language } = useTranslation();
  const [list, setList] = useState<Suggestion[]>([]);
  const [texts, setTexts] = useState<Texts>({});
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    api
      .call<Suggestion[]>("ride.suggest", {})
      .then((found) => {
        const valid = Array.isArray(found) ? found : [];
        setList(valid);
        if (valid.length === 0) return {};
        return api.call<Texts>("error.describe", { codes: valid.map((s) => s.title) });
      })
      .then((found) => setTexts(found ?? {}))
      .catch(() => {});
  }, [api]);

  const start = (s: Suggestion) => {
    setError(null);
    api
      .call("ride.start", { suggestion: s.id, ...gentleStart() })
      .catch((e: { en?: LocalizedMessage; de?: LocalizedMessage }) =>
        setError(e?.[language]?.text ?? String(e)),
      );
  };

  return (
    <section className="ride-section">
      <h3>{t("ride.title")}</h3>
      <p className="muted small">{t("ride.intro")}</p>
      {list.length > 0 && (
        <div className="ride-cards">
          {list.map((s) => (
            <button key={s.id} type="button" className="ride-card" onClick={() => start(s)}>
              <span className="ride-card-title">{texts[s.title]?.[language]?.text ?? s.id}</span>
              <span className="muted small">{texts[s.title]?.[language]?.fix ?? ""}</span>
              <span className="muted small">
                {t("ride.duration", { s: String(Math.round(s.duration_s)) })}
              </span>
            </button>
          ))}
        </div>
      )}
      <button type="button" aria-pressed={picking} onClick={onPick}>
        {t("ride.pick")}
      </button>
      {picking && <p className="muted small">{t("ride.pickHint")}</p>}
      {error && (
        <p className="error" role="alert">
          {error}
        </p>
      )}
    </section>
  );
}
