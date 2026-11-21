// SPDX-License-Identifier: GPL-3.0-or-later
// The mesh of the scenario: its statistics and a button to build it. On its own the button runs
// the blocking `mesh.generate`; inside the "Run" step the run monitor builds the mesh in the
// background (`onGenerate`).
import { useEffect, useState } from "react";
import { errorMessage, type Api, type LocalizedMessage } from "../api";
import { useTranslation } from "../i18n/i18n";

type Verdict = "good" | "acceptable" | "poor";

interface MeshResult {
  cells: number;
  quality: { verdict?: Verdict };
  layers: { target: number; achieved: number } | null;
  fallbacks: { code: string; en: LocalizedMessage; de: LocalizedMessage }[];
}

interface MeshRecord {
  cells: number;
  verdict?: Verdict;
  layers_target?: number;
  layers_achieved?: number;
}

interface Props {
  api: Api;
  /** Changes when a mesh was built elsewhere, so the statistics reload. */
  revision?: number;
  /** Builds the mesh instead of the blocking command (the caller shows the progress). */
  onGenerate?: () => void;
  /** A run is going on: the button is disabled. */
  busy?: boolean;
  generateLabel?: string;
  heading?: boolean;
  showCells?: boolean;
}

export function MeshSection({
  api,
  revision = 0,
  onGenerate,
  busy: busyOutside = false,
  generateLabel,
  heading = true,
  showCells = true,
}: Props) {
  const { t, language } = useTranslation();
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [result, setResult] = useState<MeshResult | null>(null);

  useEffect(() => {
    api
      .call<MeshRecord>("mesh.info")
      .then((r) =>
        setResult({
          cells: r.cells,
          quality: { verdict: r.verdict },
          layers:
            (r.layers_target ?? 0) > 0
              ? { target: r.layers_target ?? 0, achieved: r.layers_achieved ?? 0 }
              : null,
          fallbacks: [],
        }),
      )
      .catch(() => setResult(null));
  }, [api, revision]);

  const generate = () => {
    if (onGenerate) {
      onGenerate();
      return;
    }
    setBusy(true);
    setError(null);
    api
      .call<MeshResult>("mesh.generate", {})
      .then(setResult)
      .catch((e: unknown) => setError(errorMessage(e, language)))
      .finally(() => setBusy(false));
  };

  return (
    <section className="mesh-section">
      {heading && <h3>{t("mesh.title")}</h3>}
      <button type="button" disabled={busy || busyOutside} onClick={generate}>
        {busy ? t("mesh.running") : (generateLabel ?? t("mesh.generate"))}
      </button>
      {error && (
        <p role="alert" className="error">
          {error}
        </p>
      )}
      {result && (
        <dl className="scenario-result">
          {showCells && (
            <>
              <dt>{t("mesh.cells")}</dt>
              <dd>{t("scenario.cells", { n: result.cells.toLocaleString(language) })}</dd>
            </>
          )}
          {result.quality.verdict && (
            <>
              <dt>{t("mesh.quality")}</dt>
              <dd>{t(`mesh.verdict.${result.quality.verdict}`)}</dd>
            </>
          )}
          <dt>{t("mesh.layers")}</dt>
          <dd>
            {result.layers
              ? t("mesh.layersOf", {
                  achieved: result.layers.achieved.toLocaleString(language, {
                    maximumFractionDigits: 1,
                  }),
                  target: String(result.layers.target),
                })
              : t("mesh.noLayers")}
          </dd>
        </dl>
      )}
      {result && result.fallbacks.length > 0 && (
        <ul className="diagnosis">
          {result.fallbacks.map((f) => (
            <li key={f.code} className="warning">
              {f[language].text}
            </li>
          ))}
        </ul>
      )}
    </section>
  );
}
