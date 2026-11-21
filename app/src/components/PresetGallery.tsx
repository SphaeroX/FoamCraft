// SPDX-License-Identifier: GPL-3.0-or-later
// Ready-made views of a result with one click, and a demo that shows them one after another with
// a camera move and one sentence of explanation each.
import { useCallback, useEffect, useRef, useState } from "react";
import type { Api, LocalizedMessage } from "../api";
import { DemoPlayer, type DemoState } from "../demo";
import { useTranslation } from "../i18n/i18n";

interface Preset {
  id: string;
  /** Standard view for the camera. */
  view: string;
  explanation: string;
}

type Texts = Record<string, { en: LocalizedMessage; de: LocalizedMessage }>;

interface Props {
  api: Api;
  /** Turns the camera to a standard view. */
  onView: (name: string) => void;
}

/** Seconds each view is shown in the demo. */
const DEMO_STEP_MS = 8000;
/** The ride at the end of the demo (Task_32). */
const DEMO_RIDE_MS = 12000;
const RIDE_STEP = "ride";

/** A small picture per preset (drawn, no external images). */
function PresetIcon({ id }: { id: string }) {
  const body = <rect x="13" y="10" width="14" height="10" rx="3" className="preset-icon-body" />;
  let extra: React.ReactNode;
  switch (id) {
    case "smoke_lines":
    case "internal_path":
      extra = [8, 15, 22].map((y) => (
        <path key={y} d={`M2 ${y} C12 ${y} 14 ${y - 6} 38 ${y - 2}`} className="preset-icon-line" />
      ));
      break;
    case "smoke_curtain":
      extra = <rect x="2" y="4" width="36" height="22" className="preset-icon-veil" />;
      break;
    case "mid_slice":
    case "internal_slice":
      extra = <rect x="2" y="13" width="36" height="4" className="preset-icon-slice" />;
      break;
    case "wake":
      extra = <rect x="31" y="4" width="4" height="22" className="preset-icon-slice" />;
      break;
    case "oil_film":
      extra = [12, 15, 18].map((y) => (
        <path key={y} d={`M14 ${y} L26 ${y + 1}`} className="preset-icon-line" />
      ));
      break;
    case "tufts":
      extra = [16, 21, 26].map((x) => (
        <path key={x} d={`M${x} 12 l4 1`} className="preset-icon-tuft" />
      ));
      break;
    case "vortices":
      extra = <circle cx="32" cy="15" r="5" className="preset-icon-line" />;
      break;
    case "light_trails":
      extra = [6, 11, 19, 24].map((y) => (
        <path
          key={y}
          d={`M2 ${y} C14 ${y} 16 ${y + (y < 15 ? -3 : 3)} 38 ${y}`}
          className="preset-icon-glow"
        />
      ));
      break;
    case "ribbons":
      // A band that turns over behind the body.
      extra = (
        <path
          d="M2 13 L14 13 L14 17 L2 17 Z M27 12 C31 12 31 18 35 18 L38 18 L38 14 C34 14 33 20 27 18 Z"
          className="preset-icon-ribbon"
        />
      );
      break;
    default:
      extra = <rect x="13" y="10" width="5" height="10" rx="2" className="preset-icon-hot" />;
  }
  return (
    <svg width="40" height="30" viewBox="0 0 40 30" aria-hidden="true">
      {/* Lines and tufts lie on the body, everything else behind it. */}
      {id === "oil_film" || id === "tufts" ? (
        <>
          {body}
          {extra}
        </>
      ) : (
        <>
          {extra}
          {body}
        </>
      )}
    </svg>
  );
}

export function PresetGallery({ api, onView }: Props) {
  const { t, language } = useTranslation();
  const [presets, setPresets] = useState<Preset[]>([]);
  const [texts, setTexts] = useState<Texts>({});
  const [share, setShare] = useState<number | null>(null);
  const [current, setCurrent] = useState<string | null>(null);
  const [demo, setDemo] = useState<DemoState | null>(null);
  const player = useRef<DemoPlayer<Preset> | null>(null);
  const appliedInDemo = useRef(0);
  // The suggested ride the demo ends with (none without suggestions).
  const [rideSuggestion, setRideSuggestion] = useState<string | null>(null);
  const riding = useRef(false);

  useEffect(() => {
    api
      .call<Preset[]>("view.presets", {})
      .then((list) => {
        // Other answers (no result yet, a test double) show no gallery.
        const valid = Array.isArray(list) ? list : [];
        setPresets(valid);
        if (valid.length === 0) return {};
        return api.call<Texts>("error.describe", { codes: valid.map((p) => p.explanation) });
      })
      .then((found) => setTexts(found ?? {}))
      .catch(() => {});
    api
      .call<{ id: string }[]>("ride.suggest", {})
      .then((list) => {
        const ids = Array.isArray(list) ? list.map((s) => s.id) : [];
        setRideSuggestion(
          ids.find((id) => id === "stagnation") ?? ids.find((id) => id === "hugging") ?? null,
        );
      })
      .catch(() => {});
    api
      .call<{ figures?: { id: string; value: number | null }[] }>("results.summary", {})
      .then((s) => {
        const of = (id: string) => s.figures?.find((f) => f.id === id)?.value ?? null;
        const p = of("drag_pressure_n");
        const v = of("drag_viscous_n");
        if (p !== null && v !== null && p + v > 0) setShare(Math.round((100 * p) / (p + v)));
      })
      .catch(() => {});
  }, [api]);

  const apply = useCallback(
    (p: Preset) => {
      setCurrent(p.id);
      onView(p.view);
      return api.call("view.apply_preset", { id: p.id }).catch(() => {});
    },
    [api, onView],
  );

  // Ends a running demo when the panel goes away.
  useEffect(() => () => player.current?.stop(), []);

  const endRide = () => {
    if (!riding.current) return;
    riding.current = false;
    api.call("ride.stop", {}).catch(() => {});
  };

  const startDemo = () => {
    appliedInDemo.current = 0;
    const steps: Preset[] = rideSuggestion
      ? [...presets, { id: RIDE_STEP, view: "default", explanation: "" }]
      : presets;
    const p = new DemoPlayer<Preset>(
      steps,
      (step) => {
        endRide();
        if (step.id === RIDE_STEP) {
          riding.current = true;
          setCurrent(null);
          api.call("ride.start", { suggestion: rideSuggestion }).catch(() => {});
          return;
        }
        appliedInDemo.current += 1;
        void apply(step);
      },
      {
        stepMs: DEMO_STEP_MS,
        durationOf: (step) => (step.id === RIDE_STEP ? DEMO_RIDE_MS : DEMO_STEP_MS),
        onChange: setDemo,
        onEnd: () => {
          endRide();
          // Back to the scene from before the demo: one undo per shown view.
          const n = appliedInDemo.current;
          appliedInDemo.current = 0;
          let chain: Promise<unknown> = Promise.resolve();
          for (let i = 0; i < n; i++)
            chain = chain.then(() => api.call("history.undo", {}).catch(() => {}));
          setCurrent(null);
          setDemo(null);
          player.current = null;
        },
      },
    );
    player.current = p;
    p.start();
  };

  const explanation = (id: string) => {
    const text = texts[`preset.${id}`]?.[language]?.text ?? "";
    return text.replace("{share}", share === null ? "–" : String(share));
  };

  if (presets.length === 0) return null;
  return (
    <section className="preset-gallery">
      <h3>{t("presets.title")}</h3>
      <div className="preset-grid">
        {presets.map((p) => (
          <button
            key={p.id}
            type="button"
            className={current === p.id ? "active" : ""}
            aria-pressed={current === p.id}
            onClick={() => void apply(p)}
          >
            <PresetIcon id={p.id} />
            <span>{t(`presets.${p.id}`)}</span>
          </button>
        ))}
      </div>
      {current && <p className="preset-explanation">{explanation(current)}</p>}
      {demo ? (
        <div className="demo-controls" role="group" aria-label={t("presets.demo")}>
          <button type="button" onClick={() => player.current?.prev()}>
            {t("presets.back")}
          </button>
          {demo.playing ? (
            <button type="button" onClick={() => player.current?.pause()}>
              {t("presets.pause")}
            </button>
          ) : (
            <button type="button" onClick={() => player.current?.resume()}>
              {t("presets.play")}
            </button>
          )}
          <button type="button" onClick={() => player.current?.next()}>
            {t("presets.forward")}
          </button>
          <button type="button" onClick={() => player.current?.stop()}>
            {t("presets.end")}
          </button>
          <span className="muted small">
            {t("presets.step", {
              n: String(demo.index + 1),
              of: String(presets.length + (rideSuggestion ? 1 : 0)),
            })}
          </span>
        </div>
      ) : (
        <button type="button" onClick={startDemo}>
          {t("presets.startDemo")}
        </button>
      )}
    </section>
  );
}
