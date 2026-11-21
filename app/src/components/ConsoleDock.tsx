// SPDX-License-Identifier: GPL-3.0-or-later
// A collapsible console at the bottom of the window: one status line of the run of the project
// in every step, and its log (meshing and solving) when opened. Hidden until a run exists.
import { useEffect, useRef, useState } from "react";
import type { Api } from "../api";
import { useTranslation } from "../i18n/i18n";
import { useRunMonitor, type RunMonitor } from "../run-monitor";

interface Props {
  api: Api;
  monitor: RunMonitor;
}

/** Lines read from the end of the log. */
const LINES = 400;

export function ConsoleDock({ api, monitor }: Props) {
  const { t, language } = useTranslation();
  const state = useRunMonitor(monitor);
  const [open, setOpen] = useState(false);
  const [lines, setLines] = useState<string[]>([]);
  const box = useRef<HTMLPreElement>(null);
  const { jobId, phase } = state;

  // The log is read while the console is open: repeatedly during a run, once afterwards.
  useEffect(() => {
    if (!open || !jobId) return;
    const read = () =>
      api
        .call<{ lines: string[] }>("run.log", { job_id: jobId, lines: LINES })
        .then((l) => setLines(l.lines))
        .catch(() => {});
    void read();
    if (phase !== "running") return;
    const timer = setInterval(read, 2000);
    return () => clearInterval(timer);
  }, [open, jobId, phase, api]);

  // Follow the end of the log.
  useEffect(() => {
    const el = box.current;
    if (el) el.scrollTop = el.scrollHeight;
  }, [lines]);

  if (phase === "idle") return null;

  const parts = [t(`run.state.${phase}`)];
  if (phase === "running") {
    parts.length = 0;
    if (state.stage) parts.push(t(`run.stage.${state.stage}`));
    if (state.mesh) {
      parts.push(t("run.percent", { n: String(Math.round(state.mesh.fraction * 100)) }));
    } else if (state.iteration) {
      parts.push(
        t("run.iteration", {
          n: String(state.iteration.n),
          max: state.iteration.max.toLocaleString(language),
        }),
      );
    }
    if (parts.length === 0) parts.push(t("run.preparing"));
  }

  return (
    <section className={`console-dock panel ${open ? "open" : ""}`}>
      <button
        type="button"
        className="console-toggle"
        aria-expanded={open}
        aria-controls="console-log"
        onClick={() => setOpen((o) => !o)}
      >
        <span className={`console-light ${phase}`} aria-hidden="true" />
        <strong>{t("console.title")}</strong>
        <span className="console-status">{parts.join(" · ")}</span>
        <span className="console-chevron" aria-hidden="true">
          {open ? "▾" : "▴"}
        </span>
      </button>
      {open && (
        <pre id="console-log" className="log console-log" ref={box} tabIndex={0}>
          {lines.length > 0 ? lines.join("\n") : t("console.empty")}
        </pre>
      )}
    </section>
  );
}
