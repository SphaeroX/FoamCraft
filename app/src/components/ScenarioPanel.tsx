// SPDX-License-Identifier: GPL-3.0-or-later
// Step "Scenario": wind tunnel inputs and the reference values FoamCraft derives from them. The
// mesh is not built here: the "Run" step meshes and solves in one go. Changes apply on their own a
// moment after the last input; there is no "Apply" button.
import { useCallback, useEffect, useRef, useState } from "react";
import { errorMessage, isApiError, type Api, type LocalizedMessage } from "../api";
import { useTranslation } from "../i18n/i18n";
import { InternalFlowSection } from "./InternalFlowSection";
import { WindRose } from "./WindRose";
import { JetsSection } from "./JetsSection";
import {
  formatNumber,
  fromDisplaySpeed,
  speedUnit,
  toDisplaySpeed,
  type DisplayUnits,
} from "../units";

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
  /** Changes when the project changed elsewhere (e.g. the wind arrow in the view). */
  revision?: number;
  /** Shown under the title (the scenario tabs). */
  header?: React.ReactNode;
}

/** Milliseconds after the last input before the scenario is applied. */
const APPLY_DELAY = 400;
/** Typical speeds in m/s offered as buttons (walking pace to motorway). */
const QUICK_SPEEDS = [5, 13.8889, 27.7778, 36.1111];

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

/** Small pictures for the ground choice. */
function GroundIcon({ ground }: { ground: Ground }) {
  return (
    <svg width="40" height="24" viewBox="0 0 40 24" aria-hidden="true">
      <rect x="14" y="6" width="12" height="9" rx="2" className="ground-icon-body" />
      {ground !== "none" && <line x1="2" y1="20" x2="38" y2="20" className="ground-icon-floor" />}
      {ground === "moving" && (
        <path d="M6 23 H16 M12 21 L16 23 L12 25" className="ground-icon-floor" />
      )}
    </svg>
  );
}

export function ScenarioPanel({ api, units, revision = 0, header }: Props) {
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
  const [outline, setOutline] = useState<[number, number, number, number] | null>(null);
  // Counts inputs by the user; loading a saved scenario does not count, so it is not re-applied.
  const [edits, setEdits] = useState(0);
  const edited = useRef(false);
  const touch =
    <T,>(set: (v: T) => void) =>
    (v: T) => {
      edited.current = true;
      setEdits((n) => n + 1);
      set(v);
    };

  // Show the saved scenario (again after changes elsewhere, unless the user is typing).
  useEffect(() => {
    if (edited.current) return;
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
  }, [api, units, language, revision]);

  // Outline of the models from above for the wind rose.
  useEffect(() => {
    api
      .call<{ id: string }[]>("geometry.list", {})
      .then((list) =>
        Promise.all(
          list.map((m) =>
            api.call<{ bounds_m: { min: number[]; max: number[] } }>("geometry.info", {
              id: m.id,
            }),
          ),
        ),
      )
      .then((infos) => {
        if (infos.length === 0) return;
        const b = infos.map((i) => i.bounds_m);
        setOutline([
          Math.min(...b.map((x) => x.min[0])),
          Math.min(...b.map((x) => x.min[1])),
          Math.max(...b.map((x) => x.max[0])),
          Math.max(...b.map((x) => x.max[1])),
        ]);
      })
      .catch(() => setOutline(null));
  }, [api, revision]);

  const valid = (() => {
    const v = Number(speed);
    return Number.isFinite(v) && v > 0;
  })();

  const apply = useCallback(() => {
    const v = Number(speed);
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
      .then((r) => {
        setError(null);
        setResult(r);
      })
      .catch((e: unknown) => setError(errorMessage(e, language)))
      .finally(() => (edited.current = false));
  }, [
    api,
    speed,
    direction,
    ground,
    medium,
    density,
    viscosity,
    temperature,
    quality,
    units,
    language,
  ]);

  // Apply a moment after the last input by the user; an invalid speed shows the error at once.
  useEffect(() => {
    if (edits === 0 || kind !== "wind_tunnel") return;
    if (!valid) {
      setError(t("scenario.speedInvalid"));
      edited.current = false;
      return;
    }
    setError(null);
    const timer = setTimeout(apply, APPLY_DELAY);
    return () => clearTimeout(timer);
    // Only new inputs start the timer; `apply` changes with them.
  }, [edits]);

  const fmt = (n: number, digits = 3) =>
    n.toLocaleString(language, { maximumSignificantDigits: digits });
  const minutes = result ? Math.max(1, Math.round(result.budget.estimate_s / 60)) : 0;
  const grounds: Ground[] = ["none", "fixed", "moving"];
  const groundText = {
    none: t("scenario.groundNone"),
    fixed: t("scenario.groundFixed"),
    moving: t("scenario.groundMoving"),
  };

  return (
    <aside className="side-panel panel">
      <h2>{kind === "internal_flow" ? t("steps.scenario") : t("scenario.title")}</h2>
      {header}
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
          {numberField(t("scenario.speed", { unit: speedUnit(units) }), speed, touch(setSpeed), {
            min: 0,
          })}
          <div className="quick-speeds">
            {QUICK_SPEEDS.map((ms) => {
              const shown = formatNumber(Math.round(toDisplaySpeed(ms, units)));
              return (
                <button
                  key={ms}
                  type="button"
                  className="chip"
                  onClick={() => touch(setSpeed)(shown)}
                >
                  {`${shown} ${speedUnit(units)}`}
                </button>
              );
            })}
          </div>
          <WindRose
            value={Number(direction) || 0}
            onChange={(d) => touch(setDirection)(String(d))}
            outline={outline}
          />
          {numberField(t("scenario.direction"), direction, touch(setDirection), { step: 5 })}
          <fieldset className="ground-choice">
            <legend>{t("scenario.ground")}</legend>
            <div role="radiogroup" className="ground-buttons">
              {grounds.map((g) => (
                <button
                  key={g}
                  type="button"
                  role="radio"
                  aria-checked={ground === g}
                  aria-label={groundText[g]}
                  title={groundText[g]}
                  disabled={g === "moving"}
                  className={ground === g ? "active" : ""}
                  onClick={() => touch(setGround)(g)}
                >
                  <GroundIcon ground={g} />
                </button>
              ))}
            </div>
            {/* The same choice as a list, for keyboards and screen readers. */}
            <select
              aria-label={t("scenario.ground")}
              className="ground-select"
              value={ground}
              onChange={(e) => touch(setGround)(e.target.value as Ground)}
            >
              <option value="none">{groundText.none}</option>
              <option value="fixed">{groundText.fixed}</option>
              <option value="moving" disabled>
                {groundText.moving}
              </option>
            </select>
          </fieldset>
          <JetsSection api={api} units={units} revision={revision} />
          <label className="unit-field">
            <span>{t("scenario.medium")}</span>
            <select
              aria-label={t("scenario.medium")}
              value={medium}
              onChange={(e) => touch(setMedium)(e.target.value as MediumKind)}
            >
              <option value="air">{t("scenario.air")}</option>
              <option value="water">{t("scenario.water")}</option>
              <option value="custom">{t("scenario.custom")}</option>
            </select>
          </label>
          {medium === "custom" ? (
            <>
              {numberField(t("scenario.density"), density, touch(setDensity), { min: 0 })}
              {numberField(t("scenario.viscosity"), viscosity, touch(setViscosity), { min: 0 })}
            </>
          ) : (
            numberField(t("scenario.temperature"), temperature, touch(setTemperature))
          )}
          <label className="unit-field">
            <span>{t("scenario.quality")}</span>
            <select
              aria-label={t("scenario.quality")}
              value={quality}
              onChange={(e) => touch(setQuality)(e.target.value as Quality)}
            >
              <option value="draft">{t("scenario.draft")}</option>
              <option value="standard">{t("scenario.standard")}</option>
              <option value="fine">{t("scenario.fine")}</option>
            </select>
          </label>
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
              <p className="small">{result.flow[language].text}</p>
              <details>
                <summary>{t("scenario.details")}</summary>
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
              </details>
            </section>
          )}
        </>
      )}
    </aside>
  );
}
