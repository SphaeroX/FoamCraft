# Task_05: Import von STL, OBJ und 3MF mit Diagnose

> ## ⚠️ PFLICHT – BEVOR DU IRGENDETWAS ANDERES TUST
>
> 1. **Lies als Allererstes die Datei `AGENTS.md` im übergeordneten Ordner `Roadmap/`** (von hier: `../AGENTS.md`) und befolge sie strikt.
> 2. **Deine allererste Aktion** ist, diese Datei in `RUN_Task_05.md` umzubenennen, um den Task als laufend zu markieren. Das passiert **noch bevor irgendein Code geschrieben** wird.

## Ziel

Der Nutzer lädt ein Dreiecksmodell, legt Einheit, Größe und Lage fest und sieht Probleme der Geometrie in Klartext. Das Modell erscheint in der 3D-Ansicht.

## Kontext & Abhängigkeiten

- **Voraussetzung:** Task_04 ist archiviert (Projektmodell, Befehls-API).
- **Relevantes Vorwissen:** `docs/project-brief.md` (CAD-Bearbeitung ist ein Nicht-Ziel), `docs/testing.md` (Testdaten selbst erstellt, CC0, klein).

## Umfang

**Gehört dazu:**

- Leser für STL (ASCII und binär), OBJ und 3MF; Befehl `geometry.import`; Kopie der Datei ins Projekt
- Diagnose: wasserdicht oder offen (Zahl der Randschleifen), umgedrehte Normalen, doppelte und entartete Dreiecke, Zahl der Körper, Abmessungen
- Einheit: Vorschlag aus der Modellgröße (Millimeter oder Meter) mit Rückfrage
- Transformationen: Skalieren, Drehen (Gier, Nick, Roll), Verschieben, "auf den Boden setzen"
- Einfache Reparatur: Normalen einheitlich ausrichten
- Ansicht: Modell anzeigen, einpassen, Achsenkreuz, Abmessungen
- Schritt "Geometrie" in der Oberfläche mit Diagnoseliste in beiden Sprachen

**Gehört nicht dazu:**

- Flächen erkennen und anklicken (Task_06)
- STEP und IGES (Task_07)
- Löcher schließen (Task_12) und jede weitergehende Reparatur

## Umsetzungsschritte

1. Kleine Testdateien selbst erzeugen (Würfel binär und ASCII, OBJ, 3MF, offener Würfel, Würfel mit umgedrehten Normalen) und unter `tests/data/` ablegen.
2. Tests zuerst für die Leser, dann umsetzen.
3. Tests zuerst für die Diagnose, dann umsetzen.
4. Transformationen und Einheitenvorschlag umsetzen.
5. Modell an den Renderer übergeben, Einpassen und Achsenkreuz ergänzen.
6. Schritt "Geometrie" in der Oberfläche umsetzen (Vitest zuerst).

## Tests

Alle Tests dieses Tasks werden permanent in `tests/` im Hauptverzeichnis des Projekts abgelegt (siehe `AGENTS.md`).

- Leser: Jedes Format liefert für den Würfel 12 Dreiecke und die richtigen Abmessungen; eine defekte Datei liefert einen Fehlercode.
- Diagnose: Der offene Würfel hat eine Randschleife; umgedrehte Normalen werden erkannt.
- Transformation: Nach Drehen und "auf den Boden setzen" liegt die Unterkante bei z = 0.
- Leistung: Ein erzeugtes Modell mit 1 Million Dreiecken lädt innerhalb einer festgelegten Zeitgrenze.
- Golden Image des geladenen Würfels; Vitest für die übersetzte Diagnoseliste.

## Akzeptanzkriterien

Der Task gilt erst als fertig, wenn alle Punkte erfüllt und durch Tests oder einen konkreten Befehl belegt sind:

- [ ] `geometry.import` lädt STL, OBJ und 3MF und liefert eine Diagnose als strukturierte Antwort
- [ ] Jede Diagnosemeldung hat einen englischen und einen deutschen Text
- [ ] Einheit, Drehung und Lage lassen sich per Befehl und in der Oberfläche setzen und bleiben im Projekt gespeichert
- [ ] Das Modell ist in der App sichtbar (Bildschirmfoto in den Notizen)

## Offene Fragen / Annahmen

- Annahme: Modelle bis etwa 5 Millionen Dreiecke. Eine vereinfachte Anzeige für größere Modelle folgt nur bei Bedarf.

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
