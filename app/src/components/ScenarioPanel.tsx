// SPDX-License-Identifier: GPL-3.0-or-later
// Step "Scenario": wind tunnel inputs and the reference values FoamCraft derives from them. The
// mesh is not built here: the "Run" step meshes and solves in one go.
import { useEffect, useState } from "react";
import { errorMessage, isApiError, type Api, type LocalizedMessage } from "../api";
import { useTranslation } from "../i18n/i18n";
import { InternalFlowSection } from "./InternalFlowSection";
import { fromDisplaySpeed, speedUnit, toDisplaySpeed, type DisplayUnits } from "../units";

type Ground = "none" | "fixed" | "moving";
type MediumKind = "air" | "water" | "custom";
type Quality = "draft" | "standard" | "fine";

interface Coded {
  code: string;
  en: LocalizedMessage;
  de: LocalizedMessage;
}

interface ScenarioResult {
  scenario: {
    speed_m_s: number;
    direction_deg: number;
    ground: Ground;
    medium:
      | { kind: "air" | "water"; temperature_c: number }
      | { kind: "custom"; density: number; kinematic_viscosity: number };
    quality: Quality;
  };
  frontal_area_m2: number;
  reference_length_m: number;
  reynolds: number;
  mach: number | null;
  flow: Coded;
  budget: { cells: number; iterations: number; estimate_s: number; limited_by_memory: boolean };
  warnings: Coded[];
}

interface Props {
  api: Api;
  units: DisplayUnits;
}

function numberField(
  label: string,
  value: string,
  set: (v: string) => void,
  options: { min?: number; step?: number } = {},
) {
  return (
    <label className="unit-field">
      <span>{label}</span>
      <input
        type="number"
        aria-label={label}
        value={value}
        step={options.step ?? "any"}
        min={options.min}
        onChange={(e) => set(e.target.value)}
      />
    </label>
  );
}

export function ScenarioPanel({ api, units }: Props) {
  const { t, language } = useTranslation();
  const [speed, setSpeed] = useState(String(toDisplaySpeed(20, units)));
  const [direction, setDirection] = useState("0");
  const [ground, setGround] = useState<Ground>("fixed");
  const [medium, setMedium] = useState<MediumKind>("air");
  const [temperature, setTemperature] = useState("20");
  const [density, setDensity] = useState("1000");
  const [viscosity, setViscosity] = useState("0.000001");
  const [quality, setQuality] = useState<Quality>("standard");
  const [result, setResult] = useState<ScenarioResult | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [kind, setKind] = useState<"wind_tunnel" | "internal_flow">("wind_tunnel");

  // Show a saved scenario.
  useEffect(() => {
    api
      .call<ScenarioResult>("scenario.info")
      .then((r) => {
        if ((r.scenario as { kind?: string }).kind === "internal_flow") {
          setKind("internal_flow");
          return;
        }
        const s = r.scenario;
        setSpeed(String(Number(toDisplaySpeed(s.speed_m_s, units).toPrecision(6))));
        setDirection(String(s.direction_deg));
        setGround(s.ground);
        setMedium(s.medium.kind);
        if (s.medium.kind === "custom") {
          setDensity(String(s.medium.density));
          setViscosity(String(s.medium.kinematic_viscosity));
        } else {
          setTemperature(String(s.medium.temperature_c));
        }
        setQuality(s.quality);
        setResult(r);
      })
      .catch((e: unknown) => {
        if (!(isApiError(e) && e.code === "scenario.none")) setError(errorMessage(e, language));
      });
  }, [api, units, language]);

  const apply = () => {
    setError(null);
    const v = Number(speed);
    if (!Number.isFinite(v) || v <= 0) {
      setError(t("scenario.speedInvalid"));
      return;
    }
    const mediumSpec =
      medium === "custom"
        ? { kind: "custom", density: Number(density), kinematic_viscosity: Number(viscosity) }
        : { kind: medium, temperature_c: Number(temperature) };
    api
      .call<ScenarioResult>("scenario.set_wind_tunnel", {
        speed_m_s: Number(fromDisplaySpeed(v, units).toPrecision(12)),
        direction_deg: Number(direction),
        ground,
        medium: mediumSpec,
        quality,
      })
      .then(setResult)
      .catch((e: unknown) => setError(errorMessage(e, language)));
  };

  const fmt = (n: number, digits = 3) =>
    n.toLocaleString(language, { maximumSignificantDigits: digits });
  const minutes = result ? Math.max(1, Math.round(result.budget.estimate_s / 60)) : 0;

  return (
    <aside className="side-panel panel">
      <h2>{kind === "internal_flow" ? t("steps.scenario") : t("scenario.title")}</h2>
      <label className="unit-field">
        <span>{t("internal.kind")}</span>
        <select
          aria-label={t("internal.kind")}
          value={kind}
          onChange={(e) => setKind(e.target.value as "wind_tunnel" | "internal_flow")}
        >
          <option value="wind_tunnel">{t("internal.windTunnel")}</option>
          <option value="internal_flow">{t("internal.internalFlow")}</option>
        </select>
      </label>
      {kind === "internal_flow" ? (
        <InternalFlowSection api={api} />
      ) : (
        <>
          {numberField(t("scenario.speed", { unit: speedUnit(units) }), speed, setSpeed, {
            min: 0,
          })}
          {numberField(t("scenario.direction"), direction, setDirection, { step: 5 })}
          <label className="unit-field">
            <span>{t("scenario.ground")}</span>
            <select
              aria-label={t("scenario.ground")}
              value={ground}
              onChange={(e) => setGround(e.target.value as Ground)}
            >
              <option value="none">{t("scenario.groundNone")}</option>
              <option value="fixed">{t("scenario.groundFixed")}</option>
              <option value="moving">{t("scenario.groundMoving")}</option>
            </select>
          </label>
          <label className="unit-field">
            <span>{t("scenario.medium")}</span>
            <select
              aria-label={t("scenario.medium")}
              value={medium}
              onChange={(e) => setMedium(e.target.value as MediumKind)}
            >
              <option value="air">{t("scenario.air")}</option>
              <option value="water">{t("scenario.water")}</option>
              <option value="custom">{t("scenario.custom")}</option>
            </select>
          </label>
          {medium === "custom" ? (
            <>
              {numberField(t("scenario.density"), density, setDensity, { min: 0 })}
              {numberField(t("scenario.viscosity"), viscosity, setViscosity, { min: 0 })}
            </>
          ) : (
            numberField(t("scenario.temperature"), temperature, setTemperature)
          )}
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
          {result && (
            <section className="scenario-result">
              <ul className="diagnosis">
                {result.warnings.map((w) => (
                  <li key={w.code} className="warning">
                    {w[language].text}
                    {w[language].fix && <div className="muted small">{w[language].fix}</div>}
                  </li>
                ))}
              </ul>
              <dl>
                <dt>{t("scenario.reynolds")}</dt>
                <dd>{fmt(result.reynolds, 5)}</dd>
                <dt>{t("scenario.area")}</dt>
                <dd>{fmt(result.frontal_area_m2)} m²</dd>
                <dt>{t("scenario.length")}</dt>
                <dd>{fmt(result.reference_length_m)} m</dd>
                {result.mach !== null && (
                  <>
                    <dt>{t("scenario.mach")}</dt>
                    <dd>{fmt(result.mach, 2)}</dd>
                  </>
                )}
                <dt>{t("scenario.budget")}</dt>
                <dd>
                  {t("scenario.cells", { n: result.budget.cells.toLocaleString(language) })}
                  {result.budget.limited_by_memory && ` (${t("scenario.memoryLimited")})`}
                </dd>
                <dt>{t("scenario.time")}</dt>
                <dd>{t("scenario.minutes", { n: String(minutes) })}</dd>
              </dl>
              <p className="small">{result.flow[language].text}</p>
            </section>
          )}
        </>
      )}
    </aside>
  );
}
