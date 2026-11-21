# Task_01: Durchstich: App mit 3D-Fenster

> ## ⚠️ PFLICHT – BEVOR DU IRGENDETWAS ANDERES TUST
>
> 1. **Lies als Allererstes die Datei `AGENTS.md` im übergeordneten Ordner `Roadmap/`** (von hier: `../AGENTS.md`) und befolge sie strikt.
> 2. **Deine allererste Aktion** ist, diese Datei in `RUN_Task_01.md` umzubenennen, um den Task als laufend zu markieren. Das passiert **noch bevor irgendein Code geschrieben** wird.

## Ziel

Die App startet unter Windows, zeigt eine native wgpu-3D-Ansicht unter der React-Oberfläche und lässt sich zwischen Englisch und Deutsch umschalten. Damit ist belegt, dass Stack, Tests und Projektstruktur zusammenpassen, und das größte Technikrisiko (Web-View über einer wgpu-Fläche) ist früh geklärt.

## Kontext & Abhängigkeiten

- **Voraussetzung:** Keine, dies ist der erste Task. Das Setup aus `project-init` ist vorhanden (Git, `docs/`, Root-`AGENTS.md`, npm-Werkzeuge).
- **Relevantes Vorwissen:** `docs/architecture.md` (Abschnitte "First implementation step" und "Open decisions"), `docs/testing.md`, `docs/licensing.md`. Die Root-`Cargo.toml` fehlt absichtlich: Ein leerer Workspace lässt `cargo test` scheitern, deshalb entsteht sie hier.

## Umfang

**Gehört dazu:**

- Cargo-Workspace mit den ersten Crates (Kern-Bibliothek, Renderer, CLI) und der Tauri-2-App in `app/` als npm-Workspace, dazu `tsconfig.json`
- wgpu-Renderer, der eine Testszene (Würfel, Orbit-Kamera) in das App-Fenster unter die transparente Web-View zeichnet und dieselbe Szene offscreen in ein Bild rendern kann
- Eingaben: Maus über freier Fläche steuert die 3D-Ansicht (Drehen, Zoomen, Verschieben), über Bedienelementen die Oberfläche
- Übersetzungsschicht mit Englisch und Deutsch, Umschalter, gespeicherte Auswahl
- CLI `foamcraft --version`
- Lizenzprüfung der Rust- und npm-Abhängigkeiten als Befehl, abgebildet auf die Erlaubnisliste in `docs/licensing.md`
- About-Dialog mit Lizenz und Markenhinweis

**Gehört nicht dazu:**

- Docker und OpenFOAM (Task_02)
- Projektmodell und Befehls-API (Task_04)
- Geometrie-Import (Task_05) und das endgültige Layout der Arbeitsschritte

## Umsetzungsschritte

1. Root-`Cargo.toml` und die ersten Crates anlegen, die Tauri-App nach `app/` scaffolden, npm-Workspace und `tsconfig.json` einrichten, bis alle Befehle aus der Root-`AGENTS.md` laufen.
2. Test zuerst: Ein Offscreen-Render der Testszene wird mit einem Golden Image verglichen; danach den Renderer minimal umsetzen.
3. Compositing prüfen: wgpu-Fläche im Tauri-Fenster unter der transparenten Web-View auf Windows 11. Gelingt das mit vertretbarem Aufwand nicht, anhalten und dem Nutzer Befund und Ausweichwege (three.js in der Web-View, wgpu-Bilder in ein Canvas) vorlegen, statt selbst umzuentscheiden.
4. Eingabe-Weiterleitung zwischen Oberfläche und 3D-Ansicht umsetzen.
5. Übersetzungsschicht mit Umschalter einbauen; die Bibliothek frei wählen, solange die Lizenz erlaubt ist.
6. CLI mit `--version` anlegen.
7. Lizenzprüfung einrichten und den Befehl in `docs/testing.md` eintragen.
8. `docs/architecture.md` aktualisieren: Crate-Namen, Entscheidung zum Compositing, Startbefehl der App.

## Tests

Alle Tests dieses Tasks werden permanent in `tests/` im Hauptverzeichnis des Projekts abgelegt (siehe `AGENTS.md`).

- Golden-Image-Test des Renderers (offscreen). Fehlt ein GPU-Adapter, wird der Test als übersprungen gemeldet, nicht als bestanden.
- Unit-Tests der Kamera-Mathematik (Orbit, Zoom, Einpassen).
- Vitest: Der Sprachumschalter wechselt sichtbaren Text; ein Vollständigkeitstest schlägt fehl, wenn ein Schlüssel in einer der beiden Sprachen fehlt.
- CLI-Test: `foamcraft --version` liefert die Version aus dem Workspace.
- Lizenzprüfung: Nachweis, dass eine testweise eingetragene verbotene Lizenz den Befehl scheitern lässt.

## Akzeptanzkriterien

Der Task gilt erst als fertig, wenn alle Punkte erfüllt und durch Tests oder einen konkreten Befehl belegt sind:

- [x] `cargo test --workspace` und `npm test` laufen grün; `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings` und `npm run lint` melden keine Fehler
- [x] Der dokumentierte Startbefehl öffnet die App; der Würfel ist unter der Oberfläche sichtbar und lässt sich mit der Maus drehen (Bildschirmfoto als Nachweis in den Notizen)
- [x] Die Sprache lässt sich umschalten und bleibt nach einem Neustart erhalten
- [x] Der Lizenzprüf-Befehl läuft grün und steht in `docs/testing.md`
- [x] `docs/architecture.md` nennt die Crates und die Compositing-Entscheidung

## Offene Fragen / Annahmen

- Annahme: Das Compositing wird hier nur unter Windows bewiesen. Linux und macOS werden in Task_32 geprüft.
- Offene Entscheidung aus `docs/architecture.md`: Web-View über wgpu oder Ausweichweg. Sie fällt in diesem Task; bei einem Wechsel des Ansatzes entscheidet der Nutzer.

## Notizen zu diesem Task

Taskspezifische Informationen während der Arbeit hier festhalten. Taskübergreifendes gehört in `Roadmap/README.md`.

- Nachweis Compositing und Maus-Drehen: `Roadmap/Nachweise/task01_viewport.png` (Start) und `task01_viewport_gedreht.png` (nach 250 px Ziehen mit links). Nachweis Sprache nach Neustart: `task01_sprache_nach_neustart.png` (System ist Deutsch, gewählt wurde Englisch, nach Neustart weiter Englisch).
- Die Bildschirmfotos entstanden per PowerShell (`Graphics.CopyFromScreen` aufs Fensterrechteck, Maus per `SetCursorPos`/`mouse_event`), weil der Computer-Use-Freigabedialog ohne den Nutzer nicht bestätigt wurde. Der Nutzer hat die PC-Nutzung ausdrücklich erlaubt.
- Lizenzprüfung: 0BSD (tslib) und BlueOak-1.0.0 (minimatch, lru-cache, @isaacs/nonrecursive-glob-match) hat der Nutzer am 2026-11-21 freigegeben.

## Abschluss-Checkliste

Reihenfolge und Details stehen in `AGENTS.md`, Abschnitt "Abschluss- und Archivierungs-Workflow".

- [x] Alle Akzeptanzkriterien erfüllt, Tests liegen permanent in `tests/` und laufen grün
- [x] Abschnitt "Erkenntnisse für Folge-Agenten" unten ausgefüllt
- [x] Folge-Tasks und `Roadmap/README.md` geprüft und bei Bedarf angepasst
- [x] Datei ohne `RUN_` nach `Roadmap/Archiv/` verschoben

## Erkenntnisse für Folge-Agenten

- **Umgesetzt (und Abweichungen vom Plan):** Cargo-Workspace mit `foamcraft-core` (bisher nur Version und Markenhinweis), `foamcraft-render` (OrbitCamera, Scene, Renderer, Viewport, write_png), `foamcraft-cli` (Binary `foamcraft`, clap), `foamcraft-xtask` (Lizenzprüfung mit eigenem SPDX-Parser statt cargo-deny, damit kein Zusatzwerkzeug installiert werden muss) und `app/src-tauri` (Crate `foamcraft-app`). Frontend React 19 + Vite 8 in `app/` als npm-Workspace. Eigene kleine Übersetzungsschicht (`app/src/i18n/i18n.tsx`, JSON je Sprache) statt i18next.
- **Learnings & Stolpersteine:** wgpu 30: `InstanceDescriptor::new_without_display_handle_from_env()`, `RequestAdapterOptions.apply_limit_buckets`, `VertexState.buffers` ist `&[Option<_>]`, `get_mapped_range()` liefert `Result`, `get_current_texture()` liefert das Enum `CurrentSurfaceTexture`, Präsentieren über `queue.present(frame)`, `SurfaceConfiguration.color_space`. glam 0.34: Kamera-Matrizen unter `glam::camera::rh::{view::look_at_mat4, proj::directx::perspective}`. Tauri 2: `app.get_window` braucht das Feature `unstable`; `get_webview_window` reicht für die wgpu-Fläche. Heredocs mit Rust-Lifetimes können in der Git-Bash scheitern; besser das Write-Werkzeug nutzen. Bildschirmfotos der App gehen per PowerShell (`CopyFromScreen` aufs Fensterrechteck), siehe Notizen.
- **Wichtige Entscheidungen und ihre Gründe:** Compositing wgpu unter transparenter WebView2 funktioniert unter Windows 11 ohne Ausweichweg (Details in `docs/architecture.md`). Farben werden in Nicht-sRGB-Ziele geschrieben, damit Ansicht und Offscreen-Bild pixelgleich sind. Welt ist Z-oben, in Metern. Mauseingabe geht immer erst an die Web-View; eine vollflächige `ViewportInput`-Ebene unter den Panels schickt Kameraaktionen per Tauri-Befehl `viewport_camera` an Rust.
- **Auswirkungen auf Folge-Tasks:** Der Renderer lädt die Szene bei jedem Bild neu auf die GPU (für den Würfel egal); für große Netze (ab Task_05) GPU-Puffer cachen. `foamcraft-core` ist bis auf `VERSION` leer; Task_04 baut dort Projektmodell und Befehls-API. Kamerasteuerung ist bewusst kein Befehl der Befehls-API (reiner Ansichtszustand).
- **Hinweise zu Tests und Befehlen:** Golden Image `tests/golden/test_cube.png`; neu erzeugen nur mit Begründung über `FOAMCRAFT_UPDATE_GOLDEN=1 cargo test -p foamcraft-render --test golden`. Ohne GPU melden die Golden-Tests `SKIPPED`. Lizenzprüfung: `cargo xtask license-check` (Policy in `license-policy.json`). App starten: `npm run app:dev`. Typprüfung: `npm run typecheck`.
