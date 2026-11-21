// SPDX-License-Identifier: GPL-3.0-or-later
// Step "Run": summary before the start, one start button for meshing and solving, stage timeline,
// meshing progress, live curves, remaining time, cancel, log, result card and plain-language
// diagnosis. The state lives in a RunMonitor, so it survives leaving this step.
import { useCallback, useEffect, useState } from "react";
import { type Api, type ApiEvent, type LocalizedMessage } from "../api";
import { useTranslation } from "../i18n/i18n";
import { ExpertOnly, useMode } from "../mode";
import { RunMonitor, useRunMonitor } from "../run-monitor";
import { LineChart, type Series } from "./LineChart";
import { MeshSection } from "./MeshSection";

type Texts = Record<string, { en: LocalizedMessage; de: LocalizedMessage }>;

const STAGES = ["background", "mesh", "check", "decompose", "solve", "reconstruct"] as const;

interface Summary {
  cells: number | null;
  estimate_s: number | null;
}

interface Props {
  api: Api;
  /** Subscribes to command API events; returns the unsubscribe function. */
  subscribe: (handler: (event: ApiEvent) => void) => () => void;
  /** Shown at the end of the panel (the queue of scenarios). */
  footer?: React.ReactNode;
  /** The monitor of the open project; without one the panel follows the run on its own. */
  monitor?: RunMonitor;
}

export function RunPanel({ api, subscribe, monitor, footer }: Props) {
  const { t, language } = useTranslation();
  const { mode } = useMode();
  const [own] = useState(() => monitor ?? new RunMonitor(api, subscribe));
  const m = monitor ?? own;
  useEffect(() => {
    if (monitor) return;
    own.connect();
    return () => own.dispose();
  }, [monitor, own]);
  useEffect(() => {
    m.language = language;
  }, [m, language]);
  const state = useRunMonitor(m);
  const { phase, jobId, stage, seen, mesh, residuals, coeffs, loss, iteration, remaining, result } =
    state;

  const [summary, setSummary] = useState<Summary>({ cells: null, estimate_s: null });
  const [texts, setTexts] = useState<Texts>({});
  const [logOpen, setLogOpen] = useState(false);
  const [log, setLog] = useState<string[]>([]);

  // Summary; the cell count is read again when a mesh was built.
  useEffect(() => {
    api
      .call<{ budget: { estimate_s: number } }>("scenario.info")
      .then((s) => setSummary((old) => ({ ...old, estimate_s: s.budget.estimate_s })))
      .catch(() => {});
    api
      .call<{ cells: number }>("mesh.info")
      .then((r) => setSummary((old) => ({ ...old, cells: r.cells })))
      .catch(() => {});
  }, [api, state.meshRevision]);

  const describe = useCallback(
    (codes: string[]) =>
      api
        .call<Texts>("error.describe", { codes })
        .then((found) => setTexts((old) => ({ ...old, ...found })))
        .catch(() => {}),
    [api],
  );

  // Texts for the hints of a result and for a failure code.
  useEffect(() => {
    const codes = [...(result?.hints ?? [])];
    if (state.failure && !state.failureIsText) codes.push(state.failure);
    if (codes.length > 0) void describe(codes);
  }, [result, state.failure, state.failureIsText, describe]);

  // The log is read while it is open.
  useEffect(() => {
    if (!logOpen || !jobId) return;
    const read = () =>
      api
        .call<{ lines: string[] }>("run.log", { job_id: jobId, lines: 200 })
        .then((l) => setLog(l.lines))
        .catch(() => {});
    void read();
    const timer = setInterval(read, 2000);
    return () => clearInterval(timer);
  }, [logOpen, jobId, api]);

  const fmt = (n: number, digits = 3) =>
    n.toLocaleString(language, { maximumSignificantDigits: digits });
  const minutes = (s: number) => String(Math.max(1, Math.round(s / 60)));
  const text = (code: string) => texts[code]?.[language];
  const failureText = state.failure && !state.failureIsText ? text(state.failure) : undefined;
  const lastCd = coeffs.cd.at(-1)?.[1];
  const residualSeries: Series[] = Object.entries(residuals).map(([name, points]) => ({
    name,
    points,
  }));
  const running = phase === "running";
  const percent = mesh ? Math.round(mesh.fraction * 100) : 0;

  return (
    <aside className="side-panel panel run-panel">
      <h2>{t("run.title")}</h2>
      <dl className="scenario-result">
        <dt>{t("mesh.cells")}</dt>
        <dd>
          {summary.cells !== null
            ? t("scenario.cells", { n: summary.cells.toLocaleString(language) })
            : t("run.meshFirst")}
        </dd>
        {summary.estimate_s !== null && (
          <>
            <dt>{t("scenario.time")}</dt>
            <dd>{t("run.about", { n: minutes(summary.estimate_s) })}</dd>
          </>
        )}
      </dl>
      <div className="run-actions">
        <button type="button" className="primary" disabled={running} onClick={() => m.start()}>
          {t("run.start")}
        </button>
        {running && (
          <button type="button" onClick={() => m.cancel()}>
            {t("run.cancel")}
          </button>
        )}
        {phase !== "idle" && (
          <span className={`run-state ${phase}`}>{t(`run.state.${phase}`)}</span>
        )}
      </div>
      {phase !== "idle" && (
        <ol className="timeline" aria-label={t("run.stages")}>
          {STAGES.map((s) => (
            <li
              key={s}
              className={s === stage ? "active" : seen.includes(s) ? "done" : ""}
              aria-current={s === stage && running ? "step" : undefined}
            >
              {t(`run.stage.${s}`)}
            </li>
          ))}
        </ol>
      )}
      {running && stage === null && <p className="small muted">{t("run.preparing")}</p>}
      {running && mesh && (
        <div className="mesh-progress">
          <div
            className="progress"
            role="progressbar"
            aria-label={t("run.meshing")}
            aria-valuemin={0}
            aria-valuemax={100}
            aria-valuenow={percent}
          >
            <div className="progress-fill" style={{ width: `${percent}%` }} />
          </div>
          <p className="small">
            {mesh.phase ? t(`run.meshPhase.${mesh.phase}`) : t("run.meshing")}
            {` · ${t("run.percent", { n: String(percent) })}`}
            {mesh.remaining_s !== null &&
              ` · ${t("run.remaining", { n: minutes(mesh.remaining_s) })}`}
          </p>
          {mesh.attempt > 1 && (
            <p className="small muted">{t("run.attempt", { n: String(mesh.attempt) })}</p>
          )}
        </div>
      )}
      {iteration && (
        <p className="small">
          {t("run.iteration", {
            n: String(iteration.n),
            max: iteration.max.toLocaleString(language),
          })}
          {remaining !== null && running && ` · ${t("run.remaining", { n: minutes(remaining) })}`}
        </p>
      )}
      {residualSeries.length > 0 && (
        <ExpertOnly>
          <LineChart label={t("run.residuals")} series={residualSeries} log />
        </ExpertOnly>
      )}
      {coeffs.cd.length > 0 && mode === "expert" && (
        <LineChart
          label={`${t("run.coefficients")}: Cd ${fmt(lastCd ?? 0)}`}
          series={[
            { name: "Cd", points: coeffs.cd },
            { name: "Cl", points: coeffs.cl },
          ]}
        />
      )}
      {loss.length > 0 && (
        <LineChart
          label={`${t("internal.pressureLoss")}: ${fmt(loss[loss.length - 1][1])} Pa`}
          series={[{ name: t("internal.pressureLoss"), points: loss }]}
        />
      )}
      {phase === "failed" && state.failure && (
        <p role="alert" className="error">
          {failureText ? `${failureText.text} ${failureText.fix}` : state.failure}
        </p>
      )}
      {result && (
        <section className="result-card" aria-label={t("run.result")}>
          <h3>{t("run.result")}</h3>
          <dl>
            {result.coefficients && result.forces && (
              <>
                <dt>Cd</dt>
                <dd>{fmt(result.coefficients.cd)}</dd>
                <dt>Cl</dt>
                <dd>{fmt(result.coefficients.cl)}</dd>
                <dt>{t("run.drag")}</dt>
                <dd>{`${fmt(result.forces.drag_n)} N`}</dd>
                <dt>{t("run.lift")}</dt>
                <dd>{`${fmt(result.forces.lift_n)} N`}</dd>
              </>
            )}
            {result.pressure_loss_pa !== undefined && (
              <>
                <dt>{t("internal.pressureLoss")}</dt>
                <dd>{`${fmt(result.pressure_loss_pa)} Pa`}</dd>
                <dt>{t("internal.massBalance")}</dt>
                <dd>{`${fmt((result.mass_balance_error ?? 0) * 100, 2)} %`}</dd>
              </>
            )}
            <dt>{t("run.iterations")}</dt>
            <dd>{result.iterations}</dd>
            <dt>{t("run.duration")}</dt>
            <dd>
              {t("run.about", {
                n: minutes(Object.values(result.timings_s).reduce((a, b) => a + b, 0)),
              })}
            </dd>
          </dl>
          {result.openings && (
            <table className="openings-table">
              <thead>
                <tr>
                  <th />
                  <th>{t("internal.flow")}</th>
                  <th>{t("internal.share")}</th>
                </tr>
              </thead>
              <tbody>
                {result.openings.map((o) => (
                  <tr key={o.key}>
                    <td>{o.key}</td>
                    <td>{`${fmt(o.flow_m3_s * 1000)} l/s`}</td>
                    <td>{o.share === null ? "" : `${fmt(o.share * 100)} %`}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          )}
          <ul className="diagnosis">
            {result.hints.map((h) => (
              <li key={h}>{text(h)?.text ?? h}</li>
            ))}
          </ul>
        </section>
      )}
      <ExpertOnly>
        <details className="mesh-details">
          <summary>{t("mesh.title")}</summary>
          <MeshSection
            api={api}
            revision={state.meshRevision}
            heading={false}
            showCells={false}
            busy={running}
            generateLabel={t("run.meshOnly")}
            onGenerate={() => m.start(true)}
          />
        </details>
        {jobId && (
          <details onToggle={(e) => setLogOpen((e.target as HTMLDetailsElement).open)}>
            <summary>{t("run.log")}</summary>
            <pre className="log">{log.join("\n")}</pre>
          </details>
        )}
      </ExpertOnly>
      {footer}
    </aside>
  );
}
