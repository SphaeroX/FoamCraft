# FoamCraft – Roadmap

Projektübergreifendes Gedächtnis. Agenten lesen diese Datei vor jedem Task und aktualisieren sie, wenn sich etwas ergibt, das über einen einzelnen Task hinausgeht. Regeln für die Arbeit: [AGENTS.md](AGENTS.md).

## Projektziel

FoamCraft ist eine kostenlose, unabhängige Open-Source-Desktop-App (GPL-3.0-or-later), mit der auch Laien Strömungssimulationen mit OpenFOAM durchführen. Der Weg ist immer derselbe: Datei laden, Windkanal wählen oder Öffnungen anklicken, rechnen, ansehen, exportieren. Niemand muss dafür ein OpenFOAM-Dictionary anfassen.

**Was die Software tut:** Sie lädt 3D-Modelle (STL, OBJ, 3MF, STEP, IGES), richtet Außenströmung im Windkanal oder Innenströmung mit angeklickten Ein- und Auslässen ein, vernetzt und rechnet automatisch mit OpenFOAM in einer verwalteten Laufzeitumgebung und erkennt selbst, wann das Ergebnis stabil ist. Ergebnisse zeigt sie in einer GPU-gerenderten 3D-Ansicht mit Schnitten, Isoflächen und animierten Partikeln, dazu Kennzahlen mit Erklärung. Bilder, Videos und Berichte (PDF, HTML) lassen sich exportieren. Ein MCP-Server gibt KI-Agenten dieselben Möglichkeiten wie der Oberfläche. Erweiterungen: zeitabhängige Rechnung, Wärme und Auftrieb, rotierende und poröse Teile, freie Wasseroberfläche, GPU-Linearlöser und ein eigener GPU-Löser für die Sofort-Vorschau.

**Zielgruppe:** Maker, Studierende und Ingenieure ohne CFD-Erfahrung auf Windows (zuerst), Linux und macOS; außerdem KI-Agenten über MCP. Die Oberfläche gibt es in Englisch und Deutsch.

**Nicht-Ziele:** CAD-Bearbeitung: Geometrie wird nur geladen, ausgerichtet, skaliert und an Öffnungen verschlossen. Wünsche des Nutzers für später, für die es in dieser Roadmap keine Tasks gibt: Cloud-Dienst mit Konten, automatische Formoptimierung, Überschall und Verbrennung. In einzelnen Tasks zusätzlich ausgeklammert und dort begründet: Wärmeleitung in Festkörpern und Strahlung (Task_38), echte Drehung mit gleitenden Netzen (Task_39), frei bewegliche Körper im Wasser (Task_41).

## Aktueller Status

Stand: 2026-11-21 – Task_01 bis Task_21 erledigt (Meilensteine 1 und 2 gemeldet). Der Nutzer hat die erste Version durchgesehen und Rückmeldung zur Bedienung gegeben; daraus sind Task_22 bis Task_31 entstanden (Phase 4b), die vor dem Release bearbeitet werden. Die bis dahin offenen Tasks 22 bis 33 wurden dafür um zehn nach hinten nummeriert (jetzt 32 bis 43; alle Verweise sind angepasst). Task_22 und Task_23 sind erledigt. Task_24 bis Task_31 arbeitet ein anderer Agent ab; jeder dieser Tasks hat dafür einen Abschnitt "Umsetzungshinweise" mit den betroffenen Codestellen, Stolpersteinen und den bestehenden Tests, die den Weg vorgeben. Der nächste Task ist Task_24. Task_32 (Installer, Veröffentlichung) wartet auf die Zustimmung des Nutzers zum Veröffentlichen.

| Task | Titel | Status |
|---|---|---|
| 01 | Durchstich: App mit 3D-Fenster | erledigt |
| 02 | Runtime-Image und Docker-Runner | erledigt |
| 03 | Job-Pipeline mit Referenzfall | erledigt |
| 04 | Projektmodell und Befehls-API | erledigt |
| 05 | Import von STL, OBJ und 3MF mit Diagnose | erledigt |
| 06 | Flächen erkennen und anklicken | erledigt |
| 07 | Import von STEP und IGES | erledigt |
| 08 | Windkanal einrichten | erledigt |
| 09 | Automatische Vernetzung für die Außenströmung | erledigt |
| 10 | Stationäre Außenströmung mit Kräften | erledigt |
| 11 | Lauf-Monitor in der App | erledigt |
| 12 | Öffnungen erkennen und verschließen | erledigt |
| 13 | Innenströmung rechnen | erledigt |
| 14 | Oberflächenfelder und Volumenraster | erledigt |
| 15 | Schnitte und Isoflächen | erledigt |
| 16 | Stromlinien und Partikel-Animation | erledigt |
| 17 | Kennzahlen und Laufvergleich | erledigt |
| 18 | MCP-Server | erledigt |
| 19 | Bild- und Videoexport | erledigt |
| 20 | Report als PDF und HTML | erledigt |
| 21 | Einrichtungsassistent und Beispielprojekte | erledigt |
| 22 | Beispielprojekte und neue Projekte ohne Ordnerwahl | erledigt |
| 23 | Rechnen in einem Schritt: Vernetzung, Fortschritt, Konsole, Abbruch | erledigt |
| 24 | Ansichtswürfel und Standardansichten | offen |
| 25 | Modell in der Ansicht verschieben und drehen | offen |
| 26 | Anströmung in der Ansicht einstellen | offen |
| 27 | Schnitte in der Ansicht ziehen, Rauch aufschneiden | offen |
| 28 | Strömung verstehen: Ansichtsvorlagen und Vorführmodus | offen |
| 29 | Oberfläche als Assistent mit neuem Erscheinungsbild | offen |
| 30 | Mehrere Szenarien je Geometrie | offen |
| 31 | Lokale Strömungsquellen: Düse und Lüfter | offen |
| 32 | Installer und Release-Pipeline | offen |
| 33 | Leistungsmessung und CPU-Tuning | offen |
| 34 | GPU-Linearlöser (experimentell) | offen |
| 35 | Lösungsadaptive Netzverfeinerung | offen |
| 36 | WSL2-Runner ohne Docker Desktop | offen |
| 37 | Zeitabhängige Rechnung mit Zeitleiste | offen |
| 38 | Wärme und Auftrieb | offen |
| 39 | Rotierende Teile und bewegter Boden | offen |
| 40 | Poröse Bauteile | offen |
| 41 | Freie Wasseroberfläche | offen |
| 42 | GPU-Strömungslöser als Kern | offen |
| 43 | Sofort-Vorschau in der App | offen |

Status ist `offen`, `läuft` (Datei trägt das Präfix `RUN_`) oder `erledigt` (Datei liegt in `Archiv/`). Die Tabelle wird beim Archivieren eines Tasks aktualisiert.

## Taskübergreifende Entwicklungsinfos

Globale Informationen, Architekturentscheidungen und Wissen, das für das gesamte Projekt gilt. Wird laufend ergänzt; jeden Eintrag mit dem Task kennzeichnen, aus dem er stammt.

Alle Einträge dieses Abschnitts stammen, soweit nicht anders vermerkt, aus dem Interview vom 2026-11-20/21.

### Phasen und Meilensteine

| Phase | Tasks | Inhalt | Meilenstein am Ende |
|---|---|---|---|
| 1 | 01–04 | Fundament: App mit 3D-Fenster, Runtime, Job-Pipeline, Projektmodell und Befehls-API | – |
| 2 | 05–13 | Geometrie und beide Grundfälle (Windkanal, Innenströmung) | M1: beide Grundfälle von der Datei bis zur Kennzahl (Task_13) |
| 3 | 14–20 | Ergebnisse, Animation, MCP, Video, Report | M2: Ergebnisse, MCP und Export vollständig (Task_20) |
| 4 | 21, 32 | Einrichtungsassistent, Installer, Release | M3: erste öffentliche Version (Task_32) |
| 4b | 22–31 | Bedienung nach der ersten Durchsicht des Nutzers: Beispiele mit einem Klick, Rechnen in einem Schritt, Ansichtswürfel, Griffe in der Ansicht, Ansichtsvorlagen, Assistent und neues Erscheinungsbild, mehrere Szenarien, lokale Strömungsquellen | Stand nach Task_29 und nach Task_31 dem Nutzer melden |
| 5 | 33–36 | Leistung, GPU-Linearlöser, adaptive Verfeinerung, WSL-Runner | – |
| 6 | 37–41 | Physik: zeitabhängig, Wärme, rotierend, porös, freie Wasseroberfläche | – |
| 7 | 42–43 | Eigener GPU-Löser und Sofort-Vorschau | – |

An den Meilensteinen meldet der Agent dem Nutzer den Stand zur Abnahme (steht im jeweiligen Task).

### Architektur in Kürze

Maßgeblich ist `docs/architecture.md`; hier nur das Nötigste für die Einordnung:

- Kern in Rust (ein Cargo-Workspace), kein Python. Oberfläche mit Tauri 2, TypeScript und React in `app/`.
- Eigener Renderer in Rust auf wgpu, der unter der transparenten Web-View zeichnet und auch offscreen rendert. Derselbe Renderer bedient Ansicht, MCP-Bilder, Videoexport und die Sofort-Vorschau.
- OpenFOAM (OpenCFD-Linie, gepinnt) läuft im Container hinter einer Runner-Schnittstelle. Ein Linux-Build des Kerns (Agent) im Container ist der einzige Zugriff auf Falldateien.
- Jede Aktion ist ein Befehl mit Schema. Oberfläche, CLI und MCP sind gleichberechtigte Nutzer der Befehls-API. Alles muss ohne Fenster funktionieren.
- Auswertung ohne VTK: OpenFOAM bereitet auf, Rust liest und rastert das Volumen in 3D-Texturen, die GPU berechnet Schnitte, Isoflächen und Partikel.
- Der Nutzer hat sich im Interview ausdrücklich für "alles in Rust und TypeScript" entschieden und gegen eine Python-Engine. Diese Entscheidung nicht ohne Rückfrage umkehren.

### Offene Entscheidungen und wo sie fallen

| Entscheidung | Task |
|---|---|
| Web-View über wgpu oder Ausweichweg | entschieden in Task_01: wgpu unter transparenter WebView2 |
| Genaue OpenFOAM-Version; Datei-Zugriff über Volume mit Agent oder Bind-Mount | entschieden in Task_02: v2606, Volume mit Agent |
| Technik für STEP und IGES | entschieden in Task_07: Gmsh im Runtime-Container |
| Brücke zwischen CLI/MCP und laufender App | Task_18 (entschieden: Loopback-TCP mit Token) |
| Video-Encoder und Containerformat | Task_19 (entschieden: Motion-JPEG in AVI, eigener Encoder) |
| Beitragspolitik (DCO oder keine), Signatur-Zertifikate, Repository und Registry | Task_32 |
| GPU-Plugin (Ginkgo/OGL oder AmgX) und Lizenzfrage der CUDA-Bestandteile | Task_34 |
| Auslieferung des Root-Dateisystems für WSL | Task_36 |

### Externe Voraussetzungen

- Docker Desktop mit WSL2-Backend muss für alle `runtime_`-Tests laufen (ab Task_02).
- Für Task_32 entscheidet der Nutzer über öffentliches Repository, Registry und Signatur. Veröffentlicht wird nur mit seiner ausdrücklichen Zustimmung.
- Testgeometrie wird selbst erzeugt (CC0). STEP-Testdateien kann der Nutzer mit SolidCraft erstellen.

### Lizenz und Unabhängigkeit

- Projektlizenz GPL-3.0-or-later; Abhängigkeiten nur von der Erlaubnisliste in `docs/licensing.md`.
- FoamCraft ist unabhängig. Keine Namen, Texte, Layouts, Grafiken oder Beispielmodelle anderer CFD-Produkte übernehmen. "OpenFOAM" nur beschreibend verwenden, immer mit dem Markenhinweis.
- Code aus FluidX3D oder anderen Quellen mit Nutzungsbeschränkung darf nicht verwendet werden (wichtig für Task_42).

### Arbeitsweise für Phase 4b (Task_24 bis Task_31)

Kurzfassung für den Agenten, der diese Tasks abarbeitet. Die Regeln in `AGENTS.md` (hier und im Hauptverzeichnis) gelten unverändert; das hier sind die Erfahrungen aus Task_22 und Task_23.

1. **Ein Task nach dem anderen, in kleinen grünen Schritten.** Je Schritt: Test schreiben, rot sehen, umsetzen, grün sehen. Bei den größeren Tasks (25, 29, 30, 31) lohnt die Reihenfolge reine Logik → Befehl → Renderer → Oberfläche; die "Umsetzungshinweise" im Task nennen sie.
2. **Vor jedem Umbau den zugehörigen Test lesen.** Bestehende Tests werden nicht geändert. Die Oberflächen-Tests finden Elemente über Rolle und englischen Namen: Namen und Rollen behalten, Neues als optionale Eigenschaft oder daneben ergänzen. Steht ein alter Test wirklich im Weg: anhalten, Grund nennen, den Nutzer fragen (in Task_26 ist so eine Stelle schon benannt).
3. **Quelltext mit den Datei-Werkzeugen schreiben**, nicht über Shell-Heredocs oder `node -e`: Backslashes und Anführungszeichen gehen dort kaputt. Übersetzungen mit `node tools/i18n-merge.mjs <datei.json>` in beide Sprachdateien schreiben.
4. **Testausgaben filtern.** `cargo test --workspace 2>&1 | cut -c1-160 | grep -E "^test result: FAILED|\.\.\. FAILED|^error"` zeigt nur Fehlschläge; leer heißt grün (dazu die Zahl der `test result: ok` zählen). Ungefiltert druckt ein Linkerfehler eine Zeile mit zehntausenden Zeichen. Bei `LNK1201` die genannte `.pdb`-Datei in `target/debug/deps/` löschen und neu bauen.
5. **Nichts im Benutzerordner hinterlassen.** Tests, die Projekte ohne Pfad anlegen, setzen `session.set_projects_dir(...)`. Die App zum Prüfen mit `FOAMCRAFT_DATA_DIR` und `FOAMCRAFT_PROJECTS_DIR` auf einen temporären Ordner starten.
6. **Den Nutzer nicht stören.** Er arbeitet oft am selben Rechner. Fenster nur mit `tools/windows/app-print.ps1` aufnehmen (kein Fokus, keine Maus). `app-ui.ps1` und `app-click.ps1` bewegen die echte Maus: nur nutzen, wenn sicher niemand am Rechner ist; sonst den Nutzer um den einen Klick bitten. Fremde Prozesse nicht beenden; ist Port 1420 belegt, die App mit `tauri dev --config <JSON mit build.devUrl und build.beforeDevCommand auf einem freien Port>` starten.
7. **Docker:** nur selbst angelegte Container und Volumes entfernen. Echte Läufe sind langsam (das Windkanal-Beispiel braucht rund acht Minuten): im Hintergrund starten und währenddessen weiterarbeiten.
8. **Abschluss wie im Regelwerk:** Tests, `cargo fmt --all --check`, Clippy, `npm run lint`, `npm run typecheck`, `npm run format:check`, `cargo xtask license-check`; dann Erkenntnisse in die Task-Datei, Folge-Tasks und diese README anpassen, archivieren. Was nicht geprüft werden konnte, steht ausdrücklich in den Notizen des Tasks.
9. **Nach Task_29 und nach Task_31** dem Nutzer den Stand melden (Phase 4b). Reicht die Kraft für einen Task nicht, dem Nutzer eine Teilung vorschlagen statt still zu kürzen.

### Referenzrechner

Windows 11, Intel i9-13900HX (8 P-Kerne, 16 E-Kerne), 64 GB Arbeitsspeicher, NVIDIA RTX 4080 Laptop mit 12 GB. Leistungs- und Speicherangaben in den Tasks beziehen sich darauf.

### Erkenntnisse aus abgeschlossenen Tasks

Hier trägt jeder Agent beim Archivieren taskübergreifende Erkenntnisse ein, mit Angabe des Tasks.

- **Task_01 – Struktur:** Crates unter `crates/` (`foamcraft-core`, `foamcraft-render`, `foamcraft-cli`, `foamcraft-xtask`), Tauri-Crate `foamcraft-app` in `app/src-tauri`, Frontend in `app/src`. Tests liegen in `tests/<bereich>/` und werden per `[[test]] path = "../../tests/..."` im Crate verdrahtet. Testdaten unter `tests/data/`.
- **Task_01 – Befehle:** App starten `npm run app:dev`; Lizenzprüfung `cargo xtask license-check` (Policy `license-policy.json`, Freigaben nur durch den Nutzer); Typprüfung `npm run typecheck`.
- **Task_01 – Renderer:** Welt Z-oben in Metern; Farben als Anzeigewerte in Nicht-sRGB-Ziele, Offscreen-Bild und Ansicht sind pixelgleich. `Renderer::new_headless()` liefert `None` ohne GPU. Die Szene wird je Bild neu hochgeladen; für große Netze cachen.
- **Task_01 – Übersetzungen:** `app/src/i18n/en.json` und `de.json`; `tests/app/i18n.test.ts` schlägt fehl, wenn ein Schlüssel in einer Sprache fehlt.
- **Task_01 – App prüfen ohne Nutzer:** Bildschirmfoto und Mausbewegung per PowerShell-Skript (`CopyFromScreen`, `SetCursorPos`, `mouse_event`) auf das Fenster von `foamcraft-app.exe`; der Computer-Use-Freigabedialog wird ohne den Nutzer nicht bestätigt. Nachweise liegen in `Roadmap/Nachweise/`.
- **Task_02 – Laufzeit:** OpenFOAM v2606 im Image `foamcraft-runtime:<version>` (`runtime/Dockerfile`, bauen mit `cargo run -p foamcraft-cli -- runtime build`). Docker über die Kommandozeile (`foamcraft_runtime::DockerRunner`), Tests ohne Docker mit `fake::FakeEngine`. Fälle auf Docker-Volume unter `/work`, Agent `foamcraft-agent` im Container, Antworten als JSON auf stdout. OpenFOAM-Befehle immer über `exec_openfoam` (Login-Shell).
- **Task_02 – Fehlercodes:** Katalog `crates/foamcraft-core/data/errors.json` (en/de, Text und Abhilfe); `tests/core/errors.rs` erzwingt beide Sprachen. Neue Codes dort eintragen.
- **Task_02 – Referenzrechner-Zustand:** Docker Desktop läuft im Hyper-V-Modus (Kernel linuxkit), ~1,9 GiB Speicher, keine GPU-Durchreichung. Für GPU-Tasks (Task_34) muss der Nutzer das WSL2-Backend einschalten. Bind-Mounts neuer Host-Ordner lösen einen Docker-Freigabe-Dialog aus, der unbeaufsichtigte Läufe blockiert.
- **Task_03 – Jobs:** `foamcraft_core::job` (Stufen, Zustände, `run_job`), Ausführung im Container über Agent `run-stage`/`tail`, Job-Daten unter `%LOCALAPPDATA%FoamCraftjobs<id>` (`FOAMCRAFT_DATA_DIR` überschreibt). Neue Fälle über `foamcraft_runtime::jobs::prepare_job` und Vorlagen in `crates/foamcraft-core/templates/`. Nach jeder Agent-Änderung Image neu bauen.
- **Task_03 – Logs:** Parser `foamcraft_core::log` gegen echte v2606-Logs in `tests/data/logs/`; Fortschritt nur aus der Lösungsstufe. Fehlercodes `run.foam_fatal_error`, `run.diverged`, `run.stage_failed`, `run.cancelled`, `run.lost`, `mesh.check_failed`, `job.not_found`, `job.storage_failed`.
- **Task_04 – Befehls-API:** Jede Aktion ist ein Befehl im `foamcraft_core::command::Registry` (Kern) bzw. in `foamcraft-api` (wenn Docker nötig). Parameter-Structs mit `Deserialize + JsonSchema + deny_unknown_fields`; `undoable` + `example` angeben (Test prüft Undo/Redo für alle). App ruft Befehle nur über `app/src/api.ts` (`tauriApi.call`) und hört auf `foamcraft://event`. Regeln in `docs/conventions.md`.
- **Task_04 – Projekt:** Ordner mit `foamcraft.json` (`SCHEMA_VERSION` 1) + `geometry/`, `runs/`, `cache/`. Schema-Änderungen: Version erhöhen und Schritt in `project::migrate`. SI-Typen in `foamcraft_core::units`.
- **Task_04 – App-Bedienung ohne Nutzer:** `tools/windows/app-shot.ps1` (Fensterfoto), `app-ui.ps1` (Klicks relativ zum Fenster, Tippen per SendKeys), `app-full.ps1` (Bildschirmfoto halbe Größe), `app-click.ps1` (absoluter Klick).
- **Task_05 – Geometrie:** Crate `foamcraft-geometry` (Leser STL/OBJ/3MF, `diagnose`, `Placement`, Befehle `geometry.*`). Manifest jetzt Schema 2 mit `geometry`-Liste; Dateien liegen als Kopie in `geometry/model-N.<ext>`. Weltkoordinaten (Meter, Z oben) über `foamcraft_geometry::commands::world_meshes`. Renderer: `Renderer::upload` einmal je Änderung, App ruft `viewport_sync` nach `project.changed`.
- **Task_05 – Shell-Hinweis:** Längere Node-Skripte (Doku-Änderungen) als Datei im Scratchpad schreiben und mit `node <datei>` ausführen; lange Heredocs scheitern in der Git-Bash gelegentlich.
- **Task_06 – Flächen:** Flächenschlüssel `model-1:face-3` (deterministische Zerlegung nach Knickwinkel), Teile `model-1:part-1`; Namen und Gruppen im Manifest (Schema 3). Auswahl ist Sitzungszustand (`selection.changed`). Gruppen sind der Weg, Einlässe/Auslässe zu markieren.
- **Task_06 – Linker:** Bei LNK1201 die genannte veraltete PDB-Datei in `target/debug/deps/` löschen und neu bauen.
- **Task_07 – CAD:** STEP/IGES werden im Runtime-Container mit Gmsh in eine .msh umgewandelt (`foamcraft_api::DockerCad`); jede CAD-Fläche ist eine Fläche, CAD-Volumen sind Teile. Manifest Schema 4 (`source` = Original, `file` = Netz). Ohne Docker: `geometry.cad_unavailable`.
- **Task_08 – Szenario:** Crate `foamcraft-sim` mit Medien, Stirnfläche, Tunnelregel (5 L vor, 15 L hinter, 5 L seitlich/oben) und Zellbudget (Entwurf/Standard/Fein: 0,5/2/8 Mio. Zellen, ≤ 60 % Speicher bei 1,5 GB je Mio. Zellen). Manifest Schema 5 mit `scenario`. Regeln stehen in `docs/architecture.md`.
- **Task_09 – Vernetzung:** `mesh.generate` plant (`foamcraft_sim::mesh`), schreibt den Fall im Strömungssystem (Modelle um −Gierwinkel gedreht, Strömung +X), vernetzt per Job bis zur Prüfstufe und nutzt die Ausweichleiter. Ränder: `inlet`, `outlet`, `sides`, `ground`, Modell-Patches `model_<gruppe>` in der Patch-Gruppe `model`. Netz-Job steht in `manifest.mesh.job_id` (Schema 6). OpenFOAM-Regex in `wordRes` immer in Anführungszeichen.
- **Task_10 – Rechnung:** `run.start` rechnet Windkanal-Projekte (Netz bei Bedarf zuerst), stoppt über die Datei `<case>/stop` (abort-Funktionsobjekt) bei stabilen/schwankenden Beiwerten und speichert `WindTunnelResult` in `<projekt>/results/<job>.json` (`run.result`). Validierung: freier Würfel C_D 1,046 vs. 1,05. Grenzen stumpfer Körper: `docs/architecture.md`.
- **Task_11 – Lauf-Monitor:** Schritt "Rechnen" (`RunPanel`) folgt den Ereignissen `run.*`/`mesh.stage`; Texte zu Codes nur über `error.describe`; `run.active` + `run.attach` verbinden nach einem App-Neustart wieder. Neue Codes immer in `errors.json` (en/de + Vorschlag) – `tests/api/error_catalog.rs` prüft das.
- **Task_12 – Öffnungen:** Deckflächen (`manifest.caps`, Schlüssel `model-1:cap-N`) und Rollen (`manifest.boundary_roles`, inlet/outlet/wall) liegen im Manifest (Schema 7); `geometry.fluid_check` liefert Dichtheit und Fluidpunkt. Neue rückgängig machbare `geometry.*`-Befehle müssen mit ihrem Beispiel am Würfel laufen (bestehender Test).
- **Task_13 – Innenströmung:** Szenario `internal_flow` (Einlass: Geschwindigkeit/Volumen-/Massenstrom/Gesamtdruck, Auslass: Druck). Gemeinsame Lauf-Logik für beide Szenarien in `crates/foamcraft-api/src/solve.rs` (`Setup.kind`, `Watch`); Ergebnisse als JSON mit `kind` (Innenströmung) bzw. `coefficients` (Windkanal). Validierung Rohr laminar/turbulent grün.
- **Task_14 – Ergebnisfelder:** Nach jedem Lauf extrahiert `foamcraft_api::results` Wandfläche und Volumenraster in `<projekt>/cache/results/<job>/` (Leser `foamcraft_sim::foam_io`, Raster `foamcraft_sim::results::Grid`). Anzeige über `view.set_surface_field` (Manifest Schema 8).
- **Task_15 – Schnitte/Isoflächen:** Szene (`view.slices`, `view.isosurfaces`, `view.clip`) im Manifest (Schema 9); Szenenaufbau liegt in `foamcraft_api::scene` (App und `view.render` nutzen ihn).
- **Task_16 – Partikel:** GPU-Advektion (`foamcraft-render` `particles.wgsl`) mit CPU-Referenz (`foamcraft_sim::particles`), fester Startwert → gleiches Bild; Einstellungen in `view.particles` (Schema 10); die App animiert mit einem 60-Hz-Takt.
- **Task_17 – Kennzahlen:** `results.summary` (stabiles Schema 1, Snapshot in `tests/data/snapshots/`, Texte aus dem Katalog `figure.<id>`/`reliability.*`), `results.compare`, `results.export`, `runs.*`; Vergleich in der Ansicht über `view.set_compare` (split/difference). Manifest Schema 11 (`run_names`, `view.compare`). `tests/data/snapshots/` ist von Prettier ausgenommen.
- **Task_18 – MCP:** Crate `foamcraft-mcp`, `foamcraft mcp --root <Ordner> [--app]` (stdio). Neue Werkzeuge in `CURATED` (`crates/foamcraft-mcp/src/lib.rs`) eintragen; Pfade nur über Schlüssel wie `path` (Sandbox). Brücke zur App: `foamcraft_api::bridge` (Loopback-TCP + Token). `run.status` hat `busy` (Auswertung läuft noch). Anleitung: `docs/mcp.md`.
- **Task_19 – Export:** `export.image` (PNG bis 8K, transparent, Legende), `export.video` (Motion-JPEG/AVI, eigener 4:2:0-Encoder, Hintergrund-Job mit `export.status`/`export.cancel`), `export.preview`; Kamerafahrten in `foamcraft_render::camera_path`. Videos mit Windows prüfen: `tools/windows/video-check.ps1`.
- **Task_20 – Report:** Crate `foamcraft-report` (Typst als Bibliothek, Schrift Libertinus Serif aus `assets/fonts`, HTML als eine Datei); Befehl `export.report`, MCP `export_report`. Neue Kennzahlen brauchen eine Kurzbezeichnung `figures.label.<id>` in `app/src/i18n/*.json` (Oberfläche und Bericht). PDFs ansehen: `tools/windows/pdf-page.ps1`.
- **Task_21 – Einrichtung:** `system.check`/`system.diagnostics`/`system.finish_setup`/`system.provide_image` (`foamcraft_api::system`, Hilfen in `crates/foamcraft-api/data/system_help.json`), Beispielprojekte in `samples/` (CC0) über `samples.create`. Jeder Katalogeintrag braucht auch einen Vorschlag (`fix`), selbst bei Erfolgsmeldungen.
- **Task_22 – Projektordner:** Projekte und Beispiele ohne Pfad entstehen in `~/FoamCraft` (`Session::projects_dir`, `FOAMCRAFT_PROJECTS_DIR`), mit freiem Namen ("Name 2"). Tests setzen `session.set_projects_dir(...)`. Der MCP-Server ersetzt einen fehlenden Pfad durch einen Ordner im ersten Root. `cargo test --workspace` immer gefiltert ausgeben (siehe Task_22).
- **Task_23 – Hintergrundarbeit:** Ein Befehl hält die Sitzung gesperrt, solange er läuft. Was länger dauert (Container), läuft in einem Thread mit einer Kopie des Projekts und reicht Manifest-Änderungen über `session.updates()` ein; `Registry::execute` wendet sie vor jedem Befehl an. `run.start` ist so gebaut (`Pipeline` in `crates/foamcraft-api/src/run.rs`): Lauf-Kennung sofort, dann Vernetzung (nur wenn `mesh::mesh_is_current` nein sagt), Löser, Auswertung. Ereignisse `mesh.stage`, `mesh.progress`, `mesh.ready` tragen `run_id`.
- **Task_23 – Oberfläche:** Der Zustand eines Laufs liegt im `RunMonitor` (`app/src/run-monitor.ts`, einer je Projekt, in `App` erzeugt), nicht in Komponenten. Neue Anzeigen zum Lauf lesen ihn mit `useRunMonitor`.
- **Task_23 – App prüfen, ohne den Nutzer zu stören:** `tools/windows/app-print.ps1` nimmt das Fenster ohne Fokuswechsel auf. Die Klick-Skripte (`app-ui.ps1`, `app-click.ps1`) bewegen die echte Maus und holen das Fenster nach vorn: nur verwenden, wenn der Nutzer nicht am Rechner arbeitet. App mit eigenem Zustand starten: `FOAMCRAFT_DATA_DIR` und `FOAMCRAFT_PROJECTS_DIR` setzen; ist Port 1420 belegt, `tauri dev --config <JSON mit build.devUrl und build.beforeDevCommand auf einem anderen Port>`.

## Auffälligkeiten & Details

Dinge, die global noch zu beachten sind: Besonderheiten, bekannte Einschränkungen, kleinere Details. Wird laufend ergänzt.

- **Docker:** Docker Desktop läuft (vom Nutzer am 2026-11-21 gestartet), aber im Hyper-V-Modus mit ~1,9 GiB Speicher; siehe Task_02-Erkenntnisse.
- **GPU bei OpenFOAM:** Es gibt keine offizielle GPU-Version, nur die Auslagerung des linearen Lösers. Der Gewinn ist begrenzt und wird in Task_34 gemessen, nicht versprochen. Der große Sprung kommt erst mit dem eigenen GPU-Löser (Task_42, Task_43).
- **Hybrid-CPU:** P- und E-Kerne verhalten sich bei OpenFOAM unterschiedlich. Bis Task_33 gilt "physische Kerne" als Standard, danach die gemessene Regel.
- **Linux und macOS:** Bis Task_32 wird nur unter Windows entwickelt und geprüft. WebGPU in System-Web-Views ist unter Linux unzuverlässig, deshalb der native Renderer.
- **Prettier ignoriert `Roadmap/`**, damit das Regelwerk und die Task-Dateien nicht umformatiert werden.
- **Validierungstests:** Toleranzen stehen mit Quelle im Test und werden nie aufgeweitet, um Grün zu erreichen. Gelingt eine Toleranz nicht, anhalten und dem Nutzer berichten.
- **Reihenfolge:** Die Tasks werden streng numerisch abgearbeitet, auch wenn ein Task laut "Voraussetzung" nur von einem früheren abhängt.
- **Bekannt, noch offen:** Nach einem Neuladen der Web-Ansicht zeigt die App die Startseite, obwohl die Sitzung ein Projekt offen hat (`App` fragt beim Laden nicht `project.info`); die Werkzeugleiste bricht bei 1280 px Breite um; die Zeitschätzung im Rechnen-Schritt ist bei wenig Container-Speicher deutlich zu niedrig. Die ersten beiden gehören zu Task_29, die Schätzung zu Task_33.
- **Docker-Volumes:** Auf dem Referenzrechner liegen rund 20 Volumes `foamcraft-job-*` aus früheren Läufen. Agenten löschen nur Volumes, die sie selbst angelegt haben; über die übrigen entscheidet der Nutzer.
- **Umnummerierung am 2026-11-21:** Die Tasks der Phase 4b (22 bis 31) wurden nach der Durchsicht des Nutzers eingefügt; die früheren Tasks 22 bis 33 heißen seitdem 32 bis 43. Nachweise und Erkenntnisse der Tasks 01 bis 21 sind davon nicht berührt. Wer in alten Notizen oder Commit-Texten "Task_22" im Sinn von "Installer" liest, meint heute Task_32.
- **Unabhängigkeit bei der Gestaltung:** Der Nutzer nennt andere Produkte als Maßstab für Einfachheit. Übernommen wird davon nichts (Layout, Texte, Grafiken); siehe Root-`AGENTS.md` und `docs/licensing.md`.
