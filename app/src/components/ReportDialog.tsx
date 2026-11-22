// SPDX-License-Identifier: GPL-3.0-or-later
// Report of a run (or a comparison of two runs) as PDF and HTML: run, language, formats,
// sections, title, author and an optional logo. The work is the command export.report.
import { useEffect, useState } from "react";
import { errorMessage, type Api } from "../api";
import { useTranslation } from "../i18n/i18n";

const SECTIONS = [
  "summary",
  "setup",
  "mesh",
  "convergence",
  "figures",
  "views",
  "reliability",
  "comparison",
  "limits",
  "versions",
] as const;

interface Run {
  job_id: string;
  name: string;
}

interface Props {
  api: Api;
  projectName: string;
  /** Asks where to save; the extension is set per format. */
  pickSavePath: () => Promise<string | null>;
  pickLogo: () => Promise<string | null>;
  onClose: () => void;
}

export function ReportDialog({ api, projectName, pickSavePath, pickLogo, onClose }: Props) {
  const { t, language } = useTranslation();
  const [loaded, setLoaded] = useState<Run[] | null>(null);
  const runs = loaded ?? [];
  const [job, setJob] = useState("");
  const [other, setOther] = useState("");
  // Several runs to compare with; when some are ticked they replace the single comparison.
  const [several, setSeveral] = useState<string[]>([]);
  const [lang, setLang] = useState<string>(language);
  const [pdf, setPdf] = useState(true);
  const [html, setHtml] = useState(true);
  const [sections, setSections] = useState<string[]>([...SECTIONS]);
  const [title, setTitle] = useState(projectName);
  const [author, setAuthor] = useState("");
  const [logo, setLogo] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [message, setMessage] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    api
      // Only the names are needed: measuring the disk space of large runs takes long.
      .call<Run[]>("runs.list", { sizes: false })
      .then((list) => {
        setLoaded(list);
        setJob((j) => j || list[0]?.job_id || "");
      })
      .catch((e: unknown) => {
        setLoaded([]);
        setError(errorMessage(e, language));
      });
  }, [api, language]);

  const save = async () => {
    const path = await pickSavePath();
    if (!path) return;
    const formats = [...(pdf ? ["pdf"] : []), ...(html ? ["html"] : [])];
    const params: Record<string, unknown> = {
      path,
      job_id: job,
      language: lang,
      formats,
      title,
      author,
    };
    const many = several.filter((r) => r !== job);
    if (many.length > 0) params.compare_runs = many;
    else if (other) params.compare_with = other;
    if (logo) params.logo_path = logo;
    if (sections.length !== SECTIONS.length) params.sections = sections;
    setBusy(true);
    setMessage(null);
    setError(null);
    api
      .call<{ pdf: string | null; html: string | null }>("export.report", params)
      .then((r) =>
        setMessage(t("report.saved", { files: [r.pdf, r.html].filter(Boolean).join(", ") })),
      )
      .catch((e: unknown) => setError(errorMessage(e, language)))
      .finally(() => setBusy(false));
  };

  return (
    <div className="dialog-backdrop" role="presentation" onClick={onClose}>
      <div
        className="dialog panel report-dialog"
        role="dialog"
        aria-modal="true"
        aria-labelledby="report-title"
        onClick={(e) => e.stopPropagation()}
      >
        <h2 id="report-title">{t("report.title")}</h2>
        {loaded === null && (
          <p role="status" className="small muted">
            {t("report.loadingRuns")}
          </p>
        )}
        <div className="export-grid">
          <label className="unit-field">
            <span>{t("report.run")}</span>
            <select
              aria-label={t("report.run")}
              value={job}
              onChange={(e) => setJob(e.target.value)}
            >
              {runs.map((r) => (
                <option key={r.job_id} value={r.job_id}>
                  {r.name}
                </option>
              ))}
            </select>
          </label>
          <label className="unit-field">
            <span>{t("report.compare")}</span>
            <select
              aria-label={t("report.compare")}
              value={other}
              onChange={(e) => setOther(e.target.value)}
            >
              <option value="">{t("report.noCompare")}</option>
              {runs
                .filter((r) => r.job_id !== job)
                .map((r) => (
                  <option key={r.job_id} value={r.job_id}>
                    {r.name}
                  </option>
                ))}
            </select>
          </label>
          {runs.length > 2 && (
            <fieldset className="check-list">
              <legend>{t("report.compareRuns")}</legend>
              {runs
                .filter((r) => r.job_id !== job)
                .map((r) => (
                  <label key={r.job_id} className="check">
                    <input
                      type="checkbox"
                      aria-label={r.name}
                      checked={several.includes(r.job_id)}
                      onChange={(e) =>
                        setSeveral((s) =>
                          e.target.checked ? [...s, r.job_id] : s.filter((x) => x !== r.job_id),
                        )
                      }
                    />
                    {r.name}
                  </label>
                ))}
            </fieldset>
          )}
          <label className="unit-field">
            <span>{t("report.title_field")}</span>
            <input
              type="text"
              aria-label={t("report.title_field")}
              value={title}
              onChange={(e) => setTitle(e.target.value)}
            />
          </label>
          <label className="unit-field">
            <span>{t("report.author")}</span>
            <input
              type="text"
              aria-label={t("report.author")}
              value={author}
              onChange={(e) => setAuthor(e.target.value)}
            />
          </label>
          <label className="unit-field">
            <span>{t("report.language")}</span>
            <select
              aria-label={t("report.language")}
              value={lang}
              onChange={(e) => setLang(e.target.value)}
            >
              <option value="en">English</option>
              <option value="de">Deutsch</option>
            </select>
          </label>
          <div className="unit-field">
            <span>{t("report.formats")}</span>
            <label className="check">
              <input
                type="checkbox"
                aria-label="PDF"
                checked={pdf}
                onChange={(e) => setPdf(e.target.checked)}
              />
              PDF
            </label>
            <label className="check">
              <input
                type="checkbox"
                aria-label="HTML"
                checked={html}
                onChange={(e) => setHtml(e.target.checked)}
              />
              HTML
            </label>
          </div>
        </div>
        <fieldset className="report-sections">
          <legend>{t("report.sections")}</legend>
          {SECTIONS.map((s) => (
            <label key={s} className="check">
              <input
                type="checkbox"
                aria-label={t(`report.section.${s}`)}
                checked={sections.includes(s)}
                onChange={(e) =>
                  setSections((list) =>
                    e.target.checked
                      ? SECTIONS.filter((x) => x === s || list.includes(x))
                      : list.filter((x) => x !== s),
                  )
                }
              />
              {t(`report.section.${s}`)}
            </label>
          ))}
        </fieldset>
        <div className="actions">
          <button
            type="button"
            onClick={() =>
              pickLogo()
                .then((p) => p && setLogo(p))
                .catch(() => setLogo(null))
            }
          >
            {t("report.logo")}
          </button>
          {logo && <span className="small">{logo}</span>}
        </div>
        {message && <p className="small">{message}</p>}
        {error && (
          <p role="alert" className="error">
            {error}
          </p>
        )}
        <div className="dialog-actions">
          <button
            type="button"
            disabled={busy || !job || (!pdf && !html)}
            onClick={() => void save()}
          >
            {t("report.save")}
          </button>
          <button type="button" onClick={onClose}>
            {t("about.close")}
          </button>
        </div>
      </div>
    </div>
  );
}
