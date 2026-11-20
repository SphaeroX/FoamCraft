# Task_10: Stationäre Außenströmung mit Kräften

> ## ⚠️ PFLICHT – BEVOR DU IRGENDETWAS ANDERES TUST
>
> 1. **Lies als Allererstes die Datei `AGENTS.md` im übergeordneten Ordner `Roadmap/`** (von hier: `../AGENTS.md`) und befolge sie strikt.
> 2. **Deine allererste Aktion** ist, diese Datei in `RUN_Task_10.md` umzubenennen, um den Task als laufend zu markieren. Das passiert **noch bevor irgendein Code geschrieben** wird.

## Ziel

Ein vollständiger Windkanal-Lauf aus der Geometrie des Nutzers liefert Widerstand, Auftrieb und die Beiwerte und stoppt automatisch, sobald das Ergebnis stabil ist. Ein Vergleichsfall liegt innerhalb der festgelegten Toleranz zum Literaturwert.

## Kontext & Abhängigkeiten

- **Voraussetzung:** Task_09 ist archiviert.
- **Relevantes Vorwissen:** `docs/testing.md` (Validierungstests, Toleranzen nie aufweiten), `docs/licensing.md` (Vergleichskörper selbst modellieren). Gerechnet wird stationär mit simpleFoam und dem Turbulenzmodell k-ω SST.

## Umfang

**Gehört dazu:**

- Fallvorlagen für simpleFoam mit k-ω SST; Randbedingungen für Einlass (Geschwindigkeit, Turbulenzgrößen aus Intensität und Längenmaß), Auslass, Wände, Boden und Seiten
- Robuste Numerik-Vorgaben und Initialisierung mit potentialFoam
- Kräfte und Beiwerte mit den Bezugsgrößen aus Task_08, gesamt und je Teil
- Konvergenz-Automatik: gleitendes Fenster über die Beiwerte, Residuen, Obergrenze der Iterationen; Mittelung und Kennzeichnung bei schwankenden Kräften
- Erkennung von Divergenz und ein einmaliger Neustart mit vorsichtigerer Numerik
- Auswertung von y+ mit Hinweis
- Ergebnisobjekt im Projekt: Kennzahlen, Verläufe, Netzstatistik, Laufzeiten, Einstellungen
- `run.start` rechnet das Windkanal-Szenario von Anfang bis Ende

**Gehört nicht dazu:**

- Lauf-Monitor in der App (Task_11)
- 3D-Ergebnisse (Task_14)
- Zeitabhängige Rechnung (Task_27)

## Umsetzungsschritte

1. Tests zuerst für die Einlasswerte der Turbulenz (gegen Handrechnung) und die Fallvorlagen (Snapshots).
2. Fallerzeugung umsetzen.
3. Tests zuerst für die Konvergenz-Logik mit synthetischen Verläufen (stabil, schwankend, driftend, divergierend), dann umsetzen.
4. Auswertung der Kraft-Ausgaben und Ergebnisobjekt umsetzen.
5. Divergenz-Erkennung und Neustart umsetzen.
6. Vergleichskörper wählen, selbst modellieren und den Validierungstest mit Quelle und Toleranz schreiben.
7. Tunnelgröße aus Task_08 am Vergleichsfall prüfen und bei Bedarf begründet anpassen.

## Tests

Alle Tests dieses Tasks werden permanent in `tests/` im Hauptverzeichnis des Projekts abgelegt (siehe `AGENTS.md`).

- Snapshots der Fallvorlagen; Turbulenz-Einlasswerte gegen Handrechnung.
- Konvergenz-Logik mit den vier synthetischen Verläufen.
- `runtime_`: Der Würfel in Entwurfsqualität endet mit einem Ergebnisobjekt.
- `validation_`: Vergleichskörper mit Literaturwert. Vorschlag: Ahmed-Körper mit 25° Heckschräge, Widerstandsbeiwert aus der Literatur, Toleranz 15 % bei Standardqualität. Quelle und Toleranz stehen im Test.

## Akzeptanzkriterien

Der Task gilt erst als fertig, wenn alle Punkte erfüllt und durch Tests oder einen konkreten Befehl belegt sind:

- [ ] `run.start` liefert für ein Windkanal-Projekt Widerstand, Auftrieb und Beiwerte als strukturierte Antwort
- [ ] Der Lauf endet von selbst und nennt den Grund (stabil, schwankend gemittelt, Obergrenze erreicht)
- [ ] Der Validierungstest läuft grün (`cargo test --workspace -- --ignored validation_`)
- [ ] Die Grenzen des Verfahrens bei stumpfen Körpern stehen in `docs/` in Klartext

## Offene Fragen / Annahmen

- Die Wahl des Vergleichskörpers und der Toleranz wird hier begründet festgelegt. Gelingt die Toleranz nicht, anhalten und berichten statt sie aufzuweiten.

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
