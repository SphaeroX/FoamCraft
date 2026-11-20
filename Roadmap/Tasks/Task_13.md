# Task_13: Innenströmung rechnen

> ## ⚠️ PFLICHT – BEVOR DU IRGENDETWAS ANDERES TUST
>
> 1. **Lies als Allererstes die Datei `AGENTS.md` im übergeordneten Ordner `Roadmap/`** (von hier: `../AGENTS.md`) und befolge sie strikt.
> 2. **Deine allererste Aktion** ist, diese Datei in `RUN_Task_13.md` umzubenennen, um den Task als laufend zu markieren. Das passiert **noch bevor irgendein Code geschrieben** wird.

## Ziel

Der Nutzer rechnet eine Innenströmung und erhält Druckverlust und Durchfluss je Auslass. Ein Rohr-Vergleichsfall liegt innerhalb der Toleranz zur analytischen Lösung. Danach funktionieren beide Grundfälle von der Datei bis zur Kennzahl (Meilenstein 1).

## Kontext & Abhängigkeiten

- **Voraussetzung:** Task_09 (Vernetzung), Task_10 (Fallerzeugung, Konvergenz), Task_11 (Monitor) und Task_12 (Öffnungen) sind archiviert.
- **Relevantes Vorwissen:** Erkenntnisse aus Task_09 und Task_10 zu Vorlagen und Konvergenz; `docs/testing.md` (Validierungstests).

## Umfang

**Gehört dazu:**

- Szenario "Innenströmung": je Einlass Geschwindigkeit, Volumenstrom, Massenstrom oder Gesamtdruck; je Auslass statischer Druck; Plausibilitätsprüfung der Kombination
- Vernetzung des Innenraums: Punkt im Fluidraum aus Task_12, Verfeinerung nach den kleinsten Querschnitten, Grenzschichten, Deckflächen als Ränder
- Reynolds-Zahl aus dem hydraulischen Durchmesser; Wahl laminar oder turbulent mit Hinweis
- Auswertung: Volumenstrom und gemittelter Druck (statisch und gesamt) je Öffnung, Druckverlust, Anteil je Auslass, Fehler der Massenbilanz
- Konvergenz über ein Fenster des Druckverlusts
- Oberfläche: Szenario-Schritt für die Innenströmung, Druckverlust-Kurve im Monitor

**Gehört nicht dazu:**

- 3D-Ergebnisse (Task_14)
- Wärme (Task_28), poröse Teile (Task_30)

## Umsetzungsschritte

1. Tests zuerst für hydraulischen Durchmesser, Wahl laminar oder turbulent und die Plausibilitätsprüfung.
2. Szenario-Modell, Befehl und Randwert-Erzeugung umsetzen (Snapshots).
3. Vernetzung des Innenraums auf Basis von Task_09 umsetzen.
4. Auswertung je Öffnung und Konvergenz über den Druckverlust umsetzen.
5. Validierungstests schreiben (Quelle und Toleranz im Test).
6. Szenario-Schritt und Monitor-Anbindung in der Oberfläche umsetzen.
7. Meilenstein 1: Beide Grundfälle in der App durchspielen und dem Nutzer zur Abnahme melden.

## Tests

Alle Tests dieses Tasks werden permanent in `tests/` im Hauptverzeichnis des Projekts abgelegt (siehe `AGENTS.md`).

- Snapshots der Randwerte; unzulässige Kombinationen liefern Fehlercodes.
- `runtime_`: gerades Rohr und T-Stück rechnen durch; Massenbilanz beim T-Stück unter 1 %.
- `validation_`: laminare Rohrströmung gegen Hagen-Poiseuille (Auswertung im eingelaufenen Abschnitt, Toleranz 5 %); turbulente Rohrströmung gegen eine Rohrreibungs-Korrelation (Toleranz 15 %).
- Vitest für den Szenario-Schritt.

## Akzeptanzkriterien

Der Task gilt erst als fertig, wenn alle Punkte erfüllt und durch Tests oder einen konkreten Befehl belegt sind:

- [ ] `run.start` liefert für ein Innenströmungs-Projekt Druckverlust und Durchfluss je Öffnung
- [ ] Beide Validierungstests laufen grün
- [ ] Ein Innenströmungs-Lauf ist in der App von der Datei bis zur Kennzahl möglich
- [ ] Der Nutzer wurde über das Erreichen von Meilenstein 1 informiert

## Offene Fragen / Annahmen

- Annahme: Auslässe mit festem Druck als Standard. Vorgabe von Volumenstrom-Anteilen je Auslass ist nicht vorgesehen.

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
