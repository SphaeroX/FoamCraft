# Task_04: Projektmodell und Befehls-API

> ## ⚠️ PFLICHT – BEVOR DU IRGENDETWAS ANDERES TUST
>
> 1. **Lies als Allererstes die Datei `AGENTS.md` im übergeordneten Ordner `Roadmap/`** (von hier: `../AGENTS.md`) und befolge sie strikt.
> 2. **Deine allererste Aktion** ist, diese Datei in `RUN_Task_04.md` umzubenennen, um den Task als laufend zu markieren. Das passiert **noch bevor irgendein Code geschrieben** wird.

## Ziel

Projekte lassen sich anlegen, speichern und öffnen. Jede Aktion ist ein Befehl mit Schema und lässt sich rückgängig machen; CLI und App nutzen dieselbe Befehls-API. Das ist die Grundlage dafür, dass Oberfläche, CLI und später der MCP-Server gleich viel können.

## Kontext & Abhängigkeiten

- **Voraussetzung:** Task_01 bis Task_03 sind archiviert.
- **Relevantes Vorwissen:** `docs/architecture.md`, Regeln 1 (Everything is a command), 2 (Headless first), 5 (SI-Einheiten); `docs/conventions.md` (Benennung von Befehlen und Fehlercodes).

## Umfang

**Gehört dazu:**

- Projekt als Ordner mit versioniertem JSON-Manifest und Platz für Geometrie, Läufe und Ergebnis-Cache; Schema-Version mit Haken für spätere Migrationen
- Befehlsregister: Name, JSON-Schema der Parameter, Ergebnis, mögliche Fehlercodes
- Ausführung mit Historie für Rückgängig und Wiederholen, Ereignisse bei Zustandsänderungen
- Typen für physikalische Größen in SI-Einheiten
- Die Lauf-Befehle aus Task_03 als Befehle (`run.start`, `run.cancel`, `run.status`)
- CLI: `foamcraft project new|open|info` und `foamcraft command list|describe|run`
- Brücke in die App: Die Oberfläche ruft Befehle auf und abonniert Zustandsereignisse
- Startseite der App: neues Projekt, öffnen, zuletzt geöffnet

**Gehört nicht dazu:**

- Geometrie-Befehle (Task_05)
- MCP-Server (Task_18)
- Gepackte Projektdatei oder Autosave-Versionen

## Umsetzungsschritte

1. Tests zuerst für Speichern und Öffnen (Roundtrip, unbekannte höhere Schema-Version).
2. Projektmodell und Manifest umsetzen.
3. Tests zuerst für Befehlsregister, Schema-Prüfung, Rückgängig und Wiederholen.
4. Register, Ausführung und Historie umsetzen.
5. Lauf-Befehle aus Task_03 einhängen.
6. CLI-Befehle ergänzen.
7. Brücke in die App und Startseite umsetzen (Vitest zuerst).
8. `docs/architecture.md` und `docs/conventions.md` um das Projektformat und die Befehlsregeln ergänzen.

## Tests

Alle Tests dieses Tasks werden permanent in `tests/` im Hauptverzeichnis des Projekts abgelegt (siehe `AGENTS.md`).

- Roundtrip: Ein gespeichertes Projekt wird identisch geöffnet; eine höhere Schema-Version liefert einen Fehlercode.
- Befehle: Ungültige Parameter liefern einen Schemafehler; Rückgängig und Wiederholen stellen den Zustand exakt her.
- CLI: `command describe` gibt das JSON-Schema aus; `project new` legt ein gültiges Projekt an.
- Vitest: Die Startseite ruft `project.new` auf und zeigt zuletzt geöffnete Projekte.

## Akzeptanzkriterien

Der Task gilt erst als fertig, wenn alle Punkte erfüllt und durch Tests oder einen konkreten Befehl belegt sind:

- [x] `foamcraft project new <ordner>` erzeugt ein Projekt, das `foamcraft project info` wieder liest
- [x] `foamcraft command list` nennt alle Befehle; jeder hat ein Schema
- [x] Rückgängig und Wiederholen sind für alle zustandsändernden Befehle durch Tests belegt
- [x] Die App legt über die Startseite ein Projekt an und öffnet es wieder

## Offene Fragen / Annahmen

- Annahme: Das Projekt ist ein Ordner mit Manifest. Eine einzelne gepackte Datei kann später folgen.

## Notizen zu diesem Task

Taskspezifische Informationen während der Arbeit hier festhalten. Taskübergreifendes gehört in `Roadmap/README.md`.

- Nachweis App (2026-11-21): Startseite → Name "Demo" → "Create project" → Windows-Ordnerdialog → Projekt angelegt (`Roadmap/Nachweise/task04_projekt_angelegt.png`); "Close project" → Startseite mit "Demo" unter "Recently opened" (`task04_startseite_zuletzt.png`) → Klick öffnet es wieder.
- Nachweis CLI: `tests/cli/commands.rs` (`project new` + `project info`, `command list`, `command describe` mit Schema, `command run` ändert das Manifest, ungültige Parameter → `command.invalid_params`).
- Rückgängig/Wiederholen: `undo_and_redo_restore_state_exactly_for_every_undoable_command` läuft über alle Befehle mit `undoable: true` (derzeit `project.rename`, `project.set_display_units`).

## Abschluss-Checkliste

Reihenfolge und Details stehen in `AGENTS.md`, Abschnitt "Abschluss- und Archivierungs-Workflow".

- [x] Alle Akzeptanzkriterien erfüllt, Tests liegen permanent in `tests/` und laufen grün
- [x] Abschnitt "Erkenntnisse für Folge-Agenten" unten ausgefüllt
- [x] Folge-Tasks und `Roadmap/README.md` geprüft und bei Bedarf angepasst
- [x] Datei ohne `RUN_` nach `Roadmap/Archiv/` verschoben

## Erkenntnisse für Folge-Agenten

- **Umgesetzt (und Abweichungen vom Plan):** `foamcraft_core::{project, units, command}` (Registry, Session, EventBus, History, zuletzt geöffnete Projekte), neues Crate `foamcraft-api` (Registry inkl. `run.start|status|cancel`), CLI `project new|open|info` und `command list|describe|run`, Tauri-Brücke `api_call` + Ereignis `foamcraft://event`, Startseite in React. Zusätzlich zu den geplanten Befehlen: `project.close`, `project.recent`, `project.rename`, `project.set_display_units`, `history.state`. Abweichung: Rückgängig arbeitet mit Manifest-Schnappschüssen statt mit Umkehrbefehlen (einfacher und exakt; Dateien in `geometry/` werden dabei nicht gelöscht – Task_05 muss entscheiden, ob Importe ihre Dateien beim Rückgängig behalten).
- **Learnings & Stolpersteine:** schemars 1.x: `schemars::schema_for!(T)` → `serde_json::to_value`. clippy verbietet eine Enum-Variante `Command` im Enum `Command` (`#[command(name = "command")] Api`). Tauri-Befehle mit großem Fehlertyp: `Box<ApiError>` (clippy `result_large_err`). Für Vitest-DOM-Matcher `@testing-library/jest-dom/vitest` über `tests/app/setup.ts`. Der Windows-Ordnerdialog lässt sich per SendKeys bedienen: Pfad ins Feld "Ordner:" + Enter navigiert hinein, dann "Ordner auswählen" klicken. Hilfsskripte liegen jetzt in `tools/windows/`.
- **Wichtige Entscheidungen und ihre Gründe:** Befehls-Register im Kern (ohne Docker-Abhängigkeit), Lauf-Befehle im Crate `foamcraft-api`, weil der Kern das Runtime-Crate nicht kennen darf. Parameterprüfung über serde + `deny_unknown_fields` statt eines JSON-Schema-Validators (das Schema wird aus denselben Typen erzeugt). Jobs eines Projekts liegen in `<projekt>/runs/`; `foamcraft run` ohne Projekt nutzt weiter `%LOCALAPPDATA%FoamCraftjobs`.
- **Auswirkungen auf Folge-Tasks:** Neue Funktionen zuerst als Befehl in `foamcraft-core` (oder `foamcraft-api`, wenn Docker nötig) registrieren, dann in der App über `tauriApi.call` nutzen. Task_05 erweitert das Manifest (Geometrie) → `SCHEMA_VERSION` erhöhen oder Felder mit `#[serde(default)]` ergänzen; Migrationsschritt in `project::migrate`. Task_11 kann die `run.*`-Ereignisse direkt anzeigen. Task_18 (MCP) kann `Registry::specs()` für die Werkzeugliste nutzen.
- **Hinweise zu Tests und Befehlen:** `cargo test -p foamcraft-core --test commands --test project --test units`, `cargo test -p foamcraft-api`, `cargo test -p foamcraft-cli --test cli_commands`, `npx vitest run tests/app/start-page.test.tsx`. CLI-Tests setzen `FOAMCRAFT_DATA_DIR`, damit die echte Liste "zuletzt geöffnet" unberührt bleibt.
