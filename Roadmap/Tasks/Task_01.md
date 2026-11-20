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

- [ ] `cargo test --workspace` und `npm test` laufen grün; `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings` und `npm run lint` melden keine Fehler
- [ ] Der dokumentierte Startbefehl öffnet die App; der Würfel ist unter der Oberfläche sichtbar und lässt sich mit der Maus drehen (Bildschirmfoto als Nachweis in den Notizen)
- [ ] Die Sprache lässt sich umschalten und bleibt nach einem Neustart erhalten
- [ ] Der Lizenzprüf-Befehl läuft grün und steht in `docs/testing.md`
- [ ] `docs/architecture.md` nennt die Crates und die Compositing-Entscheidung

## Offene Fragen / Annahmen

- Annahme: Das Compositing wird hier nur unter Windows bewiesen. Linux und macOS werden in Task_22 geprüft.
- Offene Entscheidung aus `docs/architecture.md`: Web-View über wgpu oder Ausweichweg. Sie fällt in diesem Task; bei einem Wechsel des Ansatzes entscheidet der Nutzer.

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
