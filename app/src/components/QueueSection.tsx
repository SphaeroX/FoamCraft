// SPDX-License-Identifier: GPL-3.0-or-later
// "Run all scenarios": starts the queue, shows which scenario is being calculated and stops it.
import { useEffect, useState } from "react";
import type { Api, ApiEvent } from "../api";
import { useTranslation } from "../i18n/i18n";
import { scenarioName, type ScenarioList } from "./ScenarioTabs";

interface Props {
  api: Api;
  subscribe: (handler: (event: ApiEvent) => void) => () => void;
}

type QueueState =
  | { kind: "idle" }
  | { kind: "running"; index: number; total: number; scenario: string }
  | { kind: "done"; cancelled: boolean; failed: number };

export function QueueSection({ api, subscribe }: Props) {
  const { t } = useTranslation();
  const [list, setList] = useState<ScenarioList | null>(null);
  const [state, setState] = useState<QueueState>({ kind: "idle" });

  useEffect(() => {
    api
      .call<ScenarioList>("scenario.list", {})
      .then((l) => setList(l && Array.isArray(l.scenarios) ? l : null))
      .catch(() => setList(null));
  }, [api]);

  useEffect(
    () =>
      subscribe((e) => {
        const p = (e.payload ?? {}) as Record<string, unknown>;
        if (e.name === "run.queue") {
          setState({
            kind: "running",
            index: Number(p.index),
            total: Number(p.total),
            scenario: String(p.scenario),
          });
        } else if (e.name === "run.queue_finished") {
          const runs = Array.isArray(p.runs) ? (p.runs as { error: unknown }[]) : [];
          setState({
            kind: "done",
            cancelled: p.cancelled === true,
            failed: runs.filter((r) => r.error).length,
          });
        }
      }),
    [subscribe],
  );

  const name = (id: string) => {
    const s = list?.scenarios.find((x) => x.id === id);
    return s ? scenarioName(s, t) : id;
  };
  const running = state.kind === "running";
  // A queue only makes sense with two or more scenarios.
  if (!list || list.scenarios.length < 2) return null;
  return (
    <section className="queue">
      {running ? (
        <button type="button" onClick={() => void api.call("run.cancel_queue", {}).catch(() => {})}>
          {t("queue.stop")}
        </button>
      ) : (
        <button
          type="button"
          onClick={() => {
            setState({
              kind: "running",
              index: 0,
              total: list?.scenarios.length ?? 0,
              scenario: "",
            });
            void api.call("run.start_all", {}).catch(() => setState({ kind: "idle" }));
          }}
        >
          {t("queue.start")}
        </button>
      )}
      <p role="status" className="small muted">
        {state.kind === "running" && state.scenario
          ? t("queue.progress", {
              n: String(state.index + 1),
              of: String(state.total),
              name: name(state.scenario),
            })
          : state.kind === "done"
            ? state.cancelled
              ? t("queue.stopped")
              : state.failed > 0
                ? t("queue.failed", { n: String(state.failed) })
                : t("queue.done")
            : ""}
      </p>
    </section>
  );
}
