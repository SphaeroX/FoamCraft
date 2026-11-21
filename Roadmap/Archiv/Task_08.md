# Task_08: Windkanal einrichten

> ## ⚠️ PFLICHT – BEVOR DU IRGENDETWAS ANDERES TUST
>
> 1. **Lies als Allererstes die Datei `AGENTS.md` im übergeordneten Ordner `Roadmap/`** (von hier: `../AGENTS.md`) und befolge sie strikt.
> 2. **Deine allererste Aktion** ist, diese Datei in `RUN_Task_08.md` umzubenennen, um den Task als laufend zu markieren. Das passiert **noch bevor irgendein Code geschrieben** wird.

## Ziel

Der Nutzer richtet eine Außenumströmung ein: Anströmgeschwindigkeit, Richtung, Boden, Medium und Qualitätsstufe. Tunnelgröße und Bezugsgrößen berechnet FoamCraft selbst.

## Kontext & Abhängigkeiten

- **Voraussetzung:** Task_06 ist archiviert (Task_07 liegt in der Reihenfolge davor und ist ebenfalls archiviert).
- **Relevantes Vorwissen:** `docs/architecture.md`, Regel 4 (physikalische Eingaben, erzeugte Dictionaries). Referenzrechner: 64 GB Arbeitsspeicher, 24 Kerne (8 P-Kerne, 16 E-Kerne).

## Umfang

**Gehört dazu:**

- Szenario "Windkanal" im Projekt, Befehl `scenario.set_wind_tunnel`
- Medien-Vorgaben (Luft und Wasser bei einer Temperatur ergeben Dichte und Viskosität) und frei definierbare Medien
- Boden: kein Boden, fester Boden; bewegter Boden als Option vorgesehen, gerechnet wird er in Task_43
- Automatische Tunnelgröße aus den Abmessungen des Modells nach einer dokumentierten Regel
- Bezugsfläche (projizierte Stirnfläche in Anströmrichtung) und Bezugslänge
- Reynolds-Zahl mit Einordnung in Klartext
- Qualitätsstufen (Entwurf, Standard, Fein) mit Zellbudget aus Arbeitsspeicher und Kernen, grobe Zeitschätzung
- Ansicht: Tunnel als Box, Strömungspfeil, Boden
- Warnungen: Mach-Zahl über 0,3 liegt außerhalb des Gültigkeitsbereichs; winziges oder riesiges Modell deutet auf eine falsche Einheit

**Gehört nicht dazu:**

- Vernetzung (Task_09) und Rechnung (Task_10)

## Umsetzungsschritte

1. Tests zuerst für die Stirnfläche (Kugel, Würfel, gedrehter Würfel gegen die analytischen Werte).
2. Szenario-Modell und Befehl umsetzen.
3. Regeln für Tunnelgröße und Zellbudget festlegen, dokumentieren und testen.
4. Reynolds-Zahl, Mach-Warnung und Einheiten-Warnung umsetzen.
5. Tunnel, Pfeil und Boden in der Ansicht darstellen.
6. Schritt "Szenario" in der Oberfläche umsetzen (Vitest zuerst).

## Tests

Alle Tests dieses Tasks werden permanent in `tests/` im Hauptverzeichnis des Projekts abgelegt (siehe `AGENTS.md`).

- Stirnfläche innerhalb von 1 % der analytischen Werte.
- Tunnelmaße folgen der Regel; Zellbudget für 64 GB und für 8 GB Arbeitsspeicher.
- Reynolds-Zahl, Mach-Warnung, Einheiten-Warnung.
- Das Szenario übersteht Speichern und Öffnen.
- Vitest: Das Formular prüft Eingaben und zeigt Warnungen; Golden Image mit Tunnel.

## Akzeptanzkriterien

Der Task gilt erst als fertig, wenn alle Punkte erfüllt und durch Tests oder einen konkreten Befehl belegt sind:

- [x] `scenario.set_wind_tunnel` speichert ein vollständiges Szenario und liefert Bezugsfläche, Reynolds-Zahl und Zellbudget
- [x] Die Regeln für Tunnelgröße und Zellbudget stehen in `docs/` und sind durch Tests belegt
- [x] Der Tunnel ist in der App sichtbar und folgt Änderungen der Richtung

## Offene Fragen / Annahmen

- Annahme: Tunnelgröße nach üblichen Richtwerten (etwa 5 Modelllängen davor, 15 dahinter, 5 seitlich und darüber). Task_10 prüft sie am Vergleichsfall.
- Annahme: Faustwert für den Speicherbedarf je Million Zellen; in Task_09 und Task_37 nachmessen.

## Notizen zu diesem Task

Taskspezifische Informationen während der Arbeit hier festhalten. Taskübergreifendes gehört in `Roadmap/README.md`.

- Nachweis App (2026-11-21): Projekt "Demo" (Würfel + Rohr mit Flansch), Schritt "Szenario", 72 km/h, Luft 20 °C, fester Boden, Standard → Tunnel, Boden und Pfeil sichtbar; Reynolds-Zahl 69 486, Stirnfläche 0,00147 m², Bezugslänge 0,0523 m, Mach 0,058, 2 000 000 Zellen, etwa 7 min (`Roadmap/Nachweise/task08_windkanal.png`). Richtung 30° → Tunnel dreht mit, Werte ändern sich (`task08_windkanal_30grad.png`).
- Golden Image `tests/golden/wind_tunnel.png` (Würfel im Tunnel bei 30°).

## Abschluss-Checkliste

Reihenfolge und Details stehen in `AGENTS.md`, Abschnitt "Abschluss- und Archivierungs-Workflow".

- [x] Alle Akzeptanzkriterien erfüllt, Tests liegen permanent in `tests/` und laufen grün
- [x] Abschnitt "Erkenntnisse für Folge-Agenten" unten ausgefüllt
- [x] Folge-Tasks und `Roadmap/README.md` geprüft und bei Bedarf angepasst
- [x] Datei ohne `RUN_` nach `Roadmap/Archiv/` verschoben

## Erkenntnisse für Folge-Agenten

- **Umgesetzt (und Abweichungen vom Plan):** Crate `foamcraft-sim` (Stirnfläche per Rasterung, Länge in Strömungsrichtung, Medien, Tunnelregel, Zellbudget, Strömungsklasse, Warnungen, Befehle `scenario.set_wind_tunnel` und `scenario.info`); Szenario-Typen im Manifest (Schema 5); Tunnel/Boden/Pfeil im Renderer; React `ScenarioPanel` und Schritt-Tabs "Geometrie | Szenario". Abweichung: Das Zellbudget rechnet mit dem Speicher des Rechners (sysinfo); der Docker-Speicher (derzeit 1,9 GiB) wird erst beim Vernetzen geprüft.
- **Learnings & Stolpersteine:** sysinfo (neueste Version): `System::new()` liefert ein `Result`; `physical_core_count()` ist eine assoziierte Funktion. Modellkoordinaten sind f32, Vergleiche in Tests nicht enger als 1e-6 m. clippy verbietet `!(x > 0.0)` (`neg_cmp_op_on_partial_ord`).
- **Wichtige Entscheidungen und ihre Gründe:** Regeln in `docs/architecture.md` ("Wind tunnel"). Stirnfläche über Rasterung statt exakter Polygonvereinigung: einfach, robust, genau genug (< 0,2 % bei 2048 Zellen). Tunnel immer in Strömungsrichtung gedreht, damit Einlass und Auslass Stirnflächen des Tunnels sind (wichtig für blockMesh in Task_09).
- **Auswirkungen auf Folge-Tasks:** Task_09 erzeugt das Hintergrundnetz aus `tunnel_box` (gedrehter Quader: blockMesh mit gedrehten Eckpunkten oder Netz im Strömungssystem und Modell drehen) und verteilt Zellen nach `cell_budget`. Docker-Speicher (1,9 GiB im Hyper-V-Modus) begrenzt das Budget real stärker – Task_09 sollte `min(Rechner, Container)` verwenden. Der bewegte Boden ist im Modell vorhanden, wird aber erst in Task_43 gerechnet.
- **Hinweise zu Tests und Befehlen:** `cargo test -p foamcraft-sim` (Stirnfläche, Medien, Regeln, Befehle), Golden `cube_in_wind_tunnel_matches_golden_image`, Vitest `tests/app/scenario-panel.test.tsx`.
