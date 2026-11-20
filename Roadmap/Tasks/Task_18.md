# Task_18: MCP-Server

> ## ⚠️ PFLICHT – BEVOR DU IRGENDETWAS ANDERES TUST
>
> 1. **Lies als Allererstes die Datei `AGENTS.md` im übergeordneten Ordner `Roadmap/`** (von hier: `../AGENTS.md`) und befolge sie strikt.
> 2. **Deine allererste Aktion** ist, diese Datei in `RUN_Task_18.md` umzubenennen, um den Task als laufend zu markieren. Das passiert **noch bevor irgendein Code geschrieben** wird.

## Ziel

Ein KI-Agent richtet über MCP ein Projekt ein, startet einen Lauf, wertet ihn aus und bekommt Bilder der 3D-Ansicht. Der Server arbeitet ohne Fenster oder verbunden mit der laufenden App, in der der Nutzer die Schritte mitverfolgt.

## Kontext & Abhängigkeiten

- **Voraussetzung:** Task_17 ist archiviert (stabile Kennzahlen-Antwort); die Befehls-API aus Task_04 trägt alle bisherigen Funktionen.
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
- Report-Werkzeug (kommt mit Task_20), Vorschau-Werkzeug (Task_33)

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

- [ ] Ein MCP-Client (zum Beispiel Claude Code) führt mit der dokumentierten Konfiguration einen vollständigen Windkanal-Lauf aus und erhält Kennzahlen und ein Bild
- [ ] Alle Tests dieses Tasks laufen grün
- [ ] Die Einrichtungsanleitung steht in `docs/` und ist in der Root-`AGENTS.md` verlinkt

## Offene Fragen / Annahmen

- Die Technik der Brücke zwischen CLI und App wird hier entschieden.

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
