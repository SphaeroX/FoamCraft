// SPDX-License-Identifier: GPL-3.0-or-later
// Slices, isosurfaces and cutting the model open; the settings are the stored scene of the
// project (view.* commands).
import { useCallback, useEffect, useState } from "react";
import { errorMessage, type Api } from "../api";
import { useTranslation } from "../i18n/i18n";

type Vec3 = [number, number, number];
type Axis = "x" | "y" | "z";
const AXES: Axis[] = ["x", "y", "z"];

interface Level {
  min: number[];
  max: number[];
}

interface Slice {
  id: string;
  origin: Vec3;
  normal: Vec3;
  field: string;
}

interface Iso {
  id: string;
  field: string;
  value: number;
}

interface ViewState {
  slices: Slice[];
  isosurfaces: Iso[];
  clip: { origin: Vec3; normal: Vec3 } | null;
}

interface Props {
  api: Api;
  /** Grid levels of the result (the first spans the whole domain). */
  levels: Level[];
}

const ISO_FIELDS = ["U_mag", "p_total", "Q"];

/** The view state with missing lists filled in. */
function normalise(v: Partial<ViewState> | null | undefined): ViewState {
  return { slices: v?.slices ?? [], isosurfaces: v?.isosurfaces ?? [], clip: v?.clip ?? null };
}

export function FlowViewSection({ api, levels }: Props) {
  const { t, language } = useTranslation();
  const [view, setView] = useState<ViewState>({ slices: [], isosurfaces: [], clip: null });
  const [fields, setFields] = useState<{ name: string; unit: string }[]>([]);
  const [axis, setAxis] = useState<Axis>("z");
  const [sliceField, setSliceField] = useState("U_mag");
  const [isoField, setIsoField] = useState("Q");
  const [isoValue, setIsoValue] = useState("");
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    api
      .call<ViewState>("view.state")
      .then((v) => setView(normalise(v)))
      .catch(() => {});
    api
      .call<{ name: string; unit: string }[]>("view.volume_fields")
      .then((f) => setFields(Array.isArray(f) ? f : []))
      .catch(() => {});
  }, [api]);

  const act = useCallback(
    (name: string, params: Record<string, unknown>) => {
      setError(null);
      api
        .call<ViewState>(name, params)
        .then((v) => setView(normalise(v)))
        .catch((e: unknown) => setError(errorMessage(e, language)));
    },
    [api, language],
  );

  const box = levels[0] ?? { min: [0, 0, 0], max: [1, 1, 1] };
  const centre = [0, 1, 2].map((k) => (box.min[k] + box.max[k]) / 2) as Vec3;
  const axisOf = (normal: Vec3) => normal.findIndex((c) => Math.abs(c) > 0.5);
  const label = (name: string) => t(`results.field.${name}`);

  const addSlice = () => {
    const k = AXES.indexOf(axis);
    const normal = [0, 0, 0] as Vec3;
    normal[k] = 1;
    act("view.add_slice", { origin: centre, normal, field: sliceField });
  };

  const move = (s: Slice, value: number) => {
    const k = axisOf(s.normal);
    const origin = [...s.origin] as Vec3;
    origin[k] = value;
    setView((v) => ({
      ...v,
      slices: v.slices.map((x) => (x.id === s.id ? { ...x, origin } : x)),
    }));
    act("view.update_slice", { id: s.id, origin });
  };

  const first = view.slices[0];
  return (
    <section className="flow-view">
      <h3>{t("flowView.slices")}</h3>
      <div className="row">
        <select
          aria-label={t("flowView.normal")}
          value={axis}
          onChange={(e) => setAxis(e.target.value as Axis)}
        >
          {AXES.map((a) => (
            <option key={a} value={a}>
              {t(`flowView.axis.${a}`)}
            </option>
          ))}
        </select>
        <select
          aria-label={t("flowView.sliceField")}
          value={sliceField}
          onChange={(e) => setSliceField(e.target.value)}
        >
          {fields.map((f) => (
            <option key={f.name} value={f.name}>
              {label(f.name)}
            </option>
          ))}
        </select>
        <button type="button" onClick={addSlice}>
          {t("flowView.addSlice")}
        </button>
      </div>
      <ul className="view-list">
        {view.slices.map((s) => {
          const k = Math.max(0, axisOf(s.normal));
          return (
            <li key={s.id} aria-label={s.id}>
              <span>
                {s.id} · {label(s.field)}
              </span>
              <input
                type="range"
                min={box.min[k]}
                max={box.max[k]}
                step={(box.max[k] - box.min[k]) / 200}
                value={s.origin[k]}
                onChange={(e) => move(s, Number(e.target.value))}
              />
              <button type="button" onClick={() => act("view.remove", { id: s.id })}>
                {t("openings.remove")}
              </button>
            </li>
          );
        })}
      </ul>
      <h3>{t("flowView.isosurfaces")}</h3>
      <div className="row">
        <select
          aria-label={t("flowView.isoField")}
          value={isoField}
          onChange={(e) => setIsoField(e.target.value)}
        >
          {ISO_FIELDS.map((f) => (
            <option key={f} value={f}>
              {label(f)}
            </option>
          ))}
        </select>
        <input
          type="number"
          step="any"
          aria-label={t("flowView.isoValue")}
          value={isoValue}
          onChange={(e) => setIsoValue(e.target.value)}
        />
        <button
          type="button"
          onClick={() => act("view.add_isosurface", { field: isoField, value: Number(isoValue) })}
        >
          {t("flowView.addIso")}
        </button>
      </div>
      <ul className="view-list">
        {view.isosurfaces.map((s) => (
          <li key={s.id} aria-label={s.id}>
            <span>
              {s.id} · {label(s.field)} = {s.value.toLocaleString(language)}
            </span>
            <button type="button" onClick={() => act("view.remove", { id: s.id })}>
              {t("openings.remove")}
            </button>
          </li>
        ))}
      </ul>
      {(first || view.clip) && (
        <label className="check">
          <input
            type="checkbox"
            aria-label={t("flowView.clip")}
            checked={view.clip !== null}
            onChange={(e) =>
              act("view.set_clip", {
                plane:
                  e.target.checked && first ? { origin: first.origin, normal: first.normal } : null,
              })
            }
          />
          {t("flowView.clip")}
        </label>
      )}
      {view.clip && (
        <button
          type="button"
          onClick={() =>
            view.clip &&
            act("view.set_clip", {
              plane: {
                origin: view.clip.origin,
                normal: view.clip.normal.map((c) => -c) as Vec3,
              },
            })
          }
        >
          {t("flowView.flip")}
        </button>
      )}
      {error && (
        <p role="alert" className="error">
          {error}
        </p>
      )}
    </section>
  );
}
