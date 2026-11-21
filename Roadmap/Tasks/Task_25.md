# Task_25: Modell in der Ansicht verschieben und drehen

> ## ⚠️ PFLICHT – BEVOR DU IRGENDETWAS ANDERES TUST
>
> 1. **Lies als Allererstes die Datei `AGENTS.md` im übergeordneten Ordner `Roadmap/`** (von hier: `../AGENTS.md`) und befolge sie strikt.
> 2. **Deine allererste Aktion** ist, diese Datei in `RUN_Task_25.md` umzubenennen, um den Task als laufend zu markieren. Das passiert **noch bevor irgendein Code geschrieben** wird.

## Ziel

Ein ausgewähltes Modell lässt sich direkt in der 3D-Ansicht bewegen: an Pfeilen entlang der Achsen ziehen, an Ringen drehen. Das Achsenkreuz sitzt dabei am Modell (Mitte, Schwerpunkt oder Modellursprung, wählbar) statt am Weltursprung. Zahlen einzugeben bleibt möglich, ist aber nicht mehr nötig.

## Kontext & Abhängigkeiten

- **Voraussetzung:** Task_24 ist archiviert.
- **Herkunft:** Rückmeldung des Nutzers (2026-11-21): "Ich möchte es im freien Raum verschieben können, ohne Zahlen einzugeben … über das Kreuz X Y Z, wenn das Modell selektiert ist … auch Drehpunkte … das Kreuz am Schwerpunkt oder in der Mitte." Der Knopf "Per Klick in der Ansicht verschließen" (Öffnungen) wurde als "verschieben" gelesen: Die Beschriftung ist missverständlich, solange es kein Verschieben in der Ansicht gibt.
- **Relevantes Vorwissen:** `geometry.set_transform` (Gieren/Nicken/Rollen, Verschiebung in Metern), `foamcraft_geometry::Placement` (Drehung Z·Y·X), Picking über Pick-Kennungen (`viewport_pick`), Kamera-Strahl `OrbitCamera::ray`. `project.changed` lädt heute das Modell neu und zerlegt es wieder in Flächen: für jede Mausbewegung zu langsam, deshalb braucht das Ziehen eine Vorschau ohne Befehl und genau einen Befehl am Ende (ein Undo-Schritt).

## Umfang

**Gehört dazu:**

- Griffe am Bezugspunkt des aktiven Modells: drei Pfeile (verschieben entlang X, Y, Z), drei Ringe (drehen um X, Y, Z)
- Bezugspunkt wählbar: Mitte der Hüllbox (Standard), Schwerpunkt des Volumens, Modellursprung; Befehl `geometry.pivot`
- Neue Befehle `geometry.move_by` und `geometry.rotate_about` (Drehung um eine Weltachse durch den Bezugspunkt), rückgängig machbar; CLI und MCP nutzen dieselben
- Ziehen mit flüssiger Vorschau; beim Loslassen genau ein Befehl
- Einrasten: Verschieben auf runde Schritte, Drehen auf 5° (Umschalt: frei)
- Aktives Modell: das angeklickte Modell; bei nur einem Modell dieses
- Anzeige des aktuellen Werts beim Ziehen (Strecke oder Winkel)
- "Auf den Boden setzen" und Zahlenfelder bleiben

**Gehört nicht dazu:**

- Skalieren mit Griffen
- Mehrere Modelle gleichzeitig bewegen
- Ziehen der Windrichtung (Task_26) und der Schnitte (Task_27); die Griff-Technik dieses Tasks ist dafür wiederverwendbar zu bauen

## Umsetzungsschritte

1. Tests zuerst für die Mathematik: Zug entlang einer Achse aus zwei Kamera-Strahlen, Drehwinkel um eine Achse, Drehung um einen Bezugspunkt samt Rückrechnung auf Gieren/Nicken/Rollen, Bezugspunkte (Rust).
2. Befehle `geometry.pivot`, `geometry.move_by`, `geometry.rotate_about` umsetzen.
3. Griffe im Renderer: zeichnen, picken, Vorschau-Transformation für das aktive Modell.
4. Eingabe in der App: Griff hat Vorrang vor Kameradrehung; Ziehen, Loslassen, Abbrechen mit Esc.
5. Oberfläche: Bezugspunkt-Auswahl, Wertanzeige, Hinweistext; Beschriftung bei den Öffnungen entschärfen.
6. Live prüfen und Nachweise ablegen; `docs/architecture.md` ergänzen.

## Umsetzungshinweise

Aus der Sichtung des Codes am 2026-11-21. Dieser Task legt die Griff-Technik an, die Task_26, Task_27 und Task_31 wiederverwenden; deshalb lohnt hier ein sauberer Schnitt.

**Reihenfolge, die sich bewährt:** erst reine Mathematik mit Tests, dann Befehle, dann Renderer, zuletzt Eingabe. Jede Stufe ist für sich grün.

- **Mathematik der Griffe** (neues Modul, z. B. `crates/foamcraft-render/src/handles.rs`, nur `glam`, Tests in `tests/render/`):
  - Achsenzug: Parameter des Punktes auf der Achse `O + t·A`, der dem Mausstrahl `R + s·D` am nächsten liegt: `t = ((A·w)(D·D) − (D·w)(A·D)) / ((A·A)(D·D) − (A·D)²)` mit `w = R − O`. Die Verschiebung ist `t_jetzt − t_start`. Ist der Nenner fast null (Blick entlang der Achse), den letzten Wert behalten statt zu springen.
  - Ringzug: Strahl mit der Ebene durch den Bezugspunkt senkrecht zur Achse schneiden, Winkel per `atan2` in einer festen Basis der Ebene, Differenz zum Startwinkel auf ±180° normalisieren. Liegt die Ebene fast in Blickrichtung (`|D·Achse| < 0,05`), ebenfalls den letzten Wert behalten.
  - Den Strahl liefert `OrbitCamera::ray(aspect, ndc_x, ndc_y)`; die Umrechnung von Pixeln steht in `viewport_pick_point` in `app/src-tauri/src/main.rs`.
- **Drehung um einen Punkt** (in `crates/foamcraft-geometry/src/transform.rs`): `Placement::rotation()` ist `Rz(Gieren)·Ry(Nicken)·Rx(Rollen)`. Neue Drehung `R' = R_Achse(Winkel)·R`, neue Verschiebung `t' = p + R_Achse·(t − p)`. Rückrechnung auf die drei Winkel: `Nicken = asin(−R'[2][0])`, `Gieren = atan2(R'[1][0], R'[0][0])`, `Rollen = atan2(R'[2][1], R'[2][2])`; bei `|R'[2][0]| ≈ 1` Rollen = 0 setzen und `Gieren = atan2(−R'[0][1], R'[1][1])`. `glam::DMat3` ist spaltenweise gespeichert: auf Zeile/Spalte achten und mit dem Test "viermal 90° ergibt die Ausgangslage" absichern.
- **Bezugspunkte:** Mitte der Hüllbox aus `Mesh::bounds()` des Weltnetzes; Schwerpunkt als Summe über die vorzeichenbehafteten Tetraeder (Dreieck plus Ursprung), Volumen und Vorzeichen wie in `diagnose::signed_volume`; Modellursprung ist `translation`.
- **Befehle** (`crates/foamcraft-geometry/src/commands.rs`, Muster `update(...)`): `geometry.pivot {id}` (liefert alle drei Punkte), `geometry.move_by {id, delta}`, `geometry.rotate_about {id, axis, angle_deg, pivot}`. Rückgängig machbare `geometry.*`-Befehle müssen mit ihrem `example` am Testwürfel laufen (bestehender Test in `tests/geometry/commands.rs`), also ein Beispiel wählen, das den Zustand wirklich ändert. MCP: als kuratierte Werkzeuge in `CURATED` (`crates/foamcraft-mcp/src/lib.rs`) eintragen.
- **Vorschau im Renderer:** Heute hat kein Netz eine eigene Transformation (`shader.wgsl`: `view_proj * position`). `ObjectUniform` um eine `model`-Matrix erweitern (Standard: Einheitsmatrix), im Vertex-Shader auf Position und Normale anwenden, den Uniform-Puffer mit `COPY_DST` anlegen und in `GpuScene` behalten, dazu `Renderer::set_transform(...)`. Mit Einheitsmatrix bleiben alle Golden-Images pixelgleich; das vor dem Weiterbauen mit `cargo test -p foamcraft-render` prüfen. Der Pick-Durchlauf nutzt denselben Vertex-Shader und bekommt die Vorschau damit automatisch.
- **Welche GPU-Netze gehören zum Modell:** `project_scene` (`crates/foamcraft-api/src/scene.rs`) legt die Modelle als erste `models_len` Netze an. `Renderer::upload` lässt Netze ohne Indizes weg, die Positionen verschieben sich dann; die Zuordnung Modell → Netzindex deshalb in `ProjectScene` mitgeben, nicht nachzählen.
- **Griffe zeichnen und treffen:** als eigene Netze mit Pick-Kennungen, deren Schlüssel mit `handle:` beginnen (`handle:move-x`, `handle:rotate-z`). So liefert das vorhandene `viewport_hover`/`viewport_pick` den Griff ohne neuen Mechanismus. `App.onPick` darf solche Schlüssel nicht an `surface.select` schicken.
  - Griffe getrennt von der Szene hochladen (eigene kleine Liste in `GpuScene`), sonst wird beim Wechsel des aktiven Modells die ganze Szene neu hochgeladen.
  - Es gibt keinen "immer im Vordergrund"-Durchlauf. Liegt der Bezugspunkt im Modell, sind die Griffe verdeckt. Lösung: Griffe in einem zweiten Durchlauf nach dem Löschen der Tiefe zeichnen (Kennzeichen am Netz). Den Durchlauf auch für den Pick-Durchlauf übernehmen, sonst trifft man den Griff nicht.
  - Größe aus dem Radius der Modell-Hüllkugel (z. B. Pfeile 1,2 × Radius), nicht in Pixeln; das spart die Abhängigkeit vom Zoom.
- **Eingabe:** `ViewportInput` muss beim Drücken sofort wissen, ob ein Griff oder die Kamera gemeint ist; ein `invoke` ist aber asynchron. Der Ausweg ist schon da: `viewport_hover` läuft bei jeder Mausbewegung und liefert den Schlüssel unter der Maus. Den zuletzt gemeldeten Schlüssel in einem `ref` halten; beginnt er mit `handle:`, startet das Drücken einen Griffzug, sonst wie bisher die Kamera. `tests/app/viewport-input.test.tsx` gibt das heutige Verhalten vor und bleibt unverändert: die Griff-Behandlung als optionale Eigenschaft ergänzen.
- **Ablauf eines Zugs:** `viewport_handle_begin` (merkt Startstrahl und Ausgangslage) → `viewport_handle_drag {x, y, free}` bei jeder Bewegung (rechnet, setzt die Vorschau-Matrix, zeichnet, gibt den Wert für die Anzeige zurück) → `viewport_handle_end` gibt Befehlsnamen und Parameter zurück, die Oberfläche ruft den Befehl über `tauriApi.call`. So bleibt "jede Aktion ist ein Befehl" gewahrt und der Zug ist genau ein Undo-Schritt. Esc → `viewport_handle_cancel` setzt die Matrix zurück.
- **Stolperstein Kamera:** `viewport_sync` ruft nach jedem `project.changed` `view.fit()` auf. Nach einem Zug würde die Kamera springen. Nur neu einpassen, wenn sich das Projekt oder die Menge der Modelle geändert hat (oder ein Parameter `fit`).
- **Folge für das Netz:** Jede Verschiebung macht das Netz ungültig (`mesh::mesh_is_current`, Task_23); beim nächsten Start wird neu vernetzt. Das ist gewollt, im Hinweistext aber erwähnenswert.
- **Beschriftung bei den Öffnungen:** Der englische Text "Close by clicking in the view" wird in `tests/app/openings-section.test.tsx` abgefragt und bleibt. Missverständlich war nur der deutsche ("verschließen" neben "verschieben"); den deutschen Text umformulieren, zum Beispiel "Öffnung per Klick abdecken".
- **Live prüfen:** Ein Griffzug braucht echte Mausbewegung im Fenster. Nur ausführen, wenn der Nutzer nicht am Rechner arbeitet, sonst ihn um einen kurzen Test bitten. Die Mathematik, die Befehle und das Bild der Griffe (Golden-Image, Picking-Test wie `tests/render/picking.rs`) sind ohne Fenster prüfbar.

## Tests

Alle Tests dieses Tasks werden permanent in `tests/` im Hauptverzeichnis des Projekts abgelegt (siehe `AGENTS.md`).

- Rust: Zug-Mathematik (Achse, Ring) für mehrere Kamerastellungen, auch fast parallel zur Blickrichtung (kein Sprung).
- Rust: `geometry.rotate_about` lässt den Bezugspunkt an seinem Ort; viermal 90° ergibt die Ausgangslage; Undo/Redo.
- Rust: Bezugspunkte eines unsymmetrischen Körpers (Mitte ≠ Schwerpunkt).
- Golden-Image: Griffe am Modell; Picking trifft Pfeil und Ring.
- Vitest: Bezugspunkt-Auswahl ruft den Befehl; Wertanzeige beim Ziehen.

## Akzeptanzkriterien

Der Task gilt erst als fertig, wenn alle Punkte erfüllt und durch Tests oder einen konkreten Befehl belegt sind:

- [ ] Das Modell des Beispielprojekts lässt sich mit der Maus entlang jeder Achse verschieben und um jede Achse drehen
- [ ] Ein Zug ist ein Undo-Schritt
- [ ] Der Bezugspunkt ist umschaltbar, die Griffe sitzen dort
- [ ] Dieselbe Bewegung ist über CLI oder MCP als Befehl möglich

## Offene Fragen / Annahmen

- Annahme: Drehungen wirken um Weltachsen durch den Bezugspunkt; das ist für Laien am leichtesten vorherzusagen.

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
