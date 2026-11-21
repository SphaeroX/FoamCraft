// SPDX-License-Identifier: GPL-3.0-or-later
// The four steps as a streamline: numbered stations joined by a line that fills up as steps are
// done; and the Back/Next footer of a step panel.
import { useTranslation } from "../i18n/i18n";
import { STEPS, canAdvance, type Step, type StepInfo } from "../steps";

interface RailProps {
  step: Step;
  states: Record<Step, StepInfo>;
  onStep: (step: Step) => void;
}

export function StepRail({ step, states, onStep }: RailProps) {
  const { t } = useTranslation();
  const done = STEPS.filter((s) => states[s].state === "done" || states[s].state === "attention");
  // Share of the line that is filled: up to the last finished station.
  const reached = Math.max(0, ...done.map((s) => STEPS.indexOf(s)));
  const fill = done.length === 0 ? 0 : reached / (STEPS.length - 1);
  return (
    <nav className="step-rail" aria-label={t("steps.label")}>
      <span className="step-rail-line" aria-hidden="true">
        <span className="step-rail-fill" style={{ transform: `scaleX(${fill})` }} />
      </span>
      <ol>
        {STEPS.map((s, i) => {
          const info = states[s];
          const current = s === step;
          return (
            <li key={s}>
              <button
                type="button"
                data-state={info.state}
                aria-current={current ? "step" : undefined}
                className={current ? "current" : ""}
                title={info.missing ? t(info.missing) : t(`stepState.${info.state}`)}
                onClick={() => onStep(s)}
              >
                <span className="step-dot" aria-hidden="true">
                  {info.state === "done" ? "✓" : info.state === "attention" ? "!" : i + 1}
                </span>
                <span className="step-name">{t(`steps.${s}`)}</span>
                <span className="visually-hidden">{`, ${t(`stepState.${info.state}`)}`}</span>
              </button>
            </li>
          );
        })}
      </ol>
    </nav>
  );
}

export function StepFooter({ step, states, onStep }: RailProps) {
  const { t } = useTranslation();
  const index = STEPS.indexOf(step);
  const next = STEPS[index + 1];
  const ok = canAdvance(states, step);
  const missing = states[step].missing;
  return (
    <div className="step-footer">
      {index > 0 && (
        <button type="button" className="quiet" onClick={() => onStep(STEPS[index - 1])}>
          {t("nav.back")}
        </button>
      )}
      <span className="step-footer-note muted small">{!ok && missing ? t(missing) : ""}</span>
      {next && (
        <button type="button" className="primary" disabled={!ok} onClick={() => onStep(next)}>
          {t("nav.next")}
        </button>
      )}
    </div>
  );
}
