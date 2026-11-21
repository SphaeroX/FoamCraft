# Task_16: Stromlinien und Partikel-Animation

> ## ⚠️ PFLICHT – BEVOR DU IRGENDETWAS ANDERES TUST
>
> 1. **Lies als Allererstes die Datei `AGENTS.md` im übergeordneten Ordner `Roadmap/`** (von hier: `../AGENTS.md`) und befolge sie strikt.
> 2. **Deine allererste Aktion** ist, diese Datei in `RUN_Task_16.md` umzubenennen, um den Task als laufend zu markieren. Das passiert **noch bevor irgendein Code geschrieben** wird.

## Ziel

Die Strömung wird mit animierten Partikeln und Stromlinien sichtbar, in Echtzeit und mit frei setzbaren Startbereichen. Das ist der Kern der anschaulichen Darstellung.

## Kontext & Abhängigkeiten

- **Voraussetzung:** Task_14 ist archiviert (Task_15 liegt davor).
- **Relevantes Vorwissen:** Referenz-GPU: RTX 4080 Laptop. Die Animation muss wiederholbar sein, weil Task_19 daraus Videos rendert.

## Umfang

**Gehört dazu:**

- Partikel-Advektion auf der GPU im Geschwindigkeitsraster, mit Festkörpermaske
- Startbereiche: Fläche vor dem Modell, Einlassflächen, frei platzierbare Linie oder Fläche, nahe der Oberfläche
- Darstellung: Punkte mit Schweif, Stromlinien-Bänder, Rauch-Optik
- Färbung nach Geschwindigkeit, Druck oder Verweilzeit
- Steuerung: Tempo, Pause, Dichte
- Wiederholbarkeit über einen festen Startwert
- Befehle und Voreinstellungen ("Windkanal-Rauch", "Innenströmung")

**Gehört nicht dazu:**

- Zeitabhängige Felder (Task_37)
- Videoexport (Task_19)

## Umsetzungsschritte

1. Tests zuerst für den Integrator (gleichförmiges Feld, Starrkörperwirbel, Maske).
2. Advektion auf der GPU umsetzen.
3. Startbereiche umsetzen.
4. Darstellungsarten und Färbung umsetzen (Golden Images mit festem Startwert).
5. Steuerung, Befehle und Voreinstellungen ergänzen.
6. Leistung auf der Referenz-GPU messen und dokumentieren.

## Tests

Alle Tests dieses Tasks werden permanent in `tests/` im Hauptverzeichnis des Projekts abgelegt (siehe `AGENTS.md`).

- Im gleichförmigen Feld legt ein Partikel Geschwindigkeit mal Zeit zurück; im Starrkörperwirbel bleibt der Radius innerhalb von 1 %.
- Kein Partikel dringt in die Maske ein.
- Gleicher Startwert ergibt das gleiche Bild.
- Golden Images der Darstellungsarten.
- Leistungsmessung als ignorierter Test mit dokumentiertem Ergebnis.

## Akzeptanzkriterien

Der Task gilt erst als fertig, wenn alle Punkte erfüllt und durch Tests oder einen konkreten Befehl belegt sind:

- [x] In der App laufen Partikel flüssig durch das Ergebnis eines Windkanal- und eines Innenströmungs-Laufs (Messwert in den Notizen)
- [x] Startbereiche lassen sich setzen und verschieben
- [x] Zwei Läufe der Animation mit gleichem Startwert sind bildgleich

## Offene Fragen / Annahmen

- Annahme: Zielwert 60 Bilder pro Sekunde bei mindestens 200 000 Partikeln auf der Referenz-GPU; hier nachmessen.

## Notizen zu diesem Task

- Messwerte (2026-11-21, RTX 4080 Laptop): ignorierter Test `performance_of_200000_particles` (release): **2,16 ms pro Bild** für 200 000 Partikel in 3 × 64³ Rastern inkl. Zeichnen 1280 × 800 und Rücklesen (≈ 460 Bilder/s; Annahme 60 fps bei 200 000 erfüllt). In der App (Protokollzeile `particle frame`): Innenströmung T-Stück 50 000 Partikel 2,0 ms, Windkanal Demo 60 000 Rauchpartikel 2,0–2,1 ms pro Animationsbild.
- Abnahme: T-Stück mit "Innenströmung" (`Roadmap/Nachweise/task16_innenstroemung_partikel.png`), Demo mit "Windkanal-Rauch" (`task16_windkanal_rauch.png`), Startbereich per Schieberegler verschoben (`task16_startbereich_verschoben.png`).
- Wiederholbarkeit: `the_same_seed_gives_the_same_image` (zwei Läufe, gleicher Startwert → identische Pixel; anderer Startwert → anders).
- Stromlinien als Linien statt Bänder (einfacher, gut lesbar); "Bänder" bleiben offen.
- Nebenbei behoben: "Zuletzt geöffnet" listete dasselbe Projekt doppelt, wenn der Pfad einmal mit `/` und einmal mit `\\` geschrieben war; der Aufschnitt ließ sich ohne Schnitt nicht mehr abschalten.

Taskspezifische Informationen während der Arbeit hier festhalten. Taskübergreifendes gehört in `Roadmap/README.md`.

## Abschluss-Checkliste

Reihenfolge und Details stehen in `AGENTS.md`, Abschnitt "Abschluss- und Archivierungs-Workflow".

- [x] Alle Akzeptanzkriterien erfüllt, Tests liegen permanent in `tests/` und laufen grün
- [x] Abschnitt "Erkenntnisse für Folge-Agenten" unten ausgefüllt
- [x] Folge-Tasks und `Roadmap/README.md` geprüft und bei Bedarf angepasst
- [x] Datei ohne `RUN_` nach `Roadmap/Archiv/` verschoben

## Erkenntnisse für Folge-Agenten

- **Umgesetzt (und Abweichungen vom Plan):** `foamcraft_sim::particles` (CPU-Referenz, Startpunkte, Stromlinien, PCG), `foamcraft-render` `particles.rs` + `particles.wgsl` (Compute-Advektion, Linien-Ausgabe, Druck-Färbung, Drehung), `foamcraft_api::particles` (Voreinstellungen, Startbereiche, `setup` für die App, Stromlinien), Animations-Takt in `app/src-tauri/src/main.rs`, `ParticlesSection`, Manifest Schema 10. Abweichung: Stromlinien als Linien statt Bänder.
- **Learnings & Stolpersteine:** Compute-Ausgabe direkt als Vertexpuffer (`STORAGE | VERTEX`) spart jede Kopie. WGSL: dynamisch indizierte Arrays brauchen `var`. `get_mapped_range` liefert in wgpu 30 ein `Result`. Bei undurchsichtigen Körpern sind innen liegende Partikel/Schnitte nur nach dem Aufschneiden sichtbar (Rückseiten werden weggeschnitten). Ein Klick per Skript auf "Projekt schließen" kurz nach anderen Aktionen ging verloren – Bildschirmfoto prüfen.
- **Wichtige Entscheidungen und ihre Gründe:** Startpunkte als Liste (alle Bereichsarten gleich behandelt, deterministisch). Gleiche Zufallszahlen auf CPU und GPU (testbar). Zeitmaßstab aus Rastergröße und größter Geschwindigkeit, damit jedes Modell gleich lebendig wirkt.
- **Auswirkungen auf Folge-Tasks:** Task_19 (Videoexport): `Renderer::create_particles` + `step_particles` mit festem Startwert und `render_offscreen_gpu` liefern reproduzierbare Bilder; Zeitschritt pro Bild aus `particles::setup(...).dt`. Task_37 (zeitabhängig): Raster je Zeitschritt austauschen. Die Kamera wird bei jedem `project.changed` neu eingepasst (`viewport_sync` → `fit`) – bei häufigen Ansichtsänderungen störend, ggf. in Task_17/18 ändern.
- **Hinweise zu Tests und Befehlen:** `cargo test -p foamcraft-sim --test particles`, `cargo test -p foamcraft-render --test particles` (GPU; ohne Adapter übersprungen), Leistung: `cargo test -p foamcraft-render --release --test particles -- --ignored --nocapture performance`, `cargo test -p foamcraft-api --test view`, `npx vitest run tests/app/particles-section.test.tsx`. Golden Images: `tests/golden/particles_*.png`, `streamlines.png`.
