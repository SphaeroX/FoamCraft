# Task_03: Job-Pipeline mit Referenzfall

> ## ⚠️ PFLICHT – BEVOR DU IRGENDETWAS ANDERES TUST
>
> 1. **Lies als Allererstes die Datei `AGENTS.md` im übergeordneten Ordner `Roadmap/`** (von hier: `../AGENTS.md`) und befolge sie strikt.
> 2. **Deine allererste Aktion** ist, diese Datei in `RUN_Task_03.md` umzubenennen, um den Task als laufend zu markieren. Das passiert **noch bevor irgendein Code geschrieben** wird.

## Ziel

Ein eingebauter Testfall (Würfel im Windkanal aus festen Vorlagen) wird mit `foamcraft run` vollständig vernetzt und parallel gerechnet. Fortschritt und Residuen kommen live, der Lauf lässt sich abbrechen. Damit steht die Pipeline, in die spätere Tasks ihre erzeugten Fälle einspeisen.

## Kontext & Abhängigkeiten

- **Voraussetzung:** Task_02 ist archiviert (Runner, Agent, Image).
- **Relevantes Vorwissen:** `docs/architecture.md`, Regeln 6 (Jobs) und 7 (Fehlercodes). Aus Task_02: Fälle liegen auf einem Docker-Volume unter `/work` (Runner: `create_volume`, `copy_into`, `exec_openfoam`, `copy_out`); der Docker-VM des Referenzrechners hat derzeit nur ~1,9 GiB Speicher (Hyper-V-Backend), der Fall muss damit auskommen. Die Log-Formate hängen von der in Task_02 gewählten OpenFOAM-Version ab; die Erkenntnisse von Task_02 lesen.

## Umfang

**Gehört dazu:**

- Job-Modell mit Stufen (Hintergrundnetz, Vernetzung, Zerlegung, Lösung, Zusammenführung), Zustandsautomat, persistentes Log, Ereignisstrom
- Log-Auswertung für blockMesh, snappyHexMesh, checkMesh, decomposePar und simpleFoam (Iteration, Residuen, Laufzeit, Fehlermeldungen)
- Abbruch mit sauberem Aufräumen, ohne verwaiste Prozesse
- Wiederfinden eines laufenden Jobs nach einem Neustart des Hosts
- Feste Fallvorlage als Datendateien im Kern
- Wahl der Kernzahl (Standard: physische Kerne) und parallele Ausführung
- CLI `foamcraft run` mit Fortschrittsausgabe

**Gehört nicht dazu:**

- Fallerzeugung aus Nutzergeometrie (Task_09)
- Kräfte und Konvergenz-Automatik (Task_10)
- Anzeige in der App (Task_11)

## Umsetzungsschritte

1. Log-Ausschnitte der gewählten OpenFOAM-Version aufzeichnen (Erfolg, `FOAM FATAL ERROR`, Divergenz mit NaN) und als Testdaten ablegen.
2. Tests zuerst für die Log-Auswertung, dann umsetzen.
3. Tests zuerst für den Zustandsautomaten gegen den Fake-Runner (Erfolg, Fehler in einer Stufe, Abbruch), dann umsetzen.
4. Fallvorlage für den Würfel anlegen (klein, unter 200 000 Zellen).
5. Pipeline mit dem Docker-Runner verbinden, parallel rechnen lassen.
6. Abbruch und Wiederfinden umsetzen.
7. CLI und Fehlercodes ergänzen.

## Tests

Alle Tests dieses Tasks werden permanent in `tests/` im Hauptverzeichnis des Projekts abgelegt (siehe `AGENTS.md`).

- Log-Auswertung gegen die aufgezeichneten Ausschnitte: Residuen und Iterationen stimmen, Fehler werden mit dem richtigen Code erkannt.
- Zustandsautomat: Erfolg, Fehler, Abbruch, Wiederaufnahme.
- `runtime_`: Der Referenzfall läuft in unter 5 Minuten durch; ein Abbruch während der Lösung beendet alle Prozesse im Container.

## Akzeptanzkriterien

Der Task gilt erst als fertig, wenn alle Punkte erfüllt und durch Tests oder einen konkreten Befehl belegt sind:

- [x] `foamcraft run` rechnet den Referenzfall bis zum Ende und zeigt Stufe, Iteration und Residuen
- [x] Ein Abbruch hinterlässt keine laufenden OpenFOAM-Prozesse (durch einen `runtime_`-Test belegt)
- [x] Ein absichtlich fehlerhafter Fall endet mit einem Fehlercode statt mit einem Absturz
- [x] Das Log eines Jobs ist nach dem Lauf vollständig abrufbar

## Offene Fragen / Annahmen

- Annahme: Der Referenzfall dient nur der Pipeline und wird nicht validiert.

## Notizen zu diesem Task

Taskspezifische Informationen während der Arbeit hier festhalten. Taskübergreifendes gehört in `Roadmap/README.md`.

- Nachweis `foamcraft --lang de run --iterations 40` am 2026-11-21: Schritte background, mesh (Phasen refine, snap), check, decompose, solve mit Iteration und Residuen je Feld, reconstruct; "Zellen: 51680  Kerne: 5", "Fertig.".
- `runtime_reference_case_runs_in_parallel_in_under_five_minutes`: 200 Iterationen in 14,9 s mit 5 Prozessen.
- `runtime_cancel_during_solve_ends_all_processes`: simpleFoam lief vor dem Abbruch (ps im Container), danach existiert der Container nicht mehr, Zustand `cancelled`.
- `runtime_broken_case_ends_with_an_error_code`: kaputtes blockMeshDict endet mit `failed(background, run.foam_fatal_error)`.
- Log nach dem Lauf: `foamcraft job log <id>` (CLI-Test `runtime_run_command_shows_progress_and_keeps_the_log`).

## Abschluss-Checkliste

Reihenfolge und Details stehen in `AGENTS.md`, Abschnitt "Abschluss- und Archivierungs-Workflow".

- [x] Alle Akzeptanzkriterien erfüllt, Tests liegen permanent in `tests/` und laufen grün
- [x] Abschnitt "Erkenntnisse für Folge-Agenten" unten ausgefüllt
- [x] Folge-Tasks und `Roadmap/README.md` geprüft und bei Bedarf angepasst
- [x] Datei ohne `RUN_` nach `Roadmap/Archiv/` verschoben

## Erkenntnisse für Folge-Agenten

- **Umgesetzt (und Abweichungen vom Plan):** `foamcraft_core::log` (zeilenweiser Parser, `LogSummary`), `foamcraft_core::job` (Stufen, Zustände, `JobStore`, `run_job`, `choose_cores`, `stage_script`), `foamcraft_core::case` (Vorlagen mit `{{NAME}}`, Referenzfall `reference_cube`), `foamcraft_core::paths`; Agent-Befehle `run-stage` und `tail`; `foamcraft_runtime::{DockerStageExecutor, jobs::{prepare_job, execute_job, core_limit, remove_job_resources}}`, `Runner::exec_detached`; CLI `run` und `job list|log|resume|clean` (`clean` zusätzlich, damit Volumes nicht liegen bleiben). Abbruch per Strg+C (crate `ctrlc`), physische Kerne per `num_cpus`.
- **Learnings & Stolpersteine:** OpenFOAM-Logs v2606: Residuenzeilen `<solver>:  Solving for <f>, Initial residual = …`; checkMesh, decomposePar und reconstructPar schreiben ebenfalls `Time = 0` – Fortschritt nur aus der Lösungsstufe melden. Das Banner `trapFpe: Floating point exception trapping enabled` ist kein Fehler. Mit FOAM_SIGFPE (Standard) endet Divergenz als `Floating point exception` mit Stacktrace, sonst mit `nan`-Residuen. snappyHexMesh in v2606 verlangt `nSmoothScale` und `errorReduction` in `meshQualityControls` zusätzlich zur `#includeEtc "caseDicts/meshQualityDict"`. Open MPI 5 im Container läuft als root nur mit `OMPI_ALLOW_RUN_AS_ROOT=1` und `…_CONFIRM=1` (setzt der Agent). `docker exec`-Prozesse laufen weiter, wenn der Host-Prozess endet; deshalb Stufen über `exec -d` und Abfragen über `tail`. Gelegentlich scheitert der MSVC-Linker mit LNK1201 (PDB gesperrt); einfach erneut bauen.
- **Wichtige Entscheidungen und ihre Gründe:** Der Host steuert die Stufen, der Agent führt sie aus und schreibt Log und Exit-Code ins Volume – so überlebt ein Lauf einen Host-Neustart. Abbruch = Container entfernen (beendet garantiert alle Prozesse; das Volume bleibt). Kernzahl erst nach checkMesh: eine pro 10 000 Zellen, höchstens physische Kerne bzw. Container-CPUs.
- **Auswirkungen auf Folge-Tasks:** Task_09/10 erzeugen Fälle über `prepare_job(runner, store, name, |dir| …)` und können eigene Vorlagen in `crates/foamcraft-core/templates/` ablegen. Task_10 (Konvergenz, Kräfte) baut auf `LogSummary`/`JobEvent::Progress` auf; `Converged`-Ereignis (`solution converged in`) gibt es schon. Task_11 (Monitor in der App) kann `JobEvent` direkt abbilden. Task_14 muss Ergebnisse aus dem Volume holen (`Runner::copy_out` oder Agent). Nach Agent-Änderungen das Image neu bauen (`foamcraft runtime build`).
- **Hinweise zu Tests und Befehlen:** Fake-Tests: `cargo test -p foamcraft-core --test job`, `--test log_parser`, `cargo test -p foamcraft-runtime --test runner`. Echte Läufe: `cargo test --workspace -- --ignored runtime_` (Image vorher bauen). Aufgezeichnete Logs in `tests/data/logs/` (README dort).
