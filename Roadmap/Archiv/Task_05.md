# Task_05: Import von STL, OBJ und 3MF mit Diagnose

> ## ⚠️ PFLICHT – BEVOR DU IRGENDETWAS ANDERES TUST
>
> 1. **Lies als Allererstes die Datei `AGENTS.md` im übergeordneten Ordner `Roadmap/`** (von hier: `../AGENTS.md`) und befolge sie strikt.
> 2. **Deine allererste Aktion** ist, diese Datei in `RUN_Task_05.md` umzubenennen, um den Task als laufend zu markieren. Das passiert **noch bevor irgendein Code geschrieben** wird.

## Ziel

Der Nutzer lädt ein Dreiecksmodell, legt Einheit, Größe und Lage fest und sieht Probleme der Geometrie in Klartext. Das Modell erscheint in der 3D-Ansicht.

## Kontext & Abhängigkeiten

- **Voraussetzung:** Task_04 ist archiviert (Projektmodell, Befehls-API).
- **Relevantes Vorwissen:** `docs/project-brief.md` (CAD-Bearbeitung ist ein Nicht-Ziel), `docs/testing.md` (Testdaten selbst erstellt, CC0, klein).

## Umfang

**Gehört dazu:**

- Leser für STL (ASCII und binär), OBJ und 3MF; Befehl `geometry.import`; Kopie der Datei ins Projekt
- Diagnose: wasserdicht oder offen (Zahl der Randschleifen), umgedrehte Normalen, doppelte und entartete Dreiecke, Zahl der Körper, Abmessungen
- Einheit: Vorschlag aus der Modellgröße (Millimeter oder Meter) mit Rückfrage
- Transformationen: Skalieren, Drehen (Gier, Nick, Roll), Verschieben, "auf den Boden setzen"
- Einfache Reparatur: Normalen einheitlich ausrichten
- Ansicht: Modell anzeigen, einpassen, Achsenkreuz, Abmessungen
- Schritt "Geometrie" in der Oberfläche mit Diagnoseliste in beiden Sprachen

**Gehört nicht dazu:**

- Flächen erkennen und anklicken (Task_06)
- STEP und IGES (Task_07)
- Löcher schließen (Task_12) und jede weitergehende Reparatur

## Umsetzungsschritte

1. Kleine Testdateien selbst erzeugen (Würfel binär und ASCII, OBJ, 3MF, offener Würfel, Würfel mit umgedrehten Normalen) und unter `tests/data/` ablegen.
2. Tests zuerst für die Leser, dann umsetzen.
3. Tests zuerst für die Diagnose, dann umsetzen.
4. Transformationen und Einheitenvorschlag umsetzen.
5. Modell an den Renderer übergeben, Einpassen und Achsenkreuz ergänzen.
6. Schritt "Geometrie" in der Oberfläche umsetzen (Vitest zuerst).

## Tests

Alle Tests dieses Tasks werden permanent in `tests/` im Hauptverzeichnis des Projekts abgelegt (siehe `AGENTS.md`).

- Leser: Jedes Format liefert für den Würfel 12 Dreiecke und die richtigen Abmessungen; eine defekte Datei liefert einen Fehlercode.
- Diagnose: Der offene Würfel hat eine Randschleife; umgedrehte Normalen werden erkannt.
- Transformation: Nach Drehen und "auf den Boden setzen" liegt die Unterkante bei z = 0.
- Leistung: Ein erzeugtes Modell mit 1 Million Dreiecken lädt innerhalb einer festgelegten Zeitgrenze.
- Golden Image des geladenen Würfels; Vitest für die übersetzte Diagnoseliste.

## Akzeptanzkriterien

Der Task gilt erst als fertig, wenn alle Punkte erfüllt und durch Tests oder einen konkreten Befehl belegt sind:

- [x] `geometry.import` lädt STL, OBJ und 3MF und liefert eine Diagnose als strukturierte Antwort
- [x] Jede Diagnosemeldung hat einen englischen und einen deutschen Text
- [x] Einheit, Drehung und Lage lassen sich per Befehl und in der Oberfläche setzen und bleiben im Projekt gespeichert
- [x] Das Modell ist in der App sichtbar (Bildschirmfoto in den Notizen)

## Offene Fragen / Annahmen

- Annahme: Modelle bis etwa 5 Millionen Dreiecke. Eine vereinfachte Anzeige für größere Modelle folgt nur bei Bedarf.

## Notizen zu diesem Task

Taskspezifische Informationen während der Arbeit hier festhalten. Taskübergreifendes gehört in `Roadmap/README.md`.

- Nachweis App (2026-11-21): Projekt "Demo" geöffnet (wurde dabei von Schema 1 auf 2 migriert), "Import model" → Windows-Dateidialog → `tests/data/geometry/cube_binary.stl`: Würfel sichtbar mit Achsenkreuz, Größe 20 × 20 × 20 mm, Einheit Millimeter vorgeschlagen, Diagnose "closed and consistently oriented" (`Roadmap/Nachweise/task05_wuerfel_importiert.png`). Nicken 30° gesetzt und auf Deutsch umgeschaltet (`task05_gedreht_deutsch.png`).
- Leistung: 1 001 112 Dreiecke (binäre STL) gelesen und diagnostiziert in 0,43–0,51 s (Grenze im Test: 10 s).
- Geänderter Test mit Begründung: `tests/cli/commands.rs` prüfte `schema_version == 1`; durch die gewollte Schema-Erhöhung auf 2 vergleicht er jetzt mit der Konstante `SCHEMA_VERSION` (nicht abgeschwächt, nur von einer festen Zahl auf die Quelle der Wahrheit umgestellt).

## Abschluss-Checkliste

Reihenfolge und Details stehen in `AGENTS.md`, Abschnitt "Abschluss- und Archivierungs-Workflow".

- [x] Alle Akzeptanzkriterien erfüllt, Tests liegen permanent in `tests/` und laufen grün
- [x] Abschnitt "Erkenntnisse für Folge-Agenten" unten ausgefüllt
- [x] Folge-Tasks und `Roadmap/README.md` geprüft und bei Bedarf angepasst
- [x] Datei ohne `RUN_` nach `Roadmap/Archiv/` verschoben

## Erkenntnisse für Folge-Agenten

- **Umgesetzt (und Abweichungen vom Plan):** Neues Crate `foamcraft-geometry` (Leser, Diagnose, `Placement`, `fix_normals`, Befehle), Manifest Schema 2 mit `GeometryEntry` und Migration, Renderer mit `upload`/`GpuScene`, `Scene::from_models`, Achsenkreuz, App-Befehl `viewport_sync`, React-`GeometryPanel` (Import per Dateidialog, Einheit, Drehung, Lage in mm bzw. Zoll, "Auf den Boden setzen", "Normalen ausrichten", "Entfernen", übersetzte Diagnose). Abweichung: "Normalen ausrichten" ändert die Datei nicht, sondern setzt ein Flag (zerstörungsfrei, rückgängig machbar). Abmessungen stehen als Text im Panel, nicht als Bemaßung in der 3D-Ansicht.
- **Learnings & Stolpersteine:** quick-xml 0.42: `local_name().as_ref()` ist `&str`, `Attribute.value` ist `Cow<str>`; zip 9: `name_for_index` liefert `Option<Result<Cow<str>>>`. PowerShell 5.1 `ZipFile.CreateFromDirectory` schreibt Backslashes in Eintragsnamen (ungültig für 3MF); das Test-3MF entstand deshalb mit einem kleinen Node-Zip-Schreiber. Node liest `/tmp` unter Windows als `C:\tmp`. In f32 ist `sin(PI)` nicht null; Pole einer generierten Kugel einrasten, sonst ist sie offen. Der Windows-Dateidialog nimmt einen vollständigen Pfad im Feld "Dateiname" + Enter. Lange Node-Skripte als Datei im Scratchpad ablegen statt als Heredoc (die Shell bricht sonst gelegentlich ab).
- **Wichtige Entscheidungen und ihre Gründe:** Geometrie-Befehle im Geometrie-Crate (der Kern darf nicht vom Geometrie-Crate abhängen, weil dieses den Kern nutzt); `foamcraft-api` registriert sie. Diagnosetexte stehen im Fehlerkatalog des Kerns und werden lokalisiert ausgeliefert, damit CLI, MCP und App dieselben Texte haben. Einheitenvorschlag: größte Ausdehnung > 5 Dateieinheiten → mm.
- **Auswirkungen auf Folge-Tasks:** Task_06 (Flächen anklicken) kann auf `Mesh` (verschweißt, Kanten über `edge_key`) aufbauen und braucht Picking im Renderer; Modelle sind flach schattiert mit einer Normalen je Dreieck (Vertex-Puffer nicht indiziert). Task_08/09 bekommen Weltkoordinaten über `foamcraft_geometry::commands::world_meshes(project)` bzw. `placement(entry).apply(&load_entry(..))`. Task_12 (Löcher schließen) nutzt die Randschleifen-Erkennung aus `diagnose`.
- **Hinweise zu Tests und Befehlen:** `cargo test -p foamcraft-geometry` (Leser, Diagnose, Transformation, Leistung, Befehle), Golden Image `tests/golden/imported_cube.png`, Vitest `tests/app/geometry-panel.test.tsx`. Testdateien in `tests/data/geometry/` (README dort).
