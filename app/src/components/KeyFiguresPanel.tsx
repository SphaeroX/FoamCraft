// SPDX-License-Identifier: GPL-3.0-or-later
// Step "Results", right side: key figures of a run with explanation and reliability light, the
// list of runs (rename, variant, delete) and the comparison of two runs. All actions are commands.
import { useCallback, useEffect, useState } from "react";
import { errorMessage, type Api } from "../api";
import { useTranslation } from "../i18n/i18n";

type Level = "green" | "yellow" | "red";

interface Text {
  text: string;
  fix: string;
}

interface Run {
  job_id: string;
  name: string;
  kind: string;
  created: number;
  key_value: number | null;
  key_unit: string;
  reliability: Level;
  bytes: number;
}

interface Figure {
  id: string;
  value: number;
  unit: string;
  part: string | null;
  en: Text;
  de: Text;
}

interface Summary {
  job_id: string;
  name: string;
  kind: string;
  figures: Figure[];
  reliability: { level: Level; checks: { code: string; level: Level; en: Text; de: Text }[] };
}

interface Row {
  id: string;
  part: string | null;
  unit: string;
  a: number | null;
  b: number | null;
  difference: number | null;
  relative: number | null;
}

interface Comparison {
  a: string;
  b: string;
  rows: Row[];
}

interface Props {
  api: Api;
  /** Changes whenever the project changed, so the list reloads. */
  revision: number;
  /** Asks for a file to write; `null` when cancelled. */
  pickSavePath: (extension: "csv" | "json") => Promise<string | null>;
  confirm: (message: string) => boolean;
}

/** Units of the surface fields (the difference has the unit of the field). */
const FIELD_UNITS: Record<string, string> = { p: "Pa", wall_shear_stress: "Pa" };

export function KeyFiguresPanel({ api, revision, pickSavePath, confirm }: Props) {
  const { t, language } = useTranslation();
  const [runs, setRuns] = useState<Run[]>([]);
  const [job, setJob] = useState<string | null>(null);
  const [summary, setSummary] = useState<Summary | null>(null);
  const [other, setOther] = useState("");
  const [comparison, setComparison] = useState<Comparison | null>(null);
  const [name, setName] = useState("");
  const [variant, setVariant] = useState("");
  const [message, setMessage] = useState<string | null>(null);
  /** Largest absolute difference shown on the model, and the field. */
  const [difference, setDifference] = useState<number | null>(null);
  const [field, setField] = useState("p");
  const [error, setError] = useState<string | null>(null);
  const fail = useCallback((e: unknown) => setError(errorMessage(e, language)), [language]);

  // A comparison stored in the project is shown again.
  useEffect(() => {
    api
      .call<{ compare?: { a: string; b: string } | null; surface_field?: string | null }>(
        "view.state",
      )
      .then((v) => {
        if (v.surface_field) setField(v.surface_field);
        if (v.compare) {
          setJob(v.compare.b);
          setOther(v.compare.a);
        }
      })
      .catch(fail);
  }, [api, fail]);

  useEffect(() => {
    api
      .call<Run[]>("runs.list")
      .then((list) => {
        setRuns(list);
        setJob((j) => (j && list.some((r) => r.job_id === j) ? j : (list[0]?.job_id ?? null)));
      })
      .catch(fail);
  }, [api, revision, fail]);

  useEffect(() => {
    if (!job) {
      setSummary(null);
      return;
    }
    api
      .call<Summary>("results.summary", { job_id: job })
      .then((s) => {
        setSummary(s);
        setName(s.name);
      })
      .catch(fail);
  }, [api, job, revision, fail]);

  useEffect(() => {
    if (!job || !other) {
      setComparison(null);
      return;
    }
    api.call<Comparison>("results.compare", { a: other, b: job }).then(setComparison).catch(fail);
  }, [api, job, other, revision, fail]);

  if (runs.length === 0) return null;

  const fmt = (n: number) => n.toLocaleString(language, { maximumSignificantDigits: 4 });
  const label = (f: { id: string; part: string | null }) =>
    t(`figures.label.${f.id}`) + (f.part ? ` (${f.part})` : "");
  const run = runs.find((r) => r.job_id === job);
  const mb = (bytes: number) =>
    (bytes / 1e6).toLocaleString(language, { maximumFractionDigits: 1 }) + " MB";

  const exportAs = async (format: "csv" | "json") => {
    const path = await pickSavePath(format);
    if (!path || !job) return;
    api
      .call("results.export", { job_id: job, path, format })
      .then(() => setMessage(t("figures.exported", { path })))
      .catch(fail);
  };

  return (
    <aside className="side-panel right panel key-figures">
      <h2>{t("figures.title")}</h2>
      <label className="unit-field">
        <span>{t("figures.run")}</span>
        <select
          aria-label={t("figures.run")}
          value={job ?? ""}
          onChange={(e) => setJob(e.target.value)}
        >
          {runs.map((r) => (
            <option key={r.job_id} value={r.job_id}>
              {r.name}
            </option>
          ))}
        </select>
      </label>
      {summary && (
        <>
          <div
            role="status"
            aria-label={t("figures.reliability")}
            className={`light ${summary.reliability.level}`}
          >
            <strong>
              <span className="dot" aria-hidden="true" />
              {t(`figures.level.${summary.reliability.level}`)}
            </strong>
            <ul>
              {summary.reliability.checks.map((c) => (
                <li key={c.code} className={c.level}>
                  {c[language].text}
                </li>
              ))}
            </ul>
          </div>
          <table aria-label={t("figures.table")} className="figures">
            <tbody>
              {summary.figures.map((f) => (
                <tr key={`${f.id}:${f.part ?? ""}`}>
                  <th scope="row" title={f[language].text}>
                    {label(f)}
                  </th>
                  <td>{fmt(f.value)}</td>
                  <td>{f.unit === "-" ? "" : f.unit}</td>
                </tr>
              ))}
            </tbody>
          </table>
          <p className="muted small">{t("figures.hint")}</p>
          <div className="actions">
            <button type="button" onClick={() => void exportAs("csv")}>
              {t("figures.exportCsv")}
            </button>
            <button type="button" onClick={() => void exportAs("json")}>
              {t("figures.exportJson")}
            </button>
          </div>
        </>
      )}

      <h3>{t("figures.compare")}</h3>
      <label className="unit-field">
        <span>{t("figures.compareWith")}</span>
        <select
          aria-label={t("figures.compareWith")}
          value={other}
          onChange={(e) => setOther(e.target.value)}
        >
          <option value="">{t("figures.noCompare")}</option>
          {runs
            .filter((r) => r.job_id !== job)
            .map((r) => (
              <option key={r.job_id} value={r.job_id}>
                {r.name}
              </option>
            ))}
        </select>
      </label>
      {comparison && (
        <>
          <table aria-label={t("figures.comparison")} className="figures compare">
            <thead>
              <tr>
                <th />
                <th>{runs.find((r) => r.job_id === comparison.a)?.name}</th>
                <th>{runs.find((r) => r.job_id === comparison.b)?.name}</th>
                <th>Δ</th>
                <th>Δ %</th>
              </tr>
            </thead>
            <tbody>
              {comparison.rows.map((r) => (
                <tr key={`${r.id}:${r.part ?? ""}`}>
                  <th scope="row">{label(r)}</th>
                  <td>{r.a === null ? "–" : fmt(r.a)}</td>
                  <td>{r.b === null ? "–" : fmt(r.b)}</td>
                  <td>{r.difference === null ? "–" : fmt(r.difference)}</td>
                  <td>{r.relative === null ? "–" : fmt(r.relative * 100)}</td>
                </tr>
              ))}
            </tbody>
          </table>
          <div className="actions">
            {(["split", "difference"] as const).map((mode) => (
              <button
                key={mode}
                type="button"
                onClick={() =>
                  api
                    .call<{ difference?: { min: number; max: number } | null }>(
                      "view.set_compare",
                      { a: comparison.a, b: comparison.b, mode },
                    )
                    .then((r) =>
                      setDifference(
                        r.difference
                          ? Math.max(Math.abs(r.difference.min), Math.abs(r.difference.max))
                          : null,
                      ),
                    )
                    .catch(fail)
                }
              >
                {t(`figures.mode.${mode}`)}
              </button>
            ))}
            <button
              type="button"
              onClick={() =>
                api
                  .call("view.set_compare", { a: null })
                  .then(() => setDifference(null))
                  .catch(fail)
              }
            >
              {t("figures.endCompare")}
            </button>
          </div>
        </>
      )}

      {comparison && difference !== null && (
        <p className="small">
          {t("figures.differenceScale", {
            field: t(`results.field.${field}`),
            range: `−${fmt(difference)} … +${fmt(difference)}`,
            unit: FIELD_UNITS[field] ?? "",
          })}
        </p>
      )}

      <h3>{t("figures.manage")}</h3>
      {run && <p className="small">{t("figures.size", { size: mb(run.bytes) })}</p>}
      <div className="range-inputs">
        <input
          type="text"
          aria-label={t("figures.runName")}
          value={name}
          onChange={(e) => setName(e.target.value)}
        />
        <button
          type="button"
          disabled={!job || !name.trim()}
          onClick={() => job && api.call("runs.rename", { job_id: job, name }).catch(fail)}
        >
          {t("figures.rename")}
        </button>
      </div>
      <div className="range-inputs">
        <input
          type="text"
          aria-label={t("figures.variantName")}
          value={variant}
          onChange={(e) => setVariant(e.target.value)}
        />
        <button
          type="button"
          disabled={!variant.trim()}
          onClick={() =>
            api
              .call<{ path: string }>("runs.duplicate", { name: variant })
              .then((r) => setMessage(t("figures.duplicated", { path: r.path })))
              .catch(fail)
          }
        >
          {t("figures.duplicate")}
        </button>
      </div>
      <button
        type="button"
        className="danger"
        disabled={!job}
        onClick={() => {
          if (!job || !run || !confirm(t("figures.confirmDelete", { name: run.name }))) return;
          api
            .call<{ freed_bytes: number }>("runs.delete", { job_id: job })
            .then((r) => setMessage(t("figures.deleted", { size: mb(r.freed_bytes) })))
            .catch(fail);
        }}
      >
        {t("figures.delete")}
      </button>
      {message && <p className="small">{message}</p>}
      {error && (
        <p role="alert" className="error">
          {error}
        </p>
      )}
    </aside>
  );
}
