# Task_07: Import von STEP und IGES

> ## ⚠️ PFLICHT – BEVOR DU IRGENDETWAS ANDERES TUST
>
> 1. **Lies als Allererstes die Datei `AGENTS.md` im übergeordneten Ordner `Roadmap/`** (von hier: `../AGENTS.md`) und befolge sie strikt.
> 2. **Deine allererste Aktion** ist, diese Datei in `RUN_Task_07.md` umzubenennen, um den Task als laufend zu markieren. Das passiert **noch bevor irgendein Code geschrieben** wird.

## Ziel

CAD-Dateien werden mit ihren echten Flächen geladen: Jede CAD-Fläche ist eine anklickbare Fläche. Der Import funktioniert auch ohne Fenster.

## Kontext & Abhängigkeiten

- **Voraussetzung:** Task_06 ist archiviert (Flächen, Kennungen, Auswahl).
- **Relevantes Vorwissen:** `docs/architecture.md`, offene Entscheidung "STEP and IGES import"; `docs/licensing.md` (LGPL ist erlaubt). Der Nutzer kann Testdateien mit SolidCraft erzeugen.

## Umfang

**Gehört dazu:**

- Technikentscheidung per kurzem Vergleich: Open CASCADE nativ gelinkt, als WebAssembly oder Konvertierung im Container. Kriterien: Build unter Windows, Betrieb ohne Fenster, Lizenz, Paketgröße
- Import von STEP (AP203, AP214, AP242) und IGES
- Tessellierung mit einer Genauigkeit, die zur Modellgröße passt
- Flächen-Kennungen, Namen und Farben aus der Datei, soweit vorhanden; Einheiten aus der Datei; Baugruppen als Teile
- Dieselbe Diagnose wie in Task_05

**Gehört nicht dazu:**

- CAD-Bearbeitung (Nicht-Ziel des Projekts)
- Reparatur defekter CAD-Daten

## Umsetzungsschritte

1. Die drei Wege kurz erproben und die Entscheidung mit Begründung in `docs/architecture.md` festhalten.
2. Testdateien bereitstellen (Würfel, Rohr mit Flansch als STEP, eine IGES-Datei), selbst erstellt und CC0.
3. Tests zuerst für Flächenzahl, Einheiten und Teile.
4. Import und Tessellierung umsetzen.
5. In `geometry.import` einhängen, Diagnose anbinden.
6. Betrieb ohne Fenster über die CLI prüfen.

## Tests

Alle Tests dieses Tasks werden permanent in `tests/` im Hauptverzeichnis des Projekts abgelegt (siehe `AGENTS.md`).

- STEP-Würfel ergibt 6 Flächen, das Rohr mit Flansch die erwartete Flächenzahl.
- Eine Datei in Millimetern wird korrekt in Meter umgerechnet.
- Die IGES-Datei wird geladen; eine defekte Datei liefert einen Fehlercode.
- Import über die CLI ohne Fenster.

## Akzeptanzkriterien

Der Task gilt erst als fertig, wenn alle Punkte erfüllt und durch Tests oder einen konkreten Befehl belegt sind:

- [x] `geometry.import` lädt STEP und IGES mit anklickbaren CAD-Flächen
- [x] Der Import läuft über die CLI ohne Fenster
- [x] Die Technikentscheidung steht mit Begründung in `docs/architecture.md`; die Lizenzprüfung läuft weiter grün

## Offene Fragen / Annahmen

- Offene Entscheidung "STEP and IGES import" fällt in diesem Task.
- Annahme: Sehr große Baugruppen werden nicht optimiert.

## Notizen zu diesem Task

Taskspezifische Informationen während der Arbeit hier festhalten. Taskübergreifendes gehört in `Roadmap/README.md`.

- Testdateien (CC0) mit Gmsh selbst erzeugt (`tests/data/cad/`, README dort): Würfel 20 mm (STEP und IGES), Rohr Ø20/16 mm × 50 mm mit Flansch Ø50 × 5 mm (STEP). Erwartete Flächenzahl Rohr mit Flansch: 6 (Rohr außen, Rohr Stirnfläche oben, Flansch oben, Flansch außen, Flansch unten, Bohrung innen durchgehend), 1 Volumen.
- Nachweis App (2026-11-21): `pipe_flange.step` über den Dateidialog importiert (3016 Dreiecke, 50 × 50 × 50 mm); Klick auf die Flanschoberseite wählt genau diese CAD-Fläche (Kreisring) aus (`Roadmap/Nachweise/task07_step_flaeche.png`).
- Nachweis CLI ohne Fenster: `runtime_step_import_works_from_the_command_line` (`project new`, `command run geometry.import`, `surface.list` → 6 Flächen).
- Image wächst durch Gmsh von ~1,2 auf 1,41 GB.

## Abschluss-Checkliste

Reihenfolge und Details stehen in `AGENTS.md`, Abschnitt "Abschluss- und Archivierungs-Workflow".

- [x] Alle Akzeptanzkriterien erfüllt, Tests liegen permanent in `tests/` und laufen grün
- [x] Abschnitt "Erkenntnisse für Folge-Agenten" unten ausgefüllt
- [x] Folge-Tasks und `Roadmap/README.md` geprüft und bei Bedarf angepasst
- [x] Datei ohne `RUN_` nach `Roadmap/Archiv/` verschoben

## Erkenntnisse für Folge-Agenten

- **Umgesetzt (und Abweichungen vom Plan):** Konvertierung im Runtime-Container mit Gmsh (`DockerCad` in `foamcraft-api`), Gmsh-Leser (`.msh` 4.1 ASCII) mit Flächen- und Volumen-Tags je Dreieck, `segment` nutzt CAD-Flächen statt Knickwinkel, `CadConverter`-Trait mit `register_with_cad`, Manifest Schema 4 (`source`), Dateidialog um STEP/IGES erweitert, Teileliste zeigt den Modellnamen. Abweichung: Namen und Farben aus der CAD-Datei werden noch nicht übernommen (Gmsh legt sie nicht ohne Weiteres in die .msh); Baugruppen werden über CAD-Volumen zu Teilen.
- **Learnings & Stolpersteine:** `Geometry.OCCTargetUnit` wirkt nur für STEP; bei IGES meldet Gmsh "Could not set OpenCASCADE target unit" und bleibt in der Dateieinheit → Einheit aus dem IGES-Globalabschnitt (Parameter 14) lesen. Gmsh orientiert Dreiecke je Fläche nach deren Parametrisierung; nach dem Lesen `fix_normals` anwenden. Die .msh enthält auch Punkt- und Linienelemente (nur Typ 2 auf Dimension 2 lesen). Gmsh-Syntax für Booleans: `u() = BooleanUnion{ Volume{1}; Delete; }{ Volume{2}; Delete; };`. Perl-Ersetzungen mit `\n` im Ersatztext fügen wörtliche Backslash-n ein – für Dockerfile-Änderungen das Edit-Werkzeug nutzen.
- **Wichtige Entscheidungen und ihre Gründe:** Siehe Vergleichstabelle in `docs/architecture.md`: kein C++-Build unter Windows, gleiche Laufzeit wie die Simulation, funktioniert ohne Fenster. Nachteil: STEP/IGES-Import braucht Docker (Fehlercode `geometry.cad_unavailable` mit Abhilfe).
- **Auswirkungen auf Folge-Tasks:** Task_09 (Vernetzung) kann für CAD-Modelle die .msh-Dreiecke (Datei in `entry.file`) wie jede andere Oberfläche verwenden; Gruppen auf CAD-Flächen bleiben stabil, solange die Datei gleich bleibt (keine Abhängigkeit vom Knickwinkel). Task_21 (Assistent/Beispiele) und Task_32 (Release) müssen die Image-Größe (1,41 GB) berücksichtigen. Nach Dockerfile-Änderungen Image neu bauen.
- **Hinweise zu Tests und Befehlen:** Ohne Docker: `cargo test -p foamcraft-geometry --test cad` (mit aufgezeichneten Gmsh-Ausgaben `tests/data/cad/*.msh`). Mit Docker: `cargo test -p foamcraft-api --test cad_runtime -- --ignored` und `runtime_step_import_works_from_the_command_line`.
