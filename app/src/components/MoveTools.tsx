// SPDX-License-Identifier: GPL-3.0-or-later
// Small bar over the 3D view while models can be moved: where the handles sit (box centre, volume
// centre, model origin), a hint and the value of the running drag.
import { useEffect, useState } from "react";
import type { Api } from "../api";
import { useTranslation } from "../i18n/i18n";
import { formatNumber } from "../units";

export type PivotKind = "center" | "centroid" | "origin";
type Point = [number, number, number];

/** Value of a running drag, as the native view reports it. */
export interface HandleValue {
  /** `wind`: the new wind direction in degrees. */
  kind: "move" | "rotate" | "wind";
  axis: "x" | "y" | "z";
  /** Metres for moves, degrees for turns. */
  value: number;
}

/** "X +12 mm" or "Z −15°". */
export function formatHandleValue(v: HandleValue): string {
  const sign = v.value < 0 ? "−" : "+";
  const abs = Math.abs(v.value);
  const text = v.kind === "move" ? `${formatNumber(abs * 1000)} mm` : `${formatNumber(abs)}°`;
  return `${v.axis.toUpperCase()} ${sign}${text}`;
}

interface Props {
  api: Api;
  /** The model the handles belong to. */
  model: string;
  /** Changes whenever the project changed, so the pivot is asked for again. */
  revision: number;
  value: HandleValue | null;
  /** The handles should sit at this point (metres). */
  onPivot: (model: string, point: Point) => void;
}

export function MoveTools({ api, model, revision, value, onPivot }: Props) {
  const { t } = useTranslation();
  const [pivot, setPivot] = useState<PivotKind>("center");

  useEffect(() => {
    let current = true;
    api
      .call<Record<PivotKind, Point>>("geometry.pivot", { id: model })
      .then((all) => {
        if (current) onPivot(model, all[pivot]);
      })
      .catch(() => {});
    return () => {
      current = false;
    };
  }, [api, model, pivot, revision, onPivot]);

  return (
    <div className="move-tools panel">
      <label>
        {t("move.pivot")}
        <select
          aria-label={t("move.pivot")}
          value={pivot}
          onChange={(e) => setPivot(e.target.value as PivotKind)}
        >
          {(["center", "centroid", "origin"] as const).map((p) => (
            <option key={p} value={p}>
              {t(`move.pivots.${p}`)}
            </option>
          ))}
        </select>
      </label>
      <span className="muted small">{t("move.hint")}</span>
      <span role="status" className="move-value">
        {value ? formatHandleValue(value) : ""}
      </span>
    </div>
  );
}
