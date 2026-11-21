// SPDX-License-Identifier: GPL-3.0-or-later
// Flow on the model surface (Task_33): lines like an oil film and wool tufts, with spacing,
// colour and a note when the directions are approximate (run without wall shear stress vector).
import { useCallback, useEffect, useState } from "react";
import type { Api } from "../api";
import { useTranslation } from "../i18n/i18n";

interface LinesSpec {
  density: number;
  color_by: string;
  animate: boolean;
  seed: number;
}

interface TuftsSpec {
  spacing: number;
  color_by: string;
  animate: boolean;
  seed: number;
}

interface FlowSpec {
  lines: LinesSpec | null;
  tufts: TuftsSpec | null;
  speed: number;
  paused: boolean;
}

interface Info {
  surface_flow: FlowSpec | null;
  approximate: boolean;
}

interface Props {
  api: Api;
  /** Changes with every project change (a preset may switch the surface flow). */
  revision?: number;
}

export function SurfaceFlowSection({ api, revision = 0 }: Props) {
  const { t } = useTranslation();
  const [info, setInfo] = useState<Info | null>(null);

  useEffect(() => {
    api
      .call<Info>("view.surface_flow_info", {})
      .then((i) => setInfo(i && typeof i === "object" ? i : null))
      .catch(() => setInfo(null));
  }, [api, revision]);

  const set = useCallback(
    (params: Record<string, unknown>) => {
      api
        .call<Info>("view.set_surface_flow", params)
        .then((i) => {
          if (i && typeof i === "object" && "surface_flow" in i) setInfo(i);
        })
        .catch(() => {});
    },
    [api],
  );

  if (!info) return null;
  const lines = info.surface_flow?.lines ?? null;
  const tufts = info.surface_flow?.tufts ?? null;
  // Sliders work on the divisor (60 = spacing of 1/60 of the model size): larger is finer.
  return (
    <section className="surface-flow-section">
      <h3>{t("surfaceFlow.title")}</h3>
      <label className="check">
        <input
          type="checkbox"
          checked={lines !== null}
          onChange={(e) => set({ lines: { on: e.target.checked } })}
        />
        <span>{t("surfaceFlow.lines")}</span>
      </label>
      {lines && (
        <div className="surface-flow-options">
          <label>
            <span>{t("surfaceFlow.density")}</span>
            <input
              type="range"
              aria-label={t("surfaceFlow.density")}
              min={30}
              max={120}
              step={5}
              value={Math.round(1 / lines.density)}
              onChange={(e) => set({ lines: { on: true, density: 1 / Number(e.target.value) } })}
            />
          </label>
          <label>
            <span>{t("surfaceFlow.colorLines")}</span>
            <select
              aria-label={t("surfaceFlow.colorLines")}
              value={lines.color_by}
              onChange={(e) => set({ lines: { on: true, color_by: e.target.value } })}
            >
              <option value="plain">{t("surfaceFlow.plain")}</option>
              <option value="wall_shear_stress">{t("surfaceFlow.byStress")}</option>
            </select>
          </label>
        </div>
      )}
      <label className="check">
        <input
          type="checkbox"
          checked={tufts !== null}
          onChange={(e) => set({ tufts: { on: e.target.checked } })}
        />
        <span>{t("surfaceFlow.tufts")}</span>
      </label>
      {tufts && (
        <div className="surface-flow-options">
          <label>
            <span>{t("surfaceFlow.spacing")}</span>
            <input
              type="range"
              aria-label={t("surfaceFlow.spacing")}
              min={12}
              max={50}
              step={1}
              value={Math.round(1 / tufts.spacing)}
              onChange={(e) => set({ tufts: { on: true, spacing: 1 / Number(e.target.value) } })}
            />
          </label>
          <label>
            <span>{t("surfaceFlow.colorTufts")}</span>
            <select
              aria-label={t("surfaceFlow.colorTufts")}
              value={tufts.color_by}
              onChange={(e) => set({ tufts: { on: true, color_by: e.target.value } })}
            >
              <option value="plain">{t("surfaceFlow.plain")}</option>
              <option value="state">{t("surfaceFlow.byState")}</option>
            </select>
          </label>
          <p className="muted small">{t("surfaceFlow.flutterNote")}</p>
        </div>
      )}
      {(lines || tufts) && (
        <label className="check">
          <input
            type="checkbox"
            checked={!(info.surface_flow?.paused ?? false)}
            onChange={(e) => set({ paused: !e.target.checked })}
          />
          <span>{t("surfaceFlow.animate")}</span>
        </label>
      )}
      {info.approximate && (lines || tufts) && (
        <p className="muted small" role="note">
          {t("surfaceFlow.approximate")}
        </p>
      )}
    </section>
  );
}
