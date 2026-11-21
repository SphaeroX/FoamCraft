# Task_27: Schnitte in der Ansicht ziehen, Rauch aufschneiden

> ## ⚠️ PFLICHT – BEVOR DU IRGENDETWAS ANDERES TUST
>
> 1. **Lies als Allererstes die Datei `AGENTS.md` im übergeordneten Ordner `Roadmap/`** (von hier: `../AGENTS.md`) und befolge sie strikt.
> 2. **Deine allererste Aktion** ist, diese Datei in `RUN_Task_27.md` umzubenennen, um den Task als laufend zu markieren. Das passiert **noch bevor irgendein Code geschrieben** wird.

## Ziel

Schnitte durch die Strömung fasst man in der 3D-Ansicht an und zieht sie dorthin, wo man sie haben will. Derselbe Schnitt kann Modell, Rauch, Partikel und Isoflächen aufschneiden, damit man in die Strömung hineinsehen kann.

## Kontext & Abhängigkeiten

- **Voraussetzung:** Task_26 ist archiviert; die Griff-Technik aus Task_25 wird wiederverwendet.
- **Herkunft:** Rückmeldung des Nutzers (2026-11-21): "Ich kann den Schnitt nicht von Hand verschieben … einfach dahin ziehen, wo ich ihn haben möchte … den Rauch schneiden können, dass ich in den Rauch reinziehen kann … Schnitte auswählen."
- **Relevantes Vorwissen:** `view.add_slice`, `view.update_slice`, `view.set_clip` (`crates/foamcraft-api/src/view.rs`), Schnitt-Netz in `foamcraft_sim::visual::slice`, Partikel auf der GPU (`particles.wgsl`), `FlowViewSection.tsx` (Schieberegler je Schnitt). Der Schnitt wird heute bei jeder Änderung neu abgetastet und die ganze Szene neu hochgeladen.

## Umfang

**Gehört dazu:**

- Schnitt in der Ansicht anklicken (wählt ihn aus, Rahmen hervorgehoben) und entlang seiner Normalen ziehen; flüssige Vorschau, ein Befehl beim Loslassen
- Ausgewählten Schnitt kippen: Ausrichtung X, Y, Z per Knopf, feines Drehen am Ring-Griff
- Aufschneiden wirkt wahlweise auf Modell, Partikel/Rauch, Stromlinien und Isoflächen (`view.set_clip` mit Zielen)
- Liste der Schnitte mit Auswahl, Ein-/Ausblenden, Feld und Löschen
- Schneller Neuaufbau: nur der bewegte Schnitt wird neu berechnet

**Gehört nicht dazu:**

- Gekrümmte Schnittflächen, mehrere Schnittebenen gleichzeitig als Beschnitt
- Neue Felder

## Umsetzungsschritte

1. Tests zuerst: Zug entlang der Normalen bleibt im Rechengebiet; Beschnitt entfernt Partikel und Isoflächen-Dreiecke auf der abgewandten Seite (Rust, CPU-Referenz und GPU).
2. `view.set_clip` um Ziele erweitern (Manifest-Schema anheben, Migration), Partikel-Shader und Isoflächen beschneiden.
3. Schnitt als Griff: Picking, Auswahl, Ziehen mit Vorschau.
4. Oberfläche: Schnittliste mit Auswahl und Sichtbarkeit, Ausrichtungsknöpfe, Beschnitt-Ziele.
5. Live prüfen (Rauch aufgeschnitten) und Nachweise ablegen.

## Umsetzungshinweise

Aus der Sichtung des Codes am 2026-11-21.

- **Wie ein Schnitt heute entsteht:** `view::flow_meshes` (`crates/foamcraft-api/src/view.rs`) tastet jeden Schnitt mit `foamcraft_sim::visual::slice` ab (im Strömungssystem, deshalb die Drehung mit `to_flow`/`to_world` um den Gierwinkel des Laufs) und `project_scene` baut daraus die ganze Szene neu. Für jede Mausbewegung ist das zu langsam.
- **Ziehen mit Vorschau:** Während des Zugs den Schnitt nicht neu abtasten, sondern sein vorhandenes Netz mit der Vorschau-Matrix aus Task_25 entlang der Normalen verschieben (die Farben stimmen dann kurz nicht, das stört beim Ziehen nicht). Beim Loslassen `view.update_slice` rufen; erst dann wird neu abgetastet. Wer es schöner will: nach 150 ms Stillstand eine echte Abtastung nur dieses einen Schnitts nachschieben.
- **Im Gebiet bleiben:** Die Lage entlang der Normalen auf den Kasten des gröbsten Rasters begrenzen (`grids[0].spec.min/max`, im Strömungssystem), sonst verschwindet der Schnitt.
- **Schnitt treffen und auswählen:** Schnitt-Netze haben heute keine Pick-Kennung. In `project_scene` Schlüssel wie `slice:slice-1` vergeben. Der ausgewählte Schnitt ist Sitzungszustand der Oberfläche, kein Manifest-Feld; die Hervorhebung funktioniert über die vorhandene Auswahl-Einfärbung (`viewport_set_selection`).
- **Zug-Richtung:** die Achsenzug-Formel aus Task_25 mit der Normalen als Achse durch den Schnitt-Ursprung.
- **Beschnitt erweitern:** `ViewSettings.clip` ist ein `PlaneSpec`. Ziele ergänzen, zum Beispiel `clip_targets: Vec<String>` mit Standard `["model"]` (so verhalten sich alte Projekte wie bisher). `SCHEMA_VERSION` in `crates/foamcraft-core/src/project.rs` erhöhen und den Kommentar in `migrate` ergänzen; mit `#[serde(default = ...)]` ist nichts umzurechnen. Die Beispielprojekte in `samples/*/foamcraft.json` tragen die alte Nummer und werden beim Öffnen angehoben; nicht von Hand ändern.
- **Isoflächen und Stromlinien beschneiden:** `view::clip` filtert Dreiecke eines Render-Netzes nach dem Schwerpunkt; auf die Isoflächen-Netze genauso anwenden. Stromlinien sind Liniennetze (`particles::streamline_meshes`): dort Segmente statt Dreiecke filtern.
- **Partikel und Rauch beschneiden:** Die Partikel leben auf der GPU (`crates/foamcraft-render/src/particles.wgsl`). Ein `vec4` für die Ebene im `Params`-Block (es gibt dort bereits `_pad`; auf die 16-Byte-Ausrichtung des Blocks achten, die Rust-Struktur in `particles.rs` muss exakt dazu passen). In `write()` Partikel auf der abgewandten Seite nicht zeichnen (Position auf einen entarteten Punkt setzen oder Alpha null), nicht löschen: die Advektion soll weiterlaufen, sonst reißt der Rauch hinter der Ebene ab. Die Ebene ins Strömungssystem drehen wie die Schnitte. Die CPU-Referenz (`foamcraft_sim::particles`) bekommt dieselbe Prüfung, damit `tests/render/particles.rs` GPU und CPU weiter vergleichen kann.
- **Golden-Images:** `particles_smoke.png` und die anderen bleiben unverändert gültig, wenn "keine Ebene" exakt das alte Verhalten ergibt (Ebene mit Normale null = aus). Für den aufgeschnittenen Rauch ein neues Bild.
- **Oberfläche:** `FlowViewSection.tsx` hat je Schnitt einen Schieberegler; er bleibt als genaue Eingabe. `tests/app/flow-view-section.test.tsx` gibt das heutige Verhalten vor (Schnitt hinzufügen, verschieben, Beschnitt-Kästchen "Modell am ersten Schnitt aufschneiden"). Neues daneben stellen, nicht ersetzen.

## Tests

Alle Tests dieses Tasks werden permanent in `tests/` im Hauptverzeichnis des Projekts abgelegt (siehe `AGENTS.md`).

- Rust: Zug am Schnitt ändert nur die Lage entlang der Normalen und bleibt im Gebiet.
- Rust: Beschnittene Partikel (CPU-Referenz) und Isoflächen liegen alle auf der sichtbaren Seite; Golden-Image mit aufgeschnittenem Rauch.
- Rust: Manifest der Vorversion wird migriert und schneidet wie bisher nur das Modell; `view.set_clip` mit Zielen wird gespeichert und nach dem erneuten Öffnen wieder geliefert (die `view.*`-Befehle sind nicht rückgängig machbar, sie speichern selbst).
- Vitest: Auswahl in der Liste, Sichtbarkeit, Ausrichtungsknöpfe und Beschnitt-Ziele rufen die Befehle.

## Akzeptanzkriterien

Der Task gilt erst als fertig, wenn alle Punkte erfüllt und durch Tests oder einen konkreten Befehl belegt sind:

- [ ] Ein Schnitt lässt sich in der Ansicht mit der Maus verschieben
- [ ] Rauch und Partikel lassen sich am Schnitt aufschneiden
- [ ] Schnitte lassen sich in der Liste und in der Ansicht auswählen und ausblenden

## Offene Fragen / Annahmen

- Annahme: Es gibt eine Beschnitt-Ebene (die des ausgewählten Schnitts); das reicht, um in den Rauch zu sehen.

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
