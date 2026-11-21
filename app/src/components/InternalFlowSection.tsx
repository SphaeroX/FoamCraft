// SPDX-License-Identifier: GPL-3.0-or-later
// Scenario "internal flow": what is given at each inlet, the pressure at each outlet, medium and
// quality; shows hydraulic diameters, Reynolds number and laminar or turbulent.
import { useEffect, useState } from "react";
import { errorMessage, isApiError, type Api, type LocalizedMessage } from "../api";
import { useTranslation } from "../i18n/i18n";

type ConditionType = "velocity" | "volume_flow" | "mass_flow" | "total_pressure";
type MediumKind = "air" | "water";
type Quality = "draft" | "standard" | "fine";

const VALUE_FIELD: Record<ConditionType, string> = {
  velocity: "m_s",
  volume_flow: "m3_s",
  mass_flow: "kg_s",
  total_pressure: "pa",
};

interface Localized {
  code: string;
  en: LocalizedMessage;
  de: LocalizedMessage;
}

interface Info {
  reynolds: number;
  regime: "laminar" | "transitional" | "turbulent";
  flow: Localized;
  openings: { key: string; role: string; hydraulic_diameter_m: number }[];
  warnings: Localized[];
}

interface Saved {
  scenario: {
    kind: string;
    medium?: { kind: MediumKind; temperature_c: number };
    quality?: Quality;
    inlets?: { key: string; condition: Record<string, number | string> }[];
    outlets?: { key: string; pressure_pa: number }[];
  };
}

export function InternalFlowSection({ api }: { api: Api }) {
  const { t, language } = useTranslation();
  const [inlets, setInlets] = useState<string[]>([]);
  const [outlets, setOutlets] = useState<string[]>([]);
  const [types, setTypes] = useState<Record<string, ConditionType>>({});
  const [values, setValues] = useState<Record<string, string>>({});
  const [pressures, setPressures] = useState<Record<string, string>>({});
  const [medium, setMedium] = useState<MediumKind>("water");
  const [temperature, setTemperature] = useState("20");
  const [quality, setQuality] = useState<Quality>("standard");
  const [info, setInfo] = useState<Info | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [loaded, setLoaded] = useState(false);

  useEffect(() => {
    api
      .call<Record<string, string>>("scenario.boundary_roles")
      .then((roles) => {
        const keys = Object.keys(roles).sort();
        setInlets(keys.filter((k) => roles[k] === "inlet"));
        setOutlets(keys.filter((k) => roles[k] === "outlet"));
        setLoaded(true);
        return api.call<Saved & Info>("scenario.info");
      })
      .then((saved) => {
        const s = saved.scenario;
        if (s.kind !== "internal_flow") return;
        if (s.medium) {
          setMedium(s.medium.kind);
          setTemperature(String(s.medium.temperature_c));
        }
        if (s.quality) setQuality(s.quality);
        for (const i of s.inlets ?? []) {
          const type = i.condition.type as ConditionType;
          setTypes((old) => ({ ...old, [i.key]: type }));
          setValues((old) => ({ ...old, [i.key]: String(i.condition[VALUE_FIELD[type]]) }));
        }
        for (const o of s.outlets ?? []) {
          setPressures((old) => ({ ...old, [o.key]: String(o.pressure_pa) }));
        }
        setInfo(saved);
      })
      .catch((e: unknown) => {
        if (!(isApiError(e) && e.code === "scenario.none")) setError(errorMessage(e, language));
      });
  }, [api, language]);

  const apply = () => {
    setError(null);
    api
      .call<Info>("scenario.set_internal_flow", {
        medium: { kind: medium, temperature_c: Number(temperature) },
        quality,
        inlets: inlets.map((key) => {
          const type = types[key] ?? "velocity";
          return { key, condition: { type, [VALUE_FIELD[type]]: Number(values[key] ?? "0") } };
        }),
        outlets: outlets.map((key) => ({ key, pressure_pa: Number(pressures[key] ?? "0") })),
      })
      .then(setInfo)
      .catch((e: unknown) => setError(errorMessage(e, language)));
  };

  if (loaded && inlets.length + outlets.length === 0) {
    return <p className="small">{t("internal.markFirst")}</p>;
  }
  const dh = (key: string) => info?.openings.find((o) => o.key === key)?.hydraulic_diameter_m;
  return (
    <section className="internal-flow">
      <h3>{t("internal.inlets")}</h3>
      <ul aria-label={t("internal.inlets")} className="boundary-list">
        {inlets.map((key) => (
          <li key={key}>
            <span>{key}</span>
            <select
              aria-label={t("internal.givenAt", { key })}
              value={types[key] ?? "velocity"}
              onChange={(e) =>
                setTypes((old) => ({ ...old, [key]: e.target.value as ConditionType }))
              }
            >
              <option value="velocity">{t("internal.velocity")}</option>
              <option value="volume_flow">{t("internal.volumeFlow")}</option>
              <option value="mass_flow">{t("internal.massFlow")}</option>
              <option value="total_pressure">{t("internal.totalPressure")}</option>
            </select>
            <input
              type="number"
              step="any"
              aria-label={t("internal.valueAt", { key })}
              value={values[key] ?? ""}
              onChange={(e) => setValues((old) => ({ ...old, [key]: e.target.value }))}
            />
            {dh(key) !== undefined && (
              <span className="muted small">
                {t("internal.dh", {
                  mm: (dh(key)! * 1000).toLocaleString(language, { maximumSignificantDigits: 3 }),
                })}
              </span>
            )}
          </li>
        ))}
      </ul>
      <h3>{t("internal.outlets")}</h3>
      <ul aria-label={t("internal.outlets")} className="boundary-list">
        {outlets.map((key) => (
          <li key={key}>
            <span>{key}</span>
            <input
              type="number"
              step="any"
              aria-label={t("internal.pressureAt", { key })}
              value={pressures[key] ?? "0"}
              onChange={(e) => setPressures((old) => ({ ...old, [key]: e.target.value }))}
            />
          </li>
        ))}
      </ul>
      <label className="unit-field">
        <span>{t("scenario.medium")}</span>
        <select
          aria-label={t("scenario.medium")}
          value={medium}
          onChange={(e) => setMedium(e.target.value as MediumKind)}
        >
          <option value="water">{t("scenario.water")}</option>
          <option value="air">{t("scenario.air")}</option>
        </select>
      </label>
      <label className="unit-field">
        <span>{t("scenario.temperature")}</span>
        <input
          type="number"
          aria-label={t("scenario.temperature")}
          value={temperature}
          onChange={(e) => setTemperature(e.target.value)}
        />
      </label>
      <label className="unit-field">
        <span>{t("scenario.quality")}</span>
        <select
          aria-label={t("scenario.quality")}
          value={quality}
          onChange={(e) => setQuality(e.target.value as Quality)}
        >
          <option value="draft">{t("scenario.draft")}</option>
          <option value="standard">{t("scenario.standard")}</option>
          <option value="fine">{t("scenario.fine")}</option>
        </select>
      </label>
      <button type="button" onClick={apply}>
        {t("scenario.apply")}
      </button>
      {error && (
        <p role="alert" className="error">
          {error}
        </p>
      )}
      {info && (
        <section className="scenario-result">
          <ul className="diagnosis">
            {info.warnings.map((w) => (
              <li key={w.code} className="warning">
                {w[language].text}
              </li>
            ))}
          </ul>
          <dl>
            <dt>{t("scenario.reynolds")}</dt>
            <dd>{info.reynolds.toLocaleString(language, { maximumSignificantDigits: 4 })}</dd>
          </dl>
          <p className="small">{info.flow[language].text}</p>
        </section>
      )}
    </section>
  );
}
