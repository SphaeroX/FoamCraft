// SPDX-License-Identifier: GPL-3.0-or-later
import { useEffect, useState } from "react";
import { errorMessage, type Api, type ProjectInfo, type RecentProject } from "../api";
import { useTranslation } from "../i18n/i18n";
import { SamplesSection } from "./SamplesSection";

interface Props {
  api: Api;
  /** Lets the user choose a folder; null when cancelled. */
  pickFolder: () => Promise<string | null>;
  onOpened: (project: ProjectInfo) => void;
}

function joinPath(folder: string, name: string): string {
  const sep = folder.includes("\\") && !folder.includes("/") ? "\\" : "/";
  return folder.endsWith(sep) ? `${folder}${name}` : `${folder}${sep}${name}`;
}

export function StartPage({ api, pickFolder, onOpened }: Props) {
  const { t, language } = useTranslation();
  const [recent, setRecent] = useState<RecentProject[]>([]);
  const [name, setName] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  // Where a project with the typed name goes; null when the default folder is unknown.
  const [location, setLocation] = useState<{ folder: string; path: string | null } | null>(null);

  useEffect(() => {
    api
      .call<RecentProject[]>("project.recent")
      .then(setRecent)
      .catch(() => setRecent([]));
  }, [api]);

  useEffect(() => {
    let current = true;
    const projectName = name.trim();
    api
      .call<{ folder: string; path: string | null }>(
        "project.default_location",
        projectName ? { name: projectName } : {},
      )
      .then((l) => current && setLocation(l?.folder ? l : null))
      .catch(() => current && setLocation(null));
    return () => {
      current = false;
    };
  }, [api, name]);

  async function run(action: () => Promise<ProjectInfo | null>) {
    setError(null);
    setBusy(true);
    try {
      const project = await action();
      if (project) onOpened(project);
    } catch (err) {
      setError(errorMessage(err, language));
    } finally {
      setBusy(false);
    }
  }

  /** Creates the project in the default folder, or in a chosen one. */
  const create = (choose: boolean) =>
    run(async () => {
      const projectName = name.trim();
      if (!projectName) {
        setError(t("start.nameMissing"));
        return null;
      }
      if (!choose && location) {
        return api.call<ProjectInfo>("project.new", { name: projectName });
      }
      const folder = await pickFolder();
      if (!folder) return null;
      return api.call<ProjectInfo>("project.new", {
        path: joinPath(folder, projectName),
        name: projectName,
      });
    });

  const open = (path?: string) =>
    run(async () => {
      const folder = path ?? (await pickFolder());
      if (!folder) return null;
      return api.call<ProjectInfo>("project.open", { path: folder });
    });

  return (
    <main className="start-page panel">
      <h1>{t("start.title")}</h1>
      <p className="muted">{t("start.subtitle")}</p>
      <section className="start-new">
        <label>
          <span>{t("start.name.label")}</span>
          <input
            type="text"
            value={name}
            placeholder={t("start.name.placeholder")}
            onChange={(e) => setName(e.target.value)}
          />
        </label>
        <button type="button" disabled={busy} onClick={() => create(false)}>
          {t("start.create")}
        </button>
      </section>
      {location && (
        <p className="small muted start-location">
          {t("start.location", { path: location.path ?? location.folder })}{" "}
          <button type="button" className="link" disabled={busy} onClick={() => create(true)}>
            {t("start.otherFolder")}
          </button>
        </p>
      )}
      <button type="button" disabled={busy} onClick={() => open()}>
        {t("start.open")}
      </button>
      {error && (
        <p role="alert" className="error">
          {error}
        </p>
      )}
      <section>
        <h2>{t("start.recent")}</h2>
        {recent.length === 0 ? (
          <p className="muted">{t("start.recentEmpty")}</p>
        ) : (
          <ul className="recent-list">
            {recent.map((r) => (
              <li key={r.path}>
                <button type="button" disabled={busy} onClick={() => open(r.path)}>
                  <strong>{r.name}</strong>
                  <span className="muted small">{r.path}</span>
                </button>
              </li>
            ))}
          </ul>
        )}
      </section>
      <SamplesSection api={api} pickFolder={pickFolder} onOpened={onOpened} />
    </main>
  );
}
