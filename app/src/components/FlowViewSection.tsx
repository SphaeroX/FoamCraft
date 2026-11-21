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
  hidden?: boolean;
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
  /** What the clip plane cuts: model, particles, streamlines, isosurfaces. */
  clip_targets?: string[];
}

const CLIP_TARGETS = ["model", "particles", "streamlines", "isosurfaces"] as const;

interface Props {
  api: Api;
  /** Grid levels of the result (the first spans the whole domain). */
  levels: Level[];
  /** Slice chosen in the list or in the 3D view. */
  selected?: string | null;
  onSelect?: (id: string | null) => void;
}

const ISO_FIELDS = ["U_mag", "p_total", "Q"];

/** The view state with missing lists filled in. */
function normalise(v: Partial<ViewState> | null | undefined): ViewState {
  return {
    slices: v?.slices ?? [],
    isosurfaces: v?.isosurfaces ?? [],
    clip: v?.clip ?? null,
    clip_targets: v?.clip_targets ?? ["model"],
  };
}

export function FlowViewSection({ api, levels, selected = null, onSelect }: Props) {
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
  const selectedSlice = view.slices.find((s) => s.id === selected);
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
            <li key={s.id} aria-label={s.id} className={selected === s.id ? "selected" : ""}>
              <span>
                {s.id} · {label(s.field)}
              </span>
              <button
                type="button"
                aria-pressed={selected === s.id}
                onClick={() => onSelect?.(selected === s.id ? null : s.id)}
              >
                {t("flowView.select")}
              </button>
              <label className="check">
                <input
                  type="checkbox"
                  aria-label={t("flowView.visible")}
                  checked={!s.hidden}
                  onChange={(e) =>
                    act("view.update_slice", { id: s.id, hidden: !e.target.checked })
                  }
                />
                {t("flowView.visible")}
              </label>
              {selected === s.id &&
                AXES.map((a, i) => (
                  <button
                    key={a}
                    type="button"
                    aria-label={t("flowView.alongAxis", { axis: a.toUpperCase() })}
                    aria-pressed={axisOf(s.normal) === i}
                    onClick={() => {
                      const normal = [0, 0, 0] as Vec3;
                      normal[i] = 1;
                      act("view.update_slice", { id: s.id, normal });
                    }}
                  >
                    {a.toUpperCase()}
                  </button>
                ))}
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
      {selectedSlice && (
        <button
          type="button"
          onClick={() =>
            act("view.set_clip", {
              plane: { origin: selectedSlice.origin, normal: selectedSlice.normal },
            })
          }
        >
          {t("flowView.cutAtSelected")}
        </button>
      )}
      {view.clip && (
        <fieldset className="clip-targets">
          <legend>{t("flowView.cutTargets")}</legend>
          {CLIP_TARGETS.map((target) => {
            const on = (view.clip_targets ?? ["model"]).includes(target);
            return (
              <label key={target} className="check">
                <input
                  type="checkbox"
                  aria-label={t(`flowView.target.${target}`)}
                  checked={on}
                  onChange={(e) => {
                    const current = view.clip_targets ?? ["model"];
                    const targets = CLIP_TARGETS.filter((x) =>
                      x === target ? e.target.checked : current.includes(x),
                    );
                    act("view.set_clip", { plane: view.clip, targets });
                  }}
                />
                {t(`flowView.target.${target}`)}
              </label>
            );
          })}
        </fieldset>
      )}
      {error && (
        <p role="alert" className="error">
          {error}
        </p>
      )}
    </section>
  );
}
