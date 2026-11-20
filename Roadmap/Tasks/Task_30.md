# Task_30: Poröse Bauteile

> ## ⚠️ PFLICHT – BEVOR DU IRGENDETWAS ANDERES TUST
>
> 1. **Lies als Allererstes die Datei `AGENTS.md` im übergeordneten Ordner `Roadmap/`** (von hier: `../AGENTS.md`) und befolge sie strikt.
> 2. **Deine allererste Aktion** ist, diese Datei in `RUN_Task_30.md` umzubenennen, um den Task als laufend zu markieren. Das passiert **noch bevor irgendein Code geschrieben** wird.

## Ziel

Der Nutzer markiert ein Teil als durchströmt (Kühler, Filter, Lochblech) und gibt dafür einfache Kenndaten ein. Ergebnisse sind Volumenstrom durch das Teil und Druckabfall.

## Kontext & Abhängigkeiten

- **Voraussetzung:** Task_13 ist archiviert; die Zonen-Erzeugung aus Task_29 ist vorhanden.
- **Relevantes Vorwissen:** Erkenntnisse aus Task_29 zu Zellzonen. Poröse Zonen gelten für Außen- und Innenströmung.

## Umfang

**Gehört dazu:**

- Teil (geschlossener Körper) als durchströmt markieren
- Eingabe: Druckverlust bei einer Geschwindigkeit (ein oder zwei Messpunkte), daraus die Widerstandsbeiwerte; Durchströmrichtung, quer dazu gesperrt
- Vorlagen: Kühler, Filter, Lochblech mit freiem Flächenanteil
- Zellzone und Widerstandsquelle im Fall
- Ergebnisse: Volumenstrom durch das Teil, Druckabfall

**Gehört nicht dazu:**

- Wärmeübergang im Kühler

## Umsetzungsschritte

1. Tests zuerst für die Beiwerte aus einem und aus zwei Messpunkten (Handrechnung) und für die Lochblech-Korrelation (Quelle im Test).
2. Markierung, Eingabe und Vorlagen umsetzen.
3. Zone und Widerstandsquelle in der Fallerzeugung umsetzen (Snapshot).
4. Auswertung umsetzen.
5. Oberfläche, Kennzahlen und Report erweitern.
6. Validierungstest schreiben.

## Tests

Alle Tests dieses Tasks werden permanent in `tests/` im Hauptverzeichnis des Projekts abgelegt (siehe `AGENTS.md`).

- Beiwerte gegen Handrechnung; Lochblech-Korrelation.
- Snapshot der Fallerzeugung.
- `validation_`: Kanal mit porösem Block, der Druckabfall entspricht der eingegebenen Kennlinie (Toleranz 5 %).

## Akzeptanzkriterien

Der Task gilt erst als fertig, wenn alle Punkte erfüllt und durch Tests oder einen konkreten Befehl belegt sind:

- [ ] Ein Teil lässt sich in der App als durchströmt markieren und rechnen
- [ ] Volumenstrom und Druckabfall stehen in den Kennzahlen
- [ ] Der Validierungstest läuft grün

## Offene Fragen / Annahmen

- Keine

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
