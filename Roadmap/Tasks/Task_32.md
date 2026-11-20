# Task_32: GPU-Strömungslöser als Kern

> ## ⚠️ PFLICHT – BEVOR DU IRGENDETWAS ANDERES TUST
>
> 1. **Lies als Allererstes die Datei `AGENTS.md` im übergeordneten Ordner `Roadmap/`** (von hier: `../AGENTS.md`) und befolge sie strikt.
> 2. **Deine allererste Aktion** ist, diese Datei in `RUN_Task_32.md` umzubenennen, um den Task als laufend zu markieren. Das passiert **noch bevor irgendein Code geschrieben** wird.

## Ziel

FoamCraft bekommt einen eigenen Strömungslöser, der vollständig auf der GPU rechnet (Lattice-Boltzmann). Er läuft ohne Fenster und ist an Vergleichsfällen geprüft. Er ist die Grundlage der Sofort-Vorschau.

## Kontext & Abhängigkeiten

- **Voraussetzung:** Task_13 ist archiviert (Szenarien für beide Grundfälle); der Renderer und seine GPU-Anbindung aus Task_01 und Task_14 sind vorhanden.
- **Relevantes Vorwissen:** `docs/licensing.md`: Code oder Verfahren aus Quellen mit nicht erlaubter Lizenz dürfen nicht übernommen werden (ausdrücklich nicht aus FluidX3D). Verfahren aus wissenschaftlichen Veröffentlichungen eigenständig umsetzen. Der Löser ist eine Vorschau, kein Ersatz für die OpenFOAM-Rechnung. Referenz-GPU: 12 GB.

## Umfang

**Gehört dazu:**

- Neue Crate: Lattice-Boltzmann D3Q19 als Compute-Programm auf wgpu, unabhängig vom GPU-Hersteller
- Ein Kollisionsmodell, das bei hohen Reynolds-Zahlen stabil bleibt; die Wahl wird begründet
- Voxelisierung der Geometrie auf der GPU
- Ränder: Einlass mit Geschwindigkeit, Auslass mit Druck, Wand, Boden, bewegter Boden
- Windkanal und Innenströmung aus den vorhandenen Szenarien
- Umrechnung zwischen physikalischen und Gitter-Einheiten, automatische Gitterauflösung aus dem GPU-Speicher
- Kraftberechnung am Körper
- CLI `foamcraft preview run`, Leistungsmessung

**Gehört nicht dazu:**

- Kopplung an die Ansicht (Task_33)
- Wärme, zwei Phasen, rotierende Teile im GPU-Löser

## Umsetzungsschritte

1. Verfahren aus Veröffentlichungen auswählen und die Wahl mit Quellen in den Notizen festhalten.
2. Tests zuerst für Einheitenumrechnung, Voxelisierung und Massenerhaltung.
3. Kern (Strömen und Kollidieren) auf der GPU umsetzen.
4. Ränder umsetzen.
5. Szenarien anbinden, automatische Auflösung umsetzen.
6. Kraftberechnung umsetzen.
7. Vergleichsfälle als Tests schreiben und die Leistung messen.

## Tests

Alle Tests dieses Tasks werden permanent in `tests/` im Hauptverzeichnis des Projekts abgelegt (siehe `AGENTS.md`).

- Kanalströmung: Geschwindigkeitsprofil gegen die analytische Lösung (Toleranz 2 %).
- Abklingender Wirbel: Abklingrate gegen die analytische Lösung.
- Zylinder bei Reynolds-Zahl 100: Strouhal-Zahl innerhalb von 10 %.
- Kugel oder Würfel: Widerstand innerhalb von 25 % des Literaturwerts.
- Massenerhaltung; Volumen der voxelisierten Kugel innerhalb von 2 %.
- Wiederholbarkeit; kleine Gitter laufen auf einem Software-Adapter, sonst als übersprungen melden.

## Akzeptanzkriterien

Der Task gilt erst als fertig, wenn alle Punkte erfüllt und durch Tests oder einen konkreten Befehl belegt sind:

- [ ] `foamcraft preview run` rechnet ein Windkanal- und ein Innenströmungs-Szenario auf der GPU und liefert Kräfte beziehungsweise Druckverlust
- [ ] Alle Vergleichsfälle laufen innerhalb ihrer Toleranz
- [ ] Die gemessene Leistung auf der Referenz-GPU steht in `docs/performance.md`

## Offene Fragen / Annahmen

- Die Genauigkeit an Wänden bei hohen Reynolds-Zahlen ist begrenzt; ob ein Wandmodell nötig ist, wird hier bewertet.
- Annahme: einfache Genauigkeit (32 Bit) reicht für die Vorschau.

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
