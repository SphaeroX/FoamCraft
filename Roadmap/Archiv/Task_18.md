# Task_18: MCP-Server

> ## ⚠️ PFLICHT – BEVOR DU IRGENDETWAS ANDERES TUST
>
> 1. **Lies als Allererstes die Datei `AGENTS.md` im übergeordneten Ordner `Roadmap/`** (von hier: `../AGENTS.md`) und befolge sie strikt.
> 2. **Deine allererste Aktion** ist, diese Datei in `RUN_Task_18.md` umzubenennen, um den Task als laufend zu markieren. Das passiert **noch bevor irgendein Code geschrieben** wird.

## Ziel

Ein KI-Agent richtet über MCP ein Projekt ein, startet einen Lauf, wertet ihn aus und bekommt Bilder der 3D-Ansicht. Der Server arbeitet ohne Fenster oder verbunden mit der laufenden App, in der der Nutzer die Schritte mitverfolgt.

## Kontext & Abhängigkeiten

- **Voraussetzung:** Task_17 ist archiviert (stabile Kennzahlen-Antwort); die Befehls-API aus Task_04 trägt alle bisherigen Funktionen.
- **Aus Task_17:** `results.summary` (Schema 1, Texte en/de je Kennzahl, Ampel mit Begründungen), `results.compare`, `results.export`, `runs.list` und `view.set_compare` existieren als Befehle; die kuratierten Ergebnis-Werkzeuge sollen sie direkt nutzen.
- **Relevantes Vorwissen:** `docs/architecture.md` (MCP mit dem offiziellen Rust-SDK über stdio; Regel 1 und 2), `docs/conventions.md` (Pfade nur innerhalb erlaubter Wurzeln).

## Umfang

**Gehört dazu:**

- `foamcraft mcp` als stdio-Server
- Kuratierte Werkzeuge mit guten Beschreibungen: Projekt, Geometrie (laden, prüfen, auswählen, Öffnung verschließen), Szenario, Lauf (starten, Status, warten, abbrechen), Ergebnisse (Kennzahlen, Wert abfragen, vergleichen), Ansicht rendern (Bild als Antwort); dazu ein generischer Zugang zu allen Befehlen (auflisten, beschreiben, ausführen)
- Lange Läufe: sofortige Antwort mit Job-Kennung, Warten mit Zeitlimit
- Betrieb ohne Fenster (eigene Sitzung, Offscreen-Renderer) oder verbunden mit der laufenden App
- Pfad-Sandbox: nur Wurzeln, die beim Start übergeben wurden
- Server-Anweisungen und ein Leitfaden als Ressource: so richtet man eine Simulation ein, so prüft man die Plausibilität
- Antworten kompakt, in SI-Einheiten, mit Fehlercodes
- Einrichtungsanleitung in `docs/` und eine Einstellungsseite in der App, die die MCP-Konfiguration zum Kopieren zeigt

**Gehört nicht dazu:**

- HTTP-Transport
- Report-Werkzeug (kommt mit Task_20), Vorschau-Werkzeug (Task_47)

## Umsetzungsschritte

1. Aktuellen Stand des Rust-SDK für MCP prüfen (Doku lesen, nicht aus dem Gedächtnis arbeiten).
2. Tests zuerst mit einem Test-Client: Werkzeugliste und gültige Schemata.
3. Server und Werkzeuge auf Basis des Befehlsregisters umsetzen.
4. Lange Läufe mit Job-Kennung und Warten umsetzen.
5. Pfad-Sandbox umsetzen.
6. Brücke zur laufenden App umsetzen; die Technik (lokaler Socket oder Named Pipe) begründen.
7. Leitfaden, Server-Anweisungen, Doku und Einstellungsseite schreiben.

## Tests

Alle Tests dieses Tasks werden permanent in `tests/` im Hauptverzeichnis des Projekts abgelegt (siehe `AGENTS.md`).

- Protokolltest: `tools/list` liefert alle Werkzeuge mit gültigem Schema.
- Ende-zu-Ende ohne Fenster mit dem Fake-Runner: Projekt, Import, Szenario, Lauf, Kennzahlen.
- Ein Pfad außerhalb der Wurzeln wird abgelehnt.
- Das Render-Werkzeug liefert ein PNG in der angeforderten Größe.
- Verbunden mit der App: Ein per MCP ausgeführter Befehl erscheint im Zustand der App.
- `runtime_`: ein kleiner echter Lauf über MCP.

## Akzeptanzkriterien

Der Task gilt erst als fertig, wenn alle Punkte erfüllt und durch Tests oder einen konkreten Befehl belegt sind:

- [x] Ein MCP-Client (zum Beispiel Claude Code) führt mit der dokumentierten Konfiguration einen vollständigen Windkanal-Lauf aus und erhält Kennzahlen und ein Bild
- [x] Alle Tests dieses Tasks laufen grün
- [x] Die Einrichtungsanleitung steht in `docs/` und ist in der Root-`AGENTS.md` verlinkt

## Offene Fragen / Annahmen

- Die Technik der Brücke zwischen CLI und App wird hier entschieden. → Entschieden: Loopback-TCP (127.0.0.1, freier Port) mit Zufalls-Token in `<Datenordner>/app-bridge.json`; Begründung in `docs/architecture.md`, Abschnitt "MCP server".

## Notizen zu diesem Task

Taskspezifische Informationen während der Arbeit hier festhalten. Taskübergreifendes gehört in `Roadmap/README.md`.

- SDK geprüft (2026-11-21): `rmcp` 3.11.0 (Apache-2.0); die API wurde aus dem Quellcode im Cargo-Registry gelesen (`ServerHandler` mit `get_info`, `list_tools`, `call_tool` → `CallToolResponse`, `list_resources`, `read_resource` → `ReadResourceResponse`). Lizenzprüfung: 767 Pakete, 0 nicht erlaubt.
- Belege:
  - `cargo test -p foamcraft-mcp` (Protokolltest mit rmcp-Client über eine In-Memory-Leitung; Werkzeugliste mit Objekt-Schemata; Ende-zu-Ende ohne Fenster mit Fake-Engine: Projekt, Import, Szenario, Lauf, Warten, Kennzahlen; Sandbox; Render-Werkzeug liefert PNG 320 × 200; verbunden mit einer App-Sitzung über die Brücke).
  - `cargo test -p foamcraft-api --test bridge` (Befehl ändert die Sitzung, Ereignis `project.changed`, falscher Token → `bridge.refused`, keine App → `bridge.no_app`).
  - `cargo test -p foamcraft-cli --test cli_mcp` (echte Binary `foamcraft mcp --root …` über stdio) und `-- --ignored runtime_`: vollständiger Windkanal-Lauf (Würfel 20 mm, Entwurf) über stdio mit der dokumentierten Kommandozeile, Kennzahlen und PNG; 219 s, grün.
  - Live mit der App (dev): `foamcraft mcp --app` öffnete "Demo" in der laufenden App, lieferte Kennzahlen und ein Bild (Nachweise `Roadmap/Nachweise/task18_app_ueber_mcp_geoeffnet.png`, `task18_mcp_view_render.png`, `task18_agenten_dialog.png`).
  - Vitest `tests/app/mcp-dialog.test.tsx` (Konfiguration zum Kopieren).
- Als Client für die Akzeptanz diente ein minimaler stdio-Client (Test und Node-Skript), nicht Claude Code selbst: Das hätte Projektdaten an einen externen Dienst geschickt. Die Konfiguration für Claude Code steht in `docs/mcp.md`.
- Gefundener Fehler: `run.status` meldete `succeeded`, bevor Ergebnis und Felder gespeichert waren (Auswertung läuft nach dem Solver). Neu: `busy` im Status; `run_wait` wartet darauf (Test `a_run_counts_as_done_only_when_its_result_is_stored`).

## Abschluss-Checkliste

Reihenfolge und Details stehen in `AGENTS.md`, Abschnitt "Abschluss- und Archivierungs-Workflow".

- [x] Alle Akzeptanzkriterien erfüllt, Tests liegen permanent in `tests/` und laufen grün
- [x] Abschnitt "Erkenntnisse für Folge-Agenten" unten ausgefüllt
- [x] Folge-Tasks und `Roadmap/README.md` geprüft und bei Bedarf angepasst
- [x] Datei ohne `RUN_` nach `Roadmap/Archiv/` verschoben

## Erkenntnisse für Folge-Agenten

Vor der Archivierung vom Agenten auszufüllen. Schreibe so, dass ein Agent, der diesen Task nie gesehen hat, davon bei späteren Tasks profitiert:

- **Umgesetzt (und Abweichungen vom Plan):** Neue Crate `foamcraft-mcp` (`Server` mit kuratierten Werkzeugen, je ein Befehl mit dessen Parameterschema; dazu `run_wait`, `view_render`, `commands_list`, `command_describe`, `command_run`; `McpHandler` für rmcp; Anweisungen und Leitfaden `foamcraft://guide`), `foamcraft mcp --root … [--app]`, Brücke `foamcraft_api::bridge` (App startet sie in `ApiState`), Dialog "Agenten" in der App mit kopierbarer Konfiguration (`cli_path` liefert den Pfad der CLI neben der App), Anleitung `docs/mcp.md`. Zusätzlich: `run.status` hat `busy`.
- **Learnings & Stolpersteine:** rmcp 3.x hat eine andere API als ältere Beispiele (`ServerConfig`, `CallToolResponse`, `ContentBlock`) – im Quellcode nachsehen. Werkzeugfehler als `CallToolResult::error` mit `{code, message, fix, detail}` zurückgeben, nicht als Protokollfehler (sonst sieht der Agent die Meldung nicht). Blockierende Befehle im Handler mit `spawn_blocking`. Ein hart beendeter App-Prozess hinterlässt `app-bridge.json`; Clients bekommen dann `bridge.no_app`, weil der Port nicht antwortet.
- **Wichtige Entscheidungen und ihre Gründe:** Warten macht der MCP-Server (Abfrage von `run.status`), kein Befehl – ein minutenlang blockierender Befehl würde die Sitzung der App sperren. Sandbox über Schlüsselnamen (`path`, `folder`, `file`, `dir`, `source`, `root`, `*_path`, `*_dir`), nur absolute Pfade in den Wurzeln. Kennzahlen in MCP-Antworten nur mit englischem Text (kompakt). Brücke: Loopback-TCP mit Token (plattformgleich, nur Standardbibliothek).
- **Auswirkungen auf Folge-Tasks:** Task_19 (Export) und Task_20 (Report) tragen ihre Werkzeuge in die Tabelle `CURATED` in `crates/foamcraft-mcp/src/lib.rs` ein (Name mit Unterstrich, Befehl, Beschreibung) und ergänzen `tests/mcp/tools.rs` sowie `docs/mcp.md`; Pfad-Parameter heißen `path` o. ä., damit die Sandbox greift. Task_36 muss `foamcraft.exe` neben die App installieren (der Dialog "Agenten" erwartet sie dort).
- **Hinweise zu Tests und Befehlen:** `cargo test -p foamcraft-mcp`, `cargo test -p foamcraft-api --test bridge`, `cargo test -p foamcraft-cli --test cli_mcp`, echter Lauf `cargo test -p foamcraft-cli --test cli_mcp -- --ignored runtime_` (~4 min). Für eigene Versuche: `target/release/foamcraft.exe mcp --root <Ordner>`.
