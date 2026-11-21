# Task_45: Freie Wasseroberfläche

> ## ⚠️ PFLICHT – BEVOR DU IRGENDETWAS ANDERES TUST
>
> 1. **Lies als Allererstes die Datei `AGENTS.md` im übergeordneten Ordner `Roadmap/`** (von hier: `../AGENTS.md`) und befolge sie strikt.
> 2. **Deine allererste Aktion** ist, diese Datei in `RUN_Task_45.md` umzubenennen, um den Task als laufend zu markieren. Das passiert **noch bevor irgendein Code geschrieben** wird.

## Ziel

Der Nutzer rechnet einen Körper an der Wasseroberfläche (Rumpf, Schwimmer) und erhält den Widerstand und ein Bild der Wellen.

## Kontext & Abhängigkeiten

- **Voraussetzung:** Task_41 ist archiviert (zeitabhängige Rechnung, Zeitreihen).
- **Relevantes Vorwissen:** Der aufwendigste Baustein der Roadmap: eigene Vernetzung an der Wasserlinie und lange Rechenzeiten. Die Qualitätsstufen müssen für diesen Fall neu eingestellt werden.

## Umfang

**Gehört dazu:**

- Szenario "Schleppkanal": Wasserstand oder Tiefgang, Geschwindigkeit, Medien Wasser und Luft
- Zwei-Phasen-Rechnung mit interFoam, Initialisierung des Wasserstands, Dämpfung der Wellen am Rand
- Netz mit Verfeinerung an der Wasserlinie
- Quasi-stationäre Lösung mit lokalem Zeitschritt als Standard, zeitgenaue Rechnung als Option
- Ergebnisse: Widerstand (Druck- und Reibungsanteil), Auftriebskraft, Trimm-Moment, benetzte Fläche
- Ansicht: Wasseroberfläche als Isofläche mit Färbung nach Wellenhöhe
- Der Körper ist fest, er bewegt sich nicht frei

**Gehört nicht dazu:**

- Frei bewegliche Körper (Trimm und Tauchung stellen sich ein)
- Wellengang als Randbedingung

## Umsetzungsschritte

1. Testkörper erzeugen: Quader, mathematisch definierter Vergleichsrumpf (selbst erzeugt).
2. Tests zuerst für die Fallvorlagen und den Verfeinerungsbereich an der Wasserlinie.
3. Szenario, Vernetzung und Fallerzeugung umsetzen.
4. Auswertung umsetzen.
5. Wasseroberfläche in der Ansicht darstellen.
6. Qualitätsstufen für diesen Fall einstellen und Rechenzeiten dokumentieren.
7. Validierungstests schreiben.

## Tests

Alle Tests dieses Tasks werden permanent in `tests/` im Hauptverzeichnis des Projekts abgelegt (siehe `AGENTS.md`).

- Snapshots der Vorlagen; Verfeinerungsbereich um die Wasserlinie.
- `validation_`: ruhender Quader, Auftrieb gleich Dichte mal Erdbeschleunigung mal eingetauchtes Volumen (Toleranz 2 %).
- `validation_`: Vergleichsrumpf bei einer Froude-Zahl, Widerstandsbeiwert gegen Literatur (Toleranz 20 %); Quelle im Test.
- Golden Image der Wasseroberfläche.

## Akzeptanzkriterien

Der Task gilt erst als fertig, wenn alle Punkte erfüllt und durch Tests oder einen konkreten Befehl belegt sind:

- [ ] Ein Schleppkanal-Lauf liefert Widerstand und Auftrieb als Kennzahlen
- [ ] Die Wasseroberfläche ist in der App sichtbar
- [ ] Beide Validierungstests laufen grün

## Offene Fragen / Annahmen

- Rechenzeiten auf dem Referenzrechner sind unbekannt; hier messen und in der App als Schätzung anzeigen.

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
