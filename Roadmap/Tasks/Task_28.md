# Task_28: Strömung verstehen – Ansichtsvorlagen und Vorführmodus

> ## ⚠️ PFLICHT – BEVOR DU IRGENDETWAS ANDERES TUST
>
> 1. **Lies als Allererstes die Datei `AGENTS.md` im übergeordneten Ordner `Roadmap/`** (von hier: `../AGENTS.md`) und befolge sie strikt.
> 2. **Deine allererste Aktion** ist, diese Datei in `RUN_Task_28.md` umzubenennen, um den Task als laufend zu markieren. Das passiert **noch bevor irgendein Code geschrieben** wird.

## Ziel

Nach einem Lauf versteht auch ein Laie, was die Strömung an seinem Modell tut: Fertige Ansichten mit einem Klick (Rauchfäden statt Rauchwolke, Mittelschnitt, Druck am Modell, Wirbel, Nachlauf) und ein Vorführmodus, der diese Ansichten nacheinander mit kurzer Erklärung zeigt.

## Kontext & Abhängigkeiten

- **Voraussetzung:** Task_27 ist archiviert (ziehbare Schnitte, Beschnitt für Rauch).
- **Herkunft:** Rückmeldung des Nutzers (2026-11-21): "Wenn ich auf den Rauch schaue, sehe ich einfach nur die große Rauchwolke … mach mir mal Demo-Modus, dass ich wirklich gut nachvollziehen kann, wie verhält sich mein Objekt … mehr Sachen zum Visualisieren."
- **Relevantes Vorwissen:** Partikel-Vorlagen `wind_tunnel_smoke`/`internal_flow` (`crates/foamcraft-api/src/particles.rs`), Startbereiche (`SeedRegionSpec`), Schnitte und Isoflächen (`view.*`), Kamerafahrten (`foamcraft_render::camera_path`), Kennzahlen mit Erklärtexten (`results.summary`). Die Rauch-Vorlage startet heute in einer großen Fläche vor dem Modell; deshalb entsteht eine Wolke.

## Umfang

**Gehört dazu:**

- Ansichtsvorlagen als Befehl `view.apply_preset` (eine Vorlage ersetzt die Szene; rückgängig machbar, anders als die übrigen `view.*`-Befehle, siehe Umsetzungshinweise):
  - Rauchfäden: wenige dünne Fäden aus einer Reihe von Düsen in der Mittelebene, an die Modellgröße angepasst
  - Rauchvorhang: eine dünne Schicht in der Mittelebene
  - Geschwindigkeit im Mittelschnitt
  - Druck am Modell
  - Wirbel hinter dem Modell (Q-Isofläche mit passendem Wert)
  - Nachlauf: Querschnitt hinter dem Modell
  - für Innenströmung: Weg durch das Bauteil (Stromlinien von den Einlässen), Geschwindigkeit im Längsschnitt
- Vorlagen passen sich dem Lauf an (Windkanal oder Innenströmung, Modellgröße, Anströmrichtung)
- Vorführmodus: spielt die passenden Vorlagen nacheinander mit Kamerafahrt und einem Satz Erklärung je Ansicht ab; Pause, weiter, zurück, beenden
- Erklärtexte in Englisch und Deutsch, mit Zahlen aus dem Lauf, wo sinnvoll (zum Beispiel "Der Widerstand entsteht zu 85 % aus Druck")
- Vorlagen-Galerie im Ergebnisse-Schritt mit Bild-Symbolen

**Gehört nicht dazu:**

- Automatische Bewertung der Form oder Verbesserungsvorschläge
- Neue Felder oder Rechengrößen

## Umsetzungsschritte

1. Tests zuerst: jede Vorlage erzeugt für Windkanal und Innenströmung eine gültige Szene mit Startbereichen innerhalb des Rechengebiets; Rauchfäden bleiben dünn (Rust).
2. Vorlagen und `view.apply_preset` umsetzen, MCP-Werkzeug ergänzen.
3. Startbereich "Düsenreihe" für Partikel (CPU-Referenz und GPU).
4. Vorführmodus: Ablauf, Kamerafahrten, Texte.
5. Galerie und Steuerung in der Oberfläche (Vitest zuerst).
6. Live prüfen, Nachweise ablegen.

## Umsetzungshinweise

Aus der Sichtung des Codes am 2026-11-21.

- **Warum heute eine Wolke entsteht:** Die Vorlage `wind_tunnel_smoke` in `crates/foamcraft-api/src/particles.rs` startet 60 000 Partikel aus `upstream_area`: einer Fläche über 80 % der Breite und Höhe des feinsten Rasters. Die Startpunkte kommen aus `foamcraft_sim::particles::seed_points` (zufällig im Rechteck, `SEEDS = 4096` Stück).
- **Rauchfäden:** ein neuer Startbereich, zum Beispiel `kind: "rake"`: `n` Düsen gleichmäßig auf der Strecke `origin + s·u`, jede mit einem kleinen Streukreis (Radius etwa ein halber Rasterschritt des feinsten Rasters, `Grid.spec.step()`), sonst sind die Fäden mathematisch dünn und flimmern. 9 bis 15 Düsen, senkrecht in der Mittelebene (y = 0 im Strömungssystem), über 1,2 Modellhöhen, eine halbe Modelllänge vor dem Modell. Stil `smoke`, deutlich weniger Partikel als heute (etwa 20 000), Färbung nach Geschwindigkeit.
  - `seeds()` in `particles.rs` verzweigt über `region.kind`; `set_particles` prüft die erlaubten Arten in einer Liste. Beide ergänzen. Die Punkte entstehen auf der CPU und gehen als Liste an die GPU: am Shader ist dafür nichts zu ändern.
  - Rauchvorhang: dieselbe Fläche wie heute, aber `u` nur wenige Millimeter breit (ein Rasterschritt) in der Mittelebene.
  - Anpassung an Modell und Richtung: Die Raster liegen im Strömungssystem (Strömung +X); `view::load_grids` liefert den Gierwinkel des Laufs, `to_world` dreht zurück. Die Modellmaße stehen im Kasten des feinsten Rasters (er umschließt das Modell eng) oder im Netzplan `near_box`.
- **`view.apply_preset`:** Die bestehenden `view.*`-Befehle sind nicht rückgängig machbar (sie speichern selbst über `changed()`). Die Vorlage kann als `undoable: true` registriert werden: dann nur das Manifest ändern, nicht selbst speichern und kein Ereignis senden; `Registry::execute` erledigt beides und Undo stellt die vorige Szene her. Eine Vorlage setzt `view.slices`, `view.isosurfaces`, `view.clip`, `view.particles`, `view.surface_field` in einem Rutsch.
- **Passender Q-Wert:** Ein fester Wert passt nie. Aus dem feinsten Raster ein hohes Perzentil der positiven Q-Werte nehmen (um das 90.), dann zeigt die Isofläche die kräftigen Wirbel ohne Rauschen. `view::volume_fields` bzw. `max_of` in `particles.rs` zeigen, wie man die Raster durchläuft.
- **Mittelschnitt und Nachlauf:** Ebenen im Strömungssystem festlegen (Normale y bzw. x, Nachlauf eine halbe Modelllänge hinter dem Modell) und mit `to_world` drehen; `check_plane` und `check_field` in `view.rs` wiederverwenden.
- **Innenströmung:** vorhandene Vorlage `internal_flow` (Start an den Einlässen) mit Stil `streamlines`; Längsschnitt durch die längste Achse der Hüllbox.
- **Erklärtexte:** in den Meldungskatalog `crates/foamcraft-core/data/errors.json` unter eigenen Codes (`preset.<id>`), jeweils en und de mit `fix` (der Katalogtest verlangt das Feld auch hier, siehe `figure.*`). Zahlen aus dem Lauf über `results.summary` (`drag_pressure_n`, `drag_viscous_n`, `cd`); Platzhalter im Text in der Oberfläche füllen.
- **Vorführmodus:** eine reine Zustandsmaschine in TypeScript (Liste der Schritte, Index, läuft/pausiert) mit `setTimeout`, in Vitest mit `vi.useFakeTimers()` prüfbar. Je Schritt `view.apply_preset` und eine Kamerafahrt. Für die Fahrt die Übergänge aus Task_24 nutzen (benannte Ansicht je Vorlage: Seite für den Mittelschnitt, schräg von hinten für den Nachlauf). Der Vorführmodus selbst sollte am Ende die Szene wiederherstellen, die vorher da war (Zustand vor dem Start merken, `view.state`).
- **MCP:** `view_apply_preset` in `CURATED` eintragen, im Leitfaden (`crates/foamcraft-mcp/src/texts.rs`) erwähnen.
- **Golden-Image:** Partikelbilder sind deterministisch (fester Startwert); Muster in `tests/render/particles.rs`. Für die Düsenreihe ein neues Bild, die alten bleiben.
- **Live prüfen:** `tools/windows/app-print.ps1` nimmt auch die laufenden Partikel auf. Dafür braucht es ein Projekt mit Ergebnis: Beispiel anlegen und rechnen dauert auf dem Referenzrechner etwa acht Minuten (Docker mit 1,9 GiB).

## Tests

Alle Tests dieses Tasks werden permanent in `tests/` im Hauptverzeichnis des Projekts abgelegt (siehe `AGENTS.md`).

- Rust: Vorlagen für beide Szenarien; Düsenreihe liegt vor dem Modell in der Mittelebene; Undo stellt die vorige Szene her.
- Rust: Erklärtexte existieren für jede Vorlage in beiden Sprachen.
- Golden-Image: Rauchfäden am Testkörper.
- Vitest: Galerie ruft `view.apply_preset`; Vorführmodus schaltet weiter, pausiert und endet.

## Akzeptanzkriterien

Der Task gilt erst als fertig, wenn alle Punkte erfüllt und durch Tests oder einen konkreten Befehl belegt sind:

- [ ] Ein Klick zeigt Rauchfäden, an denen man die Umströmung erkennt, statt einer Wolke
- [ ] Jede Vorlage funktioniert am Windkanal-Beispiel, die passenden auch am Rohrverteiler
- [ ] Der Vorführmodus läuft ohne weiteres Zutun durch und erklärt jede Ansicht in einem Satz

## Offene Fragen / Annahmen

- Annahme: Der Vorführmodus ist eine Folge fertiger Ansichten, keine KI-Auswertung.

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
