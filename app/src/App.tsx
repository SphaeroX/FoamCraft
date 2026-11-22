// SPDX-License-Identifier: GPL-3.0-or-later
import { open, save } from "@tauri-apps/plugin-dialog";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { errorMessage, onApiEvent, subscribeApiEvents, tauriApi, type ProjectInfo } from "./api";
import { AboutDialog } from "./components/AboutDialog";
import { ConsoleDock } from "./components/ConsoleDock";
import { ExportDialog } from "./components/ExportDialog";
import { McpDialog } from "./components/McpDialog";
import { ReportDialog } from "./components/ReportDialog";
import { SetupAssistant } from "./components/SetupAssistant";
import { GeometryPanel } from "./components/GeometryPanel";
import { OpeningsSection } from "./components/OpeningsSection";
import { KeyFiguresPanel } from "./components/KeyFiguresPanel";
import { ResultsPanel, type ProbeValue } from "./components/ResultsPanel";
import { RunPanel } from "./components/RunPanel";
import { ScenarioPanel } from "./components/ScenarioPanel";
import { StartPage } from "./components/StartPage";
import { SurfacePanel } from "./components/SurfacePanel";
import { ViewportInput, type HandleInput, type PickMode } from "./components/ViewportInput";
import { MoveTools, type HandleValue } from "./components/MoveTools";
import { formatNumber } from "./units";
import { ViewCube } from "./components/ViewCube";
import { RideOverlay } from "./components/RideOverlay";
import { gentleStart } from "./components/RideSection";
import { ScenarioTabs } from "./components/ScenarioTabs";
import { QueueSection } from "./components/QueueSection";
import { AppMenu } from "./components/AppMenu";
import { StepRail } from "./components/StepRail";
import { useMode } from "./mode";
import { stepStates, type ProjectFacts, type Step } from "./steps";
import { useTranslation } from "./i18n/i18n";
import {
  cliPath,
  handleBegin,
  handleCancel,
  handleDrag,
  handleEnd,
  hoverAt,
  pickAt,
  setHandles,
  showWindHandle,
  pickPointAt,
  onCameraMoved,
  onRideClock,
  rideControl,
  ridePointAt,
  sendCamera,
  setViewInsets,
  showSelection,
  syncViewport,
} from "./native";
import { RunMonitor } from "./run-monitor";
import { measurePanels, viewInsets } from "./view-insets";
import type { CameraAction, CameraPose } from "./viewport-input";

export function App() {
  const { t, language } = useTranslation();
  const languageRef = useRef(language);
  languageRef.current = language;
  const [aboutOpen, setAboutOpen] = useState(false);
  const [mcpOpen, setMcpOpen] = useState(false);
  const [exportOpen, setExportOpen] = useState(false);
  const [reportOpen, setReportOpen] = useState(false);
  const [setupOpen, setSetupOpen] = useState(false);

  // First start: the setup assistant.
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
  const [step, setStep] = useState<Step>("geometry");
  const { mode } = useMode();
  // What the project already has, for the state of each step.
  const [facts, setFacts] = useState<ProjectFacts>({
    models: 0,
    watertight: null,
    scenario: false,
    running: false,
    hasResult: false,
  });
  const selectionRef = useRef<string[]>([]);
  const [capMode, setCapMode] = useState(false);
  const [probe, setProbe] = useState<ProbeValue | null>(null);
  // The next click in the view starts a ride (a particle or a spot on the model).
  const [ridePick, setRidePick] = useState(false);
  const ridePickRef = useRef(false);
  ridePickRef.current = ridePick;
  const stepRef = useRef(step);
  stepRef.current = step;
  const probeBusy = useRef(false);
  const capModeRef = useRef(false);
  capModeRef.current = capMode;
  const hoverBusy = useRef(false);
  // Key under the mouse from the last hover pick: a press on a handle drags it.
  const hoverKey = useRef<string | null>(null);
  // The model the move and rotate handles belong to: the clicked one, or the only one.
  const [activeModel, setActiveModel] = useState<string | null>(null);
  const [modelIds, setModelIds] = useState<string[]>([]);
  const [handleValue, setHandleValue] = useState<HandleValue | null>(null);
  // Slice chosen in the results list or by clicking it in the view.
  const [selectedSlice, setSelectedSlice] = useState<string | null>(null);
  // Source of air chosen in the view (Scenario step): the handles sit on it.
  const [selectedJet, setSelectedJet] = useState<string | null>(null);

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
  // A finished run (or queue) watched in the Run step opens the results.
  const showResults = useCallback(() => {
    if (stepRef.current === "run") setStep("results");
  }, []);
  useEffect(() => monitor?.onResult(showResults), [monitor, showResults]);

  const onPick = useCallback((x: number, y: number, mode: PickMode) => {
    if (ridePickRef.current) {
      // Start a ride from the particle or the spot on the model under the click.
      setRidePick(false);
      ridePointAt(x, y)
        .then((from) =>
          from ? tauriApi.call("ride.start", { from, ...gentleStart() }) : undefined,
        )
        .catch((err: unknown) => window.alert(errorMessage(err, languageRef.current)));
      return;
    }
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
        if (key?.startsWith("handle:")) return;
        if (key?.startsWith("jet:")) {
          setSelectedJet(key);
          return;
        }
        if (key?.startsWith("slice:")) {
          setSelectedSlice(key.slice("slice:".length));
          return;
        }
        if (key && key.startsWith("model-")) setActiveModel(key.split(":")[0]);
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
        .then((hit) => {
          hoverKey.current = hit?.key ?? null;
          return hit;
        })
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
      .then((key) => (hoverKey.current = key))
      .catch((err: unknown) => console.error(err))
      .finally(() => (hoverBusy.current = false));
  }, []);
  // Yaw and pitch of the native camera; the view cube mirrors them. Starts as the default view.
  const [pose, setPose] = useState<CameraPose>({
    yaw: (-60 * Math.PI) / 180,
    pitch: (25 * Math.PI) / 180,
  });
  const send = useCallback((action: CameraAction) => {
    sendCamera(action)
      .then((p) => p && setPose(p))
      .catch((err: unknown) => console.error(err));
  }, []);
  useEffect(() => {
    let stop: (() => void) | undefined;
    let ended = false;
    onCameraMoved(setPose)
      .then((unlisten) => (ended ? unlisten() : (stop = unlisten)))
      .catch((err: unknown) => console.error(err));
    send({ kind: "resize" });
    return () => {
      ended = true;
      stop?.();
    };
  }, [send]);
  const [rightInset, setRightInset] = useState(0);
  // The active scenario; the scenario inputs are built afresh when it changes.
  const [activeScenario, setActiveScenario] = useState("scenario-1");
  useEffect(() => {
    if (!project) return;
    tauriApi
      .call<{ active?: string }>("scenario.list", {})
      .then((l) => l?.active && setActiveScenario(l.active))
      .catch(() => {});
  }, [project, revision]);

  // After a reload of the page the session may still have a project open: show it.
  useEffect(() => {
    tauriApi
      .call<ProjectInfo>("project.info")
      .then((p) => {
        setProject(p);
        syncViewport().catch(() => {});
      })
      .catch(() => {});
  }, []);

  useEffect(() => {
    if (!project) return;
    let current = true;
    const ok = (p: Promise<unknown>) => p.then(() => true).catch(() => false);
    (async () => {
      const list = await tauriApi.call<{ id: string }[]>("geometry.list", {}).catch(() => []);
      const infos = await Promise.all(
        (Array.isArray(list) ? list : []).map((m) =>
          tauriApi
            .call<{ diagnosis?: { watertight?: boolean } }>("geometry.info", { id: m.id })
            .catch(() => null),
        ),
      );
      const watertight = infos.length
        ? infos.every((i) => i?.diagnosis?.watertight !== false)
        : null;
      const scenario = await ok(tauriApi.call("scenario.info", {}));
      const hasResult = await ok(tauriApi.call("results.list_fields", {}));
      if (current)
        setFacts({ models: infos.length, watertight, scenario, running: false, hasResult });
    })();
    return () => {
      current = false;
    };
  }, [project, revision]);
  const states = stepStates(facts);
  // A project opened outside the results step shows its plain models (and their handles).
  const openedPath = project?.path ?? null;
  useEffect(() => {
    if (openedPath && stepRef.current !== "results")
      tauriApi.call("view.set_surface_field", { field: null }).catch(() => {});
  }, [openedPath]);
  const goTo = useCallback(
    (s: Step) => {
      // Leaving the results shows the plain models again.
      if (step === "results" && s !== "results")
        tauriApi.call("view.set_surface_field", { field: null }).catch(() => {});
      setStep(s);
    },
    [step],
  );
  useEffect(() => {
    const keys = selectedSlice ? [...selection, `slice:${selectedSlice}`] : selection;
    showSelection(keys).catch((err: unknown) => console.error(err));
  }, [selection, selectedSlice, revision]);

  // Models of the project, for choosing the model the handles belong to.
  useEffect(() => {
    if (!project) {
      setModelIds([]);
      return;
    }
    tauriApi
      .call<{ id: string }[]>("geometry.list", {})
      .then((list) => setModelIds(list.map((m) => m.id)))
      .catch(() => setModelIds([]));
  }, [project, revision]);
  const handleModel =
    activeModel && modelIds.includes(activeModel)
      ? activeModel
      : modelIds.length === 1
        ? modelIds[0]
        : null;
  const handlesShown = step === "geometry" && handleModel !== null && !capMode;
  // The wind arrow is a handle in the Scenario step.
  const windShown = step === "scenario" && project !== null;
  useEffect(() => {
    showWindHandle(windShown).catch((err: unknown) => console.error(err));
  }, [windShown]);
  const jetHandles = step === "scenario" && selectedJet !== null;
  useEffect(() => {
    if (!handlesShown && !jetHandles)
      setHandles(null, null).catch((err: unknown) => console.error(err));
  }, [handlesShown, jetHandles]);
  // The handles of a chosen source sit at its centre (again after every change).
  useEffect(() => {
    if (!jetHandles || !selectedJet) return;
    const index = Number(selectedJet.slice(4)) - 1;
    tauriApi
      .call<{ jets?: { center: [number, number, number] }[] }>("scenario.info")
      .then((r) => {
        const jet = r?.jets?.[index];
        if (jet) return setHandles(selectedJet, jet.center);
        setSelectedJet(null);
      })
      .catch(() => {});
  }, [jetHandles, selectedJet, revision]);
  const onPivot = useCallback((model: string, point: [number, number, number]) => {
    setHandles(model, point).catch((err: unknown) => console.error(err));
  }, []);
  // Mouse moves during a drag: one request at a time, the newest position wins.
  const dragQueue = useRef<{ busy: boolean; next: [number, number, boolean] | null }>({
    busy: false,
    next: null,
  });
  const handleInput = useMemo<HandleInput>(() => {
    const pump = () => {
      const q = dragQueue.current;
      if (q.busy || !q.next) return;
      const [x, y, free] = q.next;
      q.next = null;
      q.busy = true;
      handleDrag(x, y, free)
        .then((v) => v && setHandleValue(v))
        .catch((err: unknown) => console.error(err))
        .finally(() => {
          q.busy = false;
          pump();
        });
    };
    return {
      keyUnderMouse: () => hoverKey.current,
      begin: (key, x, y) => {
        handleBegin(key, x, y).catch((err: unknown) => console.error(err));
      },
      drag: (x, y, free) => {
        dragQueue.current.next = [x, y, free];
        pump();
      },
      end: () => {
        dragQueue.current.next = null;
        setHandleValue(null);
        handleEnd()
          .then((cmd) => cmd && tauriApi.call(cmd.command, cmd.params))
          .catch((err: unknown) => window.alert(errorMessage(err, languageRef.current)));
      },
      cancel: () => {
        dragQueue.current.next = null;
        setHandleValue(null);
        handleCancel().catch((err: unknown) => console.error(err));
      },
    };
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
      setRightInset(right);
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
        handles={handlesShown || windShown || step === "results" ? handleInput : undefined}
      />
      {project && step === "results" && (
        <RideOverlay
          api={tauriApi}
          revision={revision}
          units={project.display_units}
          control={rideControl}
          onClock={onRideClock}
        />
      )}
      {project && (
        <div className="view-cube-anchor" style={{ right: rightInset + 12 }}>
          <ViewCube
            yaw={pose.yaw}
            pitch={pose.pitch}
            onView={(name) => send({ kind: "view", name })}
          />
        </div>
      )}
      {windShown && handleValue?.kind === "wind" && (
        <div className="move-tools panel" role="status">
          <span className="move-value">{`${t("move.wind")} ${formatNumber(handleValue.value)}°`}</span>
        </div>
      )}
      <header className="toolbar panel">
        <span className="brand">{t("app.title")}</span>
        {project && <span className="project-name">{project.name}</span>}
        {project && <StepRail step={step} states={states} onStep={goTo} />}
        <span className="spacer" />
        {project && (
          <button type="button" className="quiet" onClick={() => send({ kind: "fit" })}>
            {t("toolbar.fit")}
          </button>
        )}
        {project && (
          <button type="button" className="quiet" onClick={() => setExportOpen(true)}>
            {t("export.button")}
          </button>
        )}
        {project && (
          <button type="button" className="quiet" onClick={() => setReportOpen(true)}>
            {t("report.button")}
          </button>
        )}
        {project && (
          <button
            type="button"
            className="quiet"
            onClick={() => tauriApi.call("project.close").catch(() => {})}
          >
            {t("project.close")}
          </button>
        )}
        <AppMenu
          onSystem={() => setSetupOpen(true)}
          onAgents={() => setMcpOpen(true)}
          onAbout={() => setAboutOpen(true)}
        />
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
              {handlesShown && handleModel && (
                <MoveTools
                  api={tauriApi}
                  model={handleModel}
                  revision={revision}
                  value={handleValue}
                  onPivot={onPivot}
                />
              )}
              {mode === "expert" && (
                <SurfacePanel api={tauriApi} selection={selection} revision={revision}>
                  <OpeningsSection
                    api={tauriApi}
                    revision={revision}
                    capMode={capMode}
                    onCapMode={setCapMode}
                  />
                </SurfacePanel>
              )}
            </>
          ) : step === "results" ? (
            <>
              <ResultsPanel
                key={project.path}
                api={tauriApi}
                probe={probe}
                selectedSlice={selectedSlice}
                onSelectSlice={setSelectedSlice}
                onView={(name) => send({ kind: "view", name })}
                onRidePick={() => setRidePick((p) => !p)}
                ridePicking={ridePick}
                revision={revision}
              />
              <KeyFiguresPanel
                key={`figures-${project.path}`}
                api={tauriApi}
                revision={revision}
                pickSavePath={pickSavePath}
                confirm={(message) => window.confirm(message)}
              />
            </>
          ) : step === "scenario" ? (
            <ScenarioPanel
              key={`${project.path}-${activeScenario}`}
              header={<ScenarioTabs api={tauriApi} revision={revision} />}
              api={tauriApi}
              units={project.display_units}
              revision={revision}
            />
          ) : (
            <RunPanel
              key={project.path}
              api={tauriApi}
              subscribe={subscribeApiEvents}
              footer={
                <QueueSection api={tauriApi} subscribe={subscribeApiEvents} onDone={showResults} />
              }
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
          onClose={() => setSetupOpen(false)}
        />
      )}
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
