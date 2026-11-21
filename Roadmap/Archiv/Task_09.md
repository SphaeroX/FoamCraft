# Task_09: Automatische Vernetzung für die Außenströmung

> ## ⚠️ PFLICHT – BEVOR DU IRGENDETWAS ANDERES TUST
>
> 1. **Lies als Allererstes die Datei `AGENTS.md` im übergeordneten Ordner `Roadmap/`** (von hier: `../AGENTS.md`) und befolge sie strikt.
> 2. **Deine allererste Aktion** ist, diese Datei in `RUN_Task_09.md` umzubenennen, um den Task als laufend zu markieren. Das passiert **noch bevor irgendein Code geschrieben** wird.

## Ziel

Aus Geometrie und Windkanal-Szenario entsteht ohne Handarbeit ein rechenfähiges Netz innerhalb des Zellbudgets. Die Netzqualität wird geprüft; bei Problemen greifen Ausweichstufen.

## Kontext & Abhängigkeiten

- **Voraussetzung:** Task_03 (Job-Pipeline) und Task_08 (Windkanal-Szenario) sind archiviert.
- **Relevantes Vorwissen:** `docs/architecture.md`, Regel 4; Erkenntnisse aus Task_03 zu Log-Formaten und Stufen. Vernetzt wird mit blockMesh und snappyHexMesh aus OpenFOAM.

## Umfang

**Gehört dazu:**

- Erzeugung von Hintergrundnetz (Zellgröße aus dem Budget), Kantenerkennung und snappyHexMesh-Einstellungen aus dem Szenario
- Verfeinerung an der Oberfläche aus Modellgröße und kleinsten Merkmalen; Verfeinerungsbereiche für Nahfeld, Nachlauf und Boden
- Grenzschichten: Anzahl und Dicke aus der Geschwindigkeit, Ziel ist ein y+ für Wandfunktionen (30 bis 100)
- Punkt im Strömungsraum automatisch außerhalb des Körpers
- Flächengruppen aus Task_06 werden zu benannten Rändern
- Zellzahl-Schätzung vor dem Lauf und Regelung auf das Budget
- Auswertung von checkMesh zu einem Qualitätsurteil
- Ausweichleiter: weniger Schichten, keine Schichten, gröberes Netz
- Leck-Erkennung bei nicht wasserdichter Geometrie mit Klartextmeldung
- Netzstatistik im Ergebnis und eine einfache Netzvorschau in der Ansicht (Oberflächennetz)

**Gehört nicht dazu:**

- Vernetzung für die Innenströmung (Task_13)
- Lösungsadaptive Verfeinerung (Task_39)

## Umsetzungsschritte

1. Tests zuerst für die Berechnung von Zellgröße, Verfeinerungsstufen und Schichtdicke.
2. Vorlagen und Erzeugung der Netz-Dictionaries umsetzen (Snapshot-Tests).
3. Punkt im Strömungsraum und Zuordnung der Ränder umsetzen.
4. checkMesh-Auswertung und Qualitätsurteil umsetzen (Tests gegen aufgezeichnete Ausschnitte).
5. Budget-Regelung und Ausweichleiter umsetzen (Tests gegen den Fake-Runner).
6. Leck-Erkennung umsetzen.
7. In die Job-Pipeline einhängen und mit echten Körpern prüfen.
8. Netzvorschau und Netzstatistik ergänzen.

## Tests

Alle Tests dieses Tasks werden permanent in `tests/` im Hauptverzeichnis des Projekts abgelegt (siehe `AGENTS.md`).

- Snapshot-Tests der erzeugten Dictionaries für das Würfel-Szenario.
- Der Punkt im Strömungsraum liegt für alle Testkörper außerhalb des Körpers.
- Die Ausweichleiter wird bei einem erzwungenen Schichtfehler ausgelöst.
- `runtime_`: Würfel und Kugel werden vernetzt, checkMesh meldet keine Fehler, die Zellzahl liegt innerhalb von 30 % des Budgets.
- Der offene Würfel aus Task_05 führt zum Leck-Fehlercode.

## Akzeptanzkriterien

Der Task gilt erst als fertig, wenn alle Punkte erfüllt und durch Tests oder einen konkreten Befehl belegt sind:

- [x] Ein Befehl vernetzt ein Projekt mit Windkanal-Szenario ohne weitere Eingaben
- [x] Zellzahl, Qualitätsurteil und erreichte Schichten stehen als strukturierte Antwort bereit
- [x] Die `runtime_`-Tests für Würfel und Kugel laufen grün
- [x] Jede Ausweichstufe und der Leck-Fall haben einen Fehlercode oder Hinweis in beiden Sprachen

## Offene Fragen / Annahmen

- Annahme: snappyHexMesh genügt; ein zweiter Vernetzer ist nicht vorgesehen.
- Die Toleranz für das Treffen des Zellbudgets (30 %) ist eine Annahme und wird hier bestätigt oder begründet angepasst.

## Notizen zu diesem Task

Taskspezifische Informationen während der Arbeit hier festhalten. Taskübergreifendes gehört in `Roadmap/README.md`.

- Budget-Toleranz 30 % bestätigt: Würfel 140 286 Zellen (−6 %), Kugel 174 244 (+16 %) bei Budget 150 000; Demo-Projekt (Würfel + Rohr mit Flansch, Entwurf) 519 195 bei 500 000 (+4 %). Die Schätzung zielt auf 95 % des Budgets.
- Nachweis App (2026-11-21): Szenario "Entwurf" → "Netz erzeugen" → 519 195 Zellen, Qualität "Gut", Grenzschichten "keine" (Ausweichstufe `mesh.fallback_no_layers` griff real, weil sich Würfel und Rohr überschneiden); Netzvorschau als Kantenbild über dem Modell (`Roadmap/Nachweise/task09_netzvorschau.png`, `task09_netz_kennzahlen.png`).
- Testdaten: `tests/data/logs/snappyHexMesh_layers.log`, `checkMesh_wind_tunnel.log`; Snapshot `tests/data/snapshots/wind_tunnel_cube/`.

## Abschluss-Checkliste

Reihenfolge und Details stehen in `AGENTS.md`, Abschnitt "Abschluss- und Archivierungs-Workflow".

- [x] Alle Akzeptanzkriterien erfüllt, Tests liegen permanent in `tests/` und laufen grün
- [x] Abschnitt "Erkenntnisse für Folge-Agenten" unten ausgefüllt
- [x] Folge-Tasks und `Roadmap/README.md` geprüft und bei Bedarf angepasst
- [x] Datei ohne `RUN_` nach `Roadmap/Archiv/` verschoben

## Erkenntnisse für Folge-Agenten

- **Umgesetzt (und Abweichungen vom Plan):** `foamcraft_sim::{mesh (Plan), case (Fall-Schreiber), mesh_report (checkMesh/Schichten), mesh_steps (Ausweichleiter)}`, Vorlagen `templates/wind_tunnel/`, `RunOptions.stop_after`, Befehle `mesh.generate` und `mesh.info` in `foamcraft-api`, Manifest Schema 6 (`mesh`), Linien-Pipeline im Renderer, Netzvorschau, `MeshSection` im Szenario-Schritt. "Ansicht einpassen" richtet sich jetzt nach den Modellen, nicht nach Tunnel/Achsen. Abweichung: Kantenerkennung (surfaceFeatureExtract) wird nicht genutzt; `implicitFeatureSnap` reicht für die Testkörper. snappyHexMesh läuft seriell (parallel erst bei Bedarf, Task_37).
- **Learnings & Stolpersteine:** OpenFOAM-`wordRes` behandelt `(model.*)` wörtlich; ein regulärer Ausdruck braucht Anführungszeichen: `'("model.*")'`. `surfaceMeshExtract -constant` hängt den Zeitindex an (`preview_0.obj`). Randflächen von OpenFOAM zeigen aus dem Fluid heraus (ins Modell); für Darstellung `fix_normals` anwenden. Achsen dürfen nicht in Bounding-Box-Rechnungen für Tunnel/Fokus eingehen. Tauri-Dev startet die App bei Änderungen in `crates/` neu – danach das Projekt erneut öffnen. Mausrad per `mouse_event`: negative Delta erst in `UInt32` umdeuten (`BitConverter`), negatives Delta = Inhalt scrollt nach unten / Ansicht zoomt heraus.
- **Wichtige Entscheidungen und ihre Gründe:** Rechnung im Strömungssystem (Tunnel achsenparallel, einfache blockMesh- und Box-Verfeinerung). Ein Job pro Vernetzungsversuch (saubere Zustände, Volume des akzeptierten Netzes bleibt). Leck-Prüfung vorab über die Diagnose statt über snappyHexMesh-Ergebnis (eindeutiger Klartext).
- **Auswirkungen auf Folge-Tasks:** Task_10 rechnet im Volume des Jobs `manifest.mesh.job_id` weiter (Stufen decompose/solve/reconstruct; `record.state` z. B. auf `Running(Decompose)` setzen oder neuen Lauf mit `stop_after: None` ab Decompose). Kräfte in x-Richtung des Strömungssystems = Widerstand; Auftrieb z. Patch-Gruppe `model` für forces-Functionobject verwenden. Anfangswerte k/ω stehen in `case::template_values`. Der Docker-Speicher (1,9 GiB) begrenzt das Budget auf ~0,76 Mio. Zellen.
- **Hinweise zu Tests und Befehlen:** `cargo test -p foamcraft-sim` (Plan, Snapshot, Bericht, Leiter), `cargo test -p foamcraft-api --test mesh` (Fake: Leck, Leiter), `cargo test -p foamcraft-api --test mesh_runtime -- --ignored` (echte Vernetzung, ~40 s). Snapshot aktualisieren nur mit Begründung: `FOAMCRAFT_UPDATE_SNAPSHOTS=1`.
