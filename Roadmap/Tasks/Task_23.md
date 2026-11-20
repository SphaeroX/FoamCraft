# Task_23: Leistungsmessung und CPU-Tuning

> ## ⚠️ PFLICHT – BEVOR DU IRGENDETWAS ANDERES TUST
>
> 1. **Lies als Allererstes die Datei `AGENTS.md` im übergeordneten Ordner `Roadmap/`** (von hier: `../AGENTS.md`) und befolge sie strikt.
> 2. **Deine allererste Aktion** ist, diese Datei in `RUN_Task_23.md` umzubenennen, um den Task als laufend zu markieren. Das passiert **noch bevor irgendein Code geschrieben** wird.

## Ziel

Es gibt eine wiederholbare Leistungsmessung, und die Standardwerte für Kerne, Zerlegung und Löser sind so eingestellt, dass FoamCraft auf der vorhandenen CPU messbar schneller rechnet als mit den bisherigen Vorgaben.

## Kontext & Abhängigkeiten

- **Voraussetzung:** Task_13 ist archiviert (beide Grundfälle mit Validierungstests); Task_14 bis Task_22 liegen davor.
- **Relevantes Vorwissen:** Referenzrechner: i9-13900HX mit 8 P-Kernen und 16 E-Kernen, 64 GB. Hybrid-CPUs verhalten sich bei OpenFOAM anders als gleichförmige Kerne. Die Validierungstests aus Task_10 und Task_13 dürfen sich nicht verschlechtern.

## Umfang

**Gehört dazu:**

- Befehl `foamcraft bench` mit festen Fällen (Außenströmung klein und mittel, Innenströmung): Zeit je Stufe, Durchsatz, Speicher; Ablage und Vergleich mit einer Basislinie
- Untersuchung: Kernzahl und Kernauswahl (P- und E-Kerne, Bindung), Zerlegungsverfahren, Umnummerierung des Netzes, Löser- und Vorkonditionierer-Einstellungen, Schreibformat, parallele Vernetzung, Ressourcen des Containers
- Übernahme nur von Maßnahmen, die messbar gewinnen
- Hardware-abhängige Standardwerte
- `docs/performance.md` mit Messwerten und Schlussfolgerungen

**Gehört nicht dazu:**

- GPU-Löser (Task_24), adaptive Verfeinerung (Task_25)

## Umsetzungsschritte

1. Tests zuerst für die Messung (Fake-Runner) und die Kernauswahl-Logik.
2. `foamcraft bench` umsetzen und eine Basislinie aufzeichnen.
3. Die Maßnahmen einzeln messen und jeweils Gewinn oder Befund festhalten.
4. Gewinnbringende Maßnahmen als Standardwerte übernehmen.
5. Validierungstests erneut laufen lassen.
6. `docs/performance.md` schreiben.

## Tests

Alle Tests dieses Tasks werden permanent in `tests/` im Hauptverzeichnis des Projekts abgelegt (siehe `AGENTS.md`).

- Die Messung erfasst Zeiten je Stufe korrekt (Fake-Runner).
- Kernauswahl für Beispiel-Topologien: 8 P-Kerne plus 16 E-Kerne, 16 gleiche Kerne, 4 Kerne.
- Die `validation_`-Tests bleiben grün.
- Ignorierter Regressionstest: Messung gegen die Basislinie mit Toleranz.

## Akzeptanzkriterien

Der Task gilt erst als fertig, wenn alle Punkte erfüllt und durch Tests oder einen konkreten Befehl belegt sind:

- [ ] `foamcraft bench` liefert einen wiederholbaren Bericht
- [ ] `docs/performance.md` nennt je Maßnahme den gemessenen Gewinn oder den Befund "kein Gewinn"
- [ ] Die Validierungstests laufen mit den neuen Standardwerten grün

## Offene Fragen / Annahmen

- Kein Gewinn ist ein zulässiges Ergebnis einer Maßnahme, solange es gemessen und dokumentiert ist.

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
