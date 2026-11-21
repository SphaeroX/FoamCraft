// SPDX-License-Identifier: GPL-3.0-or-later
// Small SVG line chart for live solver curves (no chart library needed).

export interface Series {
  name: string;
  /** [x, y] pairs in order of x. */
  points: [number, number][];
}

interface Props {
  label: string;
  series: Series[];
  /** Logarithmic y axis (residuals). */
  log?: boolean;
  width?: number;
  height?: number;
}

const PAD = 4;

export function LineChart({ label, series, log = false, width = 300, height = 120 }: Props) {
  const tf = (y: number) => (log ? Math.log10(Math.max(y, 1e-12)) : y);
  const all = series.flatMap((s) => s.points).filter(([, y]) => Number.isFinite(y));
  const xs = all.map(([x]) => x);
  const ys = all.map(([, y]) => tf(y));
  const [x0, x1] = [Math.min(...xs), Math.max(...xs)];
  let [y0, y1] = [Math.min(...ys), Math.max(...ys)];
  if (!(y1 > y0)) {
    y0 -= 1;
    y1 += 1;
  }
  const sx = (x: number) => PAD + ((x - x0) / (x1 > x0 ? x1 - x0 : 1)) * (width - 2 * PAD);
  const sy = (y: number) => height - PAD - ((tf(y) - y0) / (y1 - y0)) * (height - 2 * PAD);
  return (
    <figure className="chart">
      <svg role="img" aria-label={label} viewBox={`0 0 ${width} ${height}`} width="100%">
        <rect className="chart-frame" x={0} y={0} width={width} height={height} />
        {series.map((s, i) => (
          <polyline
            key={s.name}
            className={`series series-${i % 6}`}
            fill="none"
            points={s.points
              .filter(([, y]) => Number.isFinite(y))
              .map(([x, y]) => `${sx(x).toFixed(1)},${sy(y).toFixed(1)}`)
              .join(" ")}
          />
        ))}
      </svg>
      <figcaption className="legend">
        {series.map((s, i) => (
          <span key={s.name} className={`legend-item series-${i % 6}`}>
            {s.name}
          </span>
        ))}
      </figcaption>
    </figure>
  );
}
