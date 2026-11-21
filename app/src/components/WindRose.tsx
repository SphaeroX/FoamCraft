// SPDX-License-Identifier: GPL-3.0-or-later
// Wind rose of the Scenario step: a circle seen from above (+X right, +Y up) with the wind arrow.
// Click or drag to point the arrow where the wind blows; arrow keys turn it by 5°.
import { useEffect, useRef } from "react";
import { useTranslation } from "../i18n/i18n";

/** Size of the drawing in SVG units (and CSS pixels). */
const SIZE = 120;
const C = SIZE / 2;
const R = 50;

/** Direction (degrees, -180..180) from the centre (cx, cy) to a point; SVG y points down. */
export function angleFromPoint(x: number, y: number, cx: number, cy: number): number {
  return (Math.atan2(cy - y, x - cx) * 180) / Math.PI;
}

function wrap(deg: number): number {
  return ((((deg + 180) % 360) + 360) % 360) - 180;
}

interface Props {
  /** Direction the wind blows towards, degrees (0 = +X, 90 = +Y). */
  value: number;
  onChange: (deg: number) => void;
  /** Outline of the models seen from above, metres: [min x, min y, max x, max y]. */
  outline?: [number, number, number, number] | null;
}

export function WindRose({ value, onChange, outline }: Props) {
  const { t } = useTranslation();
  const svg = useRef<SVGSVGElement>(null);
  const dragging = useRef(false);

  const fromEvent = (clientX: number, clientY: number) => {
    const rect = svg.current?.getBoundingClientRect();
    // Without layout (tests) the drawing is taken as 1:1 at the origin.
    const w = rect?.width || SIZE;
    const h = rect?.height || SIZE;
    const x = ((clientX - (rect?.left ?? 0)) * SIZE) / w;
    const y = ((clientY - (rect?.top ?? 0)) * SIZE) / h;
    if (Math.hypot(x - C, y - C) < 4) return;
    // Whole degrees: the drag snaps like the handle in the view.
    onChange(Math.round(angleFromPoint(x, y, C, C)));
  };

  useEffect(() => {
    const move = (e: MouseEvent) => {
      if (dragging.current) fromEvent(e.clientX, e.clientY);
    };
    const up = () => (dragging.current = false);
    window.addEventListener("mousemove", move);
    window.addEventListener("mouseup", up);
    return () => {
      window.removeEventListener("mousemove", move);
      window.removeEventListener("mouseup", up);
    };
  });

  const a = (value * Math.PI) / 180;
  // Arrow through the centre: tail upstream, head downstream (SVG y down).
  const dx = Math.cos(a);
  const dy = -Math.sin(a);
  const tail = [C - dx * R * 0.85, C - dy * R * 0.85];
  const head = [C + dx * R * 0.85, C + dy * R * 0.85];
  const side = [-dy, dx];
  const wing = (s: number) =>
    `${head[0] - dx * 12 + side[0] * 7 * s},${head[1] - dy * 12 + side[1] * 7 * s}`;

  let box: { x: number; y: number; w: number; h: number } | null = null;
  if (outline) {
    const [x0, y0, x1, y1] = outline;
    const span = Math.max(x1 - x0, y1 - y0, 1e-9);
    const k = (R * 0.9) / span;
    const cx = (x0 + x1) / 2;
    const cy = (y0 + y1) / 2;
    box = {
      x: C + (x0 - cx) * k,
      y: C - (y1 - cy) * k,
      w: (x1 - x0) * k,
      h: (y1 - y0) * k,
    };
  }
  const from = wrap(value + 180);

  return (
    <div className="wind-rose">
      <svg
        ref={svg}
        width={SIZE}
        height={SIZE}
        viewBox={`0 0 ${SIZE} ${SIZE}`}
        role="slider"
        tabIndex={0}
        aria-label={t("wind.rose")}
        aria-valuemin={-180}
        aria-valuemax={180}
        aria-valuenow={value}
        aria-valuetext={t("wind.towards", { deg: String(value) })}
        onMouseDown={(e) => {
          dragging.current = true;
          fromEvent(e.clientX, e.clientY);
        }}
        onKeyDown={(e) => {
          // Seen from above: right turns clockwise (negative angle).
          const step =
            e.key === "ArrowRight" || e.key === "ArrowDown"
              ? -5
              : e.key === "ArrowLeft" || e.key === "ArrowUp"
                ? 5
                : 0;
          if (step === 0) return;
          e.preventDefault();
          onChange(wrap(value + step));
        }}
      >
        <circle cx={C} cy={C} r={R} className="wind-rose-ring" />
        {[0, 90, 180, 270].map((d) => {
          const r = (d * Math.PI) / 180;
          return (
            <line
              key={d}
              x1={C + Math.cos(r) * (R - 5)}
              y1={C - Math.sin(r) * (R - 5)}
              x2={C + Math.cos(r) * R}
              y2={C - Math.sin(r) * R}
              className="wind-rose-tick"
            />
          );
        })}
        <text x={C + R - 2} y={C - 4} className="wind-rose-label" textAnchor="end">
          +X
        </text>
        <text x={C + 3} y={C - R + 11} className="wind-rose-label">
          +Y
        </text>
        {box && <rect {...box} className="wind-rose-model" />}
        <line x1={tail[0]} y1={tail[1]} x2={head[0]} y2={head[1]} className="wind-rose-arrow" />
        <polygon
          points={`${head[0]},${head[1]} ${wing(1)} ${wing(-1)}`}
          className="wind-rose-head"
        />
      </svg>
      <p className="muted small">
        {t("wind.from", { deg: String(from) })} · {t("wind.towards", { deg: String(value) })}
      </p>
    </div>
  );
}
