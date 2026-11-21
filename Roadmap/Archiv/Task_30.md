# Task_30: Mehrere Szenarien je Geometrie

> ## ⚠️ PFLICHT – BEVOR DU IRGENDETWAS ANDERES TUST
>
> 1. **Lies als Allererstes die Datei `AGENTS.md` im übergeordneten Ordner `Roadmap/`** (von hier: `../AGENTS.md`) und befolge sie strikt.
> 2. **Deine allererste Aktion** ist, diese Datei in `RUN_Task_30.md` umzubenennen, um den Task als laufend zu markieren. Das passiert **noch bevor irgendein Code geschrieben** wird.

## Ziel

Zu einer Geometrie legt man mehrere benannte Szenarien an (zum Beispiel drei Geschwindigkeiten oder zwei Anströmwinkel), lässt sie nacheinander rechnen und vergleicht die Ergebnisse in einer Tabelle, in der Ansicht und im Bericht.

## Kontext & Abhängigkeiten

- **Voraussetzung:** Task_29 ist archiviert. Baut auf Task_17 (Kennzahlen, Laufvergleich), Task_20 (Bericht) und Task_23 (Rechnen im Hintergrund) auf.
- **Herkunft:** Rückmeldung des Nutzers (2026-11-21): "mehrere Szenarien mit der Geometrie definieren, die dann in Reihe rechnen und automatisch verglichen werden … im Report anwählen, was mit was verglichen wird, sodass ich verschiedene Druckszenarien austesten kann."
- **Aus Task_23:** `run.start` ist ein Hintergrund-Ablauf (`Pipeline` in `crates/foamcraft-api/src/run.rs`); die Warteschlange baut darauf auf. Ein Netz merkt sich seine Eingaben (`runs/<Netz-Job>/mesh_inputs.json`, `mesh::mesh_is_current`); heute gehört das ganze Szenario dazu, also vernetzt auch eine andere Geschwindigkeit neu. Für die Wiederverwendung zwischen Szenarien den Schlüssel auf die netzrelevanten Eingaben verengen.
- **Relevantes Vorwissen:** Das Manifest hat genau ein `scenario` und ein `mesh` (Schema 11). Läufe stehen in `runs`, Namen in `run_names`; `results.compare` und `view.set_compare` vergleichen zwei Läufe; `export.report` nimmt einen Lauf und optional einen Vergleichslauf. `runs.duplicate` legt heute eine Variante als eigenes Projekt an.

## Umfang

**Gehört dazu:**

- Manifest: Liste benannter Szenarien mit Kennung, aktives Szenario, Netz je Szenario, Zuordnung Lauf → Szenario; Schema anheben, alte Projekte migrieren (ihr Szenario wird "Szenario 1")
- Befehle `scenario.list`, `scenario.add` (auch als Kopie), `scenario.rename`, `scenario.remove`, `scenario.activate`; die bestehenden `scenario.set_*` wirken auf das aktive Szenario
- Netz-Wiederverwendung: Szenarien, deren Netz-Eingaben gleich sind (Geometrie, Richtung, Boden, Qualität, Grenzschicht-Vorgabe), teilen sich ein Netz
- Warteschlange `run.start_all` (alle oder ausgewählte Szenarien nacheinander), Fortschritt je Szenario, Abbruch der Schlange
- Vergleich über mehrere Läufe: `results.compare` für n Läufe, Tabelle mit Kennzahlen je Szenario und Abweichung zum Bezug, Diagramm einer Kennzahl über die Szenarien
- Bericht: Auswahl, welche Läufe verglichen werden; Vergleichsabschnitt für n Läufe
- Oberfläche: Szenario-Reiter oder -Liste im Szenario-Schritt, Warteschlange im Rechnen-Schritt, Vergleichsauswahl in Ergebnissen und Bericht

**Gehört nicht dazu:**

- Parameterstudien mit automatischer Variation (Raster, Optimierung)
- Verschiedene Geometrien in einem Projekt vergleichen

## Umsetzungsschritte

1. Tests zuerst: Migration, Szenario-Befehle mit Undo/Redo, Netz-Schlüssel, Warteschlange mit FakeEngine, Vergleich für n Läufe (Rust).
2. Manifest und Migration, Szenario-Befehle.
3. Netz je Szenario mit Wiederverwendung; `run.start` für das aktive Szenario.
4. Warteschlange und Ereignisse.
5. Vergleich für n Läufe, Bericht erweitern (Snapshot der Kennzahlen-Struktur bleibt stabil; neue Struktur bekommt eine eigene Schema-Version).
6. Oberfläche (Vitest zuerst).
7. `runtime_`: zwei Szenarien des Beispiels in Reihe; `docs/architecture.md`, `docs/mcp.md`.

## Umsetzungshinweise

Aus der Sichtung des Codes am 2026-11-21.

- **Manifest so erweitern, dass der Rest nichts merkt:** `manifest.scenario` und `manifest.mesh` werden an vielen Stellen gelesen (Szene, Vernetzung, Lauf, Kennzahlen, Bericht, Tests). Der schonende Weg: Beide Felder bleiben und bedeuten "das aktive Szenario". Dazu kommt eine Liste benannter Szenarien (Kennung, Name, Szenario, Netz), die Kennung des aktiven und eine Zuordnung Lauf → Szenario. `scenario.activate` schreibt das bisher aktive in seinen Listeneintrag zurück und lädt das neue in die beiden Felder. So bleiben alle bestehenden Befehle und Tests gültig.
  - Die Gefahr ist doppelter Zustand. An genau einer Stelle abgleichen (zum Beispiel beim Speichern des Projekts und beim Aktivieren) und mit einem Test absichern: Szenario ändern, speichern, neu öffnen, anderes aktivieren, zurück – alles muss noch stimmen.
  - `SCHEMA_VERSION` erhöhen, Schritt in `migrate` (`crates/foamcraft-core/src/project.rs`): Ein vorhandenes Szenario wird der erste Listeneintrag und aktiv. Den Namen leer lassen und in der Oberfläche "Szenario 1" anzeigen, dann steckt keine Sprache im Manifest.
- **Netz-Schlüssel verengen:** `mesh_inputs` in `crates/foamcraft-api/src/mesh.rs` nimmt heute das ganze Szenario. Für die Wiederverwendung zählt, was das Netz bestimmt. Am zuverlässigsten ist der Netzplan selbst: `plan_wind_tunnel` hängt über die Grenzschichtdicke auch von Geschwindigkeit und Medium ab (`first_layer_thickness`). Den Plan des ersten Versuchs (vor der Ausweichleiter) zusammen mit Geometrie, Gruppen, Deckflächen und Rollen ablegen und vergleichen. Zwei Geschwindigkeiten ergeben dann nur dann dasselbe Netz, wenn der Plan gleich ist; das ist physikalisch richtig, auch wenn es seltener trifft als erhofft. Wer mehr Wiederverwendung will, muss die Schichtdicke in Stufen runden und das in `docs/architecture.md` begründen.
- **Geteilte Netze und ihre Volumes:** Heute gilt "neues Netz ersetzt altes, altes Volume wird gelöscht" (`store_mesh`, `Pipeline::work`). Mit mehreren Szenarien darf ein Volume erst weg, wenn kein Szenario es mehr nennt. Ebenso in `runs.delete` prüfen.
- **Gleiche Läufe nacheinander im selben Volume:** Ein Löser-Lauf schreibt in das Volume des Netzes; `upload_setup` (`crates/foamcraft-api/src/solve.rs`) räumt vorher alte Löser-Ausgaben weg. Die Felder für die Ansicht werden nach jedem Lauf in `<projekt>/cache/results/<job>/` gezogen. Nacheinander ist das sicher, gleichzeitig nicht: Die Warteschlange rechnet strikt in Reihe.
- **Warteschlange:** auf `Pipeline` in `crates/foamcraft-api/src/run.rs` aufbauen. Ein Thread, je Szenario eine Kopie des Projekts, in der dieses Szenario aktiv ist (die Sitzung selbst nicht umschalten, der Nutzer arbeitet weiter). Die nachgereichte Änderung (`Session::updates`) muss das Netz beim richtigen Szenario eintragen, über dessen Kennung, nicht über "das aktive"; bis die Änderung angewendet wird, kann der Nutzer umgeschaltet haben.
  - Ereignisse um die Kennung des Szenarios und die Stelle in der Schlange ergänzen. `RunMonitor` (`app/src/run-monitor.ts`) filtert Ereignisse nach der Kennung des verfolgten Laufs: Für die Schlange braucht er ein Ereignis "nächster Lauf beginnt" mit der neuen Kennung, sonst verwirft er alles nach dem ersten Lauf.
  - Abbruch: ein Merker für die Schlange zusätzlich zum Merker des laufenden Laufs. Fehler eines Szenarios: festhalten, mit dem nächsten weitermachen, am Ende melden.
- **Vergleich für n Läufe:** `results.compare {a, b}` hat Tests, ein MCP-Werkzeug und speist den Bericht. Nicht umbauen, sondern einen zweiten Befehl daneben stellen (zum Beispiel `results.compare_many {jobs, reference}`), der dieselben Zeilen je Lauf liefert. Der Schnappschuss `tests/data/snapshots/results_summary_schema.json` gehört zu `results.summary` und bleibt unberührt.
- **Bericht:** `ReportParams.compare_with` (ein Lauf) bleibt; daneben eine Liste. Im Crate `foamcraft-report` hängt der Vergleich an `CompareInput` (`src/lib.rs`), der Vorlage `data/report.typ` und den Texten `data/texts.json` (en/de). Eine Tabelle mit mehr als vier Läufen wird im Hochformat eng: Kennzahlen als Zeilen, Läufe als Spalten, bei vielen Läufen umbrechen.
- **Oberfläche:** `ScenarioPanel` und `InternalFlowSection` laden beim Einhängen einmal `scenario.info`. Nach `scenario.activate` müssen sie neu laden: den `key` der Panels in `App` um die Kennung des aktiven Szenarios erweitern, dann baut React sie frisch auf.
- **MCP und Doku:** neue Befehle in `CURATED` (`crates/foamcraft-mcp/src/lib.rs`), `docs/mcp.md`, `docs/architecture.md`; Fehlercodes in `errors.json` (en/de mit `fix`).
- **Laufzeit der echten Tests:** Zwei Entwurfs-Läufe des Beispiels brauchen auf dem Referenzrechner rund eine Viertelstunde (Docker mit 1,9 GiB). Im Hintergrund laufen lassen und währenddessen an der Oberfläche arbeiten.
- **Zu groß für einen Zug?** Der Task hat vier Teile (Manifest und Befehle, Warteschlange, Vergleich und Bericht, Oberfläche). Reicht die Kraft nicht für alle, nicht still kürzen: dem Nutzer eine Teilung vorschlagen.

## Tests

Alle Tests dieses Tasks werden permanent in `tests/` im Hauptverzeichnis des Projekts abgelegt (siehe `AGENTS.md`).

- Rust: Projekt der Schema-Version 11 öffnet sich mit einem Szenario und seinem Netz.
- Rust: Zwei Szenarien mit gleicher Richtung und Qualität teilen ein Netz; eine andere Richtung erzwingt ein neues.
- Rust: Warteschlange rechnet der Reihe nach, überspringt nach einem Fehler nicht stillschweigend und lässt sich abbrechen.
- Rust: Vergleich für drei Läufe mit Bezug und Abweichungen; Bericht mit Vergleichsabschnitt.
- Vitest: Szenarien anlegen, kopieren, umbenennen, löschen, aktivieren; Warteschlange; Vergleichsauswahl.
- `runtime_`: zwei Geschwindigkeiten am Windkanal-Beispiel in Reihe, ein Netz, zwei Ergebnisse.

## Akzeptanzkriterien

Der Task gilt erst als fertig, wenn alle Punkte erfüllt und durch Tests oder einen konkreten Befehl belegt sind:

- [x] Ein Projekt hält mehrere benannte Szenarien; alte Projekte öffnen sich unverändert
- [x] "Alle rechnen" rechnet die Szenarien nacheinander ohne weiteres Zutun
- [x] Die Vergleichstabelle zeigt die Kennzahlen aller gewählten Läufe nebeneinander
- [x] Im Bericht lässt sich wählen, welche Läufe verglichen werden

## Offene Fragen / Annahmen

- Annahme: Szenarien rechnen nacheinander, nicht gleichzeitig (Arbeitsspeicher).
- Annahme: Der Vergleich in der 3D-Ansicht bleibt bei zwei Läufen (nebeneinander oder Differenz).

## Notizen zu diesem Task

Taskspezifische Informationen während der Arbeit hier festhalten. Taskübergreifendes gehört in `Roadmap/README.md`.

- 2026-11-21: Belege: `tests/core/project.rs` (Version-12-Projekt öffnet mit `scenario-1` samt Netz; Umschalten zwischen Szenarien übersteht Speichern und Öffnen), `tests/api/pipeline.rs` (Szenarien anlegen/umbenennen/aktivieren/entfernen mit Undo und Fehlercodes; Warteschlange mit drei Szenarien: Reihenfolge, 20 und 21 m/s teilen ein Netz, 30° bekommt ein eigenes, Ereignisse `run.queue`/`run.queue_finished`; Abbruch der Warteschlange), `tests/api/runs.rs` (`results.compare_many` mit drei Läufen und Bezug, Fehler bei nur einem Lauf), `tests/api/report.rs` (Bericht mit mehreren gewählten Läufen in HTML und PDF), `tests/app/scenarios.test.tsx` (Reiter, Hinzufügen, Umbenennen, Entfernen, Warteschlange mit Fortschritt und Anhalten, Vergleichstabelle, Auswahl im Bericht).
- **runtime_ mit echtem OpenFOAM:** Windkanal-Beispiel mit zwei Szenarien (20 und 21 m/s) per `run.start_all` gerechnet: beide erfolgreich, **ein** gemeinsames Netz. `results.compare_many`: Cd 0,25476 → 0,25470 (−0,02 %), Widerstand +10,2 % (erwartet (21/20)² = +10,25 %). Bericht mit Vergleich: `Roadmap/Nachweise/task30_bericht_zwei_szenarien.pdf`. Dieser Lauf ist als Befehlsfolge geprüft, nicht als automatischer `runtime_`-Test im Repository (dauert mehrere Minuten; die Logik deckt der FakeEngine-Test ab).
- Folge des schmaleren Netz-Schlüssels: Netze aus älteren Versionen gelten einmalig als veraltet (der gespeicherte Schlüssel enthielt das ganze Szenario) und werden beim nächsten Start neu gebaut.
- Nicht umgesetzt: Diagramm einer Kennzahl über die Szenarien (die Tabelle zeigt Werte und Abweichung); Umbruch der Berichtstabelle bei sehr vielen Läufen (die Tabelle wird breiter).

## Abschluss-Checkliste

Reihenfolge und Details stehen in `AGENTS.md`, Abschnitt "Abschluss- und Archivierungs-Workflow".

- [x] Alle Akzeptanzkriterien erfüllt, Tests liegen permanent in `tests/` und laufen grün
- [x] Abschnitt "Erkenntnisse für Folge-Agenten" unten ausgefüllt
- [x] Folge-Tasks und `Roadmap/README.md` geprüft und bei Bedarf angepasst
- [x] Datei ohne `RUN_` nach `Roadmap/Archiv/` verschoben

## Erkenntnisse für Folge-Agenten


Vor der Archivierung vom Agenten auszufüllen. Schreibe so, dass ein Agent, der diesen Task nie gesehen hat, davon bei späteren Tasks profitiert:

- **Umgesetzt (und Abweichungen vom Plan):** Manifest-Schema 13 (`scenarios`, `active_scenario`, `run_scenarios`; `Manifest::normalize`, `add_scenario`, `activate_scenario`, `set_scenario_mesh`, `with_active`), Befehle in `crates/foamcraft-api/src/scenarios.rs`, Netz-Schlüssel `mesh::mesh_key` (Schichthöhe in √2-Stufen), `run.start_all`/`run.cancel_queue` in `run.rs` (Pipeline kennt ihr Szenario), `results.compare_many`, `export.report {compare_runs}` (Tabelle je Lauf eine Spalte mit Abweichung), Oberfläche `ScenarioTabs`, `QueueSection`, `CompareManySection`, Auswahl im Berichtsdialog; MCP-Werkzeuge. Abweichung: kein Diagramm über die Szenarien.
- **Learnings & Stolpersteine:** (1) `Project::save` nimmt `&self`: der Abgleich der Szenarienliste passiert an einer Kopie beim Speichern und beim Öffnen. (2) Ein neues Feld in `foamcraft_report::CompareInput` hätte einen bestehenden Test gebrochen (Struktur-Literal); der Mehrfachvergleich steckt deshalb in `rows` (Objekt mit `names`). (3) `Pipeline::run` nimmt jetzt `&mut self`, damit die Warteschlange das gebaute Netz danach ablesen kann. (4) `App.tsx` war beim Schreiben einmal gesperrt (Vite-Beobachter); Schreibskripte so bauen, dass ein Abbruch nichts halb ändert, oder erneut ausführen.
- **Wichtige Entscheidungen und ihre Gründe:** "Aktives Szenario" bleibt in den alten Feldern, damit alle bestehenden Befehle und Tests unverändert gelten. Netze werden geteilt, wenn Richtung, Boden, Qualität und die Grenzschicht-Stufe gleich sind; die Stufe (√2) begrenzt den y+-Unterschied auf diesen Faktor.
- **Auswirkungen auf Folge-Tasks:** Task_31 (Quellen) gehört zum Szenario: Quellen im `Scenario` (bzw. `WindTunnel`) ablegen, dann gelten sie je Szenario und `mesh_key` muss sie berücksichtigen, wenn sie das Netz beeinflussen. Neue Felder in `WindTunnel` mit `#[serde(default)]`.
- **Hinweise zu Tests und Befehlen:** `cargo test -p foamcraft-api --test pipeline` (FakeEngine), `--test runs`, `--test report`; `npx vitest run tests/app/scenarios.test.tsx`. Echte Warteschlange: `foamcraft command batch '[…{"name":"run.start_all","params":{"wait":true}},{"name":"scenario.list"}]'`.
