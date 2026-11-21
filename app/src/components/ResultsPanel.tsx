// SPDX-License-Identifier: GPL-3.0-or-later
// Step "Results": colour the model by a surface field, legend with unit and range (automatic or
// by hand), value under the mouse.
import { useEffect, useState } from "react";
import { errorMessage, isApiError, type Api } from "../api";
import { useTranslation } from "../i18n/i18n";
import { FlowViewSection } from "./FlowViewSection";
import { PresetGallery } from "./PresetGallery";
import { CompareManySection } from "./CompareManySection";
import { ExpertOnly } from "../mode";
import { ParticlesSection } from "./ParticlesSection";
import { RideSection } from "./RideSection";
import { SurfaceFlowSection } from "./SurfaceFlowSection";
import { HeavySection } from "./HeavySection";

interface FieldInfo {
  name: string;
  unit: string;
  location: "surface" | "volume";
  min: number;
  max: number;
}

interface Meta {
  job_id: string;
  fields: FieldInfo[];
  levels: { min: number[]; max: number[] }[];
  colormap: string[];
}

export interface ProbeValue {
  field: string;
  value: number;
  unit: string;
  location: string;
}

interface Props {
  api: Api;
  /** Value under the mouse, found by the app. */
  probe: ProbeValue | null;
  /** Slice chosen in the list or in the 3D view. */
  selectedSlice?: string | null;
  onSelectSlice?: (id: string | null) => void;
  /** Turns the camera to a standard view (ready-made views and the demo). */
  onView?: (name: string) => void;
  /** The next click in the view starts a ride from there. */
  onRidePick?: () => void;
  ridePicking?: boolean;
  /** Changes with every project change. */
  revision?: number;
}

export function ResultsPanel({
  api,
  probe,
  selectedSlice,
  onSelectSlice,
  onView,
  onRidePick,
  ridePicking,
  revision,
}: Props) {
  const { t, language } = useTranslation();
  const [meta, setMeta] = useState<Meta | null>(null);
  const [field, setField] = useState("p");
  const [manual, setManual] = useState(false);
  const [from, setFrom] = useState("");
  const [to, setTo] = useState("");
  const [range, setRange] = useState<[number, number] | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [none, setNone] = useState(false);

  useEffect(() => {
    api
      .call<Meta>("results.list_fields")
      .then((m) => {
        setMeta(m);
        setNone(false);
      })
      .catch((e: unknown) => {
        if (isApiError(e) && e.code === "results.none") setNone(true);
        else setError(errorMessage(e, language));
      });
  }, [api, language]);

  // Colour the model whenever field or range change.
  useEffect(() => {
    if (!meta) return;
    api
      .call("view.set_surface_field", { field, range })
      .catch((e: unknown) => setError(errorMessage(e, language)));
  }, [api, meta, field, range, language]);

  if (none) {
    return (
      <aside className="side-panel panel">
        <h2>{t("results.title")}</h2>
        <p className="small">{t("results.none")}</p>
      </aside>
    );
  }
  const surface = meta?.fields.filter((f) => f.location === "surface") ?? [];
  const info = surface.find((f) => f.name === field);
  const fmt = (n: number) => n.toLocaleString(language, { maximumSignificantDigits: 4 });
  const [lo, hi] = range ?? (info ? [info.min, info.max] : [0, 0]);
  const label = (name: string) => t(`results.field.${name}`);

  return (
    <aside className="side-panel panel results-panel">
      <h2>{t("results.title")}</h2>
      {meta && (
        <>
          <PresetGallery api={api} onView={onView ?? (() => {})} />
          <SurfaceFlowSection api={api} revision={revision} />
          <HeavySection api={api} revision={revision} />
          {onRidePick && <RideSection api={api} onPick={onRidePick} picking={ridePicking} />}
          <label className="unit-field">
            <span>{t("results.surfaceField")}</span>
            <select
              aria-label={t("results.surfaceField")}
              value={field}
              onChange={(e) => {
                setField(e.target.value);
                setRange(null);
                setManual(false);
              }}
            >
              {surface.map((f) => (
                <option key={f.name} value={f.name}>
                  {label(f.name)}
                </option>
              ))}
            </select>
          </label>
          <figure
            className="legend-bar"
            role="img"
            aria-label={`${t("results.scale")}: ${fmt(lo)} … ${fmt(hi)} ${info?.unit ?? ""}`}
          >
            <div
              className="gradient"
              style={{
                background:
                  field === "attached"
                    ? "linear-gradient(to right, #4dbd75, #db5445)"
                    : `linear-gradient(to right, ${meta.colormap.join(", ")})`,
              }}
            />
            {field === "attached" ? (
              <figcaption>
                <span>{t("surfaceFlow.attachedLegend")}</span>
              </figcaption>
            ) : (
              <figcaption>
                <span>{fmt(lo)}</span>
                <span>{info?.unit}</span>
                <span>{fmt(hi)}</span>
              </figcaption>
            )}
          </figure>
          <label className="check">
            <input
              type="checkbox"
              aria-label={t("results.manual")}
              checked={manual}
              onChange={(e) => {
                setManual(e.target.checked);
                if (!e.target.checked) setRange(null);
                else {
                  setFrom(String(lo));
                  setTo(String(hi));
                }
              }}
            />
            {t("results.manual")}
          </label>
          {manual && (
            <div className="range-inputs">
              <input
                type="number"
                step="any"
                aria-label={t("results.from")}
                value={from}
                onChange={(e) => setFrom(e.target.value)}
              />
              <input
                type="number"
                step="any"
                aria-label={t("results.to")}
                value={to}
                onChange={(e) => setTo(e.target.value)}
              />
              <button type="button" onClick={() => setRange([Number(from), Number(to)])}>
                {t("results.applyRange")}
              </button>
            </div>
          )}
          <p className="small">
            {probe
              ? `${label(probe.field)}: ${fmt(probe.value)} ${probe.unit}`
              : t("results.hover")}
          </p>
          <ExpertOnly>
            <ParticlesSection api={api} levels={meta.levels} />
            <FlowViewSection
              api={api}
              levels={meta.levels}
              selected={selectedSlice}
              onSelect={onSelectSlice}
            />
          </ExpertOnly>
          <CompareManySection api={api} revision={0} />
          <p className="muted small">{t("results.gridNote")}</p>
        </>
      )}
      {error && (
        <p role="alert" className="error">
          {error}
        </p>
      )}
    </aside>
  );
}
