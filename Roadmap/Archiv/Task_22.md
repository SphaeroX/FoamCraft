# Task_22: Beispielprojekte und neue Projekte ohne Ordnerwahl

> ## ⚠️ PFLICHT – BEVOR DU IRGENDETWAS ANDERES TUST
>
> 1. **Lies als Allererstes die Datei `AGENTS.md` im übergeordneten Ordner `Roadmap/`** (von hier: `../AGENTS.md`) und befolge sie strikt.
> 2. **Deine allererste Aktion** ist, diese Datei in `RUN_Task_22.md` umzubenennen, um den Task als laufend zu markieren. Das passiert **noch bevor irgendein Code geschrieben** wird.

## Ziel

Ein Beispielprojekt öffnet sich mit einem Klick, ohne dass der Nutzer einen Ordner wählen muss, und ist vollständig voreingestellt. Auch ein neues eigenes Projekt braucht nur einen Namen; der Speicherort ist ein Standardordner, den man bei Bedarf ändert.

## Kontext & Abhängigkeiten

- **Voraussetzung:** Task_21 ist archiviert (Beispielprojekte, `samples.create`).
- **Herkunft:** Rückmeldung des Nutzers nach der ersten Durchsicht (2026-11-21): "Wenn ich das Beispielprojekt öffne, muss ich einen Ordner wählen … das könnte automatisch angelegt werden." und "Bei den Demos sollte alles schon vorselektiert sein."
- **Relevantes Vorwissen:** `crates/foamcraft-api/src/samples.rs`, `app/src/components/SamplesSection.tsx`, `StartPage.tsx`, `foamcraft_core::paths`. Regel "Write only inside the project folder, the app data folder and locations the user chose" (`docs/conventions.md`): Der Standardordner ist ein sichtbarer, dokumentierter Ort und wird dem Nutzer angezeigt.

## Umfang

**Gehört dazu:**

- Standard-Projektordner (`~/FoamCraft`, überschreibbar mit `FOAMCRAFT_PROJECTS_DIR`; siehe Notizen zur Abweichung von `Dokumente/FoamCraft`), Befehl `project.default_location`
- `samples.create` ohne `path`: legt das Beispiel im Standardordner an; existiert der Ordner schon, wird ein freier Name gewählt ("Abgerundeter Körper 2")
- `project.new` ohne `path`: dasselbe für eigene Projekte
- Startseite: Beispiel mit einem Klick; neues Projekt nur mit Namen, der Zielordner wird angezeigt und lässt sich über "Anderer Ordner …" ändern
- Beispiele sind vollständig voreingestellt: Szenario gesetzt und im Szenario-Schritt sichtbar ausgewählt (auch die Art der Strömung beim Rohrverteiler)

**Gehört nicht dazu:**

- Umgestaltung der Startseite (Task_29)

## Umsetzungsschritte

1. Tests zuerst: Standardordner, freie Namen, `samples.create`/`project.new` ohne Pfad (Rust); Startseite ohne Ordnerdialog (Vitest).
2. Standardordner und freie Namen im Kern umsetzen, Befehle erweitern.
3. Startseite und Beispielabschnitt umstellen.
4. Prüfen, dass beide Beispiele im Szenario-Schritt fertig ausgewählt erscheinen; Lücken schließen.
5. `docs/` anpassen (Speicherorte).

## Tests

Alle Tests dieses Tasks werden permanent in `tests/` im Hauptverzeichnis des Projekts abgelegt (siehe `AGENTS.md`).

- Rust: Standardordner aus der Umgebungsvariable; `samples.create` ohne Pfad legt im Standardordner an und weicht bei belegtem Namen aus; `project.new` ebenso.
- Vitest: Klick auf ein Beispiel ruft `samples.create` ohne Ordnerdialog; neues Projekt ohne Ordnerdialog; "Anderer Ordner …" nutzt den Dialog.
- Vitest: Szenario-Schritt zeigt bei einem Innenströmungs-Projekt die Art "Innenströmung" ausgewählt.

## Akzeptanzkriterien

Der Task gilt erst als fertig, wenn alle Punkte erfüllt und durch Tests oder einen konkreten Befehl belegt sind:

- [x] Ein Klick auf ein Beispielprojekt öffnet es ohne Ordnerdialog
- [x] Zweimaliges Öffnen desselben Beispiels ergibt zwei Projekte, keines wird überschrieben
- [x] Ein neues Projekt braucht nur einen Namen; der Speicherort ist sichtbar und änderbar
- [x] Beide Beispiele zeigen im Szenario-Schritt ihre Einstellungen fertig ausgewählt

## Offene Fragen / Annahmen

- Entschieden: Standardordner ist `~/FoamCraft` (Benutzerordner), nicht `Dokumente/FoamCraft`; Begründung in den Notizen. Der Nutzer kann das mit einem Satz umstellen lassen.

## Notizen zu diesem Task

Taskspezifische Informationen während der Arbeit hier festhalten. Taskübergreifendes gehört in `Roadmap/README.md`.

- Belege:
  - `cargo test -p foamcraft-core --test commands` (Projekt ohne Pfad im Standardordner, freier Name "Wing 2", Ordnernamen ohne verbotene Zeichen, `project.default_location`).
  - `cargo test -p foamcraft-api --test samples` (Beispiel ohne Pfad, zweites Öffnen ergibt "Abgerundeter Körper 2", beide vollständig; Szenario beider Beispiele über `scenario.info`).
  - `cargo test -p foamcraft-mcp --test mcp_tools` (ohne Pfad landet ein Projekt im ersten erlaubten Ordner des Agenten).
  - Vitest `tests/app/start-without-folder.test.tsx` (Beispiel mit einem Klick, Projekt nur mit Namen, "Anderer Ordner …", Art der Strömung eines gespeicherten Innenströmungs-Szenarios ist ausgewählt).
  - Gesamt: `cargo test --workspace` (95 Test-Programme grün), `npm test` (21 Dateien, 86 Tests), fmt, clippy, lint, typecheck, format:check.
- Abweichung: Standardordner ist `~/FoamCraft` statt `Dokumente/FoamCraft`. Unter Windows 11 liegt "Dokumente" oft in OneDrive; Ergebnis-Caches (`cache/results`) würden dann hochgeladen und beim Schreiben gesperrt. Der Benutzerordner ist kurz, sichtbar und nicht synchronisiert. Dafür war keine neue Abhängigkeit nötig.
- Sicherheitsfund: Ein fehlender Pfad hätte über MCP ein Projekt außerhalb der erlaubten Ordner angelegt. Der MCP-Server setzt jetzt für `project.new` und `samples.create` einen freien Ordner im ersten Root ein (Test).
- Die bestehenden Vitest-Tests ("in a chosen folder") blieben unverändert gültig: Ist der Standardordner nicht ermittelbar (`project.default_location` schlägt fehl), fragt die Oberfläche wie bisher nach einem Ordner.
- Live-Prüfung in der App erfolgt zusammen mit Task_23 (ein App-Start für beide).
- Der rote MCP-Test hat einmal `~/FoamCraft/Wing` im echten Benutzerordner angelegt; wieder entfernt.

## Abschluss-Checkliste

Reihenfolge und Details stehen in `AGENTS.md`, Abschnitt "Abschluss- und Archivierungs-Workflow".

- [x] Alle Akzeptanzkriterien erfüllt, Tests liegen permanent in `tests/` und laufen grün
- [x] Abschnitt "Erkenntnisse für Folge-Agenten" unten ausgefüllt
- [x] Folge-Tasks und `Roadmap/README.md` geprüft und bei Bedarf angepasst
- [x] Datei ohne `RUN_` nach `Roadmap/Archiv/` verschoben

## Erkenntnisse für Folge-Agenten

Vor der Archivierung vom Agenten auszufüllen. Schreibe so, dass ein Agent, der diesen Task nie gesehen hat, davon bei späteren Tasks profitiert:

- **Umgesetzt (und Abweichungen vom Plan):** `foamcraft_core::paths::{projects_dir, folder_name, free_folder}`, `Session::projects_dir/set_projects_dir`, Befehl `project.default_location`, `project.new` und `samples.create` mit optionalem `path`; Startseite und Beispielabschnitt ohne Ordnerdialog. Abweichung: `~/FoamCraft` statt Dokumente (OneDrive).
- **Learnings & Stolpersteine:** Tests, die Projekte ohne Pfad anlegen, müssen `session.set_projects_dir(...)` setzen, sonst schreiben sie in den echten Benutzerordner. Umgebungsvariablen in Tests vermeiden (parallel, `set_var` ist unsafe). Shell-Heredocs und `node -e` zerstören Backslashes in Rust-Zeichenliteralen: Quelltext mit den Datei-Werkzeugen schreiben. `cargo test --workspace` nie ungefiltert ausgeben (ein Linkerfehler druckt eine Zeile mit zehntausenden Zeichen): `| cut -c1-160 | grep -E "^test result: FAILED|... FAILED|^error"`. LNK1201 trat wieder auf: genannte PDB löschen, neu bauen.
- **Wichtige Entscheidungen und ihre Gründe:** Der Kern wählt den freien Ordner (nicht die Oberfläche), damit CLI und MCP dasselbe Verhalten haben. Die Oberfläche fällt auf den Ordnerdialog zurück, wenn der Standardordner unbekannt ist; so blieben die alten Tests gültig.
- **Auswirkungen auf Folge-Tasks:** Task_29 (Startseite neu) baut auf `project.default_location` und dem Ein-Klick-Beispiel auf. Task_30/31: Neue Befehle, die Ordner anlegen, brauchen dieselbe MCP-Absicherung (`Server::with_default_path`).
- **Hinweise zu Tests und Befehlen:** siehe Belege oben; `npx vitest run tests/app/start-without-folder.test.tsx`.
