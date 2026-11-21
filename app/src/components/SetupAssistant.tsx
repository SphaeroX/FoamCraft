// SPDX-License-Identifier: GPL-3.0-or-later
// Setup assistant: the system check with explanations and step-by-step help, providing the
// runtime image with progress, the hardware profile and the diagnostic report. Shown at the
// first start and from the toolbar ("System").
import { useCallback, useEffect, useState } from "react";
import { errorMessage, type Api } from "../api";
import { useTranslation } from "../i18n/i18n";

interface StepText {
  text: string;
  fix: string;
  help: string[] | null;
}

interface Step {
  id: string;
  state: "ok" | "warning" | "action" | "info" | "skipped";
  code: string;
  detail: string;
  en: StepText;
  de: StepText;
}

interface Profile {
  default_cores: number;
  max_cells: number;
  quality: string;
}

interface Check {
  ready: boolean;
  steps: Step[];
  profile: Profile;
}

type Listener = (e: { name: string; payload: unknown }) => void;

interface Props {
  api: Api;
  copy: (text: string) => Promise<void>;
  onClose: () => void;
  /** Subscribes to command API events; returns the unsubscribe function. */
  subscribe: (listener: Listener) => () => void;
}

const MARK: Record<Step["state"], string> = {
  ok: "✓",
  warning: "!",
  action: "✗",
  info: "i",
  skipped: "–",
};

export function SetupAssistant({ api, copy, onClose, subscribe }: Props) {
  const { t, language } = useTranslation();
  const [check, setCheck] = useState<Check | null>(null);
  const [busy, setBusy] = useState(false);
  const [providing, setProviding] = useState(false);
  const [progress, setProgress] = useState<string | null>(null);
  const [message, setMessage] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  const run = useCallback(() => {
    setBusy(true);
    api
      .call<Check>("system.check", {})
      .then(setCheck)
      .catch((e: unknown) => setError(errorMessage(e, language)))
      .finally(() => setBusy(false));
  }, [api, language]);

  useEffect(run, [run]);

  useEffect(
    () =>
      subscribe((e) => {
        if (e.name === "system.image_progress") {
          const p = e.payload as { line: string; lines: number };
          setProgress(p.line);
        } else if (e.name === "system.image_ready") {
          setProviding(false);
          setProgress(null);
          run();
        } else if (e.name === "system.image_failed") {
          setProviding(false);
          setError(t("setup.imageFailed"));
        }
      }),
    [subscribe, run, t],
  );

  const provide = () => {
    setProviding(true);
    setError(null);
    api.call("system.provide_image", {}).catch((e: unknown) => {
      setProviding(false);
      setError(errorMessage(e, language));
    });
  };

  const millions = (n: number) => (n / 1e6).toLocaleString(language, { maximumFractionDigits: 1 });

  return (
    <div className="dialog-backdrop" role="presentation">
      <div
        className="dialog panel setup-dialog"
        role="dialog"
        aria-modal="true"
        aria-labelledby="setup-title"
        onKeyDown={(e) => e.key === "Escape" && onClose()}
      >
        <h2 id="setup-title">{t("setup.title")}</h2>
        <p className="small">{t("setup.intro")}</p>
        {check && (
          <>
            <ul className="setup-steps" aria-label={t("setup.list")}>
              {check.steps.map((s) => {
                const text = s[language];
                return (
                  <li key={s.id} className={`setup-step ${s.state}`}>
                    <span className="mark" aria-hidden="true">
                      {MARK[s.state]}
                    </span>
                    <div>
                      <strong>{t(`setup.step.${s.id}`)}</strong>
                      {s.state === "action" && <span className="badge">{t("setup.action")}</span>}
                      <p>
                        {text.text}
                        {s.detail && <span className="muted small"> ({s.detail})</span>}
                      </p>
                      {s.state !== "ok" && text.fix && <p className="small">{text.fix}</p>}
                      {s.state === "action" && text.help && text.help.length > 0 && (
                        <ol className="small">
                          {text.help.map((h) => (
                            <li key={h}>{h}</li>
                          ))}
                        </ol>
                      )}
                      {s.id === "image" && s.state === "action" && (
                        <button type="button" disabled={providing} onClick={provide}>
                          {t("setup.provide")}
                        </button>
                      )}
                    </div>
                  </li>
                );
              })}
            </ul>
            {providing && (
              <p className="small" aria-live="polite">
                {t("setup.providing")} {progress && <code>{progress}</code>}
              </p>
            )}
            <p className={check.ready ? "ready" : "muted"} aria-live="polite">
              {check.ready ? t("setup.ready") : t("setup.notReady")}
            </p>
            <section className="setup-profile" aria-label={t("setup.profile")}>
              <h3>{t("setup.profile")}</h3>
              <p className="small">
                {t("setup.profileText", {
                  cores: String(check.profile.default_cores),
                  cells: millions(check.profile.max_cells),
                  quality: t(`setup.quality.${check.profile.quality}`),
                })}
              </p>
            </section>
          </>
        )}
        {message && <p className="small">{message}</p>}
        {error && (
          <p role="alert" className="error">
            {error}
          </p>
        )}
        <div className="dialog-actions">
          <button type="button" disabled={busy} onClick={run}>
            {t("setup.recheck")}
          </button>
          <button
            type="button"
            onClick={() =>
              api
                .call<{ text: string }>("system.diagnostics", {})
                .then((d) => copy(d.text))
                .then(() => setMessage(t("setup.copied")))
                .catch((e: unknown) => setError(errorMessage(e, language)))
            }
          >
            {t("setup.copy")}
          </button>
          <button
            type="button"
            autoFocus
            onClick={() =>
              api
                .call("system.finish_setup", {})
                .then(onClose)
                .catch((e: unknown) => setError(errorMessage(e, language)))
            }
          >
            {t("setup.finish")}
          </button>
        </div>
      </div>
    </div>
  );
}
