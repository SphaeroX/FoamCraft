// SPDX-License-Identifier: GPL-3.0-or-later
// Animated particles and streamlines: presets, style, colouring, density, tempo, pause, start
// value and the position of the start area (view.set_particles).
import { useCallback, useEffect, useState } from "react";
import { errorMessage, type Api } from "../api";
import { useTranslation } from "../i18n/i18n";

type Vec3 = [number, number, number];

interface Spec {
  style: string;
  count: number;
  seed: number;
  color_by: string;
  speed: number;
  paused: boolean;
  region: { kind: string; origin: Vec3; u: Vec3; v: Vec3 };
}

interface Props {
  api: Api;
  levels: { min: number[]; max: number[] }[];
}

const COUNTS = [10000, 50000, 200000];

export function ParticlesSection({ api, levels }: Props) {
  const { t, language } = useTranslation();
  const [spec, setSpec] = useState<Spec | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    api
      .call<{ particles?: Spec | null }>("view.state")
      .then((v) => setSpec(v?.particles ?? null))
      .catch(() => {});
  }, [api]);

  const set = useCallback(
    (params: Record<string, unknown>, name = "view.set_particles") => {
      setError(null);
      api
        .call<{ particles?: Spec | null }>(name, params)
        .then((v) => setSpec(v?.particles ?? null))
        .catch((e: unknown) => setError(errorMessage(e, language)));
    },
    [api, language],
  );

  const box = levels[levels.length - 1] ?? levels[0] ?? { min: [0, 0, 0], max: [1, 1, 1] };
  return (
    <section className="particles">
      <h3>{t("particles.title")}</h3>
      <div className="row">
        <button type="button" onClick={() => set({ preset: "wind_tunnel_smoke" })}>
          {t("particles.windSmoke")}
        </button>
        <button type="button" onClick={() => set({ preset: "internal_flow" })}>
          {t("particles.internal")}
        </button>
        {spec && (
          <button type="button" onClick={() => set({}, "view.particles_off")}>
            {t("particles.off")}
          </button>
        )}
      </div>
      {spec && (
        <>
          <label className="unit-field">
            <span>{t("particles.style")}</span>
            <select
              aria-label={t("particles.style")}
              value={spec.style}
              onChange={(e) => set({ style: e.target.value })}
            >
              <option value="comets">{t("particles.comets")}</option>
              <option value="smoke">{t("particles.smoke")}</option>
              <option value="streamlines">{t("particles.streamlines")}</option>
            </select>
          </label>
          <label className="unit-field">
            <span>{t("particles.colorBy")}</span>
            <select
              aria-label={t("particles.colorBy")}
              value={spec.color_by}
              onChange={(e) => set({ color_by: e.target.value })}
            >
              <option value="speed">{t("results.field.U_mag")}</option>
              <option value="pressure">{t("results.field.p")}</option>
              <option value="age">{t("particles.age")}</option>
            </select>
          </label>
          <label className="unit-field">
            <span>{t("particles.density")}</span>
            <select
              aria-label={t("particles.density")}
              value={String(spec.count)}
              onChange={(e) => set({ count: Number(e.target.value) })}
            >
              {[...new Set([...COUNTS, spec.count])].map((n) => (
                <option key={n} value={String(n)}>
                  {n.toLocaleString(language)}
                </option>
              ))}
            </select>
          </label>
          <label className="unit-field">
            <span>{t("particles.tempo")}</span>
            <input
              type="range"
              aria-label={t("particles.tempo")}
              min={0.1}
              max={3}
              step={0.1}
              value={spec.speed}
              onChange={(e) => set({ speed: Number(e.target.value) })}
            />
          </label>
          <div className="row">
            <button type="button" onClick={() => set({ paused: !spec.paused })}>
              {spec.paused ? t("particles.play") : t("particles.pause")}
            </button>
            <label className="unit-field">
              <span>{t("particles.seed")}</span>
              <input
                type="number"
                aria-label={t("particles.seed")}
                value={spec.seed}
                onChange={(e) => set({ seed: Number(e.target.value) })}
              />
            </label>
          </div>
          {spec.region.kind === "area" && (
            <label className="unit-field">
              <span>{t("particles.areaPosition")}</span>
              <input
                type="range"
                aria-label={t("particles.areaPosition")}
                min={box.min[0]}
                max={box.max[0]}
                step={(box.max[0] - box.min[0]) / 200}
                value={spec.region.origin[0]}
                onChange={(e) => {
                  const origin = [...spec.region.origin] as Vec3;
                  origin[0] = Number(e.target.value);
                  set({ region: { ...spec.region, origin } });
                }}
              />
            </label>
          )}
        </>
      )}
      {error && (
        <p role="alert" className="error">
          {error}
        </p>
      )}
    </section>
  );
}
