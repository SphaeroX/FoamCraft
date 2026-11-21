# Task_14: Oberflächenfelder und Volumenraster

> ## ⚠️ PFLICHT – BEVOR DU IRGENDETWAS ANDERES TUST
>
> 1. **Lies als Allererstes die Datei `AGENTS.md` im übergeordneten Ordner `Roadmap/`** (von hier: `../AGENTS.md`) und befolge sie strikt.
> 2. **Deine allererste Aktion** ist, diese Datei in `RUN_Task_14.md` umzubenennen, um den Task als laufend zu markieren. Das passiert **noch bevor irgendein Code geschrieben** wird.

## Ziel

Nach einem Lauf sieht der Nutzer das Modell eingefärbt nach Druck, Druckbeiwert und Wandschubspannung und fragt Werte per Klick ab. Zusätzlich liegen die Volumendaten als 3D-Raster bereit, auf denen Schnitte, Isoflächen und Partikel aufbauen.

## Kontext & Abhängigkeiten

- **Voraussetzung:** Task_10 und Task_13 sind archiviert.
- **Relevantes Vorwissen:** `docs/architecture.md`, Regel 3: OpenFOAM bereitet auf, Rust liest, die GPU rechnet die Darstellung; kein VTK. Der Agent aus Task_02 ist der einzige Zugriff auf Falldateien. Referenz-GPU: 12 GB.

## Umfang

**Gehört dazu:**

- Extraktion im Runtime: Wandflächen mit Feldern (Druck, Druckbeiwert, Wandschubspannung, y+) als kompakte Binärdaten; der Weg (Function Objects oder direktes Lesen) wird begründet gewählt
- Volumenfelder (Geschwindigkeit, Druck, Gesamtdruck, Turbulenzgröße) auf ein mehrstufiges regelmäßiges Raster abgetastet: grob für das ganze Gebiet, fein um den Körper; Maske für Festkörper
- Übertragung zum Host und Cache im Projekt mit Größenbegrenzung
- Renderer: Feld auf der Oberfläche mit Farbskala (gleichmäßig wahrnehmbar, für Farbfehlsichtige geeignet), Legende mit Einheit, Bereich automatisch oder von Hand, Wert am Mauspunkt
- Befehle `results.list_fields`, `results.probe`, `view.set_surface_field`
- Grundgerüst des Schritts "Ergebnisse" in der Oberfläche

**Gehört nicht dazu:**

- Schnitte und Isoflächen (Task_15), Partikel (Task_16), Kennzahlen-Tafel (Task_17)

## Umsetzungsschritte

1. Kleine OpenFOAM-Felddateien aufzeichnen (ASCII und binär) und Tests für den Leser schreiben.
2. Leser und Extraktion der Wandflächen umsetzen.
3. Test zuerst für die Abtastung: Ein analytisches Feld auf einem Testnetz wird mit begrenztem Fehler abgetastet.
4. Mehrstufiges Raster, Maske, Übertragung und Cache umsetzen.
5. Farbskala, Legende und Wert-Abfrage im Renderer umsetzen (Golden Images).
6. Befehle und den Schritt "Ergebnisse" ergänzen.
7. Speicherbudget für die Raster festlegen und dokumentieren.

## Tests

Alle Tests dieses Tasks werden permanent in `tests/` im Hauptverzeichnis des Projekts abgelegt (siehe `AGENTS.md`).

- Leser: ASCII und binär, einheitliche und nicht einheitliche Felder.
- Abtastfehler für das analytische Feld unter der festgelegten Schwelle; Maske stimmt für einen Würfel.
- Cache übersteht Speichern und Öffnen; das Speicherbudget wird eingehalten.
- Golden Image des eingefärbten Würfels.
- `validation_` am Würfelfall: Der Druckbeiwert am Staupunkt liegt bei 1 (Toleranz 0,1).

## Akzeptanzkriterien

Der Task gilt erst als fertig, wenn alle Punkte erfüllt und durch Tests oder einen konkreten Befehl belegt sind:

- [x] Nach einem Lauf ist das Modell in der App nach Druck eingefärbt, mit Legende
- [x] `results.probe` liefert für einen Punkt auf der Oberfläche den Feldwert mit Einheit
- [x] Die Volumenraster liegen im Cache und bleiben unter dem dokumentierten Speicherbudget

## Offene Fragen / Annahmen

- Annahme: drei Rasterstufen mit begrenzter Auflösung. Sehr feine Details werden damit ungenau dargestellt; die App weist darauf hin.

## Notizen zu diesem Task

- Annahme bestätigt: drei Rasterstufen (Windkanal) bzw. eine (Innenströmung) mit je höchstens 64³ Punkten; die App weist im Schritt "Ergebnisse" darauf hin, dass feine Details weniger scharf erscheinen.
- `validation_cube_stagnation_pressure_coefficient_is_one` (2026-11-21): Cp am Staupunkt des freien 0,2-m-Würfels **1,017** (Soll 1, Toleranz 0,1).
- `runtime_cube_results_are_cached_and_probed`: alle Felder vorhanden, drei Stufen, Cache unter 40 MB, Abfrage auf Oberfläche (Pa) und im Volumen (m/s), Punkt im Würfel → `results.outside`, Cache übersteht Schließen/Öffnen.
- Abnahme in der App (T-Stück "Rohr"): nach dem Lauf Schritt "Ergebnisse" → Modell nach Druck eingefärbt, Legende −3,4 … 38,4 Pa, Wert unter der Maus "Druck: 10,8 Pa" (`Roadmap/Nachweise/task14_ergebnisse_druck.png`). Erste Fassung zu dunkel → Helligkeit der Farbskala auf 0,45 … 0,95 angehoben; Golden Image `coloured_cube.png` deshalb neu erzeugt.
- Stolperstein: `patches` wurde doppelt geklammert (`(("model.*"))`) → Extraktion schlug still fehl (Ereignis `results.failed`, der Lauf selbst bleibt gültig).

Taskspezifische Informationen während der Arbeit hier festhalten. Taskübergreifendes gehört in `Roadmap/README.md`.

## Abschluss-Checkliste

Reihenfolge und Details stehen in `AGENTS.md`, Abschnitt "Abschluss- und Archivierungs-Workflow".

- [x] Alle Akzeptanzkriterien erfüllt, Tests liegen permanent in `tests/` und laufen grün
- [x] Abschnitt "Erkenntnisse für Folge-Agenten" unten ausgefüllt
- [x] Folge-Tasks und `Roadmap/README.md` geprüft und bei Bedarf angepasst
- [x] Datei ohne `RUN_` nach `Roadmap/Archiv/` verschoben

## Erkenntnisse für Folge-Agenten

- **Umgesetzt (und Abweichungen vom Plan):** `foamcraft_sim::foam_io` (Leser), `foamcraft_sim::results` (Farbskala, Raster/Maske/trilineare Abfrage, Binärformat, Extraktions-Dictionary), `foamcraft_api::results` (Extraktion nach jedem Lauf, Cache mit Grenze, `results.list_fields`, `results.probe`, `view.set_surface_field`), Manifest Schema 8 (`view`), Renderer `Mesh::vertex_colors`, Tauri: Ergebnisfläche statt Modell, Schritt "Ergebnisse" (`ResultsPanel`) mit Legende, Bereich von Hand und Wert unter der Maus.
- **Learnings & Stolpersteine:** `array`-Sets brauchen `rotation { type none; }`, sonst wird `axis` als Vektor gelesen. Der foam-Flächenschreiber schreibt Listen ohne FoamFile-Kopf und einheitliche Listen als `N{wert}`. `postProcess` braucht die rekonstruierte letzte Zeit (bei Parallelläufen durch `reconstructPar -latestTime` vorhanden). Laminare Fälle: kein `yPlus`. Testdaten unter `std::env::temp_dir()` liegen in Git-Bash unter `/tmp`.
- **Wichtige Entscheidungen und ihre Gründe:** Funktionsobjekte statt eigener Netzleser (OpenFOAM interpoliert und sucht Zellen; kein VTK). Eigene Farbskala in OKLab statt einer fremden Tabelle (Lizenz, gleichmäßige Helligkeit). Felder je Dreieck (flache Färbung) – einfach und exakt zu den Zellwerten an der Wand.
- **Auswirkungen auf Folge-Tasks:** Task_15 (Schnitte, Isoflächen) und Task_16 (Partikel) bauen auf `grid_<n>.bin` auf (`Grid::sample`, Maske, `meta.levels`, Windkanal im Strömungssystem → um `yaw_deg` drehen). Volumenfelder: `p`, `p_total`, `Ux/Uy/Uz`, `k`. Die Ergebnisfläche ersetzt im Viewport die Modelle, solange `view.surface_field` gesetzt ist (Schlüssel `result:<job>`).
- **Hinweise zu Tests und Befehlen:** `cargo test -p foamcraft-sim --test foam_io --test results`, `cargo test -p foamcraft-render --test golden`, `cargo test -p foamcraft-api --test results_runtime -- --ignored runtime_` (~2 min), `... validation_` (~2 min), `npx vitest run tests/app/results-panel.test.tsx`. Mauszeiger per Skript bewegen: `wheel:x,y,0` in `tools/windows/app-ui.ps1`.
