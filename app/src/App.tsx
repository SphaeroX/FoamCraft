// SPDX-License-Identifier: GPL-3.0-or-later
import { open, save } from "@tauri-apps/plugin-dialog";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { errorMessage, onApiEvent, subscribeApiEvents, tauriApi, type ProjectInfo } from "./api";
import { AboutDialog } from "./components/AboutDialog";
import { ConsoleDock } from "./components/ConsoleDock";
import { ExportDialog } from "./components/ExportDialog";
import { GuidedTour } from "./components/GuidedTour";
import { McpDialog } from "./components/McpDialog";
import { ReportDialog } from "./components/ReportDialog";
import { SetupAssistant } from "./components/SetupAssistant";
import { LanguageSwitcher } from "./components/LanguageSwitcher";
import { GeometryPanel } from "./components/GeometryPanel";
import { OpeningsSection } from "./components/OpeningsSection";
import { KeyFiguresPanel } from "./components/KeyFiguresPanel";
import { ResultsPanel, type ProbeValue } from "./components/ResultsPanel";
import { RunPanel } from "./components/RunPanel";
import { ScenarioPanel } from "./components/ScenarioPanel";
import { StartPage } from "./components/StartPage";
import { SurfacePanel } from "./components/SurfacePanel";
import { ViewportInput, type PickMode } from "./components/ViewportInput";
import { useTranslation } from "./i18n/i18n";
import {
  cliPath,
  hoverAt,
  pickAt,
  pickPointAt,
  sendCamera,
  setViewInsets,
  showSelection,
  syncViewport,
} from "./native";
import { RunMonitor } from "./run-monitor";
import { measurePanels, viewInsets } from "./view-insets";
import type { CameraAction } from "./viewport-input";

export function App() {
  const { t, language } = useTranslation();
  const languageRef = useRef(language);
  languageRef.current = language;
  const [aboutOpen, setAboutOpen] = useState(false);
  const [mcpOpen, setMcpOpen] = useState(false);
  const [exportOpen, setExportOpen] = useState(false);
  const [reportOpen, setReportOpen] = useState(false);
  const [setupOpen, setSetupOpen] = useState(false);
  const [tourOpen, setTourOpen] = useState(false);

  // First start: the setup assistant, then the tour.
  useEffect(() => {
    tauriApi
      .call<{ done: boolean }>("system.setup_state", {})
      .then((s) => {
        if (!s.done) setSetupOpen(true);
      })
      .catch(() => {});
  }, []);
  const [project, setProject] = useState<ProjectInfo | null>(null);
  const [selection, setSelection] = useState<string[]>([]);
  const [revision, setRevision] = useState(0);
  const [step, setStep] = useState<"geometry" | "scenario" | "run" | "results">("geometry");
  const selectionRef = useRef<string[]>([]);
  const [capMode, setCapMode] = useState(false);
  const [probe, setProbe] = useState<ProbeValue | null>(null);
  const stepRef = useRef(step);
  stepRef.current = step;
  const probeBusy = useRef(false);
  const capModeRef = useRef(false);
  capModeRef.current = capMode;
  const hoverBusy = useRef(false);

  // One run monitor per open project: it keeps following the run while another step is shown.
  const projectPath = project?.path ?? null;
  const monitor = useMemo(
    () => (projectPath ? new RunMonitor(tauriApi, subscribeApiEvents) : null),
    [projectPath],
  );
  useEffect(() => {
    if (!monitor) return;
    monitor.connect();
    return () => monitor.dispose();
  }, [monitor]);

  const onPick = useCallback((x: number, y: number, mode: PickMode) => {
    if (capModeRef.current) {
      // Close the opening next to the clicked point.
      setCapMode(false);
      pickPointAt(x, y)
        .then((hit) => {
          if (!hit?.point) return;
          const model = hit.key.split(":")[0];
          return tauriApi.call("geometry.cap_opening", {
            model,
            point: hit.point,
            ...(hit.key.includes(":face-") ? { face: hit.key } : {}),
          });
        })
        .catch((err: unknown) => window.alert(errorMessage(err, languageRef.current)));
      return;
    }
    pickAt(x, y)
      .then((key) => {
        if (key) return tauriApi.call("surface.select", { keys: [key], mode });
        if (mode === "replace") return tauriApi.call("surface.clear_selection");
      })
      .catch((err: unknown) => console.error(err));
  }, []);

  const onHover = useCallback((x: number, y: number) => {
    if (stepRef.current === "results") {
      // Value of the shown field under the mouse; one request at a time.
      if (probeBusy.current) return;
      probeBusy.current = true;
      pickPointAt(x, y)
        .then((hit) =>
          hit?.point
            ? tauriApi.call<ProbeValue>("results.probe", { point: hit.point }).then(setProbe)
            : setProbe(null),
        )
        .catch(() => setProbe(null))
        .finally(() => (probeBusy.current = false));
      return;
    }
    // One hover pick at a time; extra mouse moves meanwhile are dropped.
    if (hoverBusy.current) return;
    hoverBusy.current = true;
    hoverAt(x, y)
      .catch((err: unknown) => console.error(err))
      .finally(() => (hoverBusy.current = false));
  }, []);
  const send = useCallback((action: CameraAction) => {
    sendCamera(action).catch((err: unknown) => console.error(err));
  }, []);

  const pickModel = useCallback(async () => {
    const file = await open({
      title: t("geometry.importTitle"),
      filters: [
        {
          name: t("geometry.models"),
          extensions: ["stl", "obj", "3mf", "step", "stp", "iges", "igs"],
        },
      ],
    });
    return typeof file === "string" ? file : null;
  }, [t]);

  const pickSavePath = useCallback(
    async (extension: "csv" | "json") =>
      (await save({ filters: [{ name: extension.toUpperCase(), extensions: [extension] }] })) ??
      null,
    [],
  );

  const pickFolder = useCallback(async () => {
    const folder = await open({ directory: true, title: t("start.chooseFolder") });
    return typeof folder === "string" ? folder : null;
  }, [t]);

  useEffect(() => {
    document.documentElement.lang = language;
  }, [language]);

  // The split view of two runs uses the part of the window the side panels leave free.
  useEffect(() => {
    const update = () => {
      const { left, right } = viewInsets(measurePanels(), window.innerWidth);
      setViewInsets(left, right).catch((err: unknown) => console.error(err));
    };
    update();
    window.addEventListener("resize", update);
    return () => window.removeEventListener("resize", update);
  }, [step, project]);

  // Keep the shown project in sync with changes from any client of the command API.
  useEffect(() => {
    let stop: (() => void) | undefined;
    onApiEvent((event) => {
      if (event.name === "selection.changed") {
        const keys = Array.isArray(event.payload) ? (event.payload as string[]) : [];
        selectionRef.current = keys;
        setSelection(keys);
        showSelection(keys).catch((err: unknown) => console.error(err));
        return;
      }
      if (event.name !== "project.changed") return;
      setRevision((r) => r + 1);
      syncViewport()
        .then(() => showSelection(selectionRef.current))
        .catch((err: unknown) => console.error(err));
      tauriApi
        .call<ProjectInfo>("project.info")
        .then(setProject)
        .catch(() => setProject(null));
    })
      .then((unlisten) => (stop = unlisten))
      .catch((err: unknown) => console.error(err));
    return () => stop?.();
  }, []);

  return (
    <>
      <ViewportInput
        send={send}
        onPick={project ? onPick : undefined}
        onHover={project ? onHover : undefined}
      />
      <header className="toolbar panel">
        <span className="brand">{t("app.title")}</span>
        {project && <span className="project-name">{project.name}</span>}
        {project && (
          <nav className="steps" aria-label="steps">
            {(["geometry", "scenario", "run", "results"] as const).map((s) => (
              <button
                key={s}
                type="button"
                className={step === s ? "active" : ""}
                aria-pressed={step === s}
                onClick={() => {
                  // Leaving the results shows the plain models again.
                  if (step === "results" && s !== "results")
                    tauriApi.call("view.set_surface_field", { field: null }).catch(() => {});
                  setStep(s);
                }}
              >
                {t(`steps.${s}`)}
              </button>
            ))}
          </nav>
        )}
        <span className="spacer" />
        {project && (
          <button type="button" onClick={() => tauriApi.call("project.close").catch(() => {})}>
            {t("project.close")}
          </button>
        )}
        <button type="button" onClick={() => send({ kind: "fit" })}>
          {t("toolbar.fit")}
        </button>
        <LanguageSwitcher />
        {project && (
          <button type="button" onClick={() => setExportOpen(true)}>
            {t("export.button")}
          </button>
        )}
        {project && (
          <button type="button" onClick={() => setReportOpen(true)}>
            {t("report.button")}
          </button>
        )}
        <button type="button" onClick={() => setTourOpen(true)}>
          {t("setup.tour")}
        </button>
        <button type="button" onClick={() => setSetupOpen(true)}>
          {t("setup.button")}
        </button>
        <button type="button" onClick={() => setMcpOpen(true)}>
          {t("mcp.button")}
        </button>
        <button type="button" onClick={() => setAboutOpen(true)}>
          {t("toolbar.about")}
        </button>
      </header>
      {project ? (
        <>
          {step === "geometry" ? (
            <>
              <GeometryPanel
                key={project.path}
                api={tauriApi}
                pickModel={pickModel}
                units={project.display_units}
              />
              <SurfacePanel api={tauriApi} selection={selection} revision={revision}>
                <OpeningsSection
                  api={tauriApi}
                  revision={revision}
                  capMode={capMode}
                  onCapMode={setCapMode}
                />
              </SurfacePanel>
            </>
          ) : step === "results" ? (
            <>
              <ResultsPanel key={project.path} api={tauriApi} probe={probe} />
              <KeyFiguresPanel
                key={`figures-${project.path}`}
                api={tauriApi}
                revision={revision}
                pickSavePath={pickSavePath}
                confirm={(message) => window.confirm(message)}
              />
            </>
          ) : step === "scenario" ? (
            <ScenarioPanel key={project.path} api={tauriApi} units={project.display_units} />
          ) : (
            <RunPanel
              key={project.path}
              api={tauriApi}
              subscribe={subscribeApiEvents}
              monitor={monitor ?? undefined}
            />
          )}
          {monitor && (
            <ConsoleDock key={`console-${project.path}`} api={tauriApi} monitor={monitor} />
          )}
          <footer className="hint panel">{t("viewport.hint")}</footer>
        </>
      ) : (
        <StartPage api={tauriApi} pickFolder={pickFolder} onOpened={setProject} />
      )}
      {aboutOpen && <AboutDialog onClose={() => setAboutOpen(false)} />}
      {exportOpen && project && (
        <ExportDialog
          api={tauriApi}
          pickSavePath={async (extension) =>
            (await save({
              filters: [{ name: extension.toUpperCase(), extensions: [extension] }],
            })) ?? null
          }
          onClose={() => setExportOpen(false)}
        />
      )}
      {reportOpen && project && (
        <ReportDialog
          api={tauriApi}
          projectName={project.name}
          pickSavePath={async () =>
            (await save({ filters: [{ name: "PDF / HTML", extensions: ["pdf", "html"] }] })) ?? null
          }
          pickLogo={async () => {
            const file = await open({ filters: [{ name: "PNG", extensions: ["png"] }] });
            return typeof file === "string" ? file : null;
          }}
          onClose={() => setReportOpen(false)}
        />
      )}
      {setupOpen && (
        <SetupAssistant
          api={tauriApi}
          copy={(text) => navigator.clipboard.writeText(text)}
          subscribe={subscribeApiEvents}
          onClose={() => {
            setSetupOpen(false);
            try {
              if (!localStorage.getItem("foamcraft.tourSeen")) {
                localStorage.setItem("foamcraft.tourSeen", "1");
                setTourOpen(true);
              }
            } catch {
              // Without storage the tour is still available from the toolbar.
            }
          }}
        />
      )}
      {tourOpen && <GuidedTour onClose={() => setTourOpen(false)} />}
      {mcpOpen && (
        <McpDialog
          cliPath={cliPath}
          defaultRoot={project ? project.path.replace(/[\\/][^\\/]*$/, "") : ""}
          copy={(text) => navigator.clipboard.writeText(text)}
          onClose={() => setMcpOpen(false)}
        />
      )}
    </>
  );
}
