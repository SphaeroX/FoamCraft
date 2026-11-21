// SPDX-License-Identifier: GPL-3.0-or-later
// Export of images and videos: settings, a preview of the timeline, progress and cancel.
// All work happens in the commands export.image, export.video, export.preview.
import { useEffect, useRef, useState } from "react";
import { errorMessage, type Api } from "../api";
import { useTranslation } from "../i18n/i18n";

interface Status {
  export_id: string;
  state: "running" | "done" | "failed" | "cancelled";
  frame: number;
  frames: number;
  path: string;
}

interface Props {
  api: Api;
  pickSavePath: (extension: "png" | "avi") => Promise<string | null>;
  onClose: () => void;
  /** Interval of the progress queries. */
  pollMs?: number;
}

const RESOLUTIONS = ["hd", "full_hd", "qhd", "4k"] as const;
const CAMERAS = ["turntable", "swing", "rise", "approach", "still"] as const;
const RATES = [24, 30, 60] as const;

export function ExportDialog({ api, pickSavePath, onClose, pollMs = 500 }: Props) {
  const { t, language } = useTranslation();
  const [resolution, setResolution] = useState<string>("full_hd");
  const [transparent, setTransparent] = useState(false);
  const [legend, setLegend] = useState(true);
  const [camera, setCamera] = useState<string>("turntable");
  const [fps, setFps] = useState(30);
  const [duration, setDuration] = useState(8);
  const [time, setTime] = useState(0);
  const [preview, setPreview] = useState<string | null>(null);
  const [status, setStatus] = useState<Status | null>(null);
  const [message, setMessage] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const timer = useRef<ReturnType<typeof setInterval> | null>(null);

  useEffect(() => {
    api
      .call<{ png_base64: string }>("export.preview", {
        width: 480,
        height: 270,
        time_s: time,
        duration_s: duration,
        camera: { preset: camera },
        legend,
      })
      .then((p) => setPreview(p.png_base64))
      .catch((e: unknown) => setError(errorMessage(e, language)));
  }, [api, time, duration, camera, legend, language]);

  useEffect(
    () => () => {
      if (timer.current) clearInterval(timer.current);
    },
    [],
  );

  const follow = (id: string) => {
    if (timer.current) clearInterval(timer.current);
    timer.current = setInterval(() => {
      api
        .call<Status>("export.status", { export_id: id })
        .then((s) => {
          setStatus(s);
          if (s.state !== "running") {
            if (timer.current) clearInterval(timer.current);
            timer.current = null;
            if (s.state === "done") setMessage(t("export.saved", { path: s.path }));
            if (s.state === "cancelled") setMessage(t("export.cancelled"));
            if (s.state === "failed") setError(t("export.failed"));
          }
        })
        .catch((e: unknown) => setError(errorMessage(e, language)));
    }, pollMs);
  };

  const saveImage = async () => {
    const path = await pickSavePath("png");
    if (!path) return;
    setMessage(null);
    api
      .call<{ path: string }>("export.image", { path, resolution, transparent, legend })
      .then((r) => setMessage(t("export.saved", { path: r.path })))
      .catch((e: unknown) => setError(errorMessage(e, language)));
  };

  const saveVideo = async () => {
    const path = await pickSavePath("avi");
    if (!path) return;
    setMessage(null);
    api
      .call<{ export_id: string; path: string; frames: number }>("export.video", {
        path,
        resolution,
        fps,
        duration_s: duration,
        camera: { preset: camera },
        legend,
      })
      .then((r) => {
        setStatus({
          export_id: r.export_id,
          state: "running",
          frame: 0,
          frames: r.frames,
          path: r.path,
        });
        follow(r.export_id);
      })
      .catch((e: unknown) => setError(errorMessage(e, language)));
  };

  const running = status?.state === "running";
  return (
    <div className="dialog-backdrop" role="presentation" onClick={onClose}>
      <div
        className="dialog panel export-dialog"
        role="dialog"
        aria-modal="true"
        aria-labelledby="export-title"
        onClick={(e) => e.stopPropagation()}
      >
        <h2 id="export-title">{t("export.title")}</h2>
        {preview && (
          <img
            className="export-preview"
            alt={t("export.preview")}
            src={`data:image/png;base64,${preview}`}
          />
        )}
        <label className="unit-field">
          <span>{t("export.time")}</span>
          <input
            type="range"
            aria-label={t("export.time")}
            min={0}
            max={duration}
            step={0.1}
            value={time}
            onChange={(e) => setTime(Number(e.target.value))}
          />
        </label>
        <div className="export-grid">
          <label className="unit-field">
            <span>{t("export.resolution")}</span>
            <select
              aria-label={t("export.resolution")}
              value={resolution}
              onChange={(e) => setResolution(e.target.value)}
            >
              {RESOLUTIONS.map((r) => (
                <option key={r} value={r}>
                  {t(`export.res.${r}`)}
                </option>
              ))}
            </select>
          </label>
          <label className="unit-field">
            <span>{t("export.camera")}</span>
            <select
              aria-label={t("export.camera")}
              value={camera}
              onChange={(e) => setCamera(e.target.value)}
            >
              {CAMERAS.map((c) => (
                <option key={c} value={c}>
                  {t(`export.cam.${c}`)}
                </option>
              ))}
            </select>
          </label>
          <label className="unit-field">
            <span>{t("export.fps")}</span>
            <select
              aria-label={t("export.fps")}
              value={fps}
              onChange={(e) => setFps(Number(e.target.value))}
            >
              {RATES.map((r) => (
                <option key={r} value={r}>
                  {r}
                </option>
              ))}
            </select>
          </label>
          <label className="unit-field">
            <span>{t("export.duration")}</span>
            <input
              type="number"
              aria-label={t("export.duration")}
              min={1}
              max={600}
              value={duration}
              onChange={(e) => {
                const d = Math.max(1, Number(e.target.value) || 1);
                setDuration(d);
                setTime((x) => Math.min(x, d));
              }}
            />
          </label>
        </div>
        <label className="check">
          <input type="checkbox" checked={legend} onChange={(e) => setLegend(e.target.checked)} />
          {t("export.legend")}
        </label>
        <label className="check">
          <input
            type="checkbox"
            checked={transparent}
            onChange={(e) => setTransparent(e.target.checked)}
          />
          {t("export.transparent")}
        </label>
        {status && running && (
          <div className="export-progress">
            <progress max={status.frames} value={status.frame} />
            <span className="small">
              {status.frame} / {status.frames}
            </span>
            <button
              type="button"
              onClick={() =>
                api
                  .call("export.cancel", { export_id: status.export_id })
                  .catch((e: unknown) => setError(errorMessage(e, language)))
              }
            >
              {t("export.cancel")}
            </button>
          </div>
        )}
        {message && <p className="small">{message}</p>}
        {error && (
          <p role="alert" className="error">
            {error}
          </p>
        )}
        <p className="small muted">{t("export.note")}</p>
        <div className="dialog-actions">
          <button type="button" disabled={running} onClick={() => void saveImage()}>
            {t("export.saveImage")}
          </button>
          <button type="button" disabled={running} onClick={() => void saveVideo()}>
            {t("export.saveVideo")}
          </button>
          <button type="button" onClick={onClose}>
            {t("about.close")}
          </button>
        </div>
      </div>
    </div>
  );
}
