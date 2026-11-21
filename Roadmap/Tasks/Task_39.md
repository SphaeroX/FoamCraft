# Task_39: Lösungsadaptive Netzverfeinerung

> ## ⚠️ PFLICHT – BEVOR DU IRGENDETWAS ANDERES TUST
>
> 1. **Lies als Allererstes die Datei `AGENTS.md` im übergeordneten Ordner `Roadmap/`** (von hier: `../AGENTS.md`) und befolge sie strikt.
> 2. **Deine allererste Aktion** ist, diese Datei in `RUN_Task_39.md` umzubenennen, um den Task als laufend zu markieren. Das passiert **noch bevor irgendein Code geschrieben** wird.

## Ziel

FoamCraft verfeinert das Netz dort, wo die Strömung es braucht, bei gleichem Zellbudget. Der Nachweis: Die Vergleichsfälle liegen näher am Referenzwert als mit dem statischen Netz.

## Kontext & Abhängigkeiten

- **Voraussetzung:** Task_37 ist archiviert (Messung); die Vernetzung aus Task_09 und Task_13 sowie die Volumenraster aus Task_14 sind vorhanden.
- **Relevantes Vorwissen:** Erkenntnisse aus Task_09 (Budget-Regelung) und Task_14 (Raster). Die Validierungsfälle aus Task_10 und Task_13 dienen als Maßstab.

## Umfang

**Gehört dazu:**

- Ablauf: grober Vorlauf, Indikator aus der Lösung (Gradienten von Druck und Geschwindigkeit, Wirbelstärke, Gesamtdruckverlust), daraus Verfeinerungsbereiche für eine Neuvernetzung innerhalb des Budgets, Rechnung mit Übertragung der Felder als Startlösung
- Ein bis zwei Zyklen
- Einbindung in die Qualitätsstufe "Fein", falls der Nachweis gelingt
- Vergleich statisch gegen adaptiv an den Validierungsfällen

**Gehört nicht dazu:**

- Dynamische Verfeinerung während einer zeitabhängigen Rechnung

## Umsetzungsschritte

1. Tests zuerst für den Indikator auf analytischen Feldern.
2. Indikator und Auswahl der Bereiche mit Budgeteinhaltung umsetzen.
3. Neuvernetzung und Feldübertragung in die Job-Pipeline einhängen.
4. Vergleich an den Validierungsfällen fahren und dokumentieren.
5. Je nach Ergebnis in die Qualitätsstufe einbinden oder als Option belassen.

## Tests

Alle Tests dieses Tasks werden permanent in `tests/` im Hauptverzeichnis des Projekts abgelegt (siehe `AGENTS.md`).

- Der Indikator markiert bei analytischen Feldern die Bereiche mit hohen Gradienten.
- Die Auswahl der Bereiche hält das Zellbudget ein.
- `runtime_`: Ein adaptiver Lauf rechnet durch.
- `validation_`: Vergleich statisch gegen adaptiv, Ergebnis in `docs/performance.md`.

## Akzeptanzkriterien

Der Task gilt erst als fertig, wenn alle Punkte erfüllt und durch Tests oder einen konkreten Befehl belegt sind:

- [ ] Ein adaptiver Lauf ist per Befehl und in der App wählbar
- [ ] Der Vergleich an den Validierungsfällen ist dokumentiert
- [ ] Standardmäßig aktiv ist die Funktion nur, wenn der Genauigkeitsgewinn nachgewiesen ist

## Offene Fragen / Annahmen

- Annahme: Neuvernetzung statt Verfeinerung im laufenden Löser, weil sie für stationäre Fälle robuster ist. Zeigt sich kein Gewinn, wird das dokumentiert und die Funktion bleibt abgeschaltet.

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
