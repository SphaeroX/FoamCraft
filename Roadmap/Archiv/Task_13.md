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
- Wärme (Task_38), poröse Teile (Task_40)

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

- [x] `run.start` liefert für ein Innenströmungs-Projekt Druckverlust und Durchfluss je Öffnung
- [x] Beide Validierungstests laufen grün
- [x] Ein Innenströmungs-Lauf ist in der App von der Datei bis zur Kennzahl möglich
- [x] Der Nutzer wurde über das Erreichen von Meilenstein 1 informiert

## Offene Fragen / Annahmen

- Annahme: Auslässe mit festem Druck als Standard. Vorgabe von Volumenstrom-Anteilen je Auslass ist nicht vorgesehen.

## Notizen zu diesem Task

- Validierung 2026-11-21 (Entwurf, ~515 000 Zellen): laminares Rohr D = 10 mm, Re ≈ 100, Δp zwischen x = 120 mm und 270 mm: **0,481 Pa** gegen Hagen-Poiseuille 0,483 Pa (−0,5 %, Toleranz 5 %). Turbulentes Rohr D = 50 mm, Re ≈ 49 700, Δp zwischen 1,0 m und 1,85 m: **163 Pa** gegen Blasius 180 Pa (−9,7 %, Toleranz 15 %). Die Ebenen-Mittelwerte bildet der Test per `postProcess` (sampledSurface cuttingPlane, `name` ist Pflicht).
- `runtime_`: gerades Rohr (Δp 24 Pa, Massenbilanz 4·10⁻⁸) und T-Stück (Massenbilanz 1·10⁻⁵, Anteile 96 %/4 %) laufen in je ~30 s.
- Abnahme in der App (T-Stück-Projekt "Rohr"): Szenario-Art "Innenströmung", Einlass 0,3 m/s, Auslässe 0 Pa, Entwurf → Re 4 765, turbulent; Netz 491 277 Zellen; Lauf "stabil" nach 176 Iterationen, Druckverlust 16,9 Pa, Massenbilanzfehler 0,0005 %, Durchfluss und Anteil je Auslass (`Roadmap/Nachweise/task13_innenstroemung_monitor.png`, `task13_innenstroemung_ergebnis.png`).
- Meilenstein 1 dem Nutzer am 2026-11-21 gemeldet (Nachricht mit Bildschirmfotos; Nutzer war nicht am PC, Abnahme steht aus).
- Gefunden und behoben: Fluidvolumen aus dem Flutungsgitter war bei langen dünnen Rohren viel zu klein (96 Voxel entlang der Länge → 3 Voxel über den Durchmesser) → 2,4-fach zu viele Zellen. Jetzt Voxel ≤ 1/8 der kleinsten Deckfläche (höchstens 3·10⁷ Voxel) und halbe Wandvoxel zählen mit.

Taskspezifische Informationen während der Arbeit hier festhalten. Taskübergreifendes gehört in `Roadmap/README.md`.

## Abschluss-Checkliste

Reihenfolge und Details stehen in `AGENTS.md`, Abschnitt "Abschluss- und Archivierungs-Workflow".

- [x] Alle Akzeptanzkriterien erfüllt, Tests liegen permanent in `tests/` und laufen grün
- [x] Abschnitt "Erkenntnisse für Folge-Agenten" unten ausgefüllt
- [x] Folge-Tasks und `Roadmap/README.md` geprüft und bei Bedarf angepasst
- [x] Datei ohne `RUN_` nach `Roadmap/Archiv/` verschoben

## Erkenntnisse für Folge-Agenten

- **Umgesetzt (und Abweichungen vom Plan):** Szenario `InternalFlow` (core), `foamcraft_sim::internal` (D_h, Regime, Prüfungen, `FieldValueLog`, `evaluate_openings`, `InternalFlowResult`), `foamcraft_sim::internal_case` (Plan, Felder, Funktionsobjekte, Vorlage `templates/internal_flow`), `assess_series`, `mesh_internal.rs`, gemeinsame Lauf-Logik in `solve.rs` (`Setup.kind`, `Watch`), Befehle `scenario.set_internal_flow`, `scenario.boundary_roles`; Oberfläche: Art-Auswahl im Szenario-Schritt, `InternalFlowSection`, Druckverlust-Kurve und Ergebnis je Öffnung im Monitor. Abweichung: keine Grenzschichten und keine Verfeinerungsstufen bei Innenströmung (gleichmäßige Zellen) – Validierung trotzdem innerhalb der Toleranzen.
- **Learnings & Stolpersteine:** Patch-Gruppen werden in `boundaryField` nur mit exaktem Namen erkannt, nicht per Regex → Wände als `walls { … }` (Gruppe) plus `background`. `surfaceFieldValue` mit `log true` schreibt `surfaceFieldValue <name> write:` und darunter `sum(<patch>) of phi = <wert>`. Bei sampledSurface ist `name` Pflicht. Ohne Deckflächen fehlt ein Druckrand → `pRefCell`-Fehler. Der Einlass liegt dort, wo die Rolle gesetzt ist – Tests sollten die Richtung festlegen. `run.result` liefert jetzt beliebiges JSON (beide Ergebnisarten).
- **Wichtige Entscheidungen und ihre Gründe:** Gesamtdruck als statischer Druck + ½ρ(Q/A)² (Mittelgeschwindigkeit, Näherung; einfach und robust). Druckverlust durchflussgewichtet über Ein- und Auslässe. Konvergenz über Druckverlust und Gesamtabfluss zusammen. Validierung mit Ebenen im eingelaufenen Abschnitt nur im Test, nicht als Produktfunktion.
- **Auswirkungen auf Folge-Tasks:** Task_14 (3D-Ergebnisse): Felder liegen im Volume des Netz-Jobs (`manifest.mesh.job_id`; der Solve-Job hat `solve.json` mit `volume`), Innenströmung in Weltkoordinaten, Windkanal im Strömungssystem (um +Gierwinkel zurückdrehen). Task_17: `InternalFlowResult`/`WindTunnelResult` haben Verläufe; Unterscheidung über `kind`. Alte `solve.json` ohne `kind` (vor Task_13) lassen sich nicht mehr fortsetzen (nur laufende Jobs betroffen).
- **Hinweise zu Tests und Befehlen:** `cargo test -p foamcraft-sim --test internal --test internal_case`, `cargo test -p foamcraft-api --test internal_runtime -- --ignored runtime_` (~1 min), `... -- --ignored validation_ --test-threads 1` (~3 min für beide Rohre), `npx vitest run tests/app/internal-flow-section.test.tsx tests/app/run-panel.test.tsx`. Snapshot: `tests/data/snapshots/internal_pipe`.
