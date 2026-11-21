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
- Zeitabhängige Rechnung (Task_37)

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

- [x] `run.start` liefert für ein Windkanal-Projekt Widerstand, Auftrieb und Beiwerte als strukturierte Antwort
- [x] Der Lauf endet von selbst und nennt den Grund (stabil, schwankend gemittelt, Obergrenze erreicht)
- [x] Der Validierungstest läuft grün (`cargo test --workspace -- --ignored validation_`)
- [x] Die Grenzen des Verfahrens bei stumpfen Körpern stehen in `docs/` in Klartext

## Offene Fragen / Annahmen

- Die Wahl des Vergleichskörpers und der Toleranz wird hier begründet festgelegt. Gelingt die Toleranz nicht, anhalten und berichten statt sie aufzuweiten.

## Notizen zu diesem Task

Taskspezifische Informationen während der Arbeit hier festhalten. Taskübergreifendes gehört in `Roadmap/README.md`.

- Vergleichskörper (begründet abweichend vom Vorschlag Ahmed-Körper): scharfkantiger Würfel frontal in freier Anströmung, C_D = 1,05 nach Hoerner (Fluid-Dynamic Drag, 1965), Toleranz 15 % bei Standardqualität. Gründe: Ablösung an den Kanten fest (keine Re-Abhängigkeit über ~10⁴), keine Stelzen/Bodenmodellierung, passt in den 1,9-GiB-Container. Der Ahmed-Körper (25°) ist ein bekannter Schwachpunkt stationärer RANS-Modelle.
- Validierung 2026-11-21: Würfel 0,2 m, 20 m/s, Re 2,7·10⁵, Standard (Speicher-begrenzt) 805 072 Zellen, 24 Kerne: **C_D = 1,046** (−0,4 %), Grund "stabil" nach 288 Iterationen, Schwankung 0,94 %, Rechenzeit 171 s, gesamt 4 min. y+ Mittel 14 (Hinweis `run.y_plus_low`).
- `runtime_`: Würfel 20 mm am Boden, Entwurf, 150 000 Zellen: Grund "schwankend gemittelt" nach 281 Iterationen, C_D 1,23, C_L 0,66, Schwankung 13 %, 44 s.
- Tunnelgröße aus Task_08 geprüft: Versperrung frei 1/(11·11) ≈ 0,8 %, mit Boden 1/(11·6) ≈ 1,5 %; das Validierungsergebnis liegt innerhalb 0,4 % → keine Anpassung nötig.
- Experiment v2606: `forceCoeffs` mit `log true` schreibt pro Iteration einen Block `forceCoeffs <name> write:` mit Zeilen `Cd:\t<Total>\t<Pressure>\t<Viscous>\t<Internal>`; `yPlus` schreibt `patch <name> y+ : min = …, max = …, average = …`. Testdaten: `tests/data/logs/simpleFoam_forces.log`.
- Snapshot `tests/data/snapshots/wind_tunnel_cube` aktualisiert (Grund: Funktionsobjekte in controlDict, Phi-Löser/potentialFlow in fvSolution, `div(div(phi,U))` in fvSchemes).

## Abschluss-Checkliste

Reihenfolge und Details stehen in `AGENTS.md`, Abschnitt "Abschluss- und Archivierungs-Workflow".

- [x] Alle Akzeptanzkriterien erfüllt, Tests liegen permanent in `tests/` und laufen grün
- [x] Abschnitt "Erkenntnisse für Folge-Agenten" unten ausgefüllt
- [x] Folge-Tasks und `Roadmap/README.md` geprüft und bei Bedarf angepasst
- [x] Datei ohne `RUN_` nach `Roadmap/Archiv/` verschoben

## Erkenntnisse für Folge-Agenten

- **Umgesetzt (und Abweichungen vom Plan):** `foamcraft_sim::solve` (Einlass-Turbulenz, `SolverLog` für Residuen/Beiwerte/y+, `assess` Konvergenz, `cautious_numerics`, `solver_functions`, `WindTunnelResult`), `foamcraft_api::solve` (Lauf mit Netz-Wiederverwendung, Stop-Datei, ein Neustart, Ergebnis in `<projekt>/results/<job>.json`), `run.start` (bei Szenario: Windkanal, sonst Referenzfall; `wait` liefert das Ergebnis), `run.result`, `JobRecord.start_at`, potentialFoam in der Solve-Stufe (auch Referenzfall). Abweichung: Vergleichskörper Würfel statt Ahmed-Körper (Begründung in den Notizen).
- **Learnings & Stolpersteine:** `potentialFoam` braucht `div(div(phi,U))` in fvSchemes, einen `Phi`-Löser und `potentialFlow` in fvSolution. `abort`-Funktionsobjekt mit `file "<case>/stop"; action writeNow;` beendet simpleFoam sauber (Exit 0). `docker cp <ordner case> cid:/work` führt mit dem vorhandenen `/work/case` zusammen. serde_json-Floats sind beim Zurücklesen nicht bitgenau – Ergebnisse nicht als Ganzes vergleichen. Neue Felder in `RunOptions` brechen bestehende Tests (Struct-Literale) – deshalb liegt `start_at` im `JobRecord` (`serde(default)`).
- **Wichtige Entscheidungen und ihre Gründe:** Konvergenz wird auf dem Host aus dem Solver-Log entschieden (testbar, unabhängig von OpenFOAM-runTimeControl). Grenze 3 × erwartete Iterationen, Fenster ein Fünftel der erwarteten. Teile = Patch-Regionen (Gruppen und `body`), Beiwerte auf die Gesamt-Stirnfläche bezogen (summieren sich). Solve-Job teilt das Volume des Netz-Jobs.
- **Auswirkungen auf Folge-Tasks:** Task_11 kann Ereignisse `run.stage`, `run.progress` (Residuen), `run.coefficients` (Iteration, Cd, Cl), `run.result`, `run.failed`, `run.restarted`, `run.finished` nutzen; Ergebnis über `run.result`. Hinweis-Codes (`run.stable`, `run.oscillating`, `run.limit_reached`, `run.restarted`, `run.y_plus_low/high`) stehen im Fehlerkatalog mit en/de. Zum Wiederverbinden nach Neustart fehlt noch ein Fortsetzen der Konvergenz-Automatik über `run_job`-Resume (Solve-Log ab Offset 0 erneut lesen). Task_14: Felder liegen im Volume des Netz-Jobs (`record.volume` des Solve-Jobs), im Strömungssystem (um +Gierwinkel zurückdrehen). Task_17: `WindTunnelResult` enthält Verläufe und Kennzahlen.
- **Hinweise zu Tests und Befehlen:** `cargo test -p foamcraft-sim --test convergence --test solve`, `cargo test -p foamcraft-api --test solve` (Fake, inkl. Neustart), `cargo test -p foamcraft-api --test solve_runtime -- --ignored runtime_` (~45 s), `... -- --ignored validation_` (~4 min).
