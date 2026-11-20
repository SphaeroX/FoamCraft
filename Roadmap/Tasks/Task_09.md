# Task_09: Automatische Vernetzung für die Außenströmung

> ## ⚠️ PFLICHT – BEVOR DU IRGENDETWAS ANDERES TUST
>
> 1. **Lies als Allererstes die Datei `AGENTS.md` im übergeordneten Ordner `Roadmap/`** (von hier: `../AGENTS.md`) und befolge sie strikt.
> 2. **Deine allererste Aktion** ist, diese Datei in `RUN_Task_09.md` umzubenennen, um den Task als laufend zu markieren. Das passiert **noch bevor irgendein Code geschrieben** wird.

## Ziel

Aus Geometrie und Windkanal-Szenario entsteht ohne Handarbeit ein rechenfähiges Netz innerhalb des Zellbudgets. Die Netzqualität wird geprüft; bei Problemen greifen Ausweichstufen.

## Kontext & Abhängigkeiten

- **Voraussetzung:** Task_03 (Job-Pipeline) und Task_08 (Windkanal-Szenario) sind archiviert.
- **Relevantes Vorwissen:** `docs/architecture.md`, Regel 4; Erkenntnisse aus Task_03 zu Log-Formaten und Stufen. Vernetzt wird mit blockMesh und snappyHexMesh aus OpenFOAM.

## Umfang

**Gehört dazu:**

- Erzeugung von Hintergrundnetz (Zellgröße aus dem Budget), Kantenerkennung und snappyHexMesh-Einstellungen aus dem Szenario
- Verfeinerung an der Oberfläche aus Modellgröße und kleinsten Merkmalen; Verfeinerungsbereiche für Nahfeld, Nachlauf und Boden
- Grenzschichten: Anzahl und Dicke aus der Geschwindigkeit, Ziel ist ein y+ für Wandfunktionen (30 bis 100)
- Punkt im Strömungsraum automatisch außerhalb des Körpers
- Flächengruppen aus Task_06 werden zu benannten Rändern
- Zellzahl-Schätzung vor dem Lauf und Regelung auf das Budget
- Auswertung von checkMesh zu einem Qualitätsurteil
- Ausweichleiter: weniger Schichten, keine Schichten, gröberes Netz
- Leck-Erkennung bei nicht wasserdichter Geometrie mit Klartextmeldung
- Netzstatistik im Ergebnis und eine einfache Netzvorschau in der Ansicht (Oberflächennetz)

**Gehört nicht dazu:**

- Vernetzung für die Innenströmung (Task_13)
- Lösungsadaptive Verfeinerung (Task_25)

## Umsetzungsschritte

1. Tests zuerst für die Berechnung von Zellgröße, Verfeinerungsstufen und Schichtdicke.
2. Vorlagen und Erzeugung der Netz-Dictionaries umsetzen (Snapshot-Tests).
3. Punkt im Strömungsraum und Zuordnung der Ränder umsetzen.
4. checkMesh-Auswertung und Qualitätsurteil umsetzen (Tests gegen aufgezeichnete Ausschnitte).
5. Budget-Regelung und Ausweichleiter umsetzen (Tests gegen den Fake-Runner).
6. Leck-Erkennung umsetzen.
7. In die Job-Pipeline einhängen und mit echten Körpern prüfen.
8. Netzvorschau und Netzstatistik ergänzen.

## Tests

Alle Tests dieses Tasks werden permanent in `tests/` im Hauptverzeichnis des Projekts abgelegt (siehe `AGENTS.md`).

- Snapshot-Tests der erzeugten Dictionaries für das Würfel-Szenario.
- Der Punkt im Strömungsraum liegt für alle Testkörper außerhalb des Körpers.
- Die Ausweichleiter wird bei einem erzwungenen Schichtfehler ausgelöst.
- `runtime_`: Würfel und Kugel werden vernetzt, checkMesh meldet keine Fehler, die Zellzahl liegt innerhalb von 30 % des Budgets.
- Der offene Würfel aus Task_05 führt zum Leck-Fehlercode.

## Akzeptanzkriterien

Der Task gilt erst als fertig, wenn alle Punkte erfüllt und durch Tests oder einen konkreten Befehl belegt sind:

- [ ] Ein Befehl vernetzt ein Projekt mit Windkanal-Szenario ohne weitere Eingaben
- [ ] Zellzahl, Qualitätsurteil und erreichte Schichten stehen als strukturierte Antwort bereit
- [ ] Die `runtime_`-Tests für Würfel und Kugel laufen grün
- [ ] Jede Ausweichstufe und der Leck-Fall haben einen Fehlercode oder Hinweis in beiden Sprachen

## Offene Fragen / Annahmen

- Annahme: snappyHexMesh genügt; ein zweiter Vernetzer ist nicht vorgesehen.
- Die Toleranz für das Treffen des Zellbudgets (30 %) ist eine Annahme und wird hier bestätigt oder begründet angepasst.

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
