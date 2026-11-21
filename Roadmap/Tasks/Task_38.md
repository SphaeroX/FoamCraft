# Task_38: Wärme und Auftrieb

> ## ⚠️ PFLICHT – BEVOR DU IRGENDETWAS ANDERES TUST
>
> 1. **Lies als Allererstes die Datei `AGENTS.md` im übergeordneten Ordner `Roadmap/`** (von hier: `../AGENTS.md`) und befolge sie strikt.
> 2. **Deine allererste Aktion** ist, diese Datei in `RUN_Task_38.md` umzubenennen, um den Task als laufend zu markieren. Das passiert **noch bevor irgendein Code geschrieben** wird.

## Ziel

Der Nutzer berücksichtigt Temperatur: beheizte Teile, warme und kalte Zuluft, aufsteigende Warmluft. Ergebnisse sind Temperaturfelder und Wärmeströme, mit einer Vorlage für Raumluft.

## Kontext & Abhängigkeiten

- **Voraussetzung:** Task_13 ist archiviert (Innenströmung); Task_14 bis Task_17 liefern Darstellung und Kennzahlen.
- **Relevantes Vorwissen:** Erkenntnisse aus Task_10 und Task_13 zu Vorlagen und Konvergenz. Der Nutzer hat ein eigenes Raumluft-Projekt und kennt den Anwendungsfall.

## Umfang

**Gehört dazu:**

- Option "Temperatur berücksichtigen" mit einem auftriebsfähigen Löser; die Modellwahl (ideales Gas oder Boussinesq-Näherung) wird begründet
- Schwerkraftrichtung
- Randbedingungen: Einlasstemperatur, feste Wandtemperatur, Wärmeleistung in Watt je Teil oder Fläche, wärmedicht
- Ergebnisse: Temperatur auf Oberflächen und in Schnitten, Wärmeströme je Fläche, mittlere und höchste Temperatur, Auslasstemperatur, Luftwechselrate
- Szenario-Vorlagen "Raumluft" und "Bauteilkühlung"
- Klartext-Hinweis: Wärmeleitung im Festkörper und Strahlung werden nicht gerechnet

**Gehört nicht dazu:**

- Wärmeleitung in Festkörpern (gekoppelte Rechnung), Strahlung

## Umsetzungsschritte

1. Tests zuerst für die Umrechnung von Watt in Wärmestromdichte und die Fallvorlagen (Snapshots).
2. Modellwahl treffen und begründen.
3. Fallerzeugung und Randbedingungen umsetzen.
4. Auswertung (Wärmeströme, Temperaturen, Luftwechsel) umsetzen.
5. Temperaturfelder in Ansicht, Kennzahlen und Report aufnehmen.
6. Vorlagen und Oberfläche umsetzen.
7. Validierungstests schreiben.

## Tests

Alle Tests dieses Tasks werden permanent in `tests/` im Hauptverzeichnis des Projekts abgelegt (siehe `AGENTS.md`).

- Umrechnung Watt in Wärmestromdichte; Snapshots der Vorlagen.
- `validation_`: beheiztes Rohr, Auslasstemperatur gegen die Energiebilanz (Toleranz 3 %).
- `validation_`: beheizter Hohlraum mit natürlicher Konvektion, Nusselt-Zahl gegen einen veröffentlichten Vergleichswert (Toleranz 10 %); Quelle im Test.
- Vitest für die Eingabe der Wärmequellen.

## Akzeptanzkriterien

Der Task gilt erst als fertig, wenn alle Punkte erfüllt und durch Tests oder einen konkreten Befehl belegt sind:

- [ ] Ein Lauf mit Temperatur liefert Temperaturfelder und Wärmeströme als Kennzahlen
- [ ] Beide Validierungstests laufen grün
- [ ] Die Vorlage "Raumluft" führt von der Geometrie eines Raums bis zum Ergebnis

## Offene Fragen / Annahmen

- Wärmeleitung in Festkörpern und Strahlung sind Wünsche für später, keine Aufgaben dieses Tasks.

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
