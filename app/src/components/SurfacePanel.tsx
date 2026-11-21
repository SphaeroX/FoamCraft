// SPDX-License-Identifier: GPL-3.0-or-later
import type React from "react";
// Faces, parts and face groups: selection actions, naming and grouping. All changes are commands.
import { useCallback, useEffect, useMemo, useState } from "react";
import { errorMessage, type Api, type SurfaceGroup, type SurfaceInfo } from "../api";
import { useTranslation } from "../i18n/i18n";

interface Props {
  api: Api;
  /** Selected face keys (from `selection.changed` events). */
  selection: string[];
  /** Changes whenever the project changed, so the lists reload. */
  revision: number;
  /** Extra content at the end of the panel. */
  children?: React.ReactNode;
}

function partNumber(part: string): string {
  return part.split(":part-")[1] ?? part;
}

export function SurfacePanel({ api, selection, revision, children }: Props) {
  const { t, language } = useTranslation();
  const [faces, setFaces] = useState<SurfaceInfo[]>([]);
  const [groups, setGroups] = useState<SurfaceGroup[]>([]);
  const [groupName, setGroupName] = useState("");
  const [faceName, setFaceName] = useState("");
  const [error, setError] = useState<string | null>(null);

  const reload = useCallback(async () => {
    const [f, g] = await Promise.all([
      api.call<SurfaceInfo[]>("surface.list"),
      api.call<SurfaceGroup[]>("group.list"),
    ]);
    setFaces(f);
    setGroups(g);
  }, [api]);

  useEffect(() => {
    reload().catch((e: unknown) => setError(errorMessage(e, language)));
  }, [reload, revision, language]);

  const single = selection.length === 1 ? faces.find((f) => f.key === selection[0]) : undefined;
  useEffect(() => setFaceName(single?.name ?? ""), [single?.key, single?.name]);

  const parts = useMemo(() => {
    const map = new Map<string, { model: string; keys: string[] }>();
    for (const f of faces) {
      const entry = map.get(f.part) ?? { model: f.model_name, keys: [] };
      entry.keys.push(f.key);
      map.set(f.part, entry);
    }
    return [...map.entries()];
  }, [faces]);

  const run = (name: string, params: Record<string, unknown>) => {
    setError(null);
    return api
      .call(name, params)
      .then(() => reload())
      .catch((e: unknown) => setError(errorMessage(e, language)));
  };

  const count = (n: number) => t(n === 1 ? "surfaces.oneFace" : "surfaces.faces", { n: String(n) });

  return (
    <aside className="side-panel right panel">
      <h2>{t("surfaces.title")}</h2>
      <p className="small muted">{t("surfaces.hint")}</p>
      <p>
        {selection.length === 1
          ? t("surfaces.selectedOne")
          : t("surfaces.selected", { n: String(selection.length) })}
      </p>
      <div className="actions">
        <button
          type="button"
          disabled={selection.length === 0}
          onClick={() => run("surface.select_adjacent", {})}
        >
          {t("surfaces.adjacent")}
        </button>
        <button
          type="button"
          disabled={selection.length === 0}
          onClick={() => run("surface.clear_selection", {})}
        >
          {t("surfaces.clear")}
        </button>
      </div>
      {single && (
        <label className="unit-field">
          <span>{t("surfaces.faceName")}</span>
          <input
            type="text"
            aria-label={t("surfaces.faceName")}
            value={faceName}
            onChange={(e) => setFaceName(e.target.value)}
            onBlur={() => {
              if (faceName !== (single.name ?? "")) {
                run("surface.rename", { key: single.key, name: faceName });
              }
            }}
          />
        </label>
      )}
      <div className="start-new">
        <label>
          <span>{t("surfaces.groupName")}</span>
          <input
            type="text"
            aria-label={t("surfaces.groupName")}
            value={groupName}
            onChange={(e) => setGroupName(e.target.value)}
          />
        </label>
        <button
          type="button"
          disabled={selection.length === 0 || groupName.trim() === ""}
          onClick={() =>
            run("group.create", { name: groupName.trim() }).then(() => setGroupName(""))
          }
        >
          {t("surfaces.createGroup")}
        </button>
      </div>
      {error && (
        <p role="alert" className="error">
          {error}
        </p>
      )}
      <h3>{t("surfaces.groups")}</h3>
      {groups.length === 0 && <p className="muted small">{t("surfaces.noGroups")}</p>}
      <ul className="recent-list">
        {groups.map((g) => (
          <li key={g.name} className="group-row">
            <button
              type="button"
              aria-label={t("surfaces.selectGroup", { name: g.name })}
              onClick={() => run("surface.select", { keys: g.keys })}
            >
              <strong>{g.name}</strong>
              <span className="muted small">{count(g.keys.length)}</span>
            </button>
            <div className="actions">
              <button
                type="button"
                disabled={selection.length === 0}
                onClick={() => run("group.add", { name: g.name })}
              >
                {t("surfaces.addToGroup")}
              </button>
              <button type="button" onClick={() => run("group.delete", { name: g.name })}>
                {t("surfaces.deleteGroup")}
              </button>
            </div>
          </li>
        ))}
      </ul>
      <h3>{t("surfaces.parts")}</h3>
      <ul className="recent-list">
        {parts.map(([part, { model, keys }]) => (
          <li key={part}>
            <button type="button" onClick={() => run("surface.select", { keys })}>
              <strong>{t("surfaces.part", { n: partNumber(part) })}</strong>
              <span className="muted small">
                {model} · {count(keys.length)}
              </span>
            </button>
          </li>
        ))}
      </ul>
      {children}
    </aside>
  );
}
