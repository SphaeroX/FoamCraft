# Task_34: Schwere Partikel mit Aufprall und Trefferkarte

> ## ⚠️ PFLICHT – BEVOR DU IRGENDETWAS ANDERES TUST
>
> 1. **Lies als Allererstes die Datei `AGENTS.md` im übergeordneten Ordner `Roadmap/`** (von hier: `../AGENTS.md`) und befolge sie strikt.
> 2. **Deine allererste Aktion** ist, diese Datei in `RUN_Task_34.md` umzubenennen, um den Task als laufend zu markieren. Das passiert **noch bevor irgendein Code geschrieben** wird.

## Ziel

Neben den masselosen Partikeln gibt es Partikel mit Masse: Staub, Sprühnebel, Regentropfen, Sand. Sie folgen der Strömung nicht perfekt, treffen das Modell wirklich und bleiben dort haften oder prallen ab. Eine Trefferkarte am Modell zeigt, wo sich etwas absetzt, und im Mitflug erlebt man den Aufprall aus Sicht des Partikels.

## Kontext & Abhängigkeiten

- **Voraussetzung:** Task_33 ist archiviert. Genutzt werden Task_16 (GPU-Partikel mit CPU-Referenz), Task_32 (Mitflug, `foamcraft_sim::distance`) und die Oberflächenfelder aus Task_14.
- **Herkunft:** Wunsch des Nutzers (2026-11-21): "wie man dann quasi auf das Modell prallt". Im Interview gewählt: schwere Partikel mit Aufprall und Trefferkarte.
- **Relevantes Vorwissen:**
  - Die bisherigen Partikel haben keine Masse: Ihre Geschwindigkeit **ist** die Strömungsgeschwindigkeit. Sie können eine Wand nie treffen.
  - Ein Partikel mit Masse wird von der Strömung nur mitgezogen (Widerstand) und braucht Zeit, um ihr zu folgen. Wie gut es folgt, sagt die Stokes-Zahl. Die Partikel verändern die Strömung nicht (einseitige Kopplung); das gilt, solange sie wenig Masse je Volumen tragen.
  - Medium des Laufs: Dichte und kinematische Zähigkeit stehen im Szenario (`medium.density`, `medium.kinematic_viscosity`).
  - Die Festkörpermaske der Raster (64³ je Stufe) ist für einen Aufprallort zu grob. Der Ort kommt aus den Dreiecken der Ergebnis-Oberfläche.

## Umfang

**Gehört dazu:**

- Bewegungsgleichung mit Widerstand und Schwerkraft, stabiler Integrator, CPU-Referenz und GPU
- Stoffvorlagen und freie Eingabe (Durchmesser, Dichte); Anzeige der Stokes-Zahl mit Erklärung
- Aufprall: Haften oder Abprallen
- Trefferkarte am Modell (Auftreffrate, Auftreffgeschwindigkeit) und Kennzahl "Anteil, der trifft"
- Animation der schweren Partikel in der Ansicht
- Mitflug auf einem schweren Partikel mit Aufprall
- Befehle, MCP-Werkzeuge, Vorlage in der Galerie, Bedienung in der Oberfläche
- Prüfung gegen analytische Werte

**Gehört nicht dazu:**

- Rückwirkung der Partikel auf die Strömung, Stöße der Partikel untereinander
- Leichte Partikel und Blasen (Dichte kleiner oder gleich der des Mediums)
- Verdunsten, Zerplatzen, Wasserfilme, Vereisung, Abtrag in Zahlen
- Turbulente Streuung der Partikel (siehe Offene Fragen)

## Mechanik

Zahlenwerte sind Startwerte; Änderungen mit Grund in die Notizen. `L`, `h`, `U_ref` wie in Task_32.

### 1. Bewegungsgleichung

Für ein kugelförmiges Partikel (Durchmesser `d`, Dichte `ρ_p`) im Medium (Dichte `ρ_f`, kinematische Zähigkeit `ν`), Ort `x`, Geschwindigkeit `v`, Strömungsgeschwindigkeit `u(x)`:

```
dx/dt = v
dv/dt = (u − v) / τ_e + g'
τ_p   = ρ_p · d² / (18 · ρ_f · ν)            Ansprechzeit bei schleichender Umströmung
Re_p  = |u − v| · d / ν
f     = 1 + 0,15 · Re_p^0,687                 für Re_p ≤ 1000 (Schiller und Naumann)
f     = 0,44 · Re_p / 24                      für Re_p > 1000
τ_e   = τ_p / f
g'    = g · (1 − ρ_f / ρ_p)                   Schwerkraft minus Auftrieb, g zeigt nach −Z
```

- **Stokes-Zahl** für die Anzeige: `St = τ_p · U_ref / L`.
- **Vernachlässigt** (in `docs/architecture.md` festhalten): Auftriebskräfte quer zur Strömung, mitbewegte Masse, Vorgeschichte der Umströmung, Verformung großer Tropfen. Das ist zulässig, solange `ρ_p` viel größer als `ρ_f` ist. Regel: `ρ_p ≤ ρ_f` wird abgelehnt (Fehlercode `particles.too_light`); bei `ρ_p < 5 · ρ_f` (zum Beispiel Sand in Wasser) zeigt die Oberfläche den Hinweis "grobe Näherung".

### 2. Integrator

Die Gleichung ist steif: Für feinen Staub ist `τ_e` viel kleiner als der Zeitschritt, ein einfaches Verfahren würde explodieren. Deshalb die exakte Lösung für örtlich konstante Strömung je Schritt:

```
v_inf = u + τ_e · g'                                   Geschwindigkeit, auf die das Partikel zustrebt
E     = exp(−dt / τ_e)
v_neu = v_inf + (v − v_inf) · E
x_neu = x + v_inf · dt + (v − v_inf) · τ_e · (1 − E)
```

- Zwei Durchgänge je Schritt: erst mit `u(x)` und `Re_p` am Anfang einen vorläufigen Ort rechnen, dann `u` in der Mitte zwischen altem und vorläufigem Ort abtasten und den Schritt damit endgültig rechnen.
- Das Verfahren ist für jeden Zeitschritt stabil und geht für `τ_e → 0` in die masselose Bewegung über.
- **Schrittweite:** höchstens ein halber Rasterschritt Weg je Schritt (`dt ≤ 0,5 · h / max(|v|, |u|)`), damit kein Dreieck übersprungen wird.
- **Start:** Ort aus dem Startbereich, Geschwindigkeit = Strömung am Ort plus Sinkgeschwindigkeit `τ_e · g'`.
- Schwerkraft ist abschaltbar. Sie wirkt in Welt-Z; das Strömungssystem ist nur um Z gedreht, also ist die Richtung dort dieselbe.

### 3. Aufprall

- **Genau (CPU):** Der Weg eines Schritts ist eine Strecke. Sie wird gegen die Dreiecke der Ergebnis-Oberfläche geprüft; dafür `foamcraft_sim::distance` um den Schnitt Strecke–Dreieck erweitern (nur die Zellen des Dreiecksrasters ablaufen, durch die die Strecke führt). Ergebnis: Dreieck, Trefferpunkt, Normale, Geschwindigkeit, Winkel zur Fläche.
- **Schnell (GPU, für die Animation):** ein Abstandsfeld je Rasterstufe (Abstand zur Oberfläche an jedem Rasterpunkt, innen negativ; berechnet mit `distance`, Vorzeichen aus der Festkörpermaske, im Ergebnis-Cache abgelegt). Der Shader tastet es trilinear ab; wechselt das Vorzeichen innerhalb eines Schritts, liegt der Treffer dazwischen (linear interpoliert), die Normale ist das Gefälle des Abstandsfelds. Das ist deutlich genauer als die Maske allein, aber an dünnen Teilen nur so gut wie das Raster.
- **Haften:** Das Partikel bleibt am Trefferpunkt, wird 1,5 s (Bildschirmzeit) als heller Punkt gezeigt und verblasst, dann startet es neu.
- **Abprallen:** Geschwindigkeit senkrecht zur Wand umkehren und mit der Stoßzahl `e` malnehmen, Anteil entlang der Wand mit `(1 − μ_w)`. Vorgaben: `e = 0,3`, `μ_w = 0,2`. Nach dem Abprall läuft die Bewegung weiter; höchstens fünf Abpraller, dann haftet das Partikel.

### 4. Stoffvorlagen

| Kennung | Name (de) | Durchmesser | Dichte | Aufprall |
|---|---|---|---|---|
| `fine_dust` | Feinstaub | 10 µm | 1500 kg/m³ | Haften |
| `spray` | Sprühnebel | 50 µm | 1000 kg/m³ | Haften |
| `rain` | Regentropfen | 1 mm | 1000 kg/m³ | Haften |
| `sand` | Sand | 0,3 mm | 2650 kg/m³ | Abprallen (`e = 0,5`) |
| `snow` | Schneeflocke | 3 mm | 100 kg/m³ | Haften |

Dazu "Eigene Werte". Zur gewählten Kombination zeigt die Oberfläche die Stokes-Zahl und einen Satz dazu:

- `St < 0,05`: "Folgt der Strömung fast wie Rauch und trifft das Modell kaum."
- `0,05 ≤ St ≤ 2`: "Wird um das Modell getragen, schneidet aber die Kurven. Ein Teil trifft."
- `St > 2`: "Fliegt fast geradeaus und trifft das Modell dort, wo es im Weg steht."

### 5. Trefferkarte

Die Karte ist eine Auswertung, kein Nebenprodukt der Animation: Sie braucht viele Partikel und exakte Trefferorte. Sie läuft auf der CPU im Hintergrund.

- **Freisetzen:** `N` Partikel (Vorgabe 200 000) gleichmäßig verteilt (geschichtetes Raster mit PCG-Zufall, fester Startwert).
  - Windkanal: Rechteck quer zur Strömung, `1 · L` vor dem Modell, so groß wie die Hüllbox des Modells quer zur Strömung plus Rand `0,3 · L`, nach oben erweitert um die Strecke, die die Partikel auf dem Weg zum Modell absinken (`τ_e · g' / U_ref · Abstand`).
  - Innenströmung: auf den Einlässen, gewichtet mit der örtlichen Geschwindigkeit (mehr Partikel dort, wo mehr hindurchströmt).
- **Verfolgen** mit dem Integrator bis zum Aufprall (genaue Variante), bis zum Verlassen des Gebiets oder bis zu einer Höchstzeit (`20 · L / U_ref`).
- **Auftreffrate** je Dreieck (Windkanal): `β = (Treffer / Dreiecksfläche) / (N / Freisetzfläche)`. `β = 1` heißt: Hier kommt so viel an wie in der freien Anströmung durch dieselbe Fläche quer zur Strömung fließt. Das ist die in der Technik übliche Größe für Regen, Staub und Vereisung.
- **Innenströmung:** Treffer je Fläche, bezogen auf den Mittelwert über alle Wände (dimensionslos).
- **Glätten:** Zählwerte je Dreieck rauschen. Einmal auf die Eckpunkte verteilen und zurück mitteln (flächengewichtet), sodass die Gesamtzahl der Treffer erhalten bleibt.
- **Felder am Modell:** `impact` (Auftreffrate) und `impact_speed` (mittlere Geschwindigkeit senkrecht zur Wand beim Aufprall, m/s), anzeigbar über die bestehende Feldauswahl der Oberfläche, mit Legende.
- **Kennzahlen:**
  - Windkanal: Anteil der Partikel, die auf die Stirnfläche des Modells zufliegen und es treffen (`Treffer / (N · Stirnfläche / Freisetzfläche)`; die Stirnfläche liefert `foamcraft_sim`).
  - Innenströmung: Anteil der eingetretenen Partikel, der im Bauteil hängen bleibt.
  - Dazu die statistische Unsicherheit (`1 / sqrt(Treffer)`), damit niemand drei Treffer für ein Ergebnis hält.
- **Ablage:** `cache/results/<lauf>/impact-<schlüssel>.bin`, der Schlüssel aus Stoff, Schwerkraft, Aufprallart, `N` und Startwert. Gleiche Eingaben liefern die Karte aus dem Cache.
- **Ablauf:** Hintergrund-Thread mit Fortschritt und Abbruch nach dem Muster von `export.video` (`export.status`, `export.cancel`) bzw. Task_23. Rechenzeit messen; reicht ein Kern nicht, mit `std::thread` auf mehrere Kerne verteilen (jeder Kern ein fester Block von Partikeln, damit das Ergebnis unabhängig von der Kernzahl gleich bleibt).

### 6. Animation in der Ansicht

- Eigener Shader (zum Beispiel `particles_heavy.wgsl`) mit Ort, Alter, Geschwindigkeit und Haft-Zeit je Partikel; der bestehende Shader und seine Golden-Images bleiben unberührt.
- Darstellung wie Kometen (Kopf und Schweif), Schweif entlang der **eigenen** Geschwindigkeit `v`, nicht der Strömung. Haftende Partikel als kurzer heller Strich entlang der Flächennormalen (eine Linie der Länge null zeichnet nichts).
- Färbung: Tempo, Schlupf `|u − v|` (zeigt, wo die Partikel der Strömung nicht folgen) oder einfarbig.
- Start- und Zeitlogik wie bisher (fester Startwert, PCG, `step_particles`-Takt, Tempo, Pause). Die Partikelzahl der Animation ist unabhängig von `N` der Karte.

### 7. Mitflug auf einem schweren Partikel

- `ride.start` (Task_32) bekommt den optionalen Teil `particle` (Stoff oder Durchmesser und Dichte, Schwerkraft, Aufprallart). Die Bahn rechnet dann die CPU-Referenz dieses Tasks statt der Stromlinie; rückwärts wird **nicht** integriert (die Gleichung ist rückwärts instabil): Der Flug beginnt am Freisetzpunkt.
- Auswahl: Vorschläge `impact_front` (trifft nahe dem Staupunkt), `impact_grazing` (flachster Aufprallwinkel) und `near_miss` (kommt dem Modell am nächsten, ohne zu treffen), ermittelt aus 600 Kandidaten wie in Task_32. Anklicken einer Stelle am Modell wählt den Kandidaten, der dort am nächsten auftrifft.
- **Aufprall im Flug:** neues Ereignis `impact` mit Geschwindigkeit und Winkel. Ruhig: Die Zeit wird davor gedehnt, beim Treffer hält das Bild 1 s an, der Satz nennt die Zahlen ("Aufprall mit 12 m/s unter 35° zur Fläche"). Achterbahn: kurzer Ruck der Kamera (0,2 s, abklingend), kein Anhalten. Beim Haften endet der Flug und die Kamera fährt in den Verfolger zurück; beim Abprallen geht er weiter.
- Die Anzeige zeigt zusätzlich den Schlupf (eigenes Tempo gegen Tempo der Strömung).

### 8. Befehle und Speicherung

| Befehl | Zweck |
|---|---|
| `particles.materials` | Liste der Stoffvorlagen mit Werten und der Stokes-Zahl für den gezeigten Lauf. |
| `view.set_particles` | neuer optionaler Teil `inertia`: `material` oder `diameter_m` und `density_kg_m3`, `gravity`, `on_impact` (`stick`, `bounce`), `restitution`, `wall_friction`; `inertia: null` schaltet zurück auf masselos. Neuer Wert `slip` für `color_by`. |
| `results.impact` | startet die Trefferkarte (Parameter wie `inertia`, dazu `count`, `seed`, `wait`); Antwort mit Kennung. |
| `results.impact_status`, `results.impact_cancel` | Fortschritt und Abbruch. |

- Manifest: `ParticleSpec.inertia` (optional, `#[serde(default)]`), `SCHEMA_VERSION` um eins erhöhen, Kommentar in `migrate`.
- Vorlage für die Galerie (Task_28): `rain` (Regentropfen, Haften, Karte `impact` am Modell, sofern berechnet).
- MCP: `particles_materials`, `results_impact`, `results_impact_status` in `CURATED`; Leitfaden und `docs/mcp.md` ergänzen.
- Fehlercodes: `particles.too_light`, `particles.invalid_material`, `impact.not_found`, `impact.cancelled` (en und de mit Vorschlag).

## Umsetzungsschritte

1. Tests zuerst für den Integrator (Sinkgeschwindigkeit, Ansprechzeit, großer Zeitschritt, Grenzfall masselos).
2. CPU-Referenz mit genauem Aufprall (Strecke–Dreieck in `distance`), Haften und Abprallen.
3. Prüfung an der Umströmung von Zylinder und Kugel (kritische Stokes-Zahl).
4. Trefferkarte: Freisetzen, Zählen, Glätten, Kennzahlen, Cache, Hintergrundlauf.
5. Abstandsfeld je Rasterstufe; GPU-Shader; Vergleich GPU gegen CPU.
6. Befehle, Manifest, Stoffvorlagen, MCP.
7. Mitflug: Bahn eines schweren Partikels, Ereignis `impact`, Vorschläge.
8. Oberfläche (Vitest zuerst), Vorlage, live prüfen, Nachweise ablegen, `docs/architecture.md` ergänzen.

## Umsetzungshinweise

Aus der Sichtung des Codes am 2026-11-21.

- **CPU-Referenz und GPU nebeneinander:** `foamcraft_sim::particles::Particles` (CPU) und `particles.wgsl` (GPU) sind bewusst gleich gebaut, `tests/render/particles.rs` vergleicht beide. Dasselbe Muster hier: eine CPU-Variante, die wie die GPU das Abstandsfeld nutzt, für den Vergleich; die genaue Variante mit Dreiecken für Karte und Mitflug.
- **Zustand auf der GPU:** heute ein `vec4` je Partikel (Ort, Alter) im Puffer `state`. Schwere Partikel brauchen ein zweites `vec4` (Geschwindigkeit, Haft-Zeit). `Params` ist ein Uniform-Block mit 16-Byte-Ausrichtung; die Rust-Struktur muss exakt passen (Stolperstein aus Task_16 und Task_27).
- **Abstandsfeld als weiterer Storage-Puffer** neben `field` und `pressure` (ein `f32` je Rasterpunkt, gleiche Reihenfolge und derselbe Versatz je Stufe wie `field`).
- **Medium:** `medium.density` und `medium.kinematic_viscosity` aus dem Szenario des gezeigten Laufs. Achtung bei Laufvergleichen: Das Szenario kann sich seit dem Lauf geändert haben; die Werte des Laufs nehmen (wie `results.summary` es tut).
- **Oberflächenfelder:** `view.set_surface_field` prüft den Namen gegen die Felder des Caches. `impact` und `impact_speed` liegen in einer eigenen Datei; die Feldliste und der Szenenaufbau (`foamcraft_api::scene`) müssen sie zusätzlich kennen, sobald eine Karte berechnet ist.
- **Stirnfläche:** `foamcraft_sim` berechnet sie bereits (Tests in `tests/sim/frontal_area.rs`).
- **Beschnitt (Task_27)** und **Partikelzeit im Mitflug (Task_32)** gelten auch für den neuen Shader; in deren Erkenntnissen nachlesen, wie sie eingebaut sind.
- **Keine neue Abhängigkeit:** Mehrere Kerne mit `std::thread::scope`; eine Bibliothek dafür nur nach Rückfrage (Erlaubnisliste in `docs/licensing.md`).

## Tests

Alle Tests dieses Tasks werden permanent in `tests/` im Hauptverzeichnis des Projekts abgelegt (siehe `AGENTS.md`).

- **Integrator (Rust, `tests/sim/heavy_particles.rs`):**
  - Ruhendes Medium, kleine Reynolds-Zahl: Sinkgeschwindigkeit `τ_p · g'` innerhalb 1 %.
  - Partikel ruht, gleichförmige Strömung, ohne Schwerkraft, `f = 1`: nach der Zeit `τ_p` hat es 63,2 % der Strömungsgeschwindigkeit (1 %).
  - Zeitschritt `10 · τ_p` bleibt stabil und endet auf der richtigen Geschwindigkeit.
  - `d → 0` ergibt die Bahn des masselosen Partikels (Vergleich mit `foamcraft_sim::particles`).
  - Bei `Re_p > 1` sinkt das Partikel langsamer als nach Stokes (Richtung der Korrektur stimmt).
- **Aufprall:** Strecke–Dreieck an Fläche, Kante, knapp daneben; ein Partikel, das senkrecht auf eine Platte fliegt, trifft am erwarteten Punkt; Abprallen mit `e = 0,5` halbiert die Geschwindigkeit senkrecht zur Wand; nach fünf Abprallern haftet es.
- **Kritische Stokes-Zahl (analytisch, harter Prüfwert):** Potentialströmung um Zylinder und Kugel (Radius `R`, Feld direkt aus der Formel), `f = 1`, ohne Schwerkraft, Partikel starten `10 · R` stromauf mit Strömungsgeschwindigkeit. Mit `K = τ_p · U / R` gilt: kein einziger Treffer unterhalb von `K = 1/8` (Zylinder) bzw. `K = 1/12` (Kugel), Treffer oberhalb. Herleitung zum Nachprüfen: Vor dem Staupunkt ist die Strömung `u = −a · x` mit `a = 2U/R` (Zylinder) bzw. `3U/R` (Kugel); die Gleichung `τ_p · x'' + x' + a · x = 0` erreicht die Wand nur, wenn `4 · a · τ_p > 1`. Test: bei `0,8 ·` kritisch keine Treffer, bei `2 ·` kritisch Treffer; die Trefferquote steigt mit `K` und liegt bei `K = 100` über 95 %.
- **Vergleich mit Literaturwerten:** Die Trefferquote über `K` für Zylinder und Kugel ist in der Literatur tabelliert (Langmuir und Blodgett, 1946). Der Agent sucht die Werte in einer zitierfähigen Quelle heraus, trägt Quelle und Toleranz in den Test ein und vergleicht drei Stützstellen. Nicht aus dem Gedächtnis eintragen; findet sich keine verlässliche Quelle, bleibt es beim harten Prüfwert oben, und das steht in den Notizen.
- **Trefferkarte:** Platte quer zur gleichförmigen Strömung mit sehr schweren Partikeln: `β = 1` auf der Platte (innerhalb der statistischen Unsicherheit), Kennzahl 100 %; die Summe über alle Dreiecke entspricht der Zahl der Treffer, auch nach dem Glätten; gleicher Startwert ergibt dieselbe Karte, unabhängig von der Zahl der Kerne; zweiter Aufruf kommt aus dem Cache; Abbruch hinterlässt keine Datei.
- **GPU gegen CPU (`tests/render/`):** Variante mit Abstandsfeld stimmt nach 200 Schritten mit der CPU-Variante überein (Toleranz wie im bestehenden Vergleich); kein Partikel liegt im Festkörper; Golden-Image `particles_heavy.png`; bestehende Golden-Images unverändert.
- **Befehle:** `view.set_particles` mit `inertia` speichert und lädt; `particles.too_light` bei zu kleiner Dichte; `results.impact` an Testdaten liefert Felder und Kennzahlen; Migration der Vorversion.
- **Mitflug:** Bahn eines schweren Partikels endet mit dem Ereignis `impact` am Trefferpunkt; Zeit-Tabelle bleibt monoton über das Anhalten hinweg.
- **Vitest:** Stoffkarten setzen `inertia`; Stokes-Zahl und Satz werden angezeigt; "Trefferkarte berechnen" zeigt Fortschritt und danach die Kennzahl; Hinweis "grobe Näherung" bei kleinem Dichteverhältnis.
- **Leistung (ignoriert, Ergebnis in den Notizen):** Trefferkarte mit 200 000 Partikeln am Windkanal-Beispiel; Bildzeit der Animation mit 50 000 schweren Partikeln.

## Akzeptanzkriterien

Der Task gilt erst als fertig, wenn alle Punkte erfüllt und durch Tests oder einen konkreten Befehl belegt sind:

- [x] Am Windkanal-Beispiel treffen Regentropfen sichtbar die Vorderseite des Modells und haften; Feinstaub strömt fast vollständig vorbei (Nachweis-Bilder beider Fälle)
- [x] Die Stokes-Zahl und ihr Erklärsatz ändern sich mit Stoff und Durchmesser
- [x] Die Trefferkarte lässt sich berechnen und am Modell anzeigen; die Kennzahl nennt Anteil und Unsicherheit
- [x] Die kritische Stokes-Zahl an Zylinder und Kugel wird getroffen (Test grün)
- [x] Im Mitflug auf einem schweren Partikel sieht man den Aufprall; die Anzeige nennt Geschwindigkeit und Winkel
- [x] Am Rohrverteiler zeigt die Kennzahl, welcher Anteil im Bauteil hängen bleibt
- [x] Über MCP lassen sich Stoffe abfragen, schwere Partikel einschalten und eine Trefferkarte berechnen

## Offene Fragen / Annahmen

- Annahme: einseitige Kopplung und kugelförmige Partikel reichen für eine anschauliche und in der Tendenz richtige Darstellung. Das steht als Hinweis bei der Trefferkarte ("Abschätzung").
- Annahme: Turbulente Streuung bleibt aus. Die Rechnung liefert nur die mittlere Strömung; in Wirklichkeit streuen Wirbel feine Partikel zusätzlich auf Wände. Für feinen Staub unterschätzt die Karte deshalb das Absetzen. Das steht im Erklärtext. Ein Streumodell wäre ein eigener Task.
- Offen: Reicht das Abstandsfeld auf 64³ je Stufe für eine glaubwürdige Animation an dünnen Teilen (Kanten, Flügelhinterkanten)? Am Beispiel ansehen; wenn nicht, dem Nutzer berichten (feineres Raster wäre eine Änderung an Task_14).
- Offen: Die Literaturwerte der Trefferquote müssen aus einer Quelle belegt werden (siehe Tests).

## Notizen zu diesem Task

Taskspezifische Informationen während der Arbeit hier festhalten. Taskübergreifendes gehört in `Roadmap/README.md`.

- **Nachweise (live, eigene App-Instanz, 2026-11-21):** Windkanal-Beispiel mit Vorlage "Regen": Tropfen fallen schräg, treffen Front und Oberseite und haften (dunkle Marken); "Feinstaub" strömt wie Rauch am Modell vorbei. Trefferkarte mit 200 000 Tropfen in 5,0 s (Debug-Build, alle Kerne) bzw. 8,8 s nach der Korrektur unten, am Modell als Feld `impact`: Front und Oberseite getroffen, Seiten nicht. Kennzahl: 95,1 % ± 0,6 % der auf das Modell zufliegenden Tropfen treffen. Mitflug "Treffer vorn": endet mit "Aufprall mit 19,2 m/s unter 78° zur Fläche", HUD mit Schlupf. Rohrverteiler: Sand bleibt zu 98,3 % ± 0,5 % hängen, Feinstaub zu 4,8 % ± 2,1 %. Bilder im Scratchpad (`rain3.png`, `dust1.png`, `impact2.png`, `rideimpact2.png`).
- **Leistung:** Animation mit 20 000 schweren Partikeln: 3 bis 4 ms je Bild (Debug-Build, Log "particle frame"); 50 000 nicht eigens gemessen.
- **Abweichungen und Korrekturen:** (1) Die Startgeschwindigkeit ist die echte Endgeschwindigkeit (`heavy::terminal_velocity`, mit Schiller-Naumann), nicht `tau_e(0) · g'`: für 1-mm-Tropfen sonst 30 m/s statt rund 4 m/s, die Tropfen fielen sofort unter das Modell. (2) Der Anteil "trifft" bezieht sich auf die Fläche, die das Modell entlang der Flugrichtung eines freien Teilchens (Strömung plus Sinken) zeigt, nicht nur auf die Stirnfläche quer zur Strömung: Regen fällt auch auf die Oberseite, sonst ergaben sich 392 %. (3) Das Abstandsfeld liegt auf dem Platz des Druckpuffers (höchstens 8 Speicherpuffer je Shader-Stufe); echte Abstände nur auf der feinsten Stufe, sonst das Vorzeichen aus der Maske; es wird je Lauf in der Sitzung gehalten, nicht als Datei. (4) Mitflug: kein Anhalten von 1 s beim Aufprall und kein Kamera-Ruck in der Achterbahn; der Satz zum Aufprall erscheint mit den Zahlen. (5) Vergleich mit Literaturwerten (Langmuir und Blodgett) nicht eingetragen: keine geprüfte Quelle zur Hand; es bleibt beim harten Prüfwert der kritischen Stokes-Zahl (Test grün). (6) `ride.start` zählt Neustarts (`RideSpec.run`), damit ein erneuter Start desselben Vorschlags von vorn beginnt; Einstellungsänderungen behalten die Stelle.

## Abschluss-Checkliste

Reihenfolge und Details stehen in `AGENTS.md`, Abschnitt "Abschluss- und Archivierungs-Workflow".

- [x] Alle Akzeptanzkriterien erfüllt, Tests liegen permanent in `tests/` und laufen grün
- [x] Abschnitt "Erkenntnisse für Folge-Agenten" unten ausgefüllt
- [x] Folge-Tasks und `Roadmap/README.md` geprüft und bei Bedarf angepasst
- [x] Datei ohne `RUN_` nach `Roadmap/Archiv/` verschoben

## Erkenntnisse für Folge-Agenten


Vor der Archivierung vom Agenten auszufüllen. Schreibe so, dass ein Agent, der diesen Task nie gesehen hat, davon bei späteren Tasks profitiert:

- **Umgesetzt (und Abweichungen vom Plan):** `foamcraft_sim::heavy` (Bewegungsgleichung, exakter Integrator, `terminal_velocity`, `segment_hit`, `track` mit Haften/Abprallen, `critical_hits`, `impact_counts` auf allen Kernen, `smooth`), `foamcraft_sim::heavy_gpu` (CPU-Referenz des Shaders), `particles_heavy.wgsl` und `Renderer::create_heavy_particles`/`read_motion`, `foamcraft_api::heavy` (Stoffe, `resolve`, Medium des Laufs, Abstandsfeld, `results.impact/_status/_cancel`, Karte als `impact-<schlüssel>.json` plus `impact-latest.json`), `ParticleSpec.inertia` (Schema 16), `view.set_particles` mit `inertia` (null = masselos) und Farbe `slip`, Felder `impact`/`impact_speed` über `load_surface`, Vorlage `rain` (Startfläche um die Fallstrecke angehoben), Mitflug auf schweren Partikeln (`RideSpec.particle`, Vorschläge `impact_front`, `impact_grazing`, `near_miss`, Ereignis `impact` mit Geschwindigkeit und Winkel, Spalte `slip`), MCP-Werkzeuge, `HeavySection` in der Oberfläche. Abweichungen siehe Notizen.
- **Learnings & Stolpersteine:** (1) wgpu erlaubt standardmäßig nur 8 Speicherpuffer je Shader-Stufe. (2) `view.set_surface_field` prüfte nur die gespeicherte Feldliste: abgeleitete Felder (`attached`, `impact`) brauchen `results::meta_with_derived` – jetzt überall. (3) Bei großen Tropfen ist die Stokes-Ansprechzeit um eine Größenordnung zu lang; für Start und Fallstrecke immer die korrigierte Endgeschwindigkeit nehmen. (4) Für die GPU-Prüfung reicht ein analytisches Abstandsfeld (Quader) im Test; das echte kommt aus `WallDistance`.
- **Wichtige Entscheidungen und ihre Gründe:** Zwei Varianten: genau mit Dreiecken (Karte, Mitflug) und schnell mit Abstandsfeld (GPU-Animation), jede mit eigener Referenz. Die Karte summiert feste Blöcke von 1024 Teilchen in ihrer Reihenfolge: gleiches Ergebnis bei jeder Kernzahl (Test). Der Mitflug nimmt das schwere Teilchen der Ansicht, wenn dort eines eingestellt ist: Anklicken eines schweren Teilchens startet so einen schweren Flug.
- **Auswirkungen auf Folge-Tasks:** Task_35 (Leuchtspuren) kann die schweren Partikel als weitere Quelle von Spuren nutzen (`read_motion`). Task_37 (Leistung): Trefferkarte im Release messen. Task_41 (zeitabhängig) muss den Integrator mit zeitabhängigem `u` speisen. Turbulente Streuung bleibt offen (eigener Task).
- **Hinweise zu Tests und Befehlen:** `cargo test -p foamcraft-sim --test heavy_particles`, `cargo test -p foamcraft-render --test heavy_particles` (Golden `particles_heavy.png`), `cargo test -p foamcraft-api --test heavy`, `npx vitest run tests/app/heavy-section.test.tsx`; live: `results.impact` mit `wait: true` per CLI `command batch`.
