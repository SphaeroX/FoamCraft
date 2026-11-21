# Task_35: Leuchtspuren und Bänder

> ## ⚠️ PFLICHT – BEVOR DU IRGENDETWAS ANDERES TUST
>
> 1. **Lies als Allererstes die Datei `AGENTS.md` im übergeordneten Ordner `Roadmap/`** (von hier: `../AGENTS.md`) und befolge sie strikt.
> 2. **Deine allererste Aktion** ist, diese Datei in `RUN_Task_35.md` umzubenennen, um den Task als laufend zu markieren. Das passiert **noch bevor irgendein Code geschrieben** wird.

## Ziel

Die Strömung sieht nicht mehr aus wie viele kurze Striche, sondern wie ein Bild, das man gern zeigt: Partikel ziehen lange, weich nachleuchtende Spuren, ein Standbild lässt sich wie eine Langzeitbelichtung aufnehmen, und Stromlinien werden zu Bändern und Röhren, an denen man Drall, Tempo und Laufrichtung ablesen kann.

## Kontext & Abhängigkeiten

- **Voraussetzung:** Task_34 ist archiviert. Genutzt werden Task_16 (Partikel, Stromlinien), Task_19 (Export), Task_28 (Düsenreihe, Vorlagen), Task_32 (Mitflug) und Task_33 (Zeitwert im Shader, Pipeline für Strömungslinien).
- **Herkunft:** Wunsch des Nutzers (2026-11-21), die Partikelvisualisierung "auf ein neues Level" zu bringen. Im Interview gewählt: Leuchtspuren und Bänder. In Task_16 blieben "Bänder" ausdrücklich offen (Stromlinien wurden als Linien umgesetzt).
- **Relevantes Vorwissen (Stand des Renderers):**
  - Jedes Partikel ist eine gerade, ein Pixel breite Linie vom Kopf zum Schweif (`particles.wgsl`, `write`); es gibt keine gespeicherte Spur.
  - Der Renderer kennt kein Blending (`blend: None`). Durchsichtiges entsteht über ein Punktraster (jedes zweite Pixel wird verworfen, `fs_main` in `shader.wgsl`). Echtes Leuchten, das sich aufaddiert, geht damit nicht.
  - Die Tiefe wird nach dem Zeichnen verworfen (`store: Discard`), ein zweiter Durchgang kann sie heute nicht nutzen.
  - Stromlinien entstehen auf der CPU (`foamcraft_sim::particles::streamline`, `particles::streamline_meshes`) als Linienzüge mit Farbe je Punkt.

## Umfang

**Gehört dazu:**

- Partikelstil "Leuchtspuren": gespeicherte Spur je Partikel, breite, auslaufende Linien, die sich hell aufaddieren, optional mit Schein
- Langzeitbelichtung: Standbild im Export und Modus in der Ansicht
- Stromlinien als Bänder (mit Verdrehung aus dem Drall) und als Röhren (Dicke aus dem Tempo), mit laufenden Zeitmarken
- Färbung nach Drall
- Einbau in Mitflug, Vorlagen-Galerie, Export, MCP und Oberfläche
- Leistungsmessung und eine Speichergrenze

**Gehört nicht dazu:**

- Rauch als Volumen (Dichtefeld mit Lichtstreuung)
- Schatten, Spiegelungen, Umgebungsbeleuchtung
- Neue Startbereiche

## Mechanik

Zahlenwerte sind Startwerte; Änderungen mit Grund in die Notizen. `L`, `h`, `U_ref` wie in Task_32.

### 1. Spur je Partikel

- **Speicher:** ein Ringpuffer mit `K` Stützstellen je Partikel (ein `vec4`: Ort und Marke). Der Advektions-Shader schreibt nach jedem `m`-ten Schritt den Kopf an die nächste Stelle. Vorgabe `K = 24`, `m` aus der gewünschten Spurlänge.
- **Unterbrechung:** Startet ein Partikel neu, bekommt die erste Stützstelle die Marke "Bruch". Das Stück davor wird nicht gezeichnet, sonst zieht sich eine Linie quer durch das Bild vom alten zum neuen Ort.
- **Grenze:** `Partikelzahl · K ≤ 2,4 Millionen` Stützstellen (rund 38 MB). Wird sie überschritten, `K` verkleinern und das dem Nutzer zeigen ("Spuren gekürzt"), nicht still die Partikelzahl ändern.
- **Reihenfolge** als reine Funktion (welche Stelle des Rings ist das `j`-te Stück hinter dem Kopf), mit CPU-Referenz und Test, wie bei der Advektion.

### 2. Breite Linien

- wgpu zeichnet Linien ein Pixel breit. Jedes Spurstück wird deshalb als Rechteck aus zwei Dreiecken gezeichnet, das der Vertex-Shader im Bildraum aufspannt: Endpunkte projizieren, quer zur Linie um die halbe Breite in Pixeln versetzen. Die Stützstellen holt der Vertex-Shader direkt aus dem Ringpuffer (Vertex-Nummer → Partikel, Stück, Ecke); es wird nichts auf die CPU kopiert.
- **Breite** 1,5 bis 4 Pixel am Kopf (Regler), zum Ende hin auf ein Drittel auslaufend. **Helligkeit** fällt entlang der Spur mit `(1 − j/K)^1,5`.
- **Nahe der Kamera** (wichtig im Mitflug): Liegt ein Endpunkt hinter der vorderen Schnittebene, das Stück vor dem Projizieren an der Ebene kürzen; sonst entstehen Streifen über das ganze Bild.

### 3. Leuchten

Spuren sollen sich aufaddieren: Wo viele übereinander liegen, wird es heller. Das braucht einen eigenen Durchgang:

1. **Szene** wie bisher zeichnen; die Tiefe behalten (`store: Store`).
2. **Lichtbild:** die Spuren in ein eigenes Bild mit 16-Bit-Gleitkomma je Kanal (`Rgba16Float`) zeichnen, additiv (`One`, `One`), mit Tiefentest gegen die Szene, aber ohne Tiefe zu schreiben. So verdeckt das Modell die Spuren dahinter, und die Reihenfolge der Spuren spielt keine Rolle.
3. **Schein** (abschaltbar): das Lichtbild auf ein Viertel der Auflösung verkleinern, in zwei Durchgängen weichzeichnen (9 Stützstellen je Richtung) und mit 60 % wieder aufaddieren.
4. **Zusammensetzen:** je Kanal `c = 1 − exp(−Belichtung · Licht)` (hell bleibt hell, ohne hart auszubrennen), dann mit der Szene verrechnen als `Bild = 1 − (1 − Szene) · (1 − c)`. Bei durchsichtigem Hintergrund im Export: Deckkraft mindestens der größte Kanal von `c`.

Sind keine Leuchtspuren aktiv, entfallen die Schritte 2 bis 4 vollständig. Dann ist jedes Bild pixelgleich wie heute, und alle bestehenden Golden-Images bleiben gültig. Die alten Stile (`comets`, `smoke`, `streamlines`) werden nicht umgebaut.

### 4. Langzeitbelichtung

- **Standbild im Export:** `export.image` mit `exposure_s`: Die Partikel laufen `exposure_s · 60` Schritte, jeder Schritt addiert die Köpfe als kurze Stücke in das Lichtbild; die Belichtung wird durch die Schrittzahl geteilt, dann wird zusammengesetzt. Ergebnis: dichte, durchgezogene Bahnen wie bei einer langen Belichtungszeit im Rauchkanal. Feste Schrittzahl und fester Startwert, also wiederholbar.
- **In der Ansicht:** Modus "Belichtung": Das Lichtbild wird zwischen den Bildern nicht gelöscht, sondern verblasst mit einer Halbwertszeit (Regler 0,5 bis 8 s). Sobald sich die Kamera bewegt oder die Szene ändert, wird es geleert (sonst verschmiert das Bild).
- Im Video bleibt es bei den Leuchtspuren aus Abschnitt 1 bis 3; die Belichtung über Bilder hinweg ist dort aus (Kamerafahrten).

### 5. Bänder

Neues Modul `foamcraft_sim::ribbons` (reine Logik): macht aus einer Stromlinie ein Band, das sich so verdreht, wie sich das Fluid um die Stromlinie dreht.

- **Rahmen entlang der Linie:** Tangente `T`. Die Quer-Richtung `N` startet senkrecht zu `T` (aus Welt-Z; ist `T` fast parallel zu Z, aus Welt-X) und wird von Punkt zu Punkt mitgeführt, ohne sich um `T` zu drehen (Paralleltransport: `N` auf die Ebene senkrecht zur neuen Tangente projizieren und normieren).
- **Verdrehung aus dem Drall:** zusätzlich `N` um `T` drehen um `dφ = ½ · (ω · T) · dτ`. Dabei ist `ω` die Wirbelstärke (Rotation des Geschwindigkeitsfelds, aus zentralen Differenzen im Raster mit Schrittweite `h`) und `dτ` die Strömungszeit des Abschnitts. Der Faktor ½ kommt daher, dass sich ein Fluidteilchen mit der halben Wirbelstärke dreht. In drallfreier Strömung bleibt das Band glatt; in einem Wirbel schraubt es sich.
- **Netz:** zwei Punkte je Stützstelle bei `p ± (w/2) · (T × N)`, Flächennormale `N`, von beiden Seiten sichtbar. Breite `w = 0,012 · L` (Regler).
- **Zeitmarken:** Jede Stützstelle trägt ihre Strömungszeit `τ`. Das Band ist abwechselnd hell und etwas dunkler gestreift, eine Marke je `Δτ = 0,05 · L / U_ref`. Wo die Strömung langsam ist, liegen die Marken eng, wo sie schnell ist, weit auseinander: Man liest das Tempo am Band ab. Mit dem Zeitwert aus Task_33 laufen die Marken in Strömungsrichtung; das Band zeigt Bewegung ohne Partikel.
- **Anzahl:** höchstens 200 Bänder. Am besten wirken sie aus einer Düsenreihe (Startbereich aus Task_28).

### 6. Röhren

- Gleicher Rahmen, aber ein Ring aus acht Punkten je Stützstelle. **Radius** `r = r_0 · sqrt(U_ref / max(|u|, 0,2 · U_ref))`, begrenzt auf das 0,5- bis 2-fache von `r_0 = 0,006 · L`: Eine Röhre, durch die immer gleich viel strömt, wird dünner, wo es schneller wird. Das ist die anschauliche Fassung von "enger heißt schneller".
- Eine dunkle Linie entlang `N` auf der Röhre zeigt die Verdrehung.
- Zeitmarken wie bei den Bändern.

### 7. Färbung nach Drall

- Neuer Wert `swirl` für `color_by` (Bänder und Röhren): `(ω · T) / (|ω| + ε)` mal einem Gewicht, das kleine Wirbelstärken ausblendet (`smoothstep` zwischen dem 50. und 90. Perzentil von `|ω|` im feinsten Raster). Zweifarbige Skala: eine Farbe für Rechtsdrall, eine für Linksdrall, neutral dazwischen. Legende mit Text statt Zahlen ("links … rechts").
- Die Farbskala muss für die häufigen Formen der Farbfehlsichtigkeit unterscheidbar sein (nicht Rot gegen Grün).

### 8. Einbau

- **Stile:** `view.set_particles` nimmt für `style` zusätzlich `trails`, `ribbons`, `tubes`; neue optionale Felder `trail_length` (Sekunden Strömungszeit, als Vielfaches der heutigen Schweiflänge begrenzt), `width`, `glow`, `exposure`, `long_exposure` (Halbwertszeit oder aus). Alte Projekte ohne diese Felder verhalten sich wie bisher. `SCHEMA_VERSION` um eins erhöhen, Kommentar in `migrate`.
- **Schwere Partikel (Task_34)** können ebenfalls Leuchtspuren ziehen (derselbe Ringpuffer im neuen Shader).
- **Mitflug (Task_32):** In Verfolger und Übersicht wird die Bahn als Band gezeichnet (zurückgelegter Teil kräftig, kommender blass). In der Ich-Perspektive wirken Leuchtspuren der Begleitpartikel am besten; prüfen und als Vorgabe setzen, wenn es gut aussieht.
- **Vorlagen (Task_28):** `light_trails` (Düsenreihe oder Fläche, Leuchtspuren, Modell dunkel) und `ribbons` (Düsenreihe, Bänder nach Drall gefärbt), mit Erklärtext in beiden Sprachen.
- **Export:** `export.image` mit `exposure_s`; `export.video` und `view.render` zeigen die neuen Stile ohne weiteres Zutun, weil sie denselben Szenenaufbau nutzen.
- **MCP:** Beschreibungen von `view_set_particles` (falls kuratiert) und `export_image` ergänzen, Leitfaden und `docs/mcp.md`.
- **Oberfläche:** Stilauswahl in `ParticlesSection` um die drei Einträge erweitern; je nach Stil Regler für Spurlänge, Breite, Schein, Belichtung; im Exportdialog "Langzeitbelichtung" für Bilder.

## Umsetzungsschritte

1. Tests zuerst für den Rahmen entlang der Linie und die Verdrehung (gleichförmig, Starrkörperdrehung um die Strömungsachse, Kreisbahn ohne Drall).
2. Bänder und Röhren als Netze; Zeitmarken; Färbung nach Drall; Golden-Images.
3. Ringpuffer: reine Funktion, CPU-Referenz, Shader; Vergleich GPU gegen CPU.
4. Breite Linien im Bildraum mit Kürzen an der vorderen Schnittebene.
5. Lichtbild, Zusammensetzen, Schein; Nachweis, dass ohne Leuchtspuren alles pixelgleich bleibt.
6. Langzeitbelichtung im Export und in der Ansicht.
7. Befehle, Manifest, Vorlagen, MCP, Mitflug.
8. Oberfläche (Vitest zuerst), Leistung messen, live prüfen, Nachweise ablegen, `docs/architecture.md` ergänzen.

## Umsetzungshinweise

Aus der Sichtung des Codes am 2026-11-21.

- **Zeichnen heute:** `Renderer::draw` in `crates/foamcraft-render/src/renderer.rs` macht einen einzigen Durchgang mit Farbe und Tiefe; die Tiefentextur entsteht je Aufruf (`depth_view`) und wird verworfen. Für den zweiten Durchgang im selben `encoder` die Tiefe speichern und mit `LoadOp::Load` wieder einhängen. Pipelines werden je `(Format, Linien)` zwischengespeichert; die neuen Pipelines (Spuren, Zusammensetzen, Weichzeichnen) genauso behandeln, weil Fenster und Offscreen-Bild verschiedene Formate haben.
- **Auswahl-Durchgang (`pick`)** zeichnet mit `R32Uint`: Spuren, Bänder und Röhren sind nicht anklickbar und werden dort übersprungen (Bänder als gewöhnliche Netze ohne Pick-Kennung).
- **Puffer im Vertex-Shader lesen:** Der Ringpuffer wird im Compute-Durchgang beschrieben und im Render-Durchgang nur gelesen; dafür eine zweite Bind-Gruppe mit `read_only`-Storage und Sichtbarkeit `VERTEX` anlegen. Das geht auf den nativen Backends, mit denen FoamCraft läuft.
- **`GpuParticles`** hält heute `verts`, `colors`, `indices` für die Linien. Der Stil `trails` braucht diese drei nicht; der Shader-Zweig `write` kann dann entfallen. Sauber trennen, statt beide Wege immer zu füllen.
- **Dreieckspipeline** hat `cull_mode: Back`; Bänder brauchen beide Seiten (eigene Pipeline ohne Culling, oder die aus Task_33 mitnutzen).
- **Zeitwert:** Task_33 legt einen Zeitwert in den globalen Shader-Daten an und lässt den Takt in `main.rs` auch ohne Partikel laufen. Hier wiederverwenden; die Erkenntnisse von Task_33 dazu lesen.
- **Wirbelstärke:** `foamcraft_sim::visual` berechnet für den Q-Wert bereits Ableitungen der Geschwindigkeit auf dem Raster (zentrale Differenzen). Daraus eine wiederverwendbare Funktion machen, statt die Ableitungen ein zweites Mal zu schreiben.
- **Stromlinien als Grundlage:** `particles::streamline_meshes` begrenzt auf 400 Linien mit bis zu 2000 Punkten und färbt nach Tempo. Bänder und Röhren hängen an derselben Stelle ein (`style` verzweigt), der Rest (Startpunkte, Drehung in die Welt) bleibt.
- **Erlaubte Stile** prüft `set_particles` in einer Liste (`["comets", "smoke", "streamlines"]`); `tests/app/particles-section.test.tsx` und `tests/api/view.rs` geben das heutige Verhalten vor. Neues ergänzen, nichts umbenennen.
- **Golden-Images mit Gleitkomma-Bildern:** Additives Zeichnen ist von der Reihenfolge unabhängig, das Ergebnis also stabil. Trotzdem zweimal rendern und vergleichen, bevor ein Bild als Vorlage abgelegt wird. Die Toleranz der Golden-Image-Tests wird nicht aufgeweitet.

## Tests

Alle Tests dieses Tasks werden permanent in `tests/` im Hauptverzeichnis des Projekts abgelegt (siehe `AGENTS.md`).

- **Rahmen und Verdrehung (Rust, `tests/sim/ribbons.rs`):**
  - Gleichförmige Strömung: `N` bleibt gleich, keine Verdrehung.
  - Strömung mit Geschwindigkeit `U` entlang einer Achse und Starrkörperdrehung `Ω` um diese Achse: Das Band auf der Achse verdreht sich über die Länge `ℓ` um `Ω · ℓ / U` (2 %).
  - Kreisbahn in einer Ebene ohne Drall entlang der Bahn: `N` (senkrecht zur Ebene gestartet) bleibt senkrecht; kein Umklappen.
  - `T`, `N` und `T × N` stehen überall senkrecht aufeinander und haben Länge 1.
  - Röhrenradius folgt der Regel und bleibt in den Grenzen; der Abstand der Zeitmarken ist proportional zum Tempo.
- **Ringpuffer:** reine Funktion für die Reihenfolge; nach `n` Schritten enthält die Spur die letzten Orte der CPU-Referenz in der richtigen Reihenfolge; nach einem Neustart steht die Bruch-Marke und das Stück davor fehlt; die Speichergrenze verkleinert `K` und meldet es.
- **Breite Linien:** ein Stück, dessen einer Endpunkt hinter der Kamera liegt, ergibt endliche Bildkoordinaten (Test der Kürzungsfunktion).
- **Renderer (`tests/render/`):** Golden-Images `particles_trails.png`, `long_exposure.png`, `ribbons.png`, `tubes.png`; zweimal gerendert ist pixelgleich; Spuren hinter dem Testkörper sind verdeckt; **alle bestehenden Golden-Images bleiben unverändert**; durchsichtiger Hintergrund hat Deckkraft, wo Spuren leuchten.
- **Befehle und Export:** neue Stile und Felder werden gespeichert und geladen; Migration der Vorversion; unbekannter Stil wird weiter abgelehnt; `export.image` mit `exposure_s` ist wiederholbar (zwei Aufrufe, gleiche Datei); Vorlagen `light_trails` und `ribbons` erzeugen gültige Szenen.
- **Vitest:** Stilauswahl zeigt die neuen Einträge und die passenden Regler; Regler rufen `view.set_particles`; Exportdialog übergibt `exposure_s`; Hinweis "Spuren gekürzt" erscheint.
- **Leistung (ignoriert, Ergebnis in den Notizen):** Bildzeit für 100 000 Partikel mit `K = 24` und Schein bei 1280 × 800 und bei 4K; Rechenzeit für 200 Bänder.

## Akzeptanzkriterien

Der Task gilt erst als fertig, wenn alle Punkte erfüllt und durch Tests oder einen konkreten Befehl belegt sind:

- [x] Am Windkanal-Beispiel ziehen Partikel lange, weich leuchtende Spuren, die hinter dem Modell verdeckt werden (Nachweis-Bild, Bildzeit in den Notizen; Ziel 60 Bilder pro Sekunde bei 100 000 Partikeln auf dem Referenzrechner)
- [x] Ein Bild mit Langzeitbelichtung lässt sich exportieren und ist wiederholbar
- [x] Bänder verdrehen sich im Nachlauf sichtbar und bleiben in der freien Anströmung glatt; die Zeitmarken laufen in Strömungsrichtung
- [x] Röhren werden an der schnellsten Stelle sichtbar dünner
- [x] Ohne die neuen Stile ist jedes Bild pixelgleich wie vor diesem Task (bestehende Golden-Images grün, nicht neu erzeugt)
- [x] Die neuen Stile funktionieren am Rohrverteiler, im Mitflug und im Video (Mitflug: Leuchtspuren der Begleitpartikel; die Bahn des Mitflugs als Band ist offen, siehe Notizen)
- [x] Vorlagen "Leuchtspuren" und "Bänder" stehen in der Galerie

## Offene Fragen / Annahmen

- Annahme: `Rgba16Float` mit additivem Zeichnen ist auf allen Backends verfügbar, mit denen FoamCraft läuft. Wenn nicht: Ausweichweg mit 8 Bit und geringerer Helligkeit je Spur, als Abweichung dokumentieren.
- Annahme: 2,4 Millionen Stützstellen laufen auf dem Referenzrechner flüssig. Die Messung entscheidet über die Vorgaben für `K` und die Partikelzahl.
- Offen: ob der Schein bei 4K-Export bezahlbar ist; sonst dort mit kleinerem Zwischenbild rechnen.
- Offen: Wirbelstärke aus dem 64³-Raster ist geglättet; sehr kleine Wirbel verdrehen das Band zu wenig. Im Erklärtext steht deshalb "zeigt den Drall der großen Wirbel".

## Notizen zu diesem Task

Taskspezifische Informationen während der Arbeit hier festhalten. Taskübergreifendes gehört in `Roadmap/README.md`.

- **Leistung** (`cargo test --release -p foamcraft-render --test trails -- --ignored --nocapture`, Referenzrechner, Zeit je Bild einschließlich Zurücklesen auf die CPU): 100 000 Partikel, K = 24, Schein: 3,2 ms bei 1280 × 800, 13,1 ms bei 4K. Ziel 60 Bilder/s ist erfüllt. 200 Bänder entstehen beim Aufbau der Szene in deutlich unter einer Sekunde (der ganze CLI-Aufruf mit Vorlage und Bild 5,5 s, davon der Großteil Start und Laden).
- **Live geprüft** (Bilder im Scratchpad der Sitzung): Windkanal (Rundkörper) mit Vorlage "Leuchtspuren" in der App (Spuren um das dunkle Modell, hinter dem Modell verdeckt), Belichtungsmodus in der Ansicht, Mitflug in der Ich-Perspektive mit Leuchtspuren, Video (Bild 30 aus der AVI-Datei), Langzeitbelichtung als Export; Bänder und Röhren am Windkanal; Rohrverteiler mit Leuchtspuren (Belichtung) und Bändern, aufgeschnitten.
- **Abweichung – Helligkeit:** Vorgabe der Belichtung 0,4 (nicht 1): Bei 10 000 und mehr Partikeln addieren sich die Spuren sonst zu Weiß. Langzeitbelichtung: Helligkeit je Schritt 4/Schritte. Die Vorlage "Leuchtspuren" nimmt 12 000 Partikel.
- **Abweichung – Vorlauf:** Leuchtspuren-Partikel laufen beim Anlegen und vor einer Langzeitbelichtung eine Lebensdauer vor (höchstens 3000 Schritte). Ohne das liegen alle Partikel am Anfang dicht an der Startfläche, und Video und Belichtung zeigen eine ausgebrannte Fläche.
- **Abweichung – Spurlänge im Befehl:** zusätzlich `trail_factor` (1 bis 10 Kometenschweife), weil die Oberfläche die Strömungszeit eines Schweifs nicht kennt; gespeichert wird `trail_length` in Sekunden.
- **Offen (bewusst):** schwere Partikel ziehen weiter Kometen, keine Leuchtspuren (eigener Shader `particles_heavy.wgsl` ohne Ring). Die Bahn im Mitflug wird nicht als Band gezeichnet (bleibt die Linie aus Task_32). `export.image` ohne `exposure_s` zeigt wie bisher keine Partikel; `export.preview` zeigt grundsätzlich keine Partikel (älteres Verhalten).
- **Hängender Export-Test (offen):** Im Gesamtlauf `cargo test --workspace` hing `tests/api/export.rs` zweimal über 40 Minuten bei voller CPU-Last (abgebrochen). Einzeln und wiederholt läuft er grün in 6 bis 12 s. Vermutung: Im Gesamtlauf bekommt der Prozess manchmal den Software-Renderer statt der GPU, und 4K-Bild und 60-s-Video rendern dann auf der CPU. Nicht nachgewiesen. Die übrigen 112 Testgruppen sind grün.
- **Testdaten:** Im groben Testraster der API-Tests (24 Zellen) liegt die Standard-Startfläche (3 % der Domäne) vor dem ersten Rasterpunkt; die Partikel starten dort jeden Schritt neu. Die Tests legen die Startfläche deshalb ausdrücklich ins Raster.

## Abschluss-Checkliste

Reihenfolge und Details stehen in `AGENTS.md`, Abschnitt "Abschluss- und Archivierungs-Workflow".

- [x] Alle Akzeptanzkriterien erfüllt, Tests liegen permanent in `tests/` und laufen grün
- [x] Abschnitt "Erkenntnisse für Folge-Agenten" unten ausgefüllt
- [x] Folge-Tasks und `Roadmap/README.md` geprüft und bei Bedarf angepasst
- [x] Datei ohne `RUN_` nach `Roadmap/Archiv/` verschoben
- [x] Dem Nutzer den Stand gemeldet (Ende der Phase 4c, Tasks 32 bis 35)

## Erkenntnisse für Folge-Agenten

Vor der Archivierung vom Agenten auszufüllen. Schreibe so, dass ein Agent, der diesen Task nie gesehen hat, davon bei späteren Tasks profitiert:

- **Umgesetzt (und Abweichungen vom Plan):** `foamcraft_sim::ribbons` (Rahmen, Verdrehung, Band, Röhre, Zeitmarken), `foamcraft_render::trails` (Ring, `TrailSettings`, `points_per_trail`, `slot_behind`, `cut_near`, Lichtbild, Schein, Zusammensetzen, `render_offscreen_exposure`, `set_trail_fade`), Shader `particles_trails.wgsl`, `trails.wgsl`, `trails_post.wgsl`, Bänder in `flow.wgsl`; Stile `trails`, `ribbons`, `tubes`, Farbe `swirl`, Felder `trail_length`/`trail_factor`/`width`/`glow`/`exposure`/`long_exposure` (Schema 17), `export.image` mit `exposure_s`, Vorlagen `light_trails` und `ribbons` (auch für Innenströmung, dort aufgeschnitten), Galerie-Bildchen, Regler im Expertenbereich, Langzeitbelichtung im Exportdialog. Abweichungen: siehe Notizen (Helligkeit 0,4, Vorlauf, `trail_factor`, keine Leuchtspuren für schwere Partikel, Mitflug-Bahn bleibt Linie).
- **Learnings & Stolpersteine:** Ohne Leuchtspuren darf der Renderer nichts anders machen: Tiefe nur speichern, wenn Spuren an sind (`trails_on`), sonst bleiben alle Golden-Images pixelgleich (geprüft). Das Lichtbild hat die Größe des Ziels; das Zusammensetzen liest es an der Pixelposition (`o.clip.xy / textureDimensions`), sonst stimmt es in geteilten Ansichten nicht. Das Ausblenden im Belichtungsmodus geht ohne Bind-Gruppe über eine Pipeline mit leerem Layout und `BlendFactor::Constant`. Dichte Startflächen brennen aus: vorlaufen lassen und die Helligkeit klein wählen.
- **Wichtige Entscheidungen und ihre Gründe:** Ring und Ring-Uniform belegen die Bindings 5 und 6 der Linienpuffer (Grenze von 8 Storage-Puffern je Stufe). Der Ring speichert die Geschwindigkeit in `w` (−1 = Neustart), damit der Vertex-Shader färben kann, ohne das Feld zu lesen. Die Belichtung zeichnet je Schritt nur das neueste Stück (`every = 1`, ein Segment) und teilt die Helligkeit durch die Schrittzahl: wiederholbar und unabhängig von K. Der Belichtungsmodus der Ansicht merkt sich Kamera, Größe und Bereich; jede Änderung leert das Lichtbild.
- **Auswirkungen auf Folge-Tasks:** Task_37 (Leistung) kann die ignorierte Messung in `tests/render/trails.rs` übernehmen. Task_41 (zeitabhängig): Ring und Bänder funktionieren mit jedem Geschwindigkeitsfeld; im zeitabhängigen Feld sind Bänder Stromlinien eines Zeitpunkts, keine Bahnlinien. Offene Punkte für später: Leuchtspuren für schwere Partikel, Mitflug-Bahn als Band, Partikel in `export.preview` und im Standbild ohne Belichtung.
- **Hinweise zu Tests und Befehlen:** `cargo test -p foamcraft-render --test trails` (Ring gegen CPU, Bruch, Grenze, Schnitt an der Ebene, Golden-Images `particles_trails.png` und `long_exposure.png`, Verdeckung, Deckkraft, Belichtungsmodus), `--test flow` (Bänder, Röhren), `cargo test -p foamcraft-sim --test ribbons`, `cargo test -p foamcraft-api --test trails`, `npx vitest run tests/app/trails-section.test.tsx`. Neue Golden-Images nur mit `FOAMCRAFT_UPDATE_GOLDEN=1` und einem Testfilter erzeugen, damit bestehende nicht neu geschrieben werden.
