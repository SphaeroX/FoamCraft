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
- GPU-Löser (Task_24), WSL-Runner (Task_26), Einrichtungsassistent in der Oberfläche (Task_21)
- Veröffentlichung des Images in einer Registry (Task_22)

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

- [ ] `foamcraft runtime check` gibt OpenFOAM-Version, Kerne, Arbeitsspeicher und GPU-Status aus
- [ ] Die `runtime_`-Tests laufen auf dem Referenzrechner grün (`cargo test --workspace -- --ignored runtime_`)
- [ ] Die Entscheidung zum Datei-Zugriff steht mit Messwerten in `docs/architecture.md`
- [ ] Das Image trägt ein Label mit der OpenFOAM-Quellreferenz
- [ ] Jeder der vier Fehlerfälle hat einen Fehlercode mit englischem und deutschem Text

## Offene Fragen / Annahmen

- Offene Entscheidungen "OpenFOAM release" und "Case file I/O" fallen in diesem Task.
- Annahme: Das Image wird bis Task_22 lokal gebaut.
- Docker Desktop ist für größere Firmen kostenpflichtig; die Alternative entsteht in Task_26.

## Notizen zu diesem Task

Taskspezifische Informationen während der Arbeit hier festhalten. Taskübergreifendes gehört in `Roadmap/README.md`.

## Abschluss-Checkliste

Reihenfolge und Details stehen in `AGENTS.md`, Abschnitt "Abschluss- und Archivierungs-Workflow".

- [ ] Alle Akzeptanzkriterien erfüllt, Tests liegen permanent in `tests/` und laufen grün
- [ ] Abschnitt "Erkenntnisse für Folge-Agenten" unten ausgefüllt
- [ ] Folge-Tasks und `Roadmap/README.md` geprüft und bei Bedarf angepasst
- [ ] Datei ohne `RUN_` nach `Roadmap/Archiv/` verschoben

## Erkenntnisse für Folge-Agenten

<!-- ERKENNTNISSE-OFFEN: Vor der Archivierung ausfüllen und diese Zeile löschen. -->

Vor der Archivierung vom Agenten auszufüllen. Schreibe so, dass ein Agent, der diesen Task nie gesehen hat, davon bei späteren Tasks profitiert:

- **Umgesetzt (und Abweichungen vom Plan):**
- **Learnings & Stolpersteine:**
- **Wichtige Entscheidungen und ihre Gründe:**
- **Auswirkungen auf Folge-Tasks:**
- **Hinweise zu Tests und Befehlen:**
