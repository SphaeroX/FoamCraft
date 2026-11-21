// SPDX-License-Identifier: GPL-3.0-or-later
// Step "Geometry": import models, set unit, rotation and position, read the diagnosis.
import { useCallback, useEffect, useState } from "react";
import { errorMessage, type Api, type GeometryInfo } from "../api";
import { useTranslation } from "../i18n/i18n";
import {
  formatNumber,
  fromDisplayLength,
  lengthUnit,
  toDisplayLength,
  type DisplayUnits,
} from "../units";

const UNITS = ["um", "mm", "cm", "m", "in", "ft"] as const;

interface Props {
  api: Api;
  /** Lets the user choose a model file; null when cancelled. */
  pickModel: () => Promise<string | null>;
  units: DisplayUnits;
}

interface NumberFieldProps {
  label: string;
  value: number;
  onCommit: (value: number) => void;
}

/** Text field that commits a number when it loses focus or on Enter. */
function NumberField({ label, value, onCommit }: NumberFieldProps) {
  const [text, setText] = useState(formatNumber(value));
  useEffect(() => setText(formatNumber(value)), [value]);
  const commit = () => {
    const parsed = Number(text.replace(",", "."));
    if (Number.isFinite(parsed) && parsed !== value) onCommit(parsed);
    else setText(formatNumber(value));
  };
  return (
    <label className="number-field">
      <span>{label}</span>
      <input
        type="text"
        inputMode="decimal"
        aria-label={label}
        value={text}
        onChange={(e) => setText(e.target.value)}
        onBlur={commit}
        onKeyDown={(e) => {
          if (e.key === "Enter") commit();
        }}
      />
    </label>
  );
}

function ModelCard({
  info,
  units,
  run,
}: {
  info: GeometryInfo;
  units: DisplayUnits;
  run: (name: string, params: Record<string, unknown>) => void;
}) {
  const { t, language } = useTranslation();
  const lu = lengthUnit(units);
  const size = info.size_m.map((m) => formatNumber(toDisplayLength(m, units))).join(" × ");
  const id = info.id;
  const rotation = (k: number, v: number) => {
    const r = [...info.rotation_deg];
    r[k] = v;
    run("geometry.set_transform", { id, rotation_deg: r });
  };
  const move = (k: number, v: number) => {
    const p = [...info.translation];
    p[k] = fromDisplayLength(v, units);
    run("geometry.set_transform", { id, translation: p });
  };
  return (
    <article className="model-card">
      <header>
        <strong>{info.name}</strong>
        <span className="muted small">
          {info.diagnosis.triangles} {t("geometry.triangles")}
        </span>
      </header>
      <p className="small">
        {t("geometry.size")}: {size} {lu}
      </p>
      <label className="unit-field">
        <span>{t("geometry.unit")}</span>
        <select
          aria-label={t("geometry.unit")}
          value={info.unit}
          onChange={(e) => run("geometry.set_unit", { id, unit: e.target.value })}
        >
          {UNITS.map((u) => (
            <option key={u} value={u}>
              {t(`units.${u}`)}
            </option>
          ))}
        </select>
        {info.suggested_unit !== info.unit && (
          <span className="muted small">
            {t("geometry.suggested", { unit: t(`units.${info.suggested_unit}`) })}
          </span>
        )}
      </label>
      <fieldset>
        <legend>{t("geometry.rotation")}</legend>
        {(["yaw", "pitch", "roll"] as const).map((axis, k) => (
          <NumberField
            key={axis}
            label={t(`geometry.${axis}`)}
            value={info.rotation_deg[k]}
            onCommit={(v) => rotation(k, v)}
          />
        ))}
      </fieldset>
      <fieldset>
        <legend>{t("geometry.position")}</legend>
        {(["X", "Y", "Z"] as const).map((axis, k) => (
          <NumberField
            key={axis}
            label={`${axis} (${lu})`}
            value={toDisplayLength(info.translation[k], units)}
            onCommit={(v) => move(k, v)}
          />
        ))}
      </fieldset>
      <div className="actions">
        <button type="button" onClick={() => run("geometry.place_on_ground", { id })}>
          {t("geometry.ground")}
        </button>
        {!info.fix_normals && (
          <button type="button" onClick={() => run("geometry.fix_normals", { id })}>
            {t("geometry.fixNormals")}
          </button>
        )}
        <button type="button" onClick={() => run("geometry.remove", { id })}>
          {t("geometry.remove")}
        </button>
      </div>
      <ul className="diagnosis">
        {info.messages.map((m) => (
          <li key={m.code} className={m.severity}>
            <span>{m[language].text}</span>
            {m.count > 1 && <span className="muted"> ({m.count})</span>}
            {m.severity === "warning" && <div className="muted small">{m[language].fix}</div>}
          </li>
        ))}
      </ul>
    </article>
  );
}

export function GeometryPanel({ api, pickModel, units }: Props) {
  const { t, language } = useTranslation();
  const [models, setModels] = useState<GeometryInfo[]>([]);
  const [error, setError] = useState<string | null>(null);

  const reload = useCallback(async () => {
    const list = await api.call<{ id: string }[]>("geometry.list");
    const infos = await Promise.all(
      list.map((m) => api.call<GeometryInfo>("geometry.info", { id: m.id })),
    );
    setModels(infos);
  }, [api]);

  useEffect(() => {
    reload().catch((e: unknown) => setError(errorMessage(e, language)));
  }, [reload, language]);

  const run = (name: string, params: Record<string, unknown>) => {
    setError(null);
    api
      .call<GeometryInfo | null>(name, params)
      .then((info) => {
        if (info && name !== "geometry.remove") {
          setModels((all) => all.map((m) => (m.id === info.id ? info : m)));
        } else {
          return reload();
        }
      })
      .catch((e: unknown) => setError(errorMessage(e, language)));
  };

  const importModel = async () => {
    setError(null);
    try {
      const path = await pickModel();
      if (!path) return;
      await api.call<GeometryInfo>("geometry.import", { path });
      await reload();
    } catch (e) {
      setError(errorMessage(e, language));
    }
  };

  return (
    <aside className="side-panel panel">
      <h2>{t("geometry.title")}</h2>
      <button type="button" onClick={importModel}>
        {t("geometry.import")}
      </button>
      {error && (
        <p role="alert" className="error">
          {error}
        </p>
      )}
      {models.length === 0 && <p className="muted">{t("geometry.empty")}</p>}
      {models.map((m) => (
        <ModelCard key={m.id} info={m} units={units} run={run} />
      ))}
    </aside>
  );
}
