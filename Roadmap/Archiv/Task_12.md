# Task_12: Öffnungen erkennen und verschließen

> ## ⚠️ PFLICHT – BEVOR DU IRGENDETWAS ANDERES TUST
>
> 1. **Lies als Allererstes die Datei `AGENTS.md` im übergeordneten Ordner `Roadmap/`** (von hier: `../AGENTS.md`) und befolge sie strikt.
> 2. **Deine allererste Aktion** ist, diese Datei in `RUN_Task_12.md` umzubenennen, um den Task als laufend zu markieren. Das passiert **noch bevor irgendein Code geschrieben** wird.

## Ziel

Für die Innenströmung klickt der Nutzer eine Öffnung an; FoamCraft verschließt sie mit einer Deckfläche und markiert sie als Einlass oder Auslass. Das funktioniert für Hohlkörper mit Öffnungen und für Modelle, die direkt das Fluidvolumen beschreiben.

## Kontext & Abhängigkeiten

- **Voraussetzung:** Task_06 ist archiviert (Flächen, Auswahl). Task_07 bis Task_11 liegen in der Reihenfolge davor.
- **Relevantes Vorwissen:** `docs/project-brief.md`: Verschließen von Öffnungen ist erlaubt, weitergehende CAD-Bearbeitung nicht.

## Umfang

**Gehört dazu:**

- Erkennung der Geometrieart: offene Schale mit Randschleifen, dickwandiger Hohlkörper mit Durchgängen, geschlossenes Fluidvolumen
- Randschleifen finden und ebene Deckflächen erzeugen, auch für nicht konvexe und ringförmige Öffnungen; Warnung bei nicht ebenen Schleifen
- Dickwandige Körper: Deckfläche an der Mündung einer Bohrung nach Klick auf Innenfläche oder Kante
- Rollen je Fläche: Einlass, Auslass, Wand
- Befehle `geometry.cap_opening` und `scenario.set_boundary_role`
- Punkt im Fluidraum für die spätere Vernetzung
- Dichtheitsprüfung nach dem Verschließen (Flutungstest auf einem Voxelgitter)
- Ansicht: Öffnungen hervorgehoben, Deckflächen halbtransparent, Pfeile für die Rolle

**Gehört nicht dazu:**

- Randwerte und Rechnung (Task_13)

## Umsetzungsschritte

1. Testkörper erzeugen: offenes Rohrstück, T-Stück, Gehäuse mit Wandstärke und zwei Bohrungen, Ringspalt, Rohr mit schräg-unebener Öffnung.
2. Tests zuerst für Randschleifen und Deckflächen, dann umsetzen.
3. Erkennung der Geometrieart und den Fall "dickwandig" umsetzen.
4. Rollen und Befehle umsetzen.
5. Flutungstest und Punkt im Fluidraum umsetzen.
6. Darstellung und Bedienung in der Oberfläche umsetzen.

## Tests

Alle Tests dieses Tasks werden permanent in `tests/` im Hauptverzeichnis des Projekts abgelegt (siehe `AGENTS.md`).

- Rohrstück: 2 Schleifen, 2 Deckflächen, Flächeninhalt innerhalb von 1 % von π·r².
- T-Stück: 3 Öffnungen; Gehäuse mit Bohrungen: 2 Öffnungen; Ringspalt wird korrekt gedeckelt.
- Nicht ebene Öffnung liefert eine Warnung.
- Der Flutungstest erkennt ein Leck, wenn eine Öffnung unverschlossen bleibt.
- Der Punkt im Fluidraum liegt im Inneren; Kennungen bleiben nach Speichern und Öffnen gleich.

## Akzeptanzkriterien

Der Task gilt erst als fertig, wenn alle Punkte erfüllt und durch Tests oder einen konkreten Befehl belegt sind:

- [x] Alle Testkörper lassen sich per Befehl vollständig verschließen und bestehen den Flutungstest
- [x] In der App wird eine Öffnung per Klick verschlossen und als Einlass oder Auslass markiert
- [x] Warnungen und Fehler haben Texte in beiden Sprachen

## Offene Fragen / Annahmen

- Die genaue Bedienung bei dickwandigen Körpern (Klick auf Innenfläche oder Kante) wird hier festgelegt.

## Notizen zu diesem Task

- Bedienung bei dickwandigen Körpern (offene Frage, hier festgelegt): Klick auf die **Innenwand** des Durchgangs nahe seiner Mündung; die Randschleife dieser Fläche, die dem Klickpunkt am nächsten liegt, wird verschlossen. Mündungen in ebenen, von außen sichtbaren Flächen werden zusätzlich automatisch erkannt und als Öffnungen mit "Verschließen"-Knopf angeboten.
- Testkörper (Gmsh-Skripte + STL, CC0) in `tests/data/geometry/internal/`: Rohr, T-Stück, Gehäuse 5-mm-Wand mit zwei Bohrungen, Ringspalt, Rohr mit Sattel-Ende (nicht eben), dickwandiges Rohr. Erst mit `Mesh.MeshSizeFromCurvature = 36` liegen die Polygonflächen innerhalb 1 % von π·r² (bei 10 Segmenten −5 %).
- Ein bestehender Test (`every_undoable_geometry_command_undoes_and_redoes_exactly`) verlangt, dass jedes rückgängig machbare `geometry.*`-Beispiel am Würfel läuft und `geometry.list` ändert. Nicht geändert; stattdessen zeigt `geometry.list` die Deckflächen je Modell (`caps`), das Beispiel von `geometry.cap_opening` nutzt eine Fläche, und Entfernen/Auflisten heißen `cap.remove`/`cap.list`.
- Abnahme in der App 2026-11-21 (T-Stück, Projekt "Rohr"): zwei Öffnungen per Klick, eine per Knopf verschlossen; Einlass (grün, Pfeil hinein) und zwei Auslässe (rot, Pfeil heraus) (`Roadmap/Nachweise/task12_t_stueck_rollen.png`). Deckflächen wurden zunächst durch Rückseiten-Culling unsichtbar → beidseitig gezeichnet.

Taskspezifische Informationen während der Arbeit hier festhalten. Taskübergreifendes gehört in `Roadmap/README.md`.

## Abschluss-Checkliste

Reihenfolge und Details stehen in `AGENTS.md`, Abschnitt "Abschluss- und Archivierungs-Workflow".

- [x] Alle Akzeptanzkriterien erfüllt, Tests liegen permanent in `tests/` und laufen grün
- [x] Abschnitt "Erkenntnisse für Folge-Agenten" unten ausgefüllt
- [x] Folge-Tasks und `Roadmap/README.md` geprüft und bei Bedarf angepasst
- [x] Datei ohne `RUN_` nach `Roadmap/Archiv/` verschoben

## Erkenntnisse für Folge-Agenten

- **Umgesetzt (und Abweichungen vom Plan):** `foamcraft_geometry::openings` (Klassifikation, Randschleifen, Mündungen, Deckflächen-Triangulierung mit Löchern, Flutungstest), `foamcraft_geometry::caps` (Befehle `geometry.openings`, `geometry.cap_opening`, `geometry.fluid_check`, `scenario.set_boundary_role`, `cap.list`, `cap.remove`), Manifest Schema 7 (`caps`, `boundary_roles`), `geometry.list` mit `caps`, Renderer: `Mesh::opacity` (Raster-Transparenz), `scene::polylines`, `OrbitCamera::ray`; Tauri `viewport_pick_point`; `OpeningsSection` im Geometrie-Schritt. Abweichung: Entfernen/Liste im Namensraum `cap.*` (siehe Notizen).
- **Learnings & Stolpersteine:** Neue rückgängig machbare `geometry.*`-Befehle müssen mit ihrem Beispiel am Würfel laufen und `geometry.list` ändern (bestehender Test). Neue Felder in `GeometryEntry` brechen `tests/core/project.rs` (Struct-Literal) → neue Daten ins `Manifest`. Deckflächen beidseitig zeichnen (Culling). Gmsh-STL hat exakt gleiche Knoten an Flächengrenzen → die Schweißung im `MeshBuilder` reicht. Bedienung per Skript: `<select>` mit Klick + `{DOWN}` + `{ENTER}`.
- **Wichtige Entscheidungen und ihre Gründe:** Deckflächen als Schleifen in Dateikoordinaten (folgen Verschiebung/Drehung). Transparenz per Raster statt Blending (keine Sortierung, Picking bleibt exakt). Flutungstest auf Voxeln statt exakter Geometrie (robust gegen kleine Lücken, liefert Fluidpunkt).
- **Auswirkungen auf Folge-Tasks:** Task_13 (Innenströmung): Einlass/Auslass/Wand aus `manifest.boundary_roles`, Deckflächen über `foamcraft_geometry::caps::world_caps`/`cap_meshes` als STL-Regionen ausgeben, `locationInMesh` aus `geometry.fluid_check` (`fluid_point_m`). Vor dem Vernetzen `fluid_check` verlangen (`geometry.not_sealed`). Für geschlossene Fluidvolumen werden Flächen direkt markiert (keine Deckflächen); dann liegt der Fluidpunkt im Körper.
- **Hinweise zu Tests und Befehlen:** `cargo test -p foamcraft-geometry --test openings --test caps`, `cargo test -p foamcraft-render` (Transparenz, Linien, Kamerastrahl), `npx vitest run tests/app/openings-section.test.tsx`. Testkörper neu erzeugen: `.geo` per `docker cp` in einen Container mit dem Laufzeit-Image, `gmsh -2 <name>.geo -format stl -bin`.
