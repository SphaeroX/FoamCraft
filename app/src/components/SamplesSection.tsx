// SPDX-License-Identifier: GPL-3.0-or-later
// Sample projects on the start page: one click creates a copy in the default projects folder and
// opens it. Only when that folder is unknown the user is asked for one.
import { useEffect, useState } from "react";
import { errorMessage, type Api, type ProjectInfo } from "../api";
import { useTranslation } from "../i18n/i18n";

interface Sample {
  id: string;
  en: { title: string; description: string };
  de: { title: string; description: string };
}

interface Props {
  api: Api;
  pickFolder: () => Promise<string | null>;
  onOpened: (project: ProjectInfo) => void;
}

export function SamplesSection({ api, pickFolder, onOpened }: Props) {
  const { t, language } = useTranslation();
  const [samples, setSamples] = useState<Sample[] | null>(null);
  const [hasDefault, setHasDefault] = useState(false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    api
      .call<Sample[]>("samples.list", {})
      .then(setSamples)
      .catch(() => setSamples(null));
    api
      .call<{ folder: string }>("project.default_location", {})
      .then((l) => setHasDefault(Boolean(l?.folder)))
      .catch(() => setHasDefault(false));
  }, [api]);

  if (!samples || samples.length === 0) return null;

  const create = async (s: Sample) => {
    const params: Record<string, unknown> = { sample: s.id, language };
    if (!hasDefault) {
      const folder = await pickFolder();
      if (!folder) return;
      const sep = folder.includes("\\") && !folder.includes("/") ? "\\" : "/";
      params.path = `${folder.replace(/[\\/]+$/, "")}${sep}${s[language].title}`;
    }
    setError(null);
    setBusy(true);
    try {
      await api.call("samples.create", params);
      onOpened(await api.call<ProjectInfo>("project.info", {}));
    } catch (e) {
      setError(errorMessage(e, language));
    } finally {
      setBusy(false);
    }
  };

  return (
    <section className="samples" aria-label={t("samples.title")}>
      <h3>{t("samples.title")}</h3>
      <p className="small muted">{t("samples.hint")}</p>
      {samples.map((s) => (
        <button
          key={s.id}
          type="button"
          className="recent-item"
          disabled={busy}
          onClick={() => void create(s)}
        >
          <strong>{s[language].title}</strong>
          <span className="small muted">{s[language].description}</span>
        </button>
      ))}
      {error && (
        <p role="alert" className="error">
          {error}
        </p>
      )}
    </section>
  );
}
