# Task_03: Job-Pipeline mit Referenzfall

> ## ⚠️ PFLICHT – BEVOR DU IRGENDETWAS ANDERES TUST
>
> 1. **Lies als Allererstes die Datei `AGENTS.md` im übergeordneten Ordner `Roadmap/`** (von hier: `../AGENTS.md`) und befolge sie strikt.
> 2. **Deine allererste Aktion** ist, diese Datei in `RUN_Task_03.md` umzubenennen, um den Task als laufend zu markieren. Das passiert **noch bevor irgendein Code geschrieben** wird.

## Ziel

Ein eingebauter Testfall (Würfel im Windkanal aus festen Vorlagen) wird mit `foamcraft run` vollständig vernetzt und parallel gerechnet. Fortschritt und Residuen kommen live, der Lauf lässt sich abbrechen. Damit steht die Pipeline, in die spätere Tasks ihre erzeugten Fälle einspeisen.

## Kontext & Abhängigkeiten

- **Voraussetzung:** Task_02 ist archiviert (Runner, Agent, Image).
- **Relevantes Vorwissen:** `docs/architecture.md`, Regeln 6 (Jobs) und 7 (Fehlercodes). Die Log-Formate hängen von der in Task_02 gewählten OpenFOAM-Version ab; die Erkenntnisse von Task_02 lesen.

## Umfang

**Gehört dazu:**

- Job-Modell mit Stufen (Hintergrundnetz, Vernetzung, Zerlegung, Lösung, Zusammenführung), Zustandsautomat, persistentes Log, Ereignisstrom
- Log-Auswertung für blockMesh, snappyHexMesh, checkMesh, decomposePar und simpleFoam (Iteration, Residuen, Laufzeit, Fehlermeldungen)
- Abbruch mit sauberem Aufräumen, ohne verwaiste Prozesse
- Wiederfinden eines laufenden Jobs nach einem Neustart des Hosts
- Feste Fallvorlage als Datendateien im Kern
- Wahl der Kernzahl (Standard: physische Kerne) und parallele Ausführung
- CLI `foamcraft run` mit Fortschrittsausgabe

**Gehört nicht dazu:**

- Fallerzeugung aus Nutzergeometrie (Task_09)
- Kräfte und Konvergenz-Automatik (Task_10)
- Anzeige in der App (Task_11)

## Umsetzungsschritte

1. Log-Ausschnitte der gewählten OpenFOAM-Version aufzeichnen (Erfolg, `FOAM FATAL ERROR`, Divergenz mit NaN) und als Testdaten ablegen.
2. Tests zuerst für die Log-Auswertung, dann umsetzen.
3. Tests zuerst für den Zustandsautomaten gegen den Fake-Runner (Erfolg, Fehler in einer Stufe, Abbruch), dann umsetzen.
4. Fallvorlage für den Würfel anlegen (klein, unter 200 000 Zellen).
5. Pipeline mit dem Docker-Runner verbinden, parallel rechnen lassen.
6. Abbruch und Wiederfinden umsetzen.
7. CLI und Fehlercodes ergänzen.

## Tests

Alle Tests dieses Tasks werden permanent in `tests/` im Hauptverzeichnis des Projekts abgelegt (siehe `AGENTS.md`).

- Log-Auswertung gegen die aufgezeichneten Ausschnitte: Residuen und Iterationen stimmen, Fehler werden mit dem richtigen Code erkannt.
- Zustandsautomat: Erfolg, Fehler, Abbruch, Wiederaufnahme.
- `runtime_`: Der Referenzfall läuft in unter 5 Minuten durch; ein Abbruch während der Lösung beendet alle Prozesse im Container.

## Akzeptanzkriterien

Der Task gilt erst als fertig, wenn alle Punkte erfüllt und durch Tests oder einen konkreten Befehl belegt sind:

- [ ] `foamcraft run` rechnet den Referenzfall bis zum Ende und zeigt Stufe, Iteration und Residuen
- [ ] Ein Abbruch hinterlässt keine laufenden OpenFOAM-Prozesse (durch einen `runtime_`-Test belegt)
- [ ] Ein absichtlich fehlerhafter Fall endet mit einem Fehlercode statt mit einem Absturz
- [ ] Das Log eines Jobs ist nach dem Lauf vollständig abrufbar

## Offene Fragen / Annahmen

- Annahme: Der Referenzfall dient nur der Pipeline und wird nicht validiert.

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
