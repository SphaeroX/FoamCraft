// SPDX-License-Identifier: GPL-3.0-or-later
// Openings of internal flow models: kind of model, closing openings (button or click in the view),
// caps with their role (inlet, outlet, wall) and the seal check.
import { useCallback, useEffect, useState } from "react";
import { errorMessage, type Api, type LocalizedMessage } from "../api";
import { useTranslation } from "../i18n/i18n";

type Role = "inlet" | "outlet" | "wall";

interface Localized {
  code: string;
  en: LocalizedMessage;
  de: LocalizedMessage;
}

interface OpeningInfo {
  centroid_m: [number, number, number];
  area_m2: number;
  planar: boolean;
  cap: string | null;
}

interface Report {
  model: string;
  kind: "open_shell" | "thick_walled" | "closed_volume";
  passages: number;
  openings: OpeningInfo[];
}

interface CapInfo {
  key: string;
  model: string;
  area_m2: number;
  role: Role | null;
  warnings: Localized[];
}

interface FluidReport {
  sealed: boolean;
  warnings: Localized[];
}

interface Props {
  api: Api;
  /** Changes when the project changed. */
  revision: number;
  capMode: boolean;
  onCapMode: (on: boolean) => void;
}

export function OpeningsSection({ api, revision, capMode, onCapMode }: Props) {
  const { t, language } = useTranslation();
  const [reports, setReports] = useState<Report[]>([]);
  const [caps, setCaps] = useState<CapInfo[]>([]);
  const [seal, setSeal] = useState<FluidReport | null>(null);
  const [error, setError] = useState<string | null>(null);

  const load = useCallback(async () => {
    const models = await api.call<{ id: string; name: string }[]>("geometry.list");
    const all = await Promise.all(
      models.map((m) => api.call<Report>("geometry.openings", { model: m.id })),
    );
    setReports(all);
    setCaps(await api.call<CapInfo[]>("cap.list"));
  }, [api]);

  useEffect(() => {
    setSeal(null);
    load().catch(() => {
      setReports([]);
      setCaps([]);
    });
  }, [load, revision]);

  const act = (name: string, params: Record<string, unknown>) => {
    setError(null);
    api.call(name, params).catch((e: unknown) => setError(errorMessage(e, language)));
  };

  const check = () => {
    setError(null);
    api
      .call<FluidReport>("geometry.fluid_check")
      .then(setSeal)
      .catch((e: unknown) => setError(errorMessage(e, language)));
  };

  if (reports.length === 0) return null;
  return (
    <section className="openings-section">
      <h3>{t("openings.title")}</h3>
      {reports.map((r) => {
        const closed = r.openings.filter((o) => o.cap).length;
        return (
          <div key={r.model} className="openings-model">
            <p className="small">
              {t(`openings.kind.${r.kind}`)} ·{" "}
              {t("openings.count", { n: String(r.openings.length), closed: String(closed) })}
            </p>
            <ul className="openings-list">
              {r.openings.map((o, i) =>
                o.cap ? null : (
                  <li key={i}>
                    {t("openings.opening", { n: String(i + 1) })}
                    {!o.planar && <span className="muted small"> · {t("openings.notFlat")}</span>}
                    <button
                      type="button"
                      aria-label={t("openings.closeN", { n: String(i + 1) })}
                      onClick={() =>
                        act("geometry.cap_opening", { model: r.model, point: o.centroid_m })
                      }
                    >
                      {t("openings.close")}
                    </button>
                  </li>
                ),
              )}
            </ul>
          </div>
        );
      })}
      <button type="button" aria-pressed={capMode} onClick={() => onCapMode(!capMode)}>
        {capMode ? t("openings.clickNow") : t("openings.byClick")}
      </button>
      {caps.length > 0 && (
        <ul className="caps-list" aria-label={t("openings.caps")}>
          {caps.map((c) => (
            <li key={c.key}>
              <span>{c.key}</span>
              <select
                aria-label={t("openings.roleOf", { key: c.key })}
                value={c.role ?? ""}
                onChange={(e) =>
                  act("scenario.set_boundary_role", { keys: [c.key], role: e.target.value })
                }
              >
                <option value="" disabled>
                  {t("openings.roleNone")}
                </option>
                <option value="inlet">{t("openings.inlet")}</option>
                <option value="outlet">{t("openings.outlet")}</option>
                <option value="wall">{t("openings.wall")}</option>
              </select>
              <button type="button" onClick={() => act("cap.remove", { key: c.key })}>
                {t("openings.remove")}
              </button>
              {c.warnings.map((w) => (
                <div key={w.code} className="warning small">
                  {w[language].text}
                </div>
              ))}
            </li>
          ))}
        </ul>
      )}
      <button type="button" onClick={check}>
        {t("openings.check")}
      </button>
      {seal &&
        (seal.sealed ? (
          <p className="small">{t("openings.sealed")}</p>
        ) : (
          seal.warnings.map((w) => (
            <p key={w.code} className="warning small">
              {w[language].text} {w[language].fix}
            </p>
          ))
        ))}
      {error && (
        <p role="alert" className="error">
          {error}
        </p>
      )}
    </section>
  );
}
