# Task_06: Flächen erkennen und anklicken

> ## ⚠️ PFLICHT – BEVOR DU IRGENDETWAS ANDERES TUST
>
> 1. **Lies als Allererstes die Datei `AGENTS.md` im übergeordneten Ordner `Roadmap/`** (von hier: `../AGENTS.md`) und befolge sie strikt.
> 2. **Deine allererste Aktion** ist, diese Datei in `RUN_Task_06.md` umzubenennen, um den Task als laufend zu markieren. Das passiert **noch bevor irgendein Code geschrieben** wird.

## Ziel

Dreiecksmodelle werden in sinnvolle Flächen und Teile zerlegt. Der Nutzer wählt sie per Klick, benennt und gruppiert sie. Das ist die Grundlage für Einlässe, Auslässe und teilweise Auswertungen.

## Kontext & Abhängigkeiten

- **Voraussetzung:** Task_05 ist archiviert.
- **Relevantes Vorwissen:** `docs/architecture.md`, Regel 2: Auswahl muss auch ohne Fenster funktionieren (wichtig für den MCP-Server in Task_18).

## Umfang

**Gehört dazu:**

- Zerlegung nach Knickwinkel (Regionenwachstum, einstellbarer Winkel), zusammenhängende Körper als Teile
- Stabile Kennungen für Flächen und Teile, die Speichern und Öffnen überstehen
- Picking über die GPU (Kennungs-Puffer), Hervorhebung unter der Maus, Mehrfachauswahl, angrenzende Flächen hinzufügen
- Befehle zum Auswählen, Benennen und Gruppieren; Auswahl auch über einen Punkt im Raum oder eine Kennung
- Liste der Teile und Flächengruppen in der Oberfläche

**Gehört nicht dazu:**

- Öffnungen verschließen (Task_12)
- Flächen aus CAD-Dateien (Task_07)

## Umsetzungsschritte

1. Testkörper erzeugen: Würfel, Zylinder, leicht verrauschte Kugel, zwei getrennte Körper.
2. Tests zuerst für die Zerlegung, dann umsetzen.
3. Stabile Kennungen und deren Speicherung umsetzen.
4. Picking im Renderer umsetzen (Test offscreen).
5. Befehle für Auswahl, Namen und Gruppen ergänzen.
6. Liste und Hervorhebung in der Oberfläche umsetzen.

## Tests

Alle Tests dieses Tasks werden permanent in `tests/` im Hauptverzeichnis des Projekts abgelegt (siehe `AGENTS.md`).

- Zerlegung: Würfel ergibt 6 Flächen, Zylinder einen Mantel und 2 Deckel, die verrauschte Kugel 1 Fläche, zwei Körper ergeben 2 Teile.
- Kennungen bleiben nach Speichern und Öffnen gleich.
- Picking offscreen: Ein Strahl durch die Bildmitte trifft die erwartete Fläche.
- Auswahl über einen Punkt im Raum liefert dieselbe Fläche wie ein Klick.
- Vitest für die Liste der Teile und Gruppen.

## Akzeptanzkriterien

Der Task gilt erst als fertig, wenn alle Punkte erfüllt und durch Tests oder einen konkreten Befehl belegt sind:

- [x] Die vier Testkörper werden wie beschrieben zerlegt
- [x] Eine Fläche lässt sich in der App anklicken, benennen und einer Gruppe zuordnen; die Zuordnung übersteht einen Neustart
- [x] Dieselbe Auswahl ist per CLI-Befehl ohne Fenster möglich

## Offene Fragen / Annahmen

- Annahme: Bei organischen Scans entstehen viele kleine Flächen. Eine Schwelle zum Zusammenfassen wird hier festgelegt und in den Notizen begründet.

## Notizen zu diesem Task

Taskspezifische Informationen während der Arbeit hier festhalten. Taskübergreifendes gehört in `Roadmap/README.md`.

- Schwelle zum Zusammenfassen (Annahme aus "Offene Fragen"): Flächen unter 0,5 % der Fläche ihres Teils werden mit ihrer größten Nachbarfläche vereint (`MIN_FACE_AREA_SHARE`). Begründung: Scans organischer Formen zerfallen sonst bei 30° Knickwinkel in viele winzige Flächen, die niemand anklicken kann; 0,5 % lässt bei technischen Teilen (Würfelseite 16,7 %, Zylinderdeckel ≈ 17 %) alle echten Flächen stehen.
- Nachweis App (2026-11-21): Projekt "Demo" geöffnet, obere Würfelfläche angeklickt (orange hervorgehoben), "Deckel" benannt, Gruppe "Einlass" angelegt (`Roadmap/Nachweise/task06_flaeche_gruppe.png`); App beendet und neu gestartet, Gruppe "Einlass" (1 Fläche) noch da, Klick darauf wählt die Fläche "Deckel" wieder aus (`task06_nach_neustart.png`). Im Manifest: `"surface_names": {"model-1:face-2": "Deckel"}`, `"groups": [{"name": "Einlass", "keys": ["model-1:face-2"]}]`.
- Nachweis CLI ohne Fenster: `tests/cli/commands.rs::a_face_is_selected_and_grouped_without_a_window` (`command batch` mit `geometry.import`, `surface.select_at`, `group.create`).
- Leistung: Zerlegung von 1 Mio. Dreiecken ≈ 0,4 s (Test `one_million_triangles_are_segmented_within_the_limit`).

## Abschluss-Checkliste

Reihenfolge und Details stehen in `AGENTS.md`, Abschnitt "Abschluss- und Archivierungs-Workflow".

- [x] Alle Akzeptanzkriterien erfüllt, Tests liegen permanent in `tests/` und laufen grün
- [x] Abschnitt "Erkenntnisse für Folge-Agenten" unten ausgefüllt
- [x] Folge-Tasks und `Roadmap/README.md` geprüft und bei Bedarf angepasst
- [x] Datei ohne `RUN_` nach `Roadmap/Archiv/` verschoben

## Erkenntnisse für Folge-Agenten

- **Umgesetzt (und Abweichungen vom Plan):** `foamcraft_geometry::{segment, raycast, nearest_triangle}`, Modul `surfaces` mit `surface.*`, `group.*`, `geometry.set_crease_angle`; Manifest Schema 3 (`surface_names`, `groups`, `crease_angle_deg`); Session-Auswahl im Kern; Renderer mit Pick-IDs je Vertex, Pick-Durchgang (`R32Uint`) und Hervorhebung; App-Befehle `viewport_pick|hover|set_selection`; React `SurfacePanel` (rechts) und Klick/Hover in `ViewportInput`; CLI `command batch`. Testkörper (Zylinder, verrauschte Kugel, zwei Würfel) werden im Test erzeugt statt als Dateien abgelegt.
- **Learnings & Stolpersteine:** Der MSVC-Linker scheitert gelegentlich mit LNK1201 an einer veralteten PDB-Datei; Abhilfe: genau diese Datei `target/debug/deps/<name>-<hash>.pdb` löschen und neu bauen (kein Speicherproblem). clippy begrenzt Funktionen auf 7 Argumente (`DrawTarget`-Struct). Der Pick-Shader braucht `@interpolate(flat)` für die u32-ID. In wgpu 30 heißt die Uint-Vertexformat-Variante `Uint32`.
- **Wichtige Entscheidungen und ihre Gründe:** Flächen werden bei jedem Befehl aus der Datei neu zerlegt (deterministisch, kein Zustand, der veralten kann); für sehr große Modelle wäre ein Cache sinnvoll. Die Auswahl ist Sitzungszustand und kein Projektinhalt; Gruppen sind Projektinhalt. Klick in der App ruft `surface.select` auf (Logik bleibt in der Befehls-API).
- **Auswirkungen auf Folge-Tasks:** Task_08 (Windkanal) und Task_12/13 (Öffnungen, Innenströmung) können Flächengruppen (`manifest.groups`) als Einlass/Auslass verwenden. Task_09 (Vernetzung) braucht für Patches pro Gruppe die Dreiecke je Flächenschlüssel: `surfaces::segmented_models` liefert `face_of_triangle`. Wird der Knickwinkel geändert, ändern sich die Nummern – Gruppen verweisen dann evtl. auf andere Flächen (noch keine Warnung).
- **Hinweise zu Tests und Befehlen:** `cargo test -p foamcraft-geometry --test segment --test surfaces`, `cargo test -p foamcraft-render --test picking`, Vitest `tests/app/surface-panel.test.tsx` und `viewport-input.test.tsx`.
