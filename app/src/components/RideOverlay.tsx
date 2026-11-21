// SPDX-License-Identifier: GPL-3.0-or-later
// The ride on screen: readings at the top left (HUD) with two small curves, the explaining
// sentence at an event, and the flight bar at the bottom (play, timeline with events, speed,
// calm or roller coaster, view, guide line, end). Keys: space, arrows, 1/2/3, Esc.
import { useCallback, useEffect, useRef, useState } from "react";
import type { Api, LocalizedMessage } from "../api";
import { useTranslation } from "../i18n/i18n";
import {
  curve,
  eventAt,
  readingsAt,
  type RideAction,
  type RideClock,
  type RideEvent,
  type RideInfo,
} from "../ride";
import {
  lengthUnit,
  speedUnit,
  toDisplayLength,
  toDisplaySpeed,
  type DisplayUnits,
} from "../units";

type Texts = Record<string, { en: LocalizedMessage; de: LocalizedMessage }>;

interface Props {
  api: Api;
  /** Changes with every project change: the ride is read again. */
  revision: number;
  units: DisplayUnits;
  /** Playback in the window. */
  control: (action: RideAction) => Promise<RideClock | null>;
  /** Subscribes to the clock of the window; returns the unsubscribe. */
  onClock: (handler: (clock: RideClock) => void) => () => void;
}

const VIEWS = ["first_person", "chase", "overview"] as const;
const COASTER_NOTE_KEY = "foamcraft.ride.coasterNote";

function Spark({ info, column, t }: { info: RideInfo; column: number; t: number }) {
  const pts = curve(info, column);
  const d = info.duration_s || 1;
  const path = pts
    .map((p, i) => `${i ? "L" : "M"}${((p.t / d) * 100).toFixed(2)} ${(18 - p.v * 16).toFixed(2)}`)
    .join(" ");
  const x = (Math.min(Math.max(t, 0), d) / d) * 100;
  return (
    <svg className="ride-spark" viewBox="0 0 100 20" preserveAspectRatio="none" aria-hidden="true">
      <path d={path} />
      <line x1={x} x2={x} y1="0" y2="20" />
    </svg>
  );
}

export function RideOverlay({ api, revision, units, control, onClock }: Props) {
  const { t, language } = useTranslation();
  const [info, setInfo] = useState<RideInfo | null>(null);
  const [clock, setClock] = useState<RideClock>({ t: 0, duration: 0, playing: false });
  const [texts, setTexts] = useState<Texts>({});
  const [note, setNote] = useState(false);
  const infoRef = useRef<RideInfo | null>(null);
  const clockRef = useRef(clock);
  clockRef.current = clock;
  infoRef.current = info;

  useEffect(() => {
    let alive = true;
    api
      .call<RideInfo>("ride.telemetry", {})
      .then((found) => {
        if (!alive) return;
        const valid = found && Array.isArray(found.rows) ? found : null;
        setInfo(valid);
        if (valid) {
          setClock((c) => ({ ...c, duration: valid.duration_s }));
          const codes = valid.events.map((e) => e.code);
          if (codes.length > 0)
            api
              .call<Texts>("error.describe", { codes })
              .then((x) => alive && setTexts(x ?? {}))
              .catch(() => {});
        }
      })
      .catch(() => alive && setInfo(null));
    return () => {
      alive = false;
    };
  }, [api, revision]);

  useEffect(() => onClock(setClock), [onClock]);
  // The clock as it is now (the ride may have played while the overlay was hidden).
  useEffect(() => {
    if (!info) return;
    control({ kind: "state" })
      .then((c) => c && setClock(c))
      .catch(() => {});
  }, [info, control]);

  const change = useCallback(
    (params: Record<string, unknown>) => {
      api.call("ride.start", params).catch(() => {});
    },
    [api],
  );
  const stop = useCallback(() => {
    api.call("ride.stop", {}).catch(() => {});
  }, [api]);
  const seek = useCallback(
    (to: number) => {
      const d = infoRef.current?.duration_s ?? 0;
      const next = Math.min(Math.max(to, 0), d);
      setClock((c) => ({ ...c, t: next }));
      void control({ kind: "seek", t: next });
    },
    [control],
  );
  const toggle = useCallback(() => {
    const playing = clockRef.current.playing;
    setClock((c) => ({ ...c, playing: !playing }));
    void control(playing ? { kind: "pause" } : { kind: "play" });
  }, [control]);
  const setMode = useCallback(
    (mode: "calm" | "coaster") => {
      if (mode === "coaster") {
        // A note on the first switch only; without storage no note at all.
        try {
          if (localStorage.getItem(COASTER_NOTE_KEY) !== "1") setNote(true);
          localStorage.setItem(COASTER_NOTE_KEY, "1");
        } catch {
          setNote(false);
        }
      }
      change({ mode });
    },
    [change],
  );

  useEffect(() => {
    if (!info) return;
    const onKey = (e: KeyboardEvent) => {
      const target = e.target as HTMLElement | null;
      if (target && ["INPUT", "SELECT", "TEXTAREA"].includes(target.tagName)) return;
      const d = infoRef.current?.duration_s ?? 0;
      switch (e.key) {
        case " ":
          e.preventDefault();
          toggle();
          break;
        case "ArrowRight":
          seek(clockRef.current.t + 0.05 * d);
          break;
        case "ArrowLeft":
          seek(clockRef.current.t - 0.05 * d);
          break;
        case "1":
        case "2":
        case "3":
          change({ view: VIEWS[Number(e.key) - 1] });
          break;
        case "Escape":
          stop();
          break;
        default:
          return;
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [info, toggle, seek, change, stop]);

  if (!info) return null;
  const r = readingsAt(info, clock.t);
  const event = info.ride.mode === "calm" ? eventAt(info, clock.t) : null;
  // The sentence of an event with its numbers filled in.
  const fill = (e: RideEvent) =>
    (texts[e.code]?.[language]?.text ?? e.id)
      .replace("{value}", e.value.toLocaleString(language, { maximumFractionDigits: 1 }))
      .replace("{extra}", (e.extra ?? 0).toLocaleString(language, { maximumFractionDigits: 0 }));
  const sentence = event && texts[event.code] ? fill(event) : undefined;
  const fmt = (n: number, digits = 2) =>
    n.toLocaleString(language, { maximumFractionDigits: digits, minimumFractionDigits: 0 });
  const share = r ? Math.round((100 * r.speed) / (info.reference_speed_m_s || 1)) : 0;
  const factor = r?.factor ?? 1;
  const duration = info.duration_s;

  return (
    <>
      {r && (
        <section className="ride-hud panel" aria-label={t("ride.readings")}>
          <dl>
            <dt>{t("ride.speed")}</dt>
            <dd>{`${fmt(toDisplaySpeed(r.speed, units), 0)} ${speedUnit(units)} · ${share} %`}</dd>
            <dt>{t("ride.pressure")}</dt>
            <dd>{`Cp ${fmt(r.cp)} · ${fmt(r.p, 0)} Pa`}</dd>
            <dt>{t("ride.wall")}</dt>
            <dd>{`${fmt(toDisplayLength(r.d, units), 1)} ${lengthUnit(units)}`}</dd>
            {r.slip > 0 && (
              <>
                <dt>{t("ride.slip")}</dt>
                <dd>{`${fmt(toDisplaySpeed(r.slip, units), 1)} ${speedUnit(units)}`}</dd>
              </>
            )}
            <dt>{t("ride.accel")}</dt>
            <dd>{`${fmt(r.accel / 9.81, 1)} g`}</dd>
            <dt>{t("ride.factor")}</dt>
            <dd>
              {factor < 1
                ? t("ride.slowMotion", {
                    f: factor.toLocaleString(language, { maximumSignificantDigits: 2 }),
                  })
                : t("ride.fastMotion", { f: fmt(factor, 1) })}
            </dd>
          </dl>
          <Spark info={info} column={2} t={clock.t} />
          <Spark info={info} column={3} t={clock.t} />
        </section>
      )}
      {sentence && (
        <p className="ride-sentence panel" role="status">
          {sentence}
        </p>
      )}
      {note && (
        <p className="ride-note panel" role="note">
          {t("ride.coasterNote")}
          <button type="button" className="quiet" onClick={() => setNote(false)}>
            {t("ride.ok")}
          </button>
        </p>
      )}
      <div className="ride-bar panel" role="toolbar" aria-label={t("ride.bar")}>
        <button type="button" onClick={toggle}>
          {clock.playing ? t("ride.pause") : t("ride.play")}
        </button>
        <div className="ride-timeline">
          <input
            type="range"
            aria-label={t("ride.position")}
            min={0}
            max={duration}
            step={0.01}
            value={Math.min(clock.t, duration)}
            onChange={(e) => seek(Number(e.target.value))}
          />
          <ul className="ride-events">
            {info.events.map((e) => (
              <li
                key={`${e.id}-${e.t}`}
                aria-label={t("ride.event", { name: fill(e) })}
                title={fill(e)}
                style={{ left: `${(100 * e.t) / (duration || 1)}%` }}
              />
            ))}
          </ul>
        </div>
        <label className="ride-speed">
          <span>{t("ride.tempo")}</span>
          <input
            type="range"
            aria-label={t("ride.tempo")}
            min={0.25}
            max={3}
            step={0.25}
            value={info.ride.speed}
            onChange={(e) => change({ speed: Number(e.target.value) })}
          />
        </label>
        <div className="segmented" role="group" aria-label={t("ride.mode")}>
          <button
            type="button"
            aria-pressed={info.ride.mode === "calm"}
            onClick={() => setMode("calm")}
          >
            {t("ride.calm")}
          </button>
          <button
            type="button"
            aria-pressed={info.ride.mode === "coaster"}
            onClick={() => setMode("coaster")}
          >
            {t("ride.coaster")}
          </button>
        </div>
        <div className="segmented" role="group" aria-label={t("ride.view")}>
          {VIEWS.map((v) => (
            <button
              key={v}
              type="button"
              aria-pressed={info.ride.view === v}
              onClick={() => change({ view: v })}
            >
              {t(`ride.views.${v}`)}
            </button>
          ))}
        </div>
        <label className="check">
          <input
            type="checkbox"
            checked={info.ride.guide}
            onChange={(e) => change({ guide: e.target.checked })}
          />
          <span>{t("ride.guide")}</span>
        </label>
        <button type="button" className="quiet" onClick={stop}>
          {t("ride.stop")}
        </button>
      </div>
    </>
  );
}
