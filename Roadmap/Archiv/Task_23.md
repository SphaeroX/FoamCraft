# Task_23: Rechnen in einem Schritt – Vernetzung, Fortschritt, Konsole, Abbruch

> ## ⚠️ PFLICHT – BEVOR DU IRGENDETWAS ANDERES TUST
>
> 1. **Lies als Allererstes die Datei `AGENTS.md` im übergeordneten Ordner `Roadmap/`** (von hier: `../AGENTS.md`) und befolge sie strikt.
> 2. **Deine allererste Aktion** ist, diese Datei in `RUN_Task_23.md` umzubenennen, um den Task als laufend zu markieren. Das passiert **noch bevor irgendein Code geschrieben** wird.

## Ziel

Im Schritt "Szenario" wird nur das Szenario festgelegt. Vernetzen und Rechnen passieren zusammen im Schritt "Rechnen": ein Knopf, sichtbarer Fortschritt schon während der Vernetzung, eine einklappbare Konsole, Abbruch jederzeit. Die Anzeige bleibt erhalten, wenn man zwischendurch in einen anderen Schritt wechselt.

## Kontext & Abhängigkeiten

- **Voraussetzung:** Task_22 ist archiviert. Baut auf Task_09 (Vernetzung), Task_10/13 (Rechnung) und Task_11 (Lauf-Monitor) auf.
- **Herkunft:** Rückmeldung des Nutzers (2026-11-21): Bei der Vernetzung steht nur "Vernetzung läuft … kann einige Minuten dauern", ohne Status; sie lässt sich nicht abbrechen; nach "Rechnen → Ergebnisse → Rechnen" sind Kurven und Übersicht verschwunden.
- **Relevantes Vorwissen:** Befund beim Sichten: `run.start` ruft bei fehlendem Netz `mesh::generate` innerhalb des Befehls auf (`crates/foamcraft-api/src/solve.rs`, `prepare`). Der Befehl hält dabei die Sitzung gesperrt (`app/src-tauri/src/api.rs`, `api_call`), also beantwortet die App währenddessen keinen anderen Befehl: kein Protokoll, kein Abbruch, keine Job-Kennung. `mesh.generate` ist ebenfalls blockierend. Der Monitor-Zustand liegt in `RunPanel` und geht beim Wechsel des Schritts verloren (die Komponente wird ausgehängt).

## Umfang

**Gehört dazu:**

- `run.start` kehrt sofort zurück, auch wenn erst vernetzt werden muss; Vernetzung und Rechnung laufen im Hintergrund, ohne die Sitzung zu sperren
- Abbruch während der Vernetzung (`run.cancel`) beendet alle Prozesse
- Ereignisse für die Vernetzung: Stufe, Phase (Verfeinern, Anschmiegen, Grenzschichten), Versuch der Ausweichleiter, geschätzte Restzeit
- `run.log` liefert auch das Protokoll der Vernetzung des laufenden Laufs
- Schritt "Rechnen": ein Startknopf für alles, Netz-Kennzahlen nach der Vernetzung, optional "Nur vernetzen"
- Schritt "Szenario" ohne Netz-Abschnitt
- Lauf-Monitor als dauerhafter Zustand: bleibt beim Wechsel des Schritts und füllt sich weiter
- Einklappbare Konsole am unteren Fensterrand, in jedem Schritt erreichbar, zeigt das Protokoll des laufenden Laufs

**Gehört nicht dazu:**

- Mehrere Szenarien in Reihe rechnen (Task_30)
- Änderungen an Netzregeln oder Numerik

## Umsetzungsschritte

1. Tests zuerst (Rust, FakeEngine): `run.start` kehrt bei fehlendem Netz sofort zurück; währenddessen antworten andere Befehle; Abbruch während der Vernetzung; Netz-Eintrag im Manifest nach der Vernetzung.
2. Hintergrund-Ablauf umsetzen: Vernetzung ohne gesperrte Sitzung, Manifest-Änderungen über eine Warteschlange der Sitzung.
3. Fortschritt der Vernetzung aus dem Protokoll ableiten (Parser mit aufgezeichneten Logs testen).
4. Monitor-Zustand aus `RunPanel` in einen dauerhaften Speicher verlagern (Vitest zuerst).
5. Konsole und neuer Aufbau der Schritte "Szenario" und "Rechnen".
6. `runtime_`-Test: Beispiel rechnet ohne Netz in einem Zug, Abbruch während der Vernetzung räumt auf.
7. `docs/architecture.md` und `docs/mcp.md` anpassen.

## Tests

Alle Tests dieses Tasks werden permanent in `tests/` im Hauptverzeichnis des Projekts abgelegt (siehe `AGENTS.md`).

- Rust: Start ohne Netz kehrt sofort zurück und meldet Stufen der Vernetzung als Ereignisse; ein zweiter Befehl wird währenddessen beantwortet.
- Rust: Abbruch während der Vernetzung endet mit `run.cancelled`, es bleibt kein Container zurück.
- Rust: Parser für Phasen und Fortschritt der Vernetzung gegen aufgezeichnete Logs.
- Vitest: Monitor behält Stufen, Kurven und Ergebnis über einen Wechsel des Schritts; Konsole klappt auf und zu und zeigt das Protokoll.
- `runtime_`: Windkanal-Beispiel in Entwurfsqualität von "kein Netz" bis zum Ergebnis mit einem `run.start`; Abbruch in der Vernetzung.

## Akzeptanzkriterien

Der Task gilt erst als fertig, wenn alle Punkte erfüllt und durch Tests oder einen konkreten Befehl belegt sind:

- [x] Ein Klick auf "Rechnung starten" vernetzt und rechnet; während der Vernetzung sind Stufe, Phase und Restzeit sichtbar
- [x] Die Vernetzung lässt sich abbrechen
- [x] Nach "Rechnen → Ergebnisse → Rechnen" sind Stufen, Kurven und Ergebnis noch da und aktuell
- [x] Die Konsole zeigt das Protokoll, lässt sich einklappen und stört im eingeklappten Zustand nicht
- [x] Der Schritt "Szenario" enthält keinen Netz-Knopf mehr

## Offene Fragen / Annahmen

- Annahme: Die Restzeit der Vernetzung ist eine Schätzung aus Zellbudget und Phase; sie wird als "etwa" angezeigt.
- Annahme: `mesh.generate` bleibt als blockierender Befehl für CLI und MCP erhalten.

## Notizen zu diesem Task

Taskspezifische Informationen während der Arbeit hier festhalten. Taskübergreifendes gehört in `Roadmap/README.md`.

- Belege:
  - `cargo test -p foamcraft-api --test pipeline` (FakeEngine mit anhaltbarer Vernetzung): Start antwortet sofort, andere Befehle werden währenddessen beantwortet, Stufen/Phasen/Fortschritt als Ereignisse, Protokoll der Vernetzung über `run.log`, Abbruch räumt Container und Volume ab, `mesh_only`, Netz wird wiederverwendet bzw. bei geänderter Richtung oder verschobenem Modell neu erzeugt, verlorener Lauf nach App-Ende ergibt `run.lost`.
  - `cargo test -p foamcraft-sim --test mesh_progress` (Schätzung gegen aufgezeichnetes snappyHexMesh-Protokoll, nie rückwärts).
  - `cargo test -p foamcraft-core --test commands` (Änderungen aus Hintergrundarbeit werden vor dem nächsten Befehl angewendet, kein Undo-Schritt).
  - Vitest `tests/app/run-monitor.test.tsx`: Vernetzung mit Phase, Prozent, Restzeit und Abbruch; Zustand bleibt über Aushängen/Einhängen des Schritts; "Nur vernetzen"; Konsole; Szenario-Schritt ohne Netz-Knopf. Die alten Tests in `run-panel.test.tsx` und `mesh-section.test.tsx` laufen unverändert.
  - `runtime_` (echtes OpenFOAM): `cargo test -p foamcraft-api --test pipeline -- --ignored runtime_` (Abbruch in der Vernetzung, kein Container und kein Volume bleiben zurück, 8 s) und `--test samples -- --ignored runtime_ --test-threads=1` (beide Beispiele von "kein Netz" bis zum Ergebnis mit einem `run.start`, 490 s).
  - Gesamt: `cargo test --workspace` (97 Test-Programme grün), `npm test` (22 Dateien, 92 Tests), fmt, clippy, lint, typecheck, format:check, `cargo xtask license-check`.
  - Live (2026-11-21, App mit eigenem Daten- und Projektordner): Beispiel mit einem Klick geöffnet, Szenario fertig ausgewählt und ohne Netz-Knopf, "Rechnung starten": Oberfläche bleibt bedienbar, Stufen "Grundnetz → Körpernetz" mit Balken "Netz um das Modell verfeinern · 50 % · noch etwa 1 min", später "Grenzschichten aufbauen · 72 %", Konsole mit laufendem Protokoll (`Roadmap/Nachweise/task23_vernetzung_fortschritt_konsole.png`); danach Löser mit Kurven, Zellzahl 538 299 aus dem neuen Netz und Konsole (`task23_rechnen_nach_vernetzung.png`); Lauf endete stabil mit Cd 0,255. Startseite ohne Ordnerwahl: `task22_startseite_ohne_ordnerwahl.png`.
- Nicht von Hand geprüft: der Wechsel "Rechnen → Ergebnisse → Rechnen" in der laufenden App. Der Nutzer arbeitete währenddessen am Rechner; Mausklicks per Skript hätten in sein Fenster treffen können, deshalb wurde ab da nur noch berührungslos aufgenommen (`tools/windows/app-print.ps1`). Das Verhalten ist durch den Vitest-Test belegt (gemeinsamer Monitor, Panel aus- und wieder eingehängt).
- Zusätzlich gefunden und behoben: Ein vorhandenes Netz wurde nie verworfen, wenn sich Geometrie oder Szenario änderten. Solange der Nutzer das Netz von Hand erzeugte, fiel das nicht auf; mit automatischer Vernetzung wäre mit einem alten Netz gerechnet worden. Jetzt merkt sich ein Netz seine Eingaben (`runs/<Netz-Job>/mesh_inputs.json`).
- Zusätzlich: Zwischen Start und erster Stufe (Container starten, Fall kopieren) zeigte die Oberfläche nur "Läuft"; jetzt "Lauf wird vorbereitet …".
- Port 1420 war durch einen Entwicklungsserver eines anderen Projekts des Nutzers belegt; die App wurde mit `tauri dev --config <Datei mit devUrl/beforeDevCommand auf Port 1431>` gestartet, ohne den fremden Server anzufassen.

## Abschluss-Checkliste

Reihenfolge und Details stehen in `AGENTS.md`, Abschnitt "Abschluss- und Archivierungs-Workflow".

- [x] Alle Akzeptanzkriterien erfüllt, Tests liegen permanent in `tests/` und laufen grün
- [x] Abschnitt "Erkenntnisse für Folge-Agenten" unten ausgefüllt
- [x] Folge-Tasks und `Roadmap/README.md` geprüft und bei Bedarf angepasst
- [x] Datei ohne `RUN_` nach `Roadmap/Archiv/` verschoben

## Erkenntnisse für Folge-Agenten

Vor der Archivierung vom Agenten auszufüllen. Schreibe so, dass ein Agent, der diesen Task nie gesehen hat, davon bei späteren Tasks profitiert:

- **Umgesetzt (und Abweichungen vom Plan):** `run.start` als Hintergrund-Ablauf (`Pipeline` in `crates/foamcraft-api/src/run.rs`): Lauf-Eintrag sofort, dann Vernetzung (falls nötig), Löser, Auswertung; `mesh_only`; Vernetzung ohne Sitzung (`mesh::mesh_project`, `MeshRun`), Ereignisse `mesh.stage`/`mesh.progress`/`mesh.ready`, Fortschritts-Schätzung `foamcraft_sim::mesh_progress`, `Session::updates()` für Änderungen aus Hintergrundarbeit, Netz-Gültigkeit über `mesh_inputs.json`. Oberfläche: `RunMonitor` (`app/src/run-monitor.ts`), `RunPanel` rendert nur noch, `ConsoleDock`, Szenario-Schritt ohne Netz. Abweichung: Die Phase wird nicht als eigenes Ereignis `mesh.phase` gesendet, sondern steht in `mesh.progress`.
- **Learnings & Stolpersteine:** `api_call` hält die Sitzung für die Dauer eines Befehls gesperrt: Alles, was Container arbeiten lässt, gehört in einen Hintergrund-Thread mit Projektkopie. Hintergrundarbeit darf das Manifest nicht selbst speichern (die Kopie ist veraltet, sobald der Nutzer weiterarbeitet), sondern reicht Änderungen über `Updates` ein. `RunPanel` darf nicht von der Identität von `subscribe` abhängen (Tests übergeben eine neue Funktion je Render). PowerShell: `"$var: "` ist ein Syntaxfehler, `"${var} "` schreiben. Fenster ohne Fokus und ohne Maus aufnehmen: `tools/windows/app-print.ps1` (PrintWindow, zeigt auch die 3D-Ansicht).
- **Wichtige Entscheidungen und ihre Gründe:** Die Kennung des Laufs ist die des Löser-Jobs und wird vor der Vernetzung angelegt, damit Abbruch, Status und Protokoll von Anfang an eine Adresse haben; die Vernetzungs-Versuche sind eigene Jobs (`runs/<Lauf>/mesh_jobs`). Nach einem App-Ende während der Vernetzung wird nicht fortgesetzt, sondern sauber als verloren gemeldet. Ein Netz gilt nur für exakt seine Eingaben (auch eine andere Geschwindigkeit vernetzt neu): lieber einmal zu oft vernetzen als mit falschem Netz rechnen.
- **Auswirkungen auf Folge-Tasks:** Task_25/26/27 (Griffe): Vorschau ohne Befehl, am Ende ein Befehl – das Netz wird durch die Änderung automatisch ungültig. Task_29: `App` fragt beim Laden nicht nach einem bereits geöffneten Projekt (nach einem Neuladen der Seite steht die Startseite da, obwohl die Sitzung ein Projekt hat); beim Umbau beheben. Die Werkzeugleiste bricht bei 1280 px um. Task_30: Netz-Schlüssel verfeinern (Geschwindigkeit allein braucht kein neues Netz), Warteschlange auf `Pipeline` aufbauen. Task_33: Die Zeitschätzung ("etwa 1 min") lag beim Beispiel um ein Mehrfaches zu niedrig (Docker mit 1,9 GiB).
- **Hinweise zu Tests und Befehlen:** siehe Belege. Eine Vernetzung im Test anhalten: eigener `Engine` um `FakeEngine`, der für `tail … mesh` `exit: null` liefert (`GatedEngine` in `tests/api/pipeline.rs`).
