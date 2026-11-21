// SPDX-License-Identifier: GPL-3.0-or-later
// Heavy particles (Task_34): material cards or own values, the Stokes number with one sentence,
// and the impact map on the model with its share that hits.
import { useCallback, useEffect, useRef, useState } from "react";
import type { Api, LocalizedMessage } from "../api";
import { useTranslation } from "../i18n/i18n";

interface Material {
  id: string;
  diameter_m: number;
  density_kg_m3: number;
  on_impact: string;
  stokes: number | null;
  sentence: string | null;
  rough: boolean;
}

type Texts = Record<string, { en: LocalizedMessage; de: LocalizedMessage }>;

interface ImpactStatus {
  state: string;
  done: number;
  total: number;
  map?: { share_percent: number; uncertainty_percent: number; hits: number };
}

interface Props {
  api: Api;
  /** Changes with every project change. */
  revision?: number;
  /** Milliseconds between two progress requests of the impact map. */
  pollMs?: number;
}

const CODES = [
  "particles.stokes.low",
  "particles.stokes.middle",
  "particles.stokes.high",
  "particles.rough",
  "impact.estimate",
];

export function HeavySection({ api, revision = 0, pollMs = 500 }: Props) {
  const { t, language } = useTranslation();
  const [materials, setMaterials] = useState<Material[]>([]);
  const [texts, setTexts] = useState<Texts>({});
  const [chosen, setChosen] = useState<Material | null>(null);
  const [heavy, setHeavy] = useState(false);
  const [diameter, setDiameter] = useState("100");
  const [density, setDensity] = useState("1000");
  const [impact, setImpact] = useState<ImpactStatus | null>(null);
  const [error, setError] = useState<string | null>(null);
  const timer = useRef<ReturnType<typeof setTimeout> | null>(null);

  useEffect(() => {
    api
      .call<Material[]>("particles.materials", {})
      .then((m) => setMaterials(Array.isArray(m) ? m : []))
      .catch(() => {});
    api
      .call<Texts>("error.describe", { codes: CODES })
      .then((x) => setTexts(x ?? {}))
      .catch(() => {});
  }, [api]);

  useEffect(() => {
    api
      .call<{ particles?: { inertia?: { material?: string } | null } | null }>("view.state", {})
      .then((v) => setHeavy(Boolean(v?.particles?.inertia)))
      .catch(() => {});
  }, [api, revision]);

  useEffect(
    () => () => {
      if (timer.current) clearTimeout(timer.current);
    },
    [],
  );

  const text = (code: string | null | undefined) =>
    code ? (texts[code]?.[language]?.text ?? "") : "";

  const fail = (e: { en?: LocalizedMessage; de?: LocalizedMessage }) =>
    setError(e?.[language]?.text ?? String(e));

  const choose = (m: Material) => {
    setError(null);
    setChosen(m);
    setHeavy(true);
    api.call("view.set_particles", { inertia: { material: m.id } }).catch(fail);
  };

  const custom = () => {
    const d = Number(diameter) * 1e-6;
    const rho = Number(density);
    setError(null);
    api
      .call<Material[]>("particles.materials", { custom: { diameter_m: d, density_kg_m3: rho } })
      .then((list) => setChosen(list.find((m) => m.id === "custom") ?? null))
      .catch(() => {});
    api
      .call("view.set_particles", { inertia: { diameter_m: d, density_kg_m3: rho } })
      .then(() => setHeavy(true))
      .catch(fail);
  };

  const poll = useCallback(
    (id: string) => {
      api
        .call<ImpactStatus>("results.impact_status", { impact_id: id })
        .then((s) => {
          setImpact(s);
          if (s.state === "running") timer.current = setTimeout(() => poll(id), pollMs);
        })
        .catch(() => {});
    },
    [api, pollMs],
  );

  const compute = () => {
    setError(null);
    setImpact({ state: "running", done: 0, total: 1 });
    api
      .call<{ impact_id: string; status?: ImpactStatus }>("results.impact", {})
      .then((r) => (r.status ? setImpact(r.status) : poll(r.impact_id)))
      .catch((e: { en?: LocalizedMessage; de?: LocalizedMessage }) => {
        setImpact(null);
        fail(e);
      });
  };

  const fmt = (n: number, digits: number) =>
    n.toLocaleString(language, { maximumFractionDigits: digits, minimumFractionDigits: digits });

  return (
    <section className="heavy-section">
      <h3>{t("heavy.title")}</h3>
      <div className="heavy-cards">
        {materials
          .filter((m) => m.id !== "custom")
          .map((m) => (
            <button
              key={m.id}
              type="button"
              aria-pressed={chosen?.id === m.id}
              onClick={() => choose(m)}
            >
              {t(`heavy.materials.${m.id}`)}
            </button>
          ))}
        <button
          type="button"
          aria-pressed={!heavy}
          onClick={() => {
            setChosen(null);
            setHeavy(false);
            api.call("view.set_particles", { inertia: null }).catch(() => {});
          }}
        >
          {t("heavy.massless")}
        </button>
      </div>
      <div className="heavy-custom">
        <label>
          <span>{t("heavy.diameter")}</span>
          <input
            aria-label={t("heavy.diameter")}
            value={diameter}
            inputMode="decimal"
            onChange={(e) => setDiameter(e.target.value)}
          />
        </label>
        <label>
          <span>{t("heavy.density")}</span>
          <input
            aria-label={t("heavy.density")}
            value={density}
            inputMode="decimal"
            onChange={(e) => setDensity(e.target.value)}
          />
        </label>
        <button type="button" onClick={custom}>
          {t("heavy.ownValues")}
        </button>
      </div>
      {chosen?.stokes != null && (
        <p className="small">
          <strong>
            {t("heavy.stokes", { st: fmt(chosen.stokes, chosen.stokes < 1 ? 3 : 1) })}
          </strong>{" "}
          {text(chosen.sentence)}
        </p>
      )}
      {chosen?.rough && <p className="muted small">{text("particles.rough")}</p>}
      {heavy && (
        <div className="heavy-impact">
          <button type="button" disabled={impact?.state === "running"} onClick={compute}>
            {t("heavy.computeMap")}
          </button>
          {impact?.state === "running" && (
            <progress
              max={impact.total || 1}
              value={impact.done}
              aria-label={t("heavy.progress")}
            />
          )}
          {impact?.state === "done" && impact.map && (
            <>
              <p className="small">
                {t("heavy.share", {
                  share: fmt(impact.map.share_percent, 1),
                  unc: fmt(impact.map.uncertainty_percent, 1),
                })}
              </p>
              <p className="muted small">{text("impact.estimate")}</p>
              <button
                type="button"
                className="quiet"
                onClick={() =>
                  api.call("view.set_surface_field", { field: "impact" }).catch(() => {})
                }
              >
                {t("heavy.showMap")}
              </button>
            </>
          )}
        </div>
      )}
      {error && (
        <p className="error" role="alert">
          {error}
        </p>
      )}
    </section>
  );
}
