// SPDX-License-Identifier: GPL-3.0-or-later
// Where the air comes from: the whole inlet wall (wind tunnel) or up to four sources such as
// nozzles or fans, each with shape, size, position, direction and speed (scenario.set_jets).
import { useEffect, useRef, useState } from "react";
import type { Api } from "../api";
import { useTranslation } from "../i18n/i18n";
import {
  formatNumber,
  fromDisplaySpeed,
  speedUnit,
  toDisplaySpeed,
  type DisplayUnits,
} from "../units";

type Vec3 = [number, number, number];

export interface Jet {
  shape: "circle" | "rectangle";
  center: Vec3;
  direction: Vec3;
  size: [number, number];
  speed_m_s: number;
}

/** Unit direction from yaw (about the vertical, degrees) and pitch (upwards, degrees). */
export function directionOf(yawDeg: number, pitchDeg: number): Vec3 {
  const y = (yawDeg * Math.PI) / 180;
  const p = (pitchDeg * Math.PI) / 180;
  return [Math.cos(p) * Math.cos(y), Math.cos(p) * Math.sin(y), Math.sin(p)];
}

/** Yaw and pitch (degrees) of a direction. */
export function anglesOf(d: Vec3): [number, number] {
  const h = Math.hypot(d[0], d[1]);
  const yaw = h < 1e-9 ? 0 : (Math.atan2(d[1], d[0]) * 180) / Math.PI;
  return [yaw, (Math.atan2(d[2], h) * 180) / Math.PI];
}

interface Props {
  api: Api;
  units: DisplayUnits;
  revision: number;
}

/** Milliseconds after the last input before the sources are sent. */
const DELAY = 400;

const NEW_JET: Jet = {
  shape: "circle",
  center: [-0.3, 0, 0.1],
  direction: [1, 0, 0],
  size: [0.08, 0.08],
  speed_m_s: 15,
};

export function JetsSection({ api, units, revision }: Props) {
  const { t } = useTranslation();
  const [jets, setJets] = useState<Jet[]>([]);
  const [loaded, setLoaded] = useState(false);
  const pending = useRef<ReturnType<typeof setTimeout> | null>(null);

  useEffect(() => {
    api
      .call<{ jets?: Jet[] }>("scenario.info")
      .then((r) => setJets(Array.isArray(r?.jets) ? r.jets : []))
      .catch(() => setJets([]))
      .finally(() => setLoaded(true));
  }, [api, revision]);

  const send = (next: Jet[], now = false) => {
    setJets(next);
    if (pending.current) clearTimeout(pending.current);
    const go = () => void api.call("scenario.set_jets", { jets: next }).catch(() => {});
    if (now) go();
    else pending.current = setTimeout(go, DELAY);
  };
  const change = (i: number, patch: Partial<Jet>) =>
    send(jets.map((j, k) => (k === i ? { ...j, ...patch } : j)));

  if (!loaded) return null;
  const mm = (m: number) => formatNumber(m * 1000);
  const num = (label: string, value: string, onCommit: (v: number) => void) => (
    <label className="unit-field">
      <span>{label}</span>
      <input
        type="number"
        step="any"
        aria-label={label}
        defaultValue={value}
        key={value}
        onChange={(e) => {
          const v = Number(e.target.value);
          if (Number.isFinite(v) && e.target.value !== "") onCommit(v);
        }}
      />
    </label>
  );

  return (
    <fieldset className="jets">
      <legend>{t("jets.title")}</legend>
      <div role="radiogroup" className="jet-mode">
        <label className="check">
          <input
            type="radio"
            name="jet-mode"
            aria-label={t("jets.wall")}
            checked={jets.length === 0}
            onChange={() => send([], true)}
          />
          {t("jets.wall")}
        </label>
        <label className="check">
          <input
            type="radio"
            name="jet-mode"
            aria-label={t("jets.sources")}
            checked={jets.length > 0}
            onChange={() => send([NEW_JET], true)}
          />
          {t("jets.sources")}
        </label>
      </div>
      {jets.length > 0 && <p className="muted small">{t("jets.hint")}</p>}
      {jets.map((j, i) => {
        const [yaw, pitch] = anglesOf(j.direction);
        const name = t("jets.item", { n: String(i + 1) });
        return (
          <section key={i} role="group" aria-label={name} className="jet-item">
            <header className="row">
              <strong>{name}</strong>
              <select
                aria-label={t("jets.shape")}
                value={j.shape}
                onChange={(e) => change(i, { shape: e.target.value as Jet["shape"] })}
              >
                <option value="circle">{t("jets.circle")}</option>
                <option value="rectangle">{t("jets.rectangle")}</option>
              </select>
              <button
                type="button"
                className="quiet"
                aria-label={t("jets.remove", { n: String(i + 1) })}
                onClick={() =>
                  send(
                    jets.filter((_, k) => k !== i),
                    true,
                  )
                }
              >
                ×
              </button>
            </header>
            <div className="row">
              {j.shape === "circle" ? (
                num(t("jets.diameter"), mm(j.size[0]), (v) =>
                  change(i, { size: [v / 1000, v / 1000] }),
                )
              ) : (
                <>
                  {num(t("jets.width"), mm(j.size[0]), (v) =>
                    change(i, { size: [v / 1000, j.size[1]] }),
                  )}
                  {num(t("jets.height"), mm(j.size[1]), (v) =>
                    change(i, { size: [j.size[0], v / 1000] }),
                  )}
                </>
              )}
              {num(
                t("jets.speed", { unit: speedUnit(units) }),
                formatNumber(toDisplaySpeed(j.speed_m_s, units)),
                (v) => change(i, { speed_m_s: fromDisplaySpeed(v, units) }),
              )}
            </div>
            <div className="row">
              {(["X", "Y", "Z"] as const).map((axis, k) =>
                num(`${axis} (mm)`, mm(j.center[k]), (v) => {
                  const center = [...j.center] as Vec3;
                  center[k] = v / 1000;
                  change(i, { center });
                }),
              )}
            </div>
            <div className="row">
              {num(t("jets.yaw"), formatNumber(yaw), (v) =>
                change(i, { direction: directionOf(v, pitch) }),
              )}
              {num(t("jets.pitch"), formatNumber(pitch), (v) =>
                change(i, { direction: directionOf(yaw, v) }),
              )}
            </div>
          </section>
        );
      })}
      {jets.length > 0 && jets.length < 4 && (
        <button
          type="button"
          onClick={() =>
            send([...jets, { ...NEW_JET, center: [-0.3, 0.1 * jets.length, 0.1] }], true)
          }
        >
          {t("jets.add")}
        </button>
      )}
    </fieldset>
  );
}
