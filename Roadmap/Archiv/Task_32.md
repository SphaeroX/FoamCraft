# Task_32: Mitflug – die Strömung aus Sicht eines Partikels

> ## ⚠️ PFLICHT – BEVOR DU IRGENDETWAS ANDERES TUST
>
> 1. **Lies als Allererstes die Datei `AGENTS.md` im übergeordneten Ordner `Roadmap/`** (von hier: `../AGENTS.md`) und befolge sie strikt.
> 2. **Deine allererste Aktion** ist, diese Datei in `RUN_Task_32.md` umzubenennen, um den Task als laufend zu markieren. Das passiert **noch bevor irgendein Code geschrieben** wird.

## Ziel

Der Nutzer wählt ein Partikel aus und fliegt in der Ich-Perspektive mit: Er läuft auf das Modell zu, wird abgebremst, weicht aus, gleitet dicht über die Oberfläche und gerät in den Nachlauf. Dabei sieht er Tempo, Druck und Wandabstand und bekommt an den entscheidenden Stellen einen Satz Erklärung. So entsteht ein Gefühl dafür, was ein einzelnes Luft- oder Wasserteilchen an seinem Modell erlebt.

## Kontext & Abhängigkeiten

- **Voraussetzung:** Task_31 ist archiviert. Genutzt werden Task_16 (Partikel, Stromlinien), Task_19 (Videoexport), Task_24 (Kamera-Übergänge), Task_27 (Beschnitt für Partikel) und Task_28 (Ansichtsvorlagen, Vorführmodus).
- **Herkunft:** Wunsch des Nutzers (2026-11-21): "einzelne Partikel quasi als First-Person-View sehen … wie man auf das Modell prallt, rum geht … dass man so ein Feeling bekommt für dieses einzelne Partikel. Die Partikel sollen die sein, die wir auch direkt am Objekt entlang sliden und damit man so ein bisschen auswählen kann."
- **Entscheidungen aus dem Interview (2026-11-21):**
  - Auswahl über Vorschläge der App **und** Anklicken in der Ansicht.
  - Fluggefühl umschaltbar: Standard "ruhig und erklärend", Schalter "Achterbahn".
  - Vier Tasks statt einem: Task_32 Mitflug, Task_33 Oberfläche (Wandstromlinien, Wollfäden), Task_34 schwere Partikel mit Aufprall, Task_35 Leuchtspuren und Bänder.
- **Relevantes Vorwissen:**
  - Die Rechnung ist stationär. Die Bahn eines Partikels ist deshalb eine Stromlinie und liegt fest, sobald ein Punkt auf ihr bekannt ist. Der Mitflug braucht also keine GPU-Partikel: Die Bahn wird einmal auf der CPU berechnet, die Kamera fährt sie ab.
  - Ein masseloses Partikel prallt physikalisch nie auf. Es wird vor dem Staupunkt langsamer und weicht aus. Das ist genau das Erlebnis, das dieser Task zeigt ("fast aufgeprallt"). Den echten Aufprall liefern die schweren Partikel in Task_34; der Mitflug wird dort erweitert.
  - Bestehender Code: `foamcraft_sim::particles` (`sample`, `streamline`, `VelocityLevel`, PCG-Zufall), `foamcraft_api::particles` (`seeds`, `setup`), `foamcraft_api::view::load_grids` (Raster im Strömungssystem plus Gierwinkel), `foamcraft_render::camera` (`OrbitCamera`, `CameraTransition`), `foamcraft_api::export` (`Timeline`), Animations-Takt in `app/src-tauri/src/main.rs`.

## Umfang

**Gehört dazu:**

- Bahnberechnung mit Messwerten entlang der Bahn (Tempo, Druck, Wandabstand, Beschleunigung)
- Vorschläge: die App findet je Lauf bis zu fünf lohnende Bahnen und benennt sie
- Auswahl durch Anklicken eines Partikels oder einer Stelle am Modell
- Freie Kamera im Renderer; drei Ansichten: Ich-Perspektive, Verfolger, Übersicht mit Bild-im-Bild
- Zwei Flugarten: "ruhig" und "Achterbahn"
- Anzeige (HUD), Zeitleiste mit Ereignissen, Erklärsätze in Englisch und Deutsch
- Begleitschwarm, damit man die Bewegung spürt
- Befehle `ride.*`, MCP-Werkzeuge, Videoexport des Flugs
- Ein Mitflug als Schritt im Vorführmodus (Task_28)

**Gehört nicht dazu:**

- Partikel mit Masse und echter Aufprall (Task_34)
- Zeitabhängige Felder, in denen Bahnlinie und Stromlinie auseinanderfallen (Task_41)
- Freies Herumfliegen mit Tastatur oder Gamepad, VR-Brillen
- Ton

## Mechanik

Dieser Abschnitt legt fest, **wie** der Mitflug funktioniert. Zahlenwerte sind Startwerte; wer sie nach dem Ausprobieren ändert, schreibt Wert und Grund in die Notizen.

### Bezeichnungen

- `L`: größte Ausdehnung des Modells (Hüllbox der Ergebnis-Oberfläche, `Surface.points`).
- `h`: kleinster Rasterschritt des feinsten Rasters (`Grid.spec.step()`).
- `U_ref`: Anströmgeschwindigkeit des Laufs (Windkanal) bzw. mittlere Einlassgeschwindigkeit (Innenströmung), so wie `results.summary` sie liefert. Steht sie nicht bereit: Median von `U_mag` im gröbsten Raster.
- `τ`: Strömungszeit (echte Zeit des Partikels), `t`: Bildschirmzeit, `s`: Bogenlänge entlang der Bahn.
- Alle Raster liegen im Strömungssystem (Strömung +X); die Welt ist um den Gierwinkel gedreht (`to_world`, `to_flow` wie in `particles.rs` und `view.rs`). Die Bahn wird im Strömungssystem gerechnet und erst für Kamera und Zeichnen in die Welt gedreht.

### 1. Bahn

Neues Modul `foamcraft_sim::ride` (reine Logik, ohne GPU, ohne Projekt).

- **Integration** von einem Punkt aus **vorwärts und rückwärts** (rückwärts mit negativem Zeitschritt), damit der Flug stromauf beginnt und durch den gewählten Punkt führt. Mittelpunktregel wie `streamline`, aber mit **räumlich begrenztem Schritt**: `dτ = 0,25 · h / max(|u|, 0,02 · U_ref)`. So legt jeder Schritt höchstens einen viertel Rasterschritt zurück; die Bahn ist nahe am Modell glatt, ohne im freien Feld Schritte zu verschwenden.
- **Ende einer Richtung:** außerhalb aller Raster, in der Festkörpermaske (`Sample::Blocked`), Bogenlänge über `30 · L`, mehr als 20 000 Schritte, oder Stillstand (`|u| < 0,02 · U_ref` über 200 Schritte in Folge; das ist die Staustromlinie).
- **Neu abtasten** auf gleichmäßige Bogenlänge `ds = 0,25 · h`; je Punkt speichern: Ort, `τ`, Geschwindigkeitsvektor, Druck `p` (trilinear, wie `Grid::sample`), `Cp` = `p / (0,5 · ρ · U_ref²)` relativ zum Bezugsdruck des Laufs, Wandabstand `d`, Beschleunigung `a = |du/dτ|` (zentrale Differenz entlang der Bahn), Q-Wert.
- **Wandabstand:** neues Modul `foamcraft_sim::distance`: ein räumliches Raster über die Dreiecke der Ergebnis-Oberfläche (Zellgröße `2 · h`), das zu einem Punkt den nächsten Dreieckspunkt liefert (Suche in wachsenden Ringen bis `8 · h`, darüber gilt "fern" mit `d = 8 · h`). Punkt-Dreieck-Abstand exakt (nächster Punkt auf dem Dreieck, nicht nur Ebenenabstand). Task_33 und Task_34 nutzen dieses Modul wieder; deshalb sauber und mit eigenen Tests bauen.

### 2. Vorschläge

Die App soll Bahnen vorschlagen, "die am Objekt entlang gleiten". Vorgehen (`ride::suggest`, deterministisch über einen Startwert):

1. **Kandidaten erzeugen** (rund 600):
   - Windkanal: 384 Startpunkte auf einem geschichteten Raster (24 × 16, je Zelle ein PCG-Zufallspunkt) in einem Rechteck `0,5 · L` vor dem Modell, so groß wie die Hüllbox des Modells quer zur Strömung mal 1,2.
   - Innenströmung: 300 Startpunkte auf den Einlässen (dieselbe Logik wie `seeds(...)` für `inlets`).
   - Beide: zusätzlich 216 Punkte nahe der Oberfläche (Logik des Startbereichs `surface`, Abstand `1,5 · h`), von dort rückwärts und vorwärts integriert. Das sind die Bahnen, die sicher am Modell entlangführen.
2. **Merkmale je Bahn:**
   - `d_min`: kleinster Wandabstand.
   - `near`: Bogenlänge mit `d < 3 · h`, geteilt durch `L` (wie lange gleitet das Partikel am Modell).
   - `stall`: `1 − min(|u|) / U_ref` im Bereich `d < 5 · h` (Nähe zum Staupunkt).
   - `turn`: Summe der Richtungsänderungen in Radiant (Umlenkung).
   - `reverse`: Anteil der Strömungszeit mit Geschwindigkeit gegen die Hauptrichtung (Windkanal: `u_x < 0`).
   - `peak`: `max(|u|) / U_ref`.
   - `suction`: kleinster `Cp`.
   - `stay`: Strömungszeit geteilt durch (Abstand Start–Ende / `U_ref`).
3. **Auswahl:** Nur Bahnen mit `d_min < 0,1 · L` und einer Länge über `1 · L` kommen in Frage. Je Kategorie gewinnt die Bahn mit dem höchsten Merkmal; eine Bahn darf nur eine Kategorie belegen (die nächste nimmt dann die zweitbeste); zwei Vorschläge müssen an ihrer engsten Stelle am Modell mindestens `0,05 · L` auseinanderliegen. Fehlt das Merkmal im Lauf (zum Beispiel keine Rückströmung, `reverse < 0,05`), entfällt die Kategorie.

| Kennung | Titel (de) | Windkanal | Innenströmung |
|---|---|---|---|
| `stagnation` | Auf den Staupunkt zu | größtes `stall` | größtes `stall` |
| `hugging` | Dicht an der Oberfläche | größtes `near` | größtes `near` |
| `fast` | Über die schnellste Stelle | größtes `peak` | größtes `peak` (engste Stelle) |
| `wake` | In den Nachlauf | größtes `reverse` | – |
| `turn` | Die stärkste Umlenkung | größtes `turn` | größtes `turn` |
| `long` | Der längste Weg | – | größtes `stay` |

Der Katalog wird je Ergebnis einmal berechnet und in der Sitzung gehalten (Schlüssel: Lauf-Kennung und Startwert). Die Rechenzeit wird gemessen und notiert; liegt sie über einer Sekunde, die Kandidaten in einem Thread rechnen (Muster aus Task_23).

### 3. Auswahl durch Anklicken

- **Partikel anklicken:** Die Partikel leben auf der GPU. Beim Klick liest die App ihren Zustand zurück (`Renderer::read_particles`, 16 Byte je Partikel), dreht ihn in die Welt, projiziert auf den Bildschirm und nimmt das Partikel, das dem Klick am nächsten liegt (höchstens 12 Pixel entfernt; bei mehreren das vordere). Liegt das Modell davor (Tiefe aus `viewport_pick_point`), zählt das Partikel nicht. Während des Auswahlmodus hebt ein Marker das Partikel unter der Maus hervor; dafür genügt ein Rücklesen je 100 ms, nicht je Mausbewegung.
- **Stelle am Modell anklicken:** Startpunkt ist der Trefferpunkt plus `1,5 · h` entlang der Flächennormalen (in das Fluid). Das ergibt die Bahn, die an dieser Stelle über das Modell gleitet.
- In beiden Fällen entsteht ein Weltpunkt, und der Flug startet mit `ride.start {from: [x, y, z]}`. Das Anklicken selbst ist Sache der Oberfläche (wie `viewport_pick`); der Befehl ist für Oberfläche, CLI und MCP derselbe.
- Liegt der Punkt in einem Festkörper oder außerhalb des Gebiets: Fehlercode `ride.no_path` mit Vorschlag "näher an der Strömung klicken oder einen Vorschlag wählen".

### 4. Zeit und Tempo

Die Geschwindigkeit entlang einer Bahn schwankt um Größenordnungen (Staupunkt, Totwasser). Deshalb wird die Bildschirmzeit aus der Bahn berechnet, nicht umgekehrt.

- **Ruhig:** Tempo auf dem Bildschirm `v_s = v_ref · r^0,5 · w(d)`, mit `r = |u| / U_ref`, `v_ref = L / 2,5 s` und `w(d) = 0,4 + 0,6 · smoothstep(0, 4h, d)`. Die Wurzel staucht die Unterschiede (langsame Stellen ziehen sich nicht endlos, schnelle rasen nicht), `w` bremst nahe am Modell, damit man etwas sieht. Untergrenze `v_s ≥ 0,08 · L/s`.
- **Achterbahn:** `v_s = c · |u|` mit einem festen Faktor `c` für die ganze Bahn, also proportional zur echten Strömung. `c` wird so gewählt, dass der Flug `clamp(2,5 s · Länge / L, 10 s, 45 s)` dauert.
- In beiden Fällen: `t(s) = ∫ ds / v_s`, mal Kehrwert des Tempo-Reglers (0,25 bis 3). Der **Zeitfaktor** `dτ/dt = |u| / v_s` wird angezeigt ("Zeitlupe × 0,02"), damit die Darstellung ehrlich bleibt.
- Die Zuordnung `t ↔ s` ist eine vorab berechnete Tabelle (monoton), kein Zustand, der sich je Bild fortschreibt. Damit sind Springen auf der Zeitleiste, 60 Bilder pro Sekunde in der App und 30 im Video exakt dieselbe Fahrt.

### 5. Kamera

Neue `FreeCamera` in `foamcraft-render` (`eye`, `forward`, `up`, `fov_y`, `near`, `far`). Der Renderer nimmt heute überall `&OrbitCamera`. Lösung ohne Bruch: ein Trait (zum Beispiel `ViewCamera` mit `view_proj(aspect)` und `eye()`), das beide Kameras erfüllen; die Renderer-Funktionen nehmen `&dyn ViewCamera`. `&OrbitCamera` passt dann unverändert, bestehende Tests bleiben, wie sie sind.

Die **Kameraspur** wird wie die Zeit-Tabelle vorab berechnet (Abtastung 1/120 s Bildschirmzeit) und beim Abspielen interpoliert:

- **Ort:** `eye = p(s)`. Die Kamera sitzt auf dem Partikel.
- **Blickrichtung:** auf den Bahnpunkt `p(s + Δ)` voraus. Ruhig: `Δ = 0,15 · L` und zusätzlich geglättet (exponentiell, Zeitkonstante 0,35 s). Achterbahn: `Δ = 0,02 · L`, Zeitkonstante 0,05 s. Am Bahnende die letzte Richtung halten.
- **Oben:**
  - Ruhig: Horizont bleibt waagerecht (`up` = Welt-Z, senkrecht zur Blickrichtung gemacht). Zeigt die Blickrichtung fast senkrecht (`|forward · z| > 0,95`), weich auf den mitgeführten Rahmen überblenden, sonst kippt das Bild.
  - Achterbahn: mitgeführter Rahmen ohne Eigendrehung (Paralleltransport entlang der Bahn, "Bishop-Rahmen"), dazu Schräglage in die Kurve: `tan φ = a_quer / a_0` mit der Querbeschleunigung aus der Bahnkrümmung und `v_s`, begrenzt auf 35°. Aufeinanderfolgende `up`-Vektoren dürfen nie umklappen.
- **Öffnungswinkel:** ruhig 75°, Achterbahn 90° plus bis zu 10° mit dem Tempo (`|u| / max |u|` der Bahn).
- **Schnittebenen:** `near = clamp(0,2 · d, 0,02 · h, 0,01 · L)`, `far = 100 · L`. Weil `near` mit dem Wandabstand schrumpft, schneidet die Kamera nie in das Modell. Heute leitet `OrbitCamera::projection` beides aus dem Abstand ab; die freie Kamera setzt sie selbst.
- **Umsehen:** Ziehen mit der Maus dreht den Blick gegenüber der Flugrichtung (seitlich ±150°, hoch und runter ±80°); nach dem Loslassen federt er in 0,5 s zurück.
- **Verfolger:** Kamera hinter dem Partikel: `eye = p − forward · a + up · 0,3 · a` mit `a = 0,25 · L`. Liegt der Punkt in einem Festkörper oder außerhalb (`sample`), `a` halbieren (höchstens viermal). Das Partikel ist dann als heller Punkt mit kurzer Spur sichtbar, die Bahn als Linie.
- **Übersicht:** die gewohnte Orbit-Kamera auf das ganze Modell, Bahn als Linie, Partikel als Marker; der Nutzer kann frei drehen.
- **Bild-im-Bild:** In Ich-Perspektive und Verfolger zeigt ein kleiner Einsatz (ein Viertel der Breite, unten rechts) die Übersicht. Technik: zweiter Zeichendurchgang in einen Ausschnitt, wie beim geteilten Vergleich (`DrawTarget.region`, `keep`); den Hintergrund des Einsatzes zuerst als Rechteck füllen, sonst scheint das Hauptbild durch.
- **Ein- und Ausstieg:** Beim Start fährt die Kamera in 0,8 s von der aktuellen Ansicht auf den Flugbeginn (Übergang im Stil von `CameraTransition`, hier zwischen Orbit- und freier Kamera über Augpunkt und Blickziel). Beim Beenden zurück zur vorherigen Ansicht. Die Ansicht vor dem Flug merken.

### 6. Begleitschwarm

Ein einzelnes Partikel im leeren Raum vermittelt keine Bewegung. Zwei Dinge sorgen für Anhaltspunkte:

- **Die laufende Partikelanimation bleibt an** und wird mit der Strömungszeit des Flugs gekoppelt: Je Bild rücken die GPU-Partikel um dasselbe `dτ` vor wie der Reiter (`step_particles` mit diesem Zeitschritt; große Schritte in Teilschritte von höchstens dem heutigen `setup.dt` zerlegen). Dann ruhen Nachbarn relativ zum Reiter, und man sieht nur die Unterschiede der Strömung.
- **Formation voraus:** ein zweiter, kleiner Partikelsatz (3000 Partikel, Stil Kometen) startet laufend auf einer Scheibe (Radius `0,08 · L`, senkrecht zur Bahn) am Bahnpunkt `0,3 · L` vor dem Reiter und lebt etwa 2 s Bildschirmzeit. Diese Partikel fliegen mit fast gleichem Tempo vorweg; wie sie auseinanderlaufen, zeigt Stauchung und Scherung. Dafür braucht der Renderer einen zweiten Partikelplatz (`GpuScene.particles` ist heute ein einzelnes `Option`) und die Möglichkeit, den Seed-Puffer je Bild neu zu beschreiben (heute ohne `COPY_DST`). Die Startpunkte hängen nur vom Bildindex ab, also bleibt alles wiederholbar.
- **Leitlinie:** die Bahn voraus als dünne Linie, abschaltbar.

Reicht die Zeit nicht für die Formation, ist sie der erste Kandidat zum Streichen: in den Notizen vermerken und dem Nutzer melden, nicht still weglassen.

### 7. Ereignisse und Erklärsätze

Eine reine Funktion über die Bahn (`ride::events`) findet Stellen, an denen etwas passiert, und liefert Kennung, Bogenlänge und Zahlen für den Text. Jede Stelle erscheint in der Zeitleiste als Marker; im ruhigen Flug wird der Satz eingeblendet (2,5 s, höchstens einer zugleich). Mindestdauer 0,3 s Bildschirmzeit, dieselbe Kennung frühestens nach 2 s erneut.

| Kennung | Bedingung | Satz (sinngemäß, de) |
|---|---|---|
| `approach` | `|u|` fällt unter 70 % von `U_ref`, während `Cp` steigt | Vor dem Modell staut sich die Strömung: Du wirst langsamer, der Druck steigt. |
| `stagnation` | `|u|` unter 25 % bei `d < 5 · h` | Fast Stillstand am Staupunkt. Hier ist der Druck am höchsten. |
| `acceleration` | `|u|` über 115 % von `U_ref` | Um die Kante wird die Strömung schneller, der Druck fällt. |
| `suction` | kleinster `Cp` der Bahn, wenn unter −0,3 | Tiefster Druck der Reise: Hier zieht die Strömung am Modell. |
| `near_wall` | `d < 2 · h` über mehr als `0,1 · L` | Du gleitest dicht an der Oberfläche entlang. |
| `reverse` | Geschwindigkeit gegen die Hauptrichtung | Rückströmung: Du bist im Totwasser hinter dem Modell. |
| `vortex` | Q über dem 90. Perzentil der positiven Q-Werte (wie in Task_28) | Wirbel: Die Strömung dreht sich hier um eine Achse. |
| `calm` | nach dem Modell wieder 95 % von `U_ref` | Die Strömung hat sich beruhigt. |

Texte in den Meldungskatalog (`crates/foamcraft-core/data/errors.json`, Codes `ride.event.<kennung>` und `ride.suggestion.<kennung>`, je en und de mit `fix`, wie bei `figure.*`). Zahlen als Platzhalter, die die Oberfläche füllt.

### 8. Anzeige während des Flugs

- **HUD** (HTML über der Ansicht, oben links): Tempo in Anzeigeeinheiten und in Prozent der Anströmung, Druck (`Cp` und Pa), Wandabstand, Beschleunigung in g, Zeitfaktor. Darunter zwei schmale Verlaufslinien (Tempo und Druck über die ganze Bahn) mit Zeiger.
- **Flugleiste** (unten): Abspielen/Pause, Zeitleiste zum Springen mit Ereignis-Markern, Tempo-Regler, Schalter Ruhig/Achterbahn, Ansicht (Ich, Verfolger, Übersicht), Leitlinie an/aus, Beenden. Tasten: Leertaste (Pause), Pfeile links/rechts (5 % springen), `1`/`2`/`3` (Ansicht), `Esc` (beenden).
- **Datenweg:** `ride.start` liefert die Messwerte der ganzen Bahn als Tabelle (höchstens 512 Stützstellen: `t`, `s`, Tempo, `Cp`, `p`, `d`, `a`, Zeitfaktor) und die Ereignisliste. Der Takt in `main.rs` sendet je Bild nur die Bildschirmzeit (Ereignis `viewport://ride`); das HUD interpoliert selbst. Keine großen Nutzlasten je Bild.
- **Rücksicht:** Meldet das System "weniger Bewegung" (`prefers-reduced-motion`), startet der Flug im Verfolger und ruhig, ohne Tempo-Aufweitung des Öffnungswinkels. Beim ersten Einschalten der Achterbahn ein einmaliger Hinweis, dass die Ansicht unruhig ist.

### 9. Befehle und Speicherung

Wie die übrigen `view.*`-Befehle nicht rückgängig machbar (sie speichern selbst).

| Befehl | Zweck |
|---|---|
| `ride.suggest` | Katalog der Vorschläge: Kennung, Titel-Code, Startpunkt (Welt), Dauer, Merkmale. Parameter: `seed` (optional). |
| `ride.start` | Startet einen Flug: `suggestion` **oder** `from: [x, y, z]`; dazu `mode` (`calm`, `coaster`), `view` (`first_person`, `chase`, `overview`), `speed`. Antwort: Bahnübersicht, Messwert-Tabelle, Ereignisse. |
| `ride.telemetry` | Messwerte und Ereignisse des gespeicherten Flugs (für CLI und MCP: ein Agent kann nachlesen, was das Partikel erlebt). |
| `ride.stop` | Beendet den Flug und entfernt ihn aus der Ansicht. |

- **Manifest:** `view.ride: Option<RideSpec>` mit `start` (Weltpunkt), `mode`, `view`, `speed`, `guide`. Nur der Startpunkt wird gespeichert, nicht die Bahn; sie entsteht daraus jedes Mal gleich. `SCHEMA_VERSION` um eins erhöhen (die nächste freie Nummer; Task_27 und Task_30 erhöhen ebenfalls), Kommentar in `migrate` ergänzen, mit `#[serde(default)]` ist nichts umzurechnen.
- **Export:** `export.video` bekommt die Kameravorgabe `ride` (nutzt `view.ride`; ohne `duration_s` so lang wie der Flug), `export.image` und `view.render` einen optionalen Parameter `ride_t` (Bildschirmzeit in Sekunden) für ein Standbild aus Flugsicht. Im Video optional eine Zeile mit Tempo, Druck und Wandabstand über `foamcraft_render::overlay` (prüfen, was die eingebaute Schrift kann; Erklärsätze nur, wenn Umlaute sauber gehen).
- **MCP:** `ride_suggest`, `ride_start`, `ride_telemetry`, `ride_stop` in `CURATED` (`crates/foamcraft-mcp/src/lib.rs`), `view_render` um `ride_t` erweitern, Leitfaden in `texts.rs` und `docs/mcp.md` ergänzen.
- **Nur in der App** (Tauri, kein Befehl der API, weil es das Abspielen im Fenster steuert): `viewport_ride` mit Abspielen, Pause, Springen, Umsehen, Ansicht wechseln.

## Umsetzungsschritte

Reihenfolge reine Logik → Befehl → Renderer → Oberfläche, je Schritt erst der Test.

1. `foamcraft_sim::distance`: Dreiecksraster und Wandabstand (Tests an Kugel und Würfel).
2. `foamcraft_sim::ride`: Bahn vorwärts/rückwärts, Abtastung, Messwerte (Tests an gleichförmigem Feld, Starrkörperwirbel, Umströmung einer Kugel).
3. Zeit-Tabelle für beide Flugarten, Kameraspur, Ereignisse (reine Funktionen, Tests).
4. Vorschläge: Kandidaten, Merkmale, Auswahl (Tests an der Kugel-Umströmung).
5. `FreeCamera` und Kamera-Trait im Renderer; Golden-Image aus Flugsicht.
6. Befehle `ride.*`, Manifest, Katalogtexte, MCP.
7. Export: Kameravorgabe `ride`, `ride_t`.
8. App: Flugzustand in `ViewState`, Takt, Übergänge, Bild-im-Bild, Anklicken.
9. Begleitschwarm (Kopplung der Partikelzeit, Formation, Leitlinie).
10. Oberfläche: Vorschlagskarten, Auswahlmodus, Flugleiste, HUD (Vitest zuerst).
11. Schritt im Vorführmodus, Leistung messen, live prüfen, Nachweise ablegen, `docs/architecture.md` und `docs/mcp.md` ergänzen.

## Umsetzungshinweise

Aus der Sichtung des Codes am 2026-11-21.

- **Stromlinien gibt es schon:** `foamcraft_sim::particles::streamline` integriert mit festem Zeitschritt in eine Richtung. Für den Flug nicht umbauen, sondern daneben die neue Bahnfunktion stellen; `sample` und `VelocityLevel` wiederverwenden. Der Druck steht in den Rastern als Feld `p` (schon in Pa), Q und `U_mag` fügt `view::load_grids` über `add_derived` hinzu.
- **Drehung:** Die Ergebnis-Oberfläche im Cache (`results::load_surface`) liegt bereits in der Welt (`convert` dreht die Punkte), die Raster im Strömungssystem. Für den Wandabstand die Oberfläche einmal ins Strömungssystem drehen, nicht jeden Bahnpunkt in die Welt.
- **Kamera im Renderer:** `Renderer::draw`, `render_gpu`, `render_offscreen_gpu`, `render_split_gpu`, `pick` und `Viewport::render` nehmen `&OrbitCamera`. `Globals` braucht nur `view_proj` und `eye`; der Umbau auf ein Trait ist klein. `tests/render/*.rs` rufen diese Funktionen mit `&OrbitCamera` auf und dürfen nicht geändert werden.
- **Flugzustand in der App:** `ViewState` in `app/src-tauri/src/main.rs` hält Kamera, Übergang und Partikel. Der Takt-Thread (60 Hz) ruft heute `step_transition` und `step_particles`. Der Flug kommt als weiterer Zustand dazu (Bahn, Tabellen, Bildschirmzeit, läuft/pausiert). Achtung: `viewport_sync` baut nach jedem `project.changed` Szene und Partikel neu und ruft `view.fit()`; ein laufender Flug muss das überstehen (Flug aus `view.ride` wiederherstellen, Bildschirmzeit behalten, nicht neu einpassen).
- **Kamerabewegung von Hand** beendet heute einen laufenden Übergang (`viewport_camera`). Im Flug bedeuten Ziehen "Umsehen" und Mausrad "Tempo"; die Orbit-Aktionen wirken nur in der Übersicht.
- **Rücklesen der Partikel** (`read_particles`) blockiert kurz (`poll` mit Warten). Für 50 000 Partikel unkritisch; nicht in jeder Mausbewegung aufrufen.
- **Export:** `Timeline::frame` in `crates/foamcraft-api/src/export.rs` interpoliert Orbit-Schlüsselbilder. Die Kameravorgabe `ride` liefert stattdessen die freie Kamera aus der Kameraspur und koppelt den Partikelschritt an `dτ` des Flugs. `CameraSpec` hat `deny_unknown_fields`: neue Felder als optionale Felder ergänzen, das Beispiel im Register anpassen.
- **Vorführmodus (Task_28):** ist eine Zustandsmaschine in TypeScript mit einer Liste von Schritten. Ein Schritt "Mitflug" ruft `ride.start` mit dem Vorschlag `stagnation` (ersatzweise `hugging`), spielt 12 s und ruft `ride.stop`. In die Erkenntnisse von Task_28 schauen, wie Schritte dort heißen.
- **Oberflächen-Tests** finden Elemente über Rolle und englischen Namen (`tests/app/particles-section.test.tsx` als Muster). Übersetzungen mit `node tools/i18n-merge.mjs`.
- **Neue Fehlercodes** (`ride.no_path`, `ride.none`, `ride.unknown_suggestion`) in `errors.json`, en und de mit Vorschlag; `tests/api/error_catalog.rs` prüft das.
- **Keine neue Abhängigkeit nötig:** `glam` reicht für Rahmen und Projektionen.

## Tests

Alle Tests dieses Tasks werden permanent in `tests/` im Hauptverzeichnis des Projekts abgelegt (siehe `AGENTS.md`). Analytische Felder werden wie in `tests/sim/particles.rs` auf Raster abgetastet.

- **Wandabstand (Rust, `tests/sim/distance.rs`):** Kugel aus Dreiecken: Abstand weicht höchstens um die Sehnenhöhe der Facetten ab; Würfel: Punkte vor Fläche, Kante und Ecke liefern den exakten Abstand; "fern" ab `8 · h`.
- **Bahn (Rust, `tests/sim/ride.rs`):**
  - Gleichförmiges Feld: Bahn ist gerade, Strömungszeit = Länge / Geschwindigkeit (1 %).
  - Starrkörperwirbel: Radius bleibt innerhalb 1 %; ein Umlauf ergibt `turn` = 2π (2 %).
  - Vorwärts- und Rückwärtsteil schließen am Startpunkt lückenlos an; die Bogenlängen-Abtastung ist gleichmäßig.
  - Kein Bahnpunkt liegt in der Maske.
  - Umströmung einer Kugel (Potentialströmung, analytisch): Tempo auf der Staustromlinie folgt `1 − R³/x³` (Toleranz im Test begründen, Rasterauflösung beachten); die Bahn endet mit "Stillstand".
- **Zeit:** `t(s)` ist streng monoton; ruhig unterschreitet nie die Untergrenze; Achterbahn hat einen konstanten Zeitfaktor; Tempo-Regler skaliert die Dauer genau.
- **Kameraspur:** Blickrichtung hat Länge 1, `up` steht senkrecht darauf; auf einer Schraubenlinie klappt `up` nie um (Skalarprodukt aufeinanderfolgender Werte über 0,9); ruhig hält den Horizont waagerecht; Abtastung mit 60 und mit 30 Bildern pro Sekunde liefert an gleichen Zeiten dieselbe Kamera.
- **Ereignisse:** künstliche Bahn mit Stau, Beschleunigung und Rückströmung ergibt die erwartete Liste in der richtigen Reihenfolge; Mindestdauer und Sperrzeit wirken.
- **Vorschläge:** an der Kugel gewinnt für `stagnation` die Bahn, die der Achse am nächsten startet; `hugging` hat den kleinsten Wandabstand über die längste Strecke; gleicher Startwert ergibt denselben Katalog, zwei Vorschläge liegen nie zu dicht.
- **Renderer (`tests/render/`):** `FreeCamera` stimmt mit einer Vergleichsrechnung überein (Blickpunkt landet in der Bildmitte); Golden-Images `ride_first_person.png` und `ride_chase.png` (Testkörper, festes Feld, feste Bildschirmzeit); bestehende Golden-Images bleiben unverändert.
- **Befehle (`tests/api/ride.rs`):** `ride.suggest` und `ride.start` an einem Ergebnis aus Testdaten; `view.ride` wird gespeichert und nach dem erneuten Öffnen geliefert; Manifest der Vorversion wird migriert; Punkt im Festkörper ergibt `ride.no_path`; Video mit Kameravorgabe `ride` zweimal exportiert ist bytegleich.
- **MCP (`tests/mcp/tools.rs`):** neue Werkzeuge sind gelistet und rufen die Befehle.
- **Vitest (`tests/app/ride-*.test.tsx`):** Vorschlagskarte ruft `ride.start`; Flugleiste: Pause, Springen, Tempo, Flugart, Ansicht; `Esc` ruft `ride.stop`; HUD interpoliert die Messwerte zur gesendeten Zeit; Erklärsatz erscheint am Ereignis.
- **Leistung (ignorierter Test, Ergebnis in den Notizen):** Katalog mit 600 Kandidaten; Bildzeit im Flug mit 50 000 Partikeln und Bild-im-Bild.

## Akzeptanzkriterien

Der Task gilt erst als fertig, wenn alle Punkte erfüllt und durch Tests oder einen konkreten Befehl belegt sind:

- [x] Am Windkanal-Beispiel startet der Vorschlag "Auf den Staupunkt zu" einen Flug in Ich-Perspektive, der bis hinter das Modell führt (Nachweis-Bilder, Bildzeit in den Notizen; Ziel 60 Bilder pro Sekunde auf dem Referenzrechner)
- [x] Ein angeklicktes Partikel und eine angeklickte Stelle am Modell starten jeweils einen Flug von dort
- [x] Ruhig und Achterbahn sowie Ich-Perspektive, Verfolger und Übersicht lassen sich während des Flugs umschalten
- [x] Die Anzeige zeigt Tempo, Druck, Wandabstand und Zeitfaktor; Erklärsätze erscheinen in Englisch und Deutsch
- [x] Der Flug funktioniert auch am Rohrverteiler (Innenströmung)
- [x] Ein Video des Flugs lässt sich exportieren; zwei Exporte sind bildgleich
- [x] Über MCP lassen sich Vorschläge holen, ein Flug starten, die Messwerte lesen und ein Bild aus Flugsicht rendern
- [x] Der Vorführmodus enthält einen Mitflug

## Offene Fragen / Annahmen

- Annahme: Die Rechnung ist stationär; die Bahn ist eine Stromlinie. Für zeitabhängige Läufe passt Task_41 den Mitflug an.
- Annahme: Die Startwerte der Mechanik (Tempo-Formel, Blick voraus, Öffnungswinkel, Schwellen der Ereignisse) werden am Windkanal-Beispiel und am Rohrverteiler ausprobiert und bei Bedarf angepasst; Änderungen stehen in den Notizen.
- Offen: Ob die eingebaute Schrift von `overlay` für Erklärsätze im Video reicht. Wenn nicht, bleibt das Video bei den Zahlen.
- Offen: Die Raster haben 64³ Punkte je Stufe. Nahe am Modell ist die Bahn nur so genau wie das feinste Raster. Fällt das im Flug störend auf (Ruckeln an der Wand), dem Nutzer berichten; ein feineres Raster um das Modell wäre eine Änderung an Task_14 und gehört nicht still in diesen Task.

## Notizen zu diesem Task

Taskspezifische Informationen während der Arbeit hier festhalten. Taskübergreifendes gehört in `Roadmap/README.md`.

- **Nachweise (live, eigene App-Instanz, 2026-11-21):** Windkanal-Beispiel "Rounded body": Vorschlag "Auf den Staupunkt zu" (15 s) fliegt vom Anlauf über das Modell bis dahinter; Ich-Perspektive, Verfolger, Übersicht, Ruhig/Achterbahn (mit einmaligem Hinweis) umgeschaltet; Klick auf das Modell (10,5 s) und auf ein Rauchteilchen (8,8 s) starten Flüge. Rohrverteiler: zwei Vorschläge, Flug durchs Bauteil mit dem Satz "Du gleitest dicht an der Oberfläche entlang". Bilder lagen im Scratchpad (`ride2.png`, `chase-a.png`, `coaster-a.png`, `overview-a.png`, `manifold-a.png`, `pick1.png`, `pick2.png`); nicht im Repo (keine Simulationsdaten).
- **Leistung:** Bildzeit im Flug mit Rauch und Bild-im-Bild 5,3 bis 6,4 ms im Debug-Build (Log "ride frame"), also deutlich unter 16,7 ms für 60 Bilder pro Sekunde. Vorschlagskatalog (600 Kandidaten, alle Kerne) am Windkanal-Beispiel 8 s im Debug-Build über die CLI; er wird je Lauf und Startwert in der Sitzung gehalten.
- **Geänderte Startwerte:** keine an den Formeln. Neu gegenüber dem Plan: (1) Bahnen werden für den Flug auf das Stück von 1 L vor bis 1,5 L hinter der Modell-Hüllbox gekürzt (`Path::trimmed`), sonst dauert ein Flug vom Einlass bis zum Auslass 1 bis 5 Minuten. (2) Vorschläge nur aus Bahnen, die hereinkommen und wieder gehen (`Features::through`); Bahnen im Ablösegebiet kreisen sonst 30 L lang und gewannen alle Kategorien. (3) "Auf den Staupunkt zu" muss am Modell vorbeiführen (`passes`), damit der Flug bis hinter das Modell geht. (4) Ein Bahnende, das fern einer Wand stehen bleibt (Öffnung einer Innenströmung, hinter der das Raster keine Fluidpunkte hat), zählt als Verlassen.
- **Zeitfaktor:** Der Plan schreibt `dτ/dt = |u| / v_s`; richtig ist `v_s / |u|` (Strömungszeit je Bildschirmzeit, < 1 = Zeitlupe). Umgesetzt und getestet.

## Abschluss-Checkliste

Reihenfolge und Details stehen in `AGENTS.md`, Abschnitt "Abschluss- und Archivierungs-Workflow".

- [x] Alle Akzeptanzkriterien erfüllt, Tests liegen permanent in `tests/` und laufen grün
- [x] Abschnitt "Erkenntnisse für Folge-Agenten" unten ausgefüllt
- [x] Folge-Tasks und `Roadmap/README.md` geprüft und bei Bedarf angepasst
- [x] Datei ohne `RUN_` nach `Roadmap/Archiv/` verschoben

## Erkenntnisse für Folge-Agenten


Vor der Archivierung vom Agenten auszufüllen. Schreibe so, dass ein Agent, der diesen Task nie gesehen hat, davon bei späteren Tasks profitiert:

- **Umgesetzt (und Abweichungen vom Plan):** `foamcraft_sim::distance` (Wandabstand), `foamcraft_sim::ride` (Bahn, Zeit-Tabelle, Kameraspur, Ereignisse, Merkmale, Vorschläge, Kürzen), `FreeCamera` und Trait `ViewCamera` im Renderer (alle Renderer-Funktionen nehmen `&dyn ViewCamera`, bestehende Tests unverändert), `render_inset_gpu`/`Viewport::render_inset` (Bild-im-Bild, Hintergrund als Rechteck im Clip-Raum), Befehle `ride.suggest/start/telemetry/stop` (`foamcraft_api::ride`), Schema 14 (`view.ride`), Katalogtexte `ride.*`, MCP-Werkzeuge, Export (`camera.preset = "ride"`, `ride_t` in `export.image` und `view.render`), App-Flug (`app/src-tauri/src/ride.rs`, `viewport_ride`, `viewport_ride_point`, Ereignis `viewport://ride`), Oberfläche (`RideSection`, `RideOverlay`, `app/src/ride.ts`), Mitflug als letzter Schritt der Vorführung (12 s, `DemoPlayer.durationOf`). **Gestrichen:** die Formation voraus (zweiter Partikelsatz) – der Begleitschwarm besteht aus den laufenden GPU-Partikeln, die mit der Strömungszeit des Reiters weiterlaufen, plus Leitlinie. **Vereinfacht:** Umsehen läuft über das normale Ziehen (die App deutet Orbit-Ziehen im Flug als Umsehen und lässt 150 ms nach dem letzten Ziehen los); Mausrad als Tempo fehlt; kein Hervorheben des Partikels unter der Maus im Auswahlmodus; der Startpunkt einer Modell-Stelle liegt 2 % des Modellradius über der Fläche (statt 1,5 h, weil die App h nicht kennt). Video ohne Zahlenzeile (Overlay nicht eingebaut).
- **Learnings & Stolpersteine:** (1) Maskierte Rasterpunkte als ruhende Wand (Geschwindigkeit 0) zu behandeln und erst bei mehr als halbem Maskengewicht "blockiert" zu sagen, lässt Bahnen vor einer Wand stehen bleiben statt zu enden; vor der Wand den Schritt halbieren lässt sie entlanggleiten. "Außerhalb" darf dabei nie halbiert werden, sonst kriecht die Bahn an den Gebietsrand und meldet "Stillstand". (2) Im Ablösegebiet einer RANS-Lösung gibt es geschlossene Stromlinien: ohne Filter gewinnen sie jede Kategorie. (3) Bei Innenströmung liegen hinter den Öffnungen keine Fluidpunkte: Bahnen enden dort im Stillstand fern der Wand. (4) Der Bash-Heredoc mit Backticks und ein `cat >` ohne Eingabe hängen; lange Änderungen als Node-Skript per Write-Tool. (5) Oberflächen-Checks ohne Maus: eigene App mit `WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS=--remote-debugging-port=9333` starten und per CDP (`Runtime.evaluate`, `Input.dispatchMouseEvent`) klicken; `window.__TAURI_INTERNALS__.invoke` ruft Tauri-Befehle direkt.
- **Wichtige Entscheidungen und ihre Gründe:** Die Bahn wird nur aus dem Startpunkt berechnet und nie gespeichert: App, Bild und Video zeigen damit exakt dieselbe Fahrt. Zeit-Tabelle und Kameraspur sind vorab berechnet (1/120 s), so sind 60 Bilder in der App und 30 im Video an gleichen Zeiten gleich. Die Flugmeshes (Leitlinie, Bahn, Marker) laufen über die Griff-Ebene (`set_handles`), die je Bild ersetzt werden kann; dadurch sind sie vor dem Modell sichtbar. Die Partikel laufen im Flug mit `flow_dt` (Strömungszeit zwischen zwei Bildern) in Teilschritten höchstens so groß wie der normale Partikelschritt.
- **Auswirkungen auf Folge-Tasks:** Task_33/34 nutzen `WallDistance` (`triangles()`, `closest_on_triangle`); Task_34 erweitert sie um Strecke–Dreieck. Task_34 hängt den Mitflug auf schweren Partikeln an `ride.start` an (`Ride::with_flow`, `Flow` mit maskierten Punkten als Wand). Vorschläge-Kandidaten (`wind_candidates`, `surface_candidates`, `trace_all`) sind wiederverwendbar. Task_37 (Leistung): Vorschlagskatalog im Release messen. Task_41: Mitflug im zeitabhängigen Feld neu denken (Bahnlinie statt Stromlinie).
- **Hinweise zu Tests und Befehlen:** `cargo test -p foamcraft-sim --test distance --test ride`, `cargo test -p foamcraft-render --test ride` (Golden `ride_first_person.png`, `ride_chase.png`, neu erzeugen mit `FOAMCRAFT_UPDATE_GOLDEN=1`), `cargo test -p foamcraft-api --test ride`, `cargo test -p foamcraft-mcp --test mcp_tools`, `npx vitest run tests/app/ride-overlay.test.tsx tests/app/ride-demo.test.tsx`. Live: `ride.suggest`/`ride.start` per CLI `command batch` oder MCP-Brücke.
