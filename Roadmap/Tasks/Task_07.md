# Task_07: Import von STEP und IGES

> ## ⚠️ PFLICHT – BEVOR DU IRGENDETWAS ANDERES TUST
>
> 1. **Lies als Allererstes die Datei `AGENTS.md` im übergeordneten Ordner `Roadmap/`** (von hier: `../AGENTS.md`) und befolge sie strikt.
> 2. **Deine allererste Aktion** ist, diese Datei in `RUN_Task_07.md` umzubenennen, um den Task als laufend zu markieren. Das passiert **noch bevor irgendein Code geschrieben** wird.

## Ziel

CAD-Dateien werden mit ihren echten Flächen geladen: Jede CAD-Fläche ist eine anklickbare Fläche. Der Import funktioniert auch ohne Fenster.

## Kontext & Abhängigkeiten

- **Voraussetzung:** Task_06 ist archiviert (Flächen, Kennungen, Auswahl).
- **Relevantes Vorwissen:** `docs/architecture.md`, offene Entscheidung "STEP and IGES import"; `docs/licensing.md` (LGPL ist erlaubt). Der Nutzer kann Testdateien mit SolidCraft erzeugen.

## Umfang

**Gehört dazu:**

- Technikentscheidung per kurzem Vergleich: Open CASCADE nativ gelinkt, als WebAssembly oder Konvertierung im Container. Kriterien: Build unter Windows, Betrieb ohne Fenster, Lizenz, Paketgröße
- Import von STEP (AP203, AP214, AP242) und IGES
- Tessellierung mit einer Genauigkeit, die zur Modellgröße passt
- Flächen-Kennungen, Namen und Farben aus der Datei, soweit vorhanden; Einheiten aus der Datei; Baugruppen als Teile
- Dieselbe Diagnose wie in Task_05

**Gehört nicht dazu:**

- CAD-Bearbeitung (Nicht-Ziel des Projekts)
- Reparatur defekter CAD-Daten

## Umsetzungsschritte

1. Die drei Wege kurz erproben und die Entscheidung mit Begründung in `docs/architecture.md` festhalten.
2. Testdateien bereitstellen (Würfel, Rohr mit Flansch als STEP, eine IGES-Datei), selbst erstellt und CC0.
3. Tests zuerst für Flächenzahl, Einheiten und Teile.
4. Import und Tessellierung umsetzen.
5. In `geometry.import` einhängen, Diagnose anbinden.
6. Betrieb ohne Fenster über die CLI prüfen.

## Tests

Alle Tests dieses Tasks werden permanent in `tests/` im Hauptverzeichnis des Projekts abgelegt (siehe `AGENTS.md`).

- STEP-Würfel ergibt 6 Flächen, das Rohr mit Flansch die erwartete Flächenzahl.
- Eine Datei in Millimetern wird korrekt in Meter umgerechnet.
- Die IGES-Datei wird geladen; eine defekte Datei liefert einen Fehlercode.
- Import über die CLI ohne Fenster.

## Akzeptanzkriterien

Der Task gilt erst als fertig, wenn alle Punkte erfüllt und durch Tests oder einen konkreten Befehl belegt sind:

- [ ] `geometry.import` lädt STEP und IGES mit anklickbaren CAD-Flächen
- [ ] Der Import läuft über die CLI ohne Fenster
- [ ] Die Technikentscheidung steht mit Begründung in `docs/architecture.md`; die Lizenzprüfung läuft weiter grün

## Offene Fragen / Annahmen

- Offene Entscheidung "STEP and IGES import" fällt in diesem Task.
- Annahme: Sehr große Baugruppen werden nicht optimiert.

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
