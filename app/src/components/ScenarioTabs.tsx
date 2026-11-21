// SPDX-License-Identifier: GPL-3.0-or-later
// The named scenarios of the project as tabs above the scenario inputs: switch, add a copy,
// rename, remove.
import { useCallback, useEffect, useState } from "react";
import type { Api } from "../api";
import { useTranslation } from "../i18n/i18n";

export interface ListedScenario {
  id: string;
  name: string;
  runs: string[];
}

export interface ScenarioList {
  active: string;
  scenarios: ListedScenario[];
}

/** The shown name: the given one, or "Scenario N" from the number in the id. */
export function scenarioName(
  s: { id: string; name: string },
  t: (k: string, p?: Record<string, string>) => string,
) {
  return s.name || t("scenarios.unnamed", { n: s.id.replace(/^scenario-/, "") });
}

interface Props {
  api: Api;
  /** Changes when the project changed elsewhere. */
  revision: number;
}

export function ScenarioTabs({ api, revision }: Props) {
  const { t } = useTranslation();
  const [list, setList] = useState<ScenarioList | null>(null);
  const [draft, setDraft] = useState<string | null>(null);

  const load = useCallback(() => {
    api
      .call<ScenarioList>("scenario.list", {})
      .then((l) => setList(l && Array.isArray(l.scenarios) ? l : null))
      .catch(() => setList(null));
  }, [api]);
  useEffect(load, [load, revision]);

  const act = (name: string, params: Record<string, unknown>) =>
    api
      .call(name, params)
      .then(load)
      .catch(() => {});

  if (!list) return null;
  const active = list.scenarios.find((s) => s.id === list.active);
  return (
    <section className="scenario-tabs" aria-label={t("scenarios.title")}>
      <div role="tablist" className="tabs">
        {list.scenarios.map((s) => (
          <button
            key={s.id}
            type="button"
            role="tab"
            aria-selected={s.id === list.active}
            onClick={() => s.id !== list.active && void act("scenario.activate", { id: s.id })}
          >
            {scenarioName(s, t)}
          </button>
        ))}
        <button
          type="button"
          className="quiet"
          aria-label={t("scenarios.add")}
          title={t("scenarios.add")}
          onClick={() => void act("scenario.add", { name: "", copy: true })}
        >
          +
        </button>
      </div>
      {active && (
        <div className="row">
          <input
            type="text"
            aria-label={t("scenarios.name")}
            value={draft ?? scenarioName(active, t)}
            onChange={(e) => setDraft(e.target.value)}
            onBlur={() => {
              if (draft !== null && draft !== scenarioName(active, t))
                void act("scenario.rename", { id: active.id, name: draft });
              setDraft(null);
            }}
          />
        </div>
      )}
      {list.scenarios
        .filter((s) => s.id !== list.active)
        .map((s) => (
          <button
            key={s.id}
            type="button"
            className="quiet small"
            aria-label={t("scenarios.remove", { name: scenarioName(s, t) })}
            onClick={() => void act("scenario.remove", { id: s.id })}
          >
            {t("scenarios.remove", { name: scenarioName(s, t) })}
          </button>
        ))}
    </section>
  );
}
