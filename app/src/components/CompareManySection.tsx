// SPDX-License-Identifier: GPL-3.0-or-later
// Key figures of several chosen runs side by side; the first chosen run is the reference.
import { useEffect, useState } from "react";
import type { Api } from "../api";
import { useTranslation } from "../i18n/i18n";
import { formatNumber } from "../units";

interface Run {
  job_id: string;
  name: string;
}

interface Row {
  id: string;
  part: string | null;
  unit: string;
  values: (number | null)[];
  relative: (number | null)[];
}

interface Comparison {
  jobs: string[];
  names: string[];
  reference: string;
  rows: Row[];
}

interface Props {
  api: Api;
  revision: number;
}

/** "+10 %" or "−4.5 %"; nothing for the reference itself. */
function change(r: number | null): string {
  if (r === null || Math.abs(r) < 1e-12) return "";
  return `${r < 0 ? "−" : "+"}${formatNumber(Math.abs(r) * 100)} %`;
}

export function CompareManySection({ api, revision }: Props) {
  const { t } = useTranslation();
  const [runs, setRuns] = useState<Run[]>([]);
  const [chosen, setChosen] = useState<string[]>([]);
  const [cmp, setCmp] = useState<Comparison | null>(null);

  useEffect(() => {
    api
      .call<Run[]>("runs.list", { sizes: false })
      .then((l) => setRuns(Array.isArray(l) ? l : []))
      .catch(() => setRuns([]));
  }, [api, revision]);

  useEffect(() => {
    if (chosen.length < 2) {
      setCmp(null);
      return;
    }
    api
      .call<Comparison>("results.compare_many", { jobs: chosen, reference: chosen[0] })
      .then(setCmp)
      .catch(() => setCmp(null));
  }, [api, chosen]);

  if (runs.length < 2) return null;
  return (
    <section className="compare-many">
      <h3>{t("compareMany.title")}</h3>
      <p className="small muted">{t("compareMany.hint")}</p>
      <div className="check-list">
        {runs.map((r) => (
          <label key={r.job_id} className="check">
            <input
              type="checkbox"
              aria-label={r.name || r.job_id}
              checked={chosen.includes(r.job_id)}
              onChange={(e) =>
                setChosen((c) =>
                  e.target.checked ? [...c, r.job_id] : c.filter((x) => x !== r.job_id),
                )
              }
            />
            {r.name || r.job_id}
          </label>
        ))}
      </div>
      {cmp && (
        <table className="figures compare" aria-label={t("compareMany.table")}>
          <thead>
            <tr>
              <th />
              {cmp.names.map((n) => (
                <th key={n}>{n}</th>
              ))}
            </tr>
          </thead>
          <tbody>
            {cmp.rows.map((row) => (
              <tr key={`${row.id}-${row.part ?? ""}`}>
                <th>
                  {t(`figures.label.${row.id}`)}
                  {row.part ? ` (${row.part})` : ""}
                  {row.unit !== "-" ? ` [${row.unit}]` : ""}
                </th>
                {row.values.map((v, i) => (
                  <td key={i} className="num">
                    {v === null ? "–" : formatNumber(v)}
                    {change(row.relative[i]) && (
                      <span className="change">{change(row.relative[i])}</span>
                    )}
                  </td>
                ))}
              </tr>
            ))}
          </tbody>
        </table>
      )}
    </section>
  );
}
