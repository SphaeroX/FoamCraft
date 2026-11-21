# Task_33: Strömung an der Oberfläche – Wandstromlinien und Wollfäden

> ## ⚠️ PFLICHT – BEVOR DU IRGENDETWAS ANDERES TUST
>
> 1. **Lies als Allererstes die Datei `AGENTS.md` im übergeordneten Ordner `Roadmap/`** (von hier: `../AGENTS.md`) und befolge sie strikt.
> 2. **Deine allererste Aktion** ist, diese Datei in `RUN_Task_33.md` umzubenennen, um den Task als laufend zu markieren. Das passiert **noch bevor irgendein Code geschrieben** wird.

## Ziel

Man sieht direkt auf dem Modell, was die Strömung dort tut: Linien, die wie ein Ölanstrich im Windkanal über die Oberfläche laufen und zeigen, wo die Strömung anliegt, sich ablöst und wieder anlegt; dazu kurze Wollfäden, die bei anliegender Strömung glatt liegen und in abgelösten Gebieten zappeln.

## Kontext & Abhängigkeiten

- **Voraussetzung:** Task_32 ist archiviert (Modul `foamcraft_sim::distance`, freie Kamera).
- **Herkunft:** Wunsch des Nutzers (2026-11-21) nach Partikeln, die "direkt am Objekt entlang sliden", und nach weiteren anschaulichen Darstellungen. Im Interview gewählt: Wandstromlinien (Ölfilm) und Wollfäden am Modell.
- **Relevantes Vorwissen:**
  - Heute gibt es nur den Startbereich `surface`: Partikel starten nahe der Oberfläche und fliegen dann frei im Volumen weiter. Sie gleiten nicht auf der Fläche.
  - Was die Strömung an der Wand tut, steht in der **Wandschubspannung** (Vektor je Wandfläche). Ihre Richtung ist die Richtung der wandnahen Strömung. Linien, die diesem Vektorfeld auf der Oberfläche folgen, heißen Wandstromlinien; im Windkanal macht man sie mit Ölanstrich sichtbar.
  - Die Auswertung liest den Vektor schon aus OpenFOAM (`read_vector_list(... "vectorField/wallShearStress")` in `crates/foamcraft-api/src/results.rs`), speichert im Cache aber nur den Betrag (`wall_shear_stress`).
  - `foamcraft_sim::results::Surface` (Punkte, Dreiecke, Felder je Dreieck), Meldungskatalog, `view.set_surface_field`, Vorlagen-Galerie aus Task_28.

## Umfang

**Gehört dazu:**

- Wandschubspannung als Vektor im Ergebnis-Cache; Ersatzweg für alte Läufe
- Wandstromlinien: gleichmäßig verteilte Linien auf der Oberfläche, mit laufenden Lichtpunkten animiert
- Karte "anliegend / Rückströmung" als Oberflächenfeld (Windkanal)
- Wollfäden: Richtung aus der Wandschubspannung, Zappeln als Veranschaulichung der Turbulenz, Durchhängen im Totwasser
- Befehl `view.set_surface_flow`, MCP-Werkzeug, zwei Einträge in der Vorlagen-Galerie, Bedienung in der Oberfläche
- Wiederholbarkeit (fester Startwert, Zeit nur aus dem Bildindex), damit Videoexport und Golden-Images funktionieren

**Gehört nicht dazu:**

- Zeitgenaues Flattern (dafür bräuchte es eine zeitabhängige Rechnung, Task_41)
- Automatisches Auffinden und Benennen von Ablöselinien und kritischen Punkten
- Wandstromlinien auf Deckflächen von Öffnungen (dort ist keine Wand)

## Mechanik

Zahlenwerte sind Startwerte; Änderungen mit Grund in die Notizen. `L`, `h`, `U_ref` wie in Task_32.

### 1. Daten

- **Cache:** Beim Auswerten zusätzlich drei Felder je Dreieck schreiben: `wss_x`, `wss_y`, `wss_z` (Pa, in der Welt: mit derselben Drehung wie die Punkte, mit der Dichte multipliziert wie der Betrag).
- **Vorzeichen:** OpenFOAM liefert die Schubspannung, die die Wand auf das Fluid ausübt; sie zeigt **gegen** die wandnahe Strömung. Die Richtung der Wandstromlinien ist also `−wss`. Das nicht annehmen, sondern mit einem Test belegen (siehe Tests: am Rohr zeigt `−wss` stromab).
- **Alte Läufe** ohne diese Felder: Ersatzrichtung aus dem Volumenraster. An jedem Dreieck die Geschwindigkeit im Punkt `Schwerpunkt + 1,5 · h · n` (ins Fluid) abtasten und den Anteil entlang der Normalen abziehen. Das ist ungenauer (das Raster ist grob) und wird in der Oberfläche als "genähert" gekennzeichnet. Kein erneutes Auswerten erzwingen: Die Docker-Volumes alter Läufe gibt es oft nicht mehr.
- **Tangentialfeld:** Vektor je Dreieck in die Dreiecksebene projizieren. Für glatte Linien je Eckpunkt mitteln (flächengewichtet über die angrenzenden Dreiecke) und im Dreieck baryzentrisch interpolieren, danach wieder in die Ebene projizieren. An scharfen Kanten (Winkel zwischen den Normalen über 40°) nicht über die Kante mitteln.
- **Nachbarschaft:** Kantenliste der Oberfläche (Kante → bis zu zwei Dreiecke). Die Dreiecke stammen aus der OpenFOAM-Ausgabe mit gemeinsamen Punkten; prüfen, ob benachbarte Dreiecke wirklich dieselben Punktnummern nutzen. Wenn nicht, Punkte mit einer Toleranz von `1e-6 · L` verschmelzen.

### 2. Linien auf der Oberfläche verfolgen

Neues Modul `foamcraft_sim::surface_flow` (reine Logik).

- **Zustand** eines Linienpunkts: Dreieck und Ort im Dreieck.
- **Schritt** fester Länge `ds = 0,3 · mittlere Kantenlänge`: Richtung = normiertes Tangentialfeld am Ort (Mittelpunktregel: erst halber Schritt, dort Richtung neu bestimmen, dann ganzer Schritt).
- **Über eine Kante:** Verlässt der Schritt das Dreieck, am Schnittpunkt mit der Kante abschneiden, in das Nachbardreieck wechseln und den Rest des Schritts um die gemeinsame Kante in dessen Ebene klappen (die Richtung bleibt im aufgeklappten Zustand gerade). Höchstens acht Wechsel je Schritt.
- **Ende:** Rand der Oberfläche (Kante mit nur einem Dreieck), Betrag des Felds unter `1e-4` des Größtwerts (kritischer Punkt: dort läuft die Strömung zusammen oder auseinander), Länge über `3 · L`, oder die Linie kommt einer anderen zu nahe (siehe unten).
- Jede Linie wird vom Startpunkt aus **in beide Richtungen** verfolgt.

### 3. Gleichmäßige Verteilung

Zufällig gestreute Linien klumpen dort, wo die Strömung zusammenläuft, und lassen anderswo Löcher. Deshalb:

- Ein Belegungsraster im Raum (Würfel der Kantenlänge `d_sep`, Standard `L / 60`, Regler "Dichte" von `L / 30` bis `L / 120`) merkt sich, welche Linie durch welche Zelle läuft.
- Startpunkte kommen in fester Reihenfolge (PCG mit Startwert, flächengewichtet über die Dreiecke). Ein Startpunkt wird verworfen, wenn seine Zelle belegt ist.
- Eine Linie endet, sobald sie in eine Zelle läuft, die eine **andere** Linie belegt hat.
- Linien kürzer als `4 · d_sep` verwerfen (sie sind nur Rauschen).
- Ergebnis: Linien mit ungefähr gleichem Abstand über die ganze Oberfläche; wo sie zusammenlaufen und enden, liegt eine Ablöselinie. Genau das will man sehen.

### 4. Darstellung der Linien

- **Lage:** um `0,002 · L` entlang der Normalen angehoben, damit sie nicht mit der Oberfläche flimmern.
- **Aussehen "Ölfilm":** Modell dunkel und matt, Linien hell. Alternativ Linien nach Betrag der Wandschubspannung gefärbt (Farbskala und Legende wie bei den Oberflächenfeldern).
- **Bewegung ohne Partikel:** Jeder Linienpunkt trägt seine Laufzeit entlang der Linie (Bogenlänge geteilt durch ein Tempo proportional zu `|wss|^0,5`, damit schnelle Gebiete schneller laufen, aber nicht um Größenordnungen). Ein Zeitwert im Shader lässt helle Punkte mit dunklem Schweif die Linien entlanglaufen (Helligkeit als Funktion von `Laufzeit − Zeit`, periodisch, Phase je Linie aus dem PCG). Das sind die "Partikel, die am Objekt entlang gleiten", ohne dass auf der GPU ein Netz verfolgt werden muss; die Linien werden einmal je Ergebnis berechnet und hochgeladen.
- **Renderer:** eigene kleine Pipeline für "Strömungslinien": Vertex mit Ort, Farbe, Laufzeit, Phase; ein Zeitwert in den globalen Daten. Die bestehende Linien-Pipeline (`shader.wgsl`, `fs_main`) nicht ändern, damit alle Golden-Images gültig bleiben. Linien sind in wgpu ein Pixel breit; reicht das auf hochauflösenden Bildschirmen nicht, als schmale Bänder aus Dreiecken zeichnen (zwei Dreiecke je Abschnitt, in der Tangentialebene). Task_35 baut auf dieser Zeit-Infrastruktur auf.
- **Zeit:** Die App erhöht den Zeitwert im 60-Hz-Takt; der Export setzt ihn aus Bildindex und Bildrate. Pause und Tempo wie bei den Partikeln.

### 5. Karte "anliegend / Rückströmung"

- Nur Windkanal: je Dreieck der Kosinus des Winkels zwischen der Richtung `−wss` und der Anströmrichtung, beide in die Dreiecksebene projiziert. `+1` heißt anliegend in Strömungsrichtung, `−1` Rückströmung.
- Als Oberflächenfeld `attached` über die bestehende Anzeige (`view.set_surface_field`), mit eigener zweifarbiger Skala und Legendentext "anliegend … Rückströmung".
- Kennzahl in `results.summary`: Anteil der Oberfläche in Rückströmung (Kennung `reverse_flow_area`, mit Erklärtext und Kurzbezeichnung wie in Task_17/Task_20 beschrieben). Das Schema von `results.summary` ist als Snapshot festgehalten: neue Kennzahl ergänzen und den Snapshot mit Begründung aktualisieren; besteht daran ein Zweifel, den Nutzer fragen.

### 6. Wollfäden

- **Verteilung:** Fußpunkte mit Mindestabstand `d_tuft = L / 25` (dasselbe Belegungsraster, Regler von `L / 12` bis `L / 50`), höchstens 4000 Fäden. Länge `ℓ = 0,6 · d_tuft`.
- **Ruhelage:** Richtung `−wss` in der Tangentialebene, um 8° von der Wand abgehoben. Der Faden besteht aus vier Gliedern; jedes folgende Glied nimmt die Richtung des Felds an seinem Ort (so biegt er sich in gekrümmter Strömung).
- **Durchhängen:** Ist die Wandschubspannung sehr klein (unter 2 % des 90. Perzentils auf dem Modell), hat die Strömung keine Kraft auf den Faden. Dann überblendet die Richtung weich auf "hängt nach unten" (Schwerkraft in die Tangentialebene projiziert; auf waagerechten Flächen bleibt die letzte Richtung, verkürzt dargestellt).
- **Zappeln** (Veranschaulichung, keine zeitgenaue Rechnung):
  - Stärke aus dem Turbulenzgrad nahe der Wand: `Tu = sqrt(2k/3) / max(|u|, 0,05 · U_ref)`, abgetastet im Punkt `Fußpunkt + 2 · h · n` aus dem Raster (`k` gibt es nur bei turbulenter Rechnung).
  - Ausschlag `A = 55° · smoothstep(0,10, 0,50, Tu)`; in Rückströmung (`attached < 0`) mindestens 20°. Ohne `k` (laminar) nur der Anteil aus der Rückströmung.
  - Bewegung: Summe aus zwei Sinusschwingungen quer zur Ruhelage, Frequenz `clamp(|u| / ℓ, 2 Hz, 12 Hz)` und das 1,7-fache davon, Phasen je Faden aus dem PCG. Der Ausschlag wächst vom Fuß zur Spitze. Alles ist eine Funktion von Faden-Kennung und Zeit, also wiederholbar.
  - In der Oberfläche steht dabei der Satz: "Das Zappeln zeigt, wo die Strömung turbulent oder abgelöst ist. Es ist eine Veranschaulichung, keine zeitgenaue Rechnung."
- **Zeichnen:** je Faden ein schmales Band (Breite `0,08 · ℓ`, flach auf der Tangentialebene, von beiden Seiten sichtbar). Die Bewegung rechnet der Vertex-Shader aus festen Attributen (Fußpunkt, Ruherichtung, Normale, Ausschlag, Frequenz, Phase, Lage entlang des Fadens) und dem Zeitwert. Kein Hochladen je Bild.
- **Farbe:** helles Garn; wahlweise ruhig = hell, zappelnd = warm.

### 7. Befehle und Speicherung

- `view.set_surface_flow` mit optionalen Teilen:
  - `lines`: `on`, `density` (Abstand als Bruchteil von `L`), `color_by` (`plain`, `wall_shear_stress`), `animate`, `seed`
  - `tufts`: `on`, `spacing`, `color_by` (`plain`, `state`), `animate`, `seed`
  - `speed`, `paused` (gemeinsam)
- `view.surface_flow_off`.
- Manifest: `view.surface_flow` (optional), `SCHEMA_VERSION` um eins erhöhen, Kommentar in `migrate`.
- Nicht rückgängig machbar, wie die übrigen `view.*`-Befehle.
- **Vorlagen (Task_28):** `oil_film` (Linien an, Modell dunkel, kein Oberflächenfeld, Partikel aus) und `tufts` (Fäden an, Karte `attached` am Modell). In `view.apply_preset` und in die Galerie eintragen, mit Erklärtext `preset.<id>` in beiden Sprachen.
- **MCP:** `view_set_surface_flow` in `CURATED`, Leitfaden und `docs/mcp.md` ergänzen.
- **Szenenaufbau:** in `foamcraft_api::scene`, damit App, `view.render` und Export dasselbe zeigen. Linien und Fäden je Ergebnis und Einstellung einmal berechnen und in der Sitzung halten; Rechenzeit messen.

## Umsetzungsschritte

1. Tests zuerst für Nachbarschaft, Tangentialfeld und Linienverfolgung (Ebene, Würfelkante, Kugel).
2. Linienverfolgung und gleichmäßige Verteilung umsetzen.
3. Cache um `wss_x/y/z` erweitern; Ersatzweg für alte Läufe; Vorzeichen-Test.
4. Karte `attached` und Kennzahl.
5. Wollfäden: Verteilung, Ruhelage, Durchhängen, Zappeln als reine Funktionen (Tests).
6. Renderer: Zeitwert, Pipeline für Strömungslinien, Fäden im Vertex-Shader; Golden-Images.
7. Befehle, Manifest, Vorlagen, MCP.
8. Oberfläche (Vitest zuerst), Export prüfen, live prüfen, Nachweise ablegen, `docs/architecture.md` ergänzen.

## Umsetzungshinweise

Aus der Sichtung des Codes am 2026-11-21.

- **Wo der Vektor heute verloren geht:** `convert` in `crates/foamcraft-api/src/results.rs` bildet aus `wss` nur `length() * rho`. Dort die drei Komponenten ergänzen (mit `turn` drehen). `Surface.fields` ist eine Liste benannter Felder je Dreieck; drei weitere Einträge passen ohne Formatänderung. `FieldInfo` wird für jedes Feld erzeugt: Die Komponenten sollen **nicht** in der Feldauswahl der Oberfläche auftauchen (dort filtern, wo die Liste für die Oberfläche entsteht; `tests/app/results-panel.test.tsx` und `tests/api/view.rs` zeigen die heutige Erwartung).
- **Aufgezeichnete Testdaten** der Wandfelder liegen in `tests/data/fields` (siehe README dort). Für den Leser reicht das; der Vorzeichen-Test braucht einen echten Lauf (`runtime_`).
- **Oberfläche im Cache ist in der Welt**, die Raster sind im Strömungssystem: für das Abtasten von `k` und `U` am Fußpunkt den Punkt mit `to_flow` drehen (Muster in `particles.rs`, `seeds`).
- **Anströmrichtung** in der Welt: `+X` des Strömungssystems, mit dem Gierwinkel des Laufs gedreht (`view::load_grids` liefert ihn).
- **Dreieckspipeline:** `cull_mode` ist `Back`; Bänder und Fäden müssen von beiden Seiten sichtbar sein (eigene Pipeline ohne Culling oder beide Umlaufrichtungen ausgeben).
- **Durchsichtigkeit** macht der Renderer heute mit einem Punktraster (jedes zweite Pixel, `fs_main`), nicht mit Blending. Für die Lichtpunkte auf den Linien genügt Helligkeit; kein Blending nötig.
- **Globale Daten** des Shaders (`Globals` in `renderer.rs`): `_pad` hat drei freie `u32`; ein `f32` für die Zeit passt dort hinein, ohne die Größe zu ändern. Rust-Struktur und WGSL-Struktur müssen exakt zusammenpassen.
- **Dunkles Modell für den Ölfilm:** Die Modellfarbe kommt aus `foamcraft_api::scene` (Oberflächenfeld oder `MODEL_COLOR`). Für die Vorlage eine feste dunkle Farbe setzen, solange Linien ohne Oberflächenfeld aktiv sind.
- **Laufende Animation:** Der Takt in `main.rs` zeichnet nur, wenn Partikel laufen. Linien und Fäden brauchen denselben Takt auch ohne Partikel (Bedingung erweitern, Bildzeit weiter protokollieren).
- **Meldungskatalog:** neue Texte (`preset.oil_film`, `preset.tufts`, `figure.reverse_flow_area`, Hinweis "genähert") in `errors.json` mit en, de und `fix`.

## Tests

Alle Tests dieses Tasks werden permanent in `tests/` im Hauptverzeichnis des Projekts abgelegt (siehe `AGENTS.md`).

- **Linienverfolgung (Rust, `tests/sim/surface_flow.rs`):**
  - Ebene Platte mit gleichförmigem Feld: Linien sind gerade und parallel, Länge = Plattenlänge.
  - Würfel, Feld läuft über eine Kante: Die Linie setzt sich auf der Nachbarfläche fort, ohne Lücke (Abstand aufeinanderfolgender Punkte höchstens `1,5 · ds`) und ohne Knick im aufgeklappten Zustand.
  - Kugel mit Feld von Pol zu Pol (Tangentialgeschwindigkeit der Potentialströmung): Linien bleiben auf der Kugel (Abweichung vom Radius unter der Sehnenhöhe der Facetten) und folgen den Meridianen (Längengrad ändert sich um weniger als 2°).
  - Feld mit Nullstelle: Die Linie endet dort und läuft nicht im Kreis.
  - Gleichmäßige Verteilung: Zwei verschiedene Linien kommen sich nie näher als `0,5 · d_sep`; kein Gebiet größer als `3 · d_sep` bleibt leer (auf Platte und Kugel).
  - Gleicher Startwert ergibt dieselben Linien.
- **Daten (Rust):** Cache mit `wss_x/y/z` wird geschrieben und gelesen; alter Cache ohne die Felder nutzt den Ersatzweg und meldet "genähert"; die Komponenten erscheinen nicht in der Feldliste der Oberfläche.
- **Vorzeichen (`runtime_`, ignoriert):** am Rohr-Beispiel zeigt `−wss` für mindestens 99 % der Wandfläche stromab.
- **Karte:** ebene Platte in Strömungsrichtung ergibt `attached = 1`, mit umgedrehtem Feld `−1`; Flächenanteil stimmt.
- **Wollfäden:** Ruhelage folgt dem Feld; `Tu = 0` und anliegend ergibt Ausschlag 0; Ausschlag bleibt unter 55°; sehr kleine Schubspannung lässt den Faden hängen; gleiche Zeit ergibt dieselbe Lage; Mindestabstand der Fußpunkte eingehalten.
- **Renderer:** Golden-Images `surface_lines.png` (Kugel, Zeit fest) und `tufts.png`; zwei Zeitwerte ergeben verschiedene Bilder, gleicher Zeitwert dasselbe; bestehende Golden-Images unverändert.
- **Befehle (`tests/api/view.rs` oder neue Datei):** `view.set_surface_flow` speichert, erneutes Öffnen liefert dieselben Einstellungen; Migration der Vorversion; Vorlagen `oil_film` und `tufts` erzeugen gültige Szenen.
- **Vitest:** Schalter und Regler rufen `view.set_surface_flow`; der Hinweis zum Zappeln und der Hinweis "genähert" werden angezeigt.
- **Leistung (ignoriert, Ergebnis in den Notizen):** Rechenzeit der Linien am Windkanal-Beispiel; Bildzeit mit Linien und 4000 Fäden.

## Akzeptanzkriterien

Der Task gilt erst als fertig, wenn alle Punkte erfüllt und durch Tests oder einen konkreten Befehl belegt sind:

- [x] Am Windkanal-Beispiel laufen Linien mit Lichtpunkten über die Modelloberfläche; an der Rückseite ist die Ablösung erkennbar (Nachweis-Bild)
- [x] Die Linien sind gleichmäßig verteilt und folgen der Oberfläche über Kanten hinweg
- [x] Wollfäden liegen vorn an und zappeln oder hängen im abgelösten Gebiet; der Hinweis zur Veranschaulichung ist sichtbar
- [x] Die Karte "anliegend / Rückströmung" und die Kennzahl dazu stimmen am Testfall
- [x] Beides funktioniert am Rohrverteiler (Linien an den Innenwänden, sichtbar mit Aufschnitt)
- [x] Ein alter Lauf ohne Vektorfeld zeigt Linien mit dem Hinweis "genähert"
- [x] Vorlagen "Ölfilm" und "Wollfäden" stehen in der Galerie; Bild und Video lassen sich exportieren und sind wiederholbar

## Offene Fragen / Annahmen

- Annahme: Das Vorzeichen der Wandschubspannung ist wie oben beschrieben; der `runtime_`-Test entscheidet. Fällt er anders aus, die Richtung umdrehen und das in `docs/architecture.md` festhalten.
- Annahme: Die Oberfläche aus der Auswertung ist fein genug für glatte Linien. Bei sehr groben Wandnetzen wirken die Linien eckig; das ist dann ein Hinweis auf das Netz, kein Fehler dieser Darstellung.
- Offen: ob Linien als ein Pixel breite Linien reichen oder als Bänder gezeichnet werden müssen. Am Referenzrechner ansehen und entscheiden.
- Offen: Innenwände sieht man nur mit Aufschnitt (Task_27). Prüfen, ob der Beschnitt auch Linien und Fäden erfasst; wenn nicht, hier ergänzen.

## Notizen zu diesem Task

Taskspezifische Informationen während der Arbeit hier festhalten. Taskübergreifendes gehört in `Roadmap/README.md`.

- **Nachweise (live, eigene App-Instanz, 2026-11-21):** Windkanal-Beispiel mit Vorlage "Ölfilm": 114 Linien laufen über das dunkle Modell und ziehen sich an der Rückseite zusammen (Ablösung); Vorlage "Wollfäden": Fäden über das ganze Modell, zwei Aufnahmen kurz nacheinander unterscheiden sich (Bewegung). Rohrverteiler mit "Ölfilm": Aufschnitt, Linien an den Innenwänden, Hinweis "Genähert" sichtbar. Beide Beispiel-Läufe stammen von vor Task_33, haben also keinen Vektor: die Bilder zeigen den Ersatzweg. Bilder im Scratchpad (`oil1.png`, `tufts1.png`, `tufts-t2.png`, `oil-manifold.png`).
- **Vorzeichen:** `runtime_minus_wall_shear_stress_points_downstream_in_a_pipe` (echter Lauf, 162 s): −wss zeigt auf 100 % der Wandfläche stromab. Die Annahme stimmt.
- **Leistung:** Linien am Windkanal-Beispiel (Ersatzweg, Debug-Build) 135 ms; Ergebnis wird je Lauf, Einstellung und Schnittebene in der Sitzung gehalten.
- **Abweichungen:** Linien als ein Pixel breite Linien (Bänder nicht nötig, am Referenzrechner gut sichtbar). Die Fluidseite der Oberfläche wird aus dem Raster bestimmt (Stimmen an Stichproben-Dreiecken), weil importierte Modelle und OpenFOAM-Oberflächen entgegengesetzte Normalen haben. Das Modell wird auch für Wollfäden dunkel (Kontrast). Der Schnitt (Task_27) erfasst Linien und Fäden über einen CPU-Filter beim Aufbau.

## Abschluss-Checkliste

Reihenfolge und Details stehen in `AGENTS.md`, Abschnitt "Abschluss- und Archivierungs-Workflow".

- [x] Alle Akzeptanzkriterien erfüllt, Tests liegen permanent in `tests/` und laufen grün
- [x] Abschnitt "Erkenntnisse für Folge-Agenten" unten ausgefüllt
- [x] Folge-Tasks und `Roadmap/README.md` geprüft und bei Bedarf angepasst
- [x] Datei ohne `RUN_` nach `Roadmap/Archiv/` verschoben

## Erkenntnisse für Folge-Agenten


Vor der Archivierung vom Agenten auszufüllen. Schreibe so, dass ein Agent, der diesen Task nie gesehen hat, davon bei späteren Tasks profitiert:

- **Umgesetzt (und Abweichungen vom Plan):** Cache mit `wss_x/y/z` (Welt, Pa), aus der Feldliste gefiltert; `load_surface` leitet die Karte `attached` ab, `results.list_fields`/`results.probe` nutzen `meta_with_derived`; Kennzahl `reverse_flow_area` in `results.summary` (nur wenn der Cache die Karte hat, der Snapshot bleibt unverändert). `foamcraft_sim::surface_flow` (Netz mit Nachbarn, Tangentialfeld, Kanten-Klappen, gleichmäßige Linien, Wollfäden, Zappeln), `foamcraft_render::flow` (eigene Pipelines, `flow.wgsl`, `GpuScene.time`, Globals mit `time`), `foamcraft_api::surface_flow` (Geometrie, Ersatzweg, Befehle `view.set_surface_flow`, `view.surface_flow_off`, `view.surface_flow_info`), `ProjectScene.flow` und `ProjectScene::upload`, Schema 15, Vorlagen `oil_film`/`tufts` (auch Innenströmung, mit Aufschnitt), MCP `view_set_surface_flow`, `SurfaceFlowSection`, zweifarbige Legende für `attached`. Linien bleiben 1 Pixel breit (keine Bänder).
- **Learnings & Stolpersteine:** (1) Die Normalen der Ergebnis-Oberfläche zeigen bei OpenFOAM ins Modell, beim Test-STL nach außen: die Fluidseite aus dem Raster bestimmen, nicht annehmen. (2) Beim Verschweißen können entartete Dreiecke herausfallen; `SurfaceMesh.source` hält den Bezug zu den Feldern je Dreieck. (3) Die PDB-Dateien der Testprogramme in `target/agent` wachsen auf über 650 MB; dann scheitert der Linker mit LNK1201. Abhilfe: `find target/agent/debug/deps -name "*.pdb" -size +300M -delete` vor einem vollen Lauf. (4) `cargo test --workspace` bricht ohne `--no-fail-fast` nach dem ersten roten Testprogramm ab und verschweigt den Rest.
- **Wichtige Entscheidungen und ihre Gründe:** Eigene Pipelines statt Änderung von `shader.wgsl`: alle bisherigen Golden-Images bleiben gültig. Die Bewegung ist eine reine Funktion der Zeit (Lichtpunkte aus Laufzeit und Phase, Zappeln aus Amplitude, Frequenz, Phasen): App, Bild und Video zeigen bei gleicher Zeit dasselbe. Die Karte `attached` wird beim Laden abgeleitet statt beim Auswerten gespeichert: so gilt sie für jeden Cache mit Vektor, auch für Testdaten.
- **Auswirkungen auf Folge-Tasks:** Task_35 nutzt `GpuScene.time`, `FlowVertex` und die Flow-Pipelines (Leuchtspuren, Bänder). Task_34 kann `SurfaceMesh`/`TangentField` für Ablagerungen auf der Oberfläche nutzen. Die Beispiel-Läufe im Scratchpad haben noch keinen Vektor; ein neuer Lauf liefert ihn automatisch.
- **Hinweise zu Tests und Befehlen:** `cargo test -p foamcraft-sim --test surface_flow`, `cargo test -p foamcraft-render --test flow` (Golden `surface_lines.png`, `tufts.png`), `cargo test -p foamcraft-api --test surface_flow`, `npx vitest run tests/app/surface-flow-section.test.tsx`; Vorzeichen: `cargo test -p foamcraft-api --test internal_runtime runtime_minus_wall -- --ignored --nocapture`.
