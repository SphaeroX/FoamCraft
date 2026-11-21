# Task_02: Runtime-Image und Docker-Runner

> ## ⚠️ PFLICHT – BEVOR DU IRGENDETWAS ANDERES TUST
>
> 1. **Lies als Allererstes die Datei `AGENTS.md` im übergeordneten Ordner `Roadmap/`** (von hier: `../AGENTS.md`) und befolge sie strikt.
> 2. **Deine allererste Aktion** ist, diese Datei in `RUN_Task_02.md` umzubenennen, um den Task als laufend zu markieren. Das passiert **noch bevor irgendein Code geschrieben** wird.

## Ziel

`foamcraft runtime check` startet einen Container aus dem eigenen Image und meldet OpenFOAM-Version, nutzbare Kerne, Arbeitsspeicher und GPU. Dahinter steht eine austauschbare Runner-Schnittstelle, auf der alle späteren Rechen-Tasks aufbauen.

## Kontext & Abhängigkeiten

- **Voraussetzung:** Task_01 ist archiviert. Extern: Docker Desktop läuft mit WSL2-Backend. Beim Anlegen der Roadmap war der Docker-Dienst aus und keine WSL-Distribution installiert; den Nutzer bitten, Docker Desktop zu starten, falls das noch so ist.
- **Relevantes Vorwissen:** `docs/architecture.md` (Runtime layer, In-runtime agent, offene Entscheidungen "OpenFOAM release" und "Case file I/O"), `docs/licensing.md` (Abschnitt OpenFOAM: Quellverfügbarkeit).

## Umfang

**Gehört dazu:**

- `runtime/Dockerfile` mit einer gepinnten OpenFOAM-Version der OpenCFD-Linie und Labels, die die genaue Quellversion nennen
- Agent: ein Linux-Build des Kerns im Image (Multi-Stage-Build), der Zustand und Version meldet
- Runner-Schnittstelle: Image prüfen und bauen, Container starten und stoppen, Befehle mit Log-Strom ausführen, Arbeitsverzeichnis bereitstellen, Kerne und Arbeitsspeicher begrenzen, GPU-Durchreichung erkennen
- Docker-Implementierung der Schnittstelle und eine Fake-Implementierung für Tests
- Messung des Datei-Zugriffs: Volume mit Agent gegen Bind-Mount, an einem OpenFOAM-Tutorialfall
- CLI `foamcraft runtime check`, `build`, `stop`
- Fehlercodes mit Klartext in beiden Sprachen: Docker fehlt, Dienst läuft nicht, kein WSL2-Backend, zu wenig Speicher

**Gehört nicht dazu:**

- Eigene Fälle und die Job-Pipeline (Task_03)
- GPU-Löser (Task_34), WSL-Runner (Task_36), Einrichtungsassistent in der Oberfläche (Task_21)
- Veröffentlichung des Images in einer Registry (Task_32)

## Umsetzungsschritte

1. OpenFOAM-Release wählen: aktuelle Releases im Web prüfen, Verträglichkeit mit den GPU-Plugins (OGL/Ginkgo, petsc4Foam) berücksichtigen, Wahl und Begründung in `docs/architecture.md` festhalten.
2. Tests zuerst für die Runner-Schnittstelle mit der Fake-Implementierung (Zustände, Fehlerabbildung).
3. Dockerfile schreiben und das Image lokal bauen.
4. Docker-Runner umsetzen; die Docker-Bibliothek muss auf der Erlaubnisliste stehen.
5. Agent minimal umsetzen und die Verbindung zwischen Host und Agent herstellen; die Protokollwahl begründen.
6. Datei-Zugriff messen und die Entscheidung mit Messwerten dokumentieren.
7. GPU-Erkennung umsetzen (GPU im Container sichtbar oder sauberer Hinweis).
8. CLI-Befehle, Fehlercodes und Doku ergänzen.

## Tests

Alle Tests dieses Tasks werden permanent in `tests/` im Hauptverzeichnis des Projekts abgelegt (siehe `AGENTS.md`).

- Unit-Tests der Runner-Logik gegen die Fake-Implementierung, inklusive Abbildung der Docker-Fehler auf Fehlercodes.
- `runtime_`-Tests: Container startet, ein OpenFOAM-Programm antwortet mit Exit-Code 0, der Agent meldet seine Version, Stoppen räumt den Container auf, das Kernlimit wirkt.
- Test des Fehlerfalls "Dienst läuft nicht" (simuliert).

## Akzeptanzkriterien

Der Task gilt erst als fertig, wenn alle Punkte erfüllt und durch Tests oder einen konkreten Befehl belegt sind:

- [x] `foamcraft runtime check` gibt OpenFOAM-Version, Kerne, Arbeitsspeicher und GPU-Status aus
- [x] Die `runtime_`-Tests laufen auf dem Referenzrechner grün (`cargo test --workspace -- --ignored runtime_`)
- [x] Die Entscheidung zum Datei-Zugriff steht mit Messwerten in `docs/architecture.md`
- [x] Das Image trägt ein Label mit der OpenFOAM-Quellreferenz
- [x] Jeder der vier Fehlerfälle hat einen Fehlercode mit englischem und deutschem Text

## Offene Fragen / Annahmen

- Offene Entscheidungen "OpenFOAM release" und "Case file I/O" fallen in diesem Task.
- Annahme: Das Image wird bis Task_32 lokal gebaut.
- Docker Desktop ist für größere Firmen kostenpflichtig; die Alternative entsteht in Task_36.

## Notizen zu diesem Task

Taskspezifische Informationen während der Arbeit hier festhalten. Taskübergreifendes gehört in `Roadmap/README.md`.

- Ausgabe von `foamcraft runtime check` am 2026-11-21: Docker 29.9.2 (virtuelle Maschine), OpenFOAM v2606 (build _481094f-20260618), Agent 0.1.0, Kerne 32, Arbeitsspeicher 1.9 GiB, GPU nicht verfügbar (keine Durchreichung), Warnungen `runtime.no_wsl2_backend` und `runtime.insufficient_memory`.
- Image-Label: `org.foamcraft.openfoam.source=https://develop.openfoam.com/Development/openfoam/-/tree/OpenFOAM-v2606`.
- Der Nutzer hat am 2026-11-21 erlaubt, `target\io-bench` für Docker Desktop freizugeben (Bind-Mount-Messung). Ein früherer Dialog für einen Temp-Ordner wurde mit "No" abgelehnt.

## Abschluss-Checkliste

Reihenfolge und Details stehen in `AGENTS.md`, Abschnitt "Abschluss- und Archivierungs-Workflow".

- [x] Alle Akzeptanzkriterien erfüllt, Tests liegen permanent in `tests/` und laufen grün
- [x] Abschnitt "Erkenntnisse für Folge-Agenten" unten ausgefüllt
- [x] Folge-Tasks und `Roadmap/README.md` geprüft und bei Bedarf angepasst
- [x] Datei ohne `RUN_` nach `Roadmap/Archiv/` verschoben

## Erkenntnisse für Folge-Agenten

- **Umgesetzt (und Abweichungen vom Plan):** Crates `foamcraft-runtime` (Trait `Runner`, `DockerRunner`, `fake::FakeEngine`, `report`) und `foamcraft-agent` (Linux-Binary im Image; `version`, `status`, `idle`). Fehlerkatalog `crates/foamcraft-core/data/errors.json` mit `foamcraft_core::error::{FoamError, Language, message}`. Protokoll-Typen in `foamcraft_core::runtime`. `runtime/Dockerfile` (Multi-Stage: rust:1.99-bookworm baut den Agenten, Basis `opencfd/openfoam-run:2606` per Digest). Messfall `runtime/cases/cavity` (aus dem OpenFOAM-Tutorial adaptiert). CLI `foamcraft [--lang de] runtime check|build|stop`. Abweichung: Docker wird über die Kommandozeile angesprochen statt über eine Bibliothek (Begründung in `docs/architecture.md`). Zusätzlich zu den vier verlangten Fehlercodes gibt es `runtime.image_missing`, `runtime.command_failed`, `runtime.agent_failed`.
- **Learnings & Stolpersteine:** Das OpenCFD-`-run`-Image enthält keine Tutorials und keine Compiler; `-dev`/`-default` gab es für 2606 noch nicht. Die OpenFOAM-Umgebung ist nur in einer Login-Shell geladen (`bash -lc`); `Runner::exec_openfoam` macht genau das. Der Agent findet OpenFOAM auch ohne Umgebung unter `/usr/lib/openfoam/openfoam*`. Rusts `available_parallelism` beachtet das cgroup-CPU-Limit (`--cpus 2` → 2). Docker Desktop zeigt bei jedem neuen Bind-Mount-Ordner einen Freigabe-Dialog; ohne Bestätigung hängt `docker run` unbegrenzt. Die Sicherheitsprüfung des Agenten-Werkzeugs blockt Shell-Kommandos mit `rm` in `bash -c`-Skripten; Aufräumen deshalb über Rust (`Runner::stop`, `remove_volume`).
- **Wichtige Entscheidungen und ihre Gründe:** OpenFOAM v2606 (aktuelles OpenCFD-Release). Fall-Dateien auf einem benannten Volume mit Agent (Messwerte in `docs/architecture.md`: Volume ~1,07 s gegen Bind-Mount ~1,2 s, dazu der Freigabe-Dialog). Host↔Agent über `docker exec … foamcraft-agent <befehl>` mit JSON auf stdout, kein Netzwerkport. "Kein WSL2-Backend" und "zu wenig Speicher" sind Warnungen, keine harten Fehler, weil Rechnen trotzdem geht.
- **Auswirkungen auf Folge-Tasks:** Der Referenzrechner läuft derzeit mit Docker Desktop im Hyper-V-Modus (Kernel `linuxkit`), nur ~1,9 GiB Speicher im Docker-VM und ohne GPU-Durchreichung. Task_03 (Fall < 200 000 Zellen) muss mit 2 GiB auskommen; Task_34 (GPU) braucht das WSL2-Backend – den Nutzer vorher bitten, es einzuschalten. Neue Runner-Funktionen für Task_03: `create_volume`, `remove_volume`, `copy_into`, `copy_out`, `exec_openfoam`, `stop_all`. Container tragen das Label `org.foamcraft.managed=true`.
- **Hinweise zu Tests und Befehlen:** Image bauen: `cargo run -p foamcraft-cli -- runtime build`. Runtime-Tests: `cargo test --workspace -- --ignored runtime_` (brauchen das Image; 8 Tests inkl. CLI und beider I/O-Messungen). Der Ordner `target/io-bench` ist in Docker Desktop freigegeben (vom Nutzer am 2026-11-21 erlaubt); andere Host-Ordner lösen den Dialog erneut aus.
