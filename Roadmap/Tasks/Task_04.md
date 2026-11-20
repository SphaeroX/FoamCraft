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

- [ ] `foamcraft project new <ordner>` erzeugt ein Projekt, das `foamcraft project info` wieder liest
- [ ] `foamcraft command list` nennt alle Befehle; jeder hat ein Schema
- [ ] Rückgängig und Wiederholen sind für alle zustandsändernden Befehle durch Tests belegt
- [ ] Die App legt über die Startseite ein Projekt an und öffnet es wieder

## Offene Fragen / Annahmen

- Annahme: Das Projekt ist ein Ordner mit Manifest. Eine einzelne gepackte Datei kann später folgen.

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
