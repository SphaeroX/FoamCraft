# Task_20: Report als PDF und HTML

> ## ⚠️ PFLICHT – BEVOR DU IRGENDETWAS ANDERES TUST
>
> 1. **Lies als Allererstes die Datei `AGENTS.md` im übergeordneten Ordner `Roadmap/`** (von hier: `../AGENTS.md`) und befolge sie strikt.
> 2. **Deine allererste Aktion** ist, diese Datei in `RUN_Task_20.md` umzubenennen, um den Task als laufend zu markieren. Das passiert **noch bevor irgendein Code geschrieben** wird.

## Ziel

Der Nutzer exportiert zu einem Lauf einen verständlichen Bericht als PDF und als eigenständige HTML-Datei: Aufbau, Netz, Konvergenz, Kennzahlen, Bilder und die Grenzen der Aussagekraft. Danach ist Meilenstein 2 erreicht.

## Kontext & Abhängigkeiten

- **Voraussetzung:** Task_17 (Kennzahlen, Ampel) und Task_19 (Bildexport) sind archiviert.
- **Aus Task_19:** Standardansichten über `export.image` (`camera` als `CameraKey`, `legend`, `transparent`); Zahlenformat für Legenden: `foamcraft_render::overlay::format_value`.
- **Relevantes Vorwissen:** `docs/architecture.md` (PDF mit Typst), `docs/licensing.md` (eigenes Layout, nicht an fremde Reports anlehnen; Markenhinweis im Bericht; Schriften nur mit erlaubter Lizenz).

## Umfang

**Gehört dazu:**

- Bericht-Modell aus Projekt und Lauf
- Abschnitte: Deckblatt, Zusammenfassung in Klartext, Aufbau (Geometrie, Maße, Szenario, Medium), Netz, Konvergenz mit Diagrammen, Kennzahlen mit Erklärung, automatisch gerenderte Standardansichten, Verlässlichkeits-Ampel, Grenzen der Aussagekraft, Versionsangaben, Markenhinweis
- PDF über eine eigene Typst-Vorlage mit eingebetteter freier Schrift; HTML als eine Datei ohne externe Verweise
- Englisch und Deutsch
- Vergleichsbericht für zwei Läufe
- Befehl `export.report`, MCP-Werkzeug (Eintrag in `CURATED` in `crates/foamcraft-mcp/src/lib.rs`, siehe Task_18), Berichtsdialog (Abschnitte wählen, Titel, Autor, optionales Logo); Kennzahlen aus `results.summary` (Schema 1, Task_17)

**Gehört nicht dazu:**

- Frei gestaltbare Vorlagen durch den Nutzer

## Umsetzungsschritte

1. Tests zuerst für das Bericht-Modell aus einem Beispiel-Ergebnis.
2. Bericht-Modell und Standardansichten umsetzen.
3. Typst-Vorlage und PDF-Erzeugung umsetzen.
4. HTML-Erzeugung umsetzen.
5. Vergleichsbericht umsetzen.
6. Befehl, MCP-Werkzeug und Dialog ergänzen.
7. Meilenstein 2: Ergebnisse, MCP, Video und Report dem Nutzer zur Abnahme melden.

## Tests

Alle Tests dieses Tasks werden permanent in `tests/` im Hauptverzeichnis des Projekts abgelegt (siehe `AGENTS.md`).

- Das Bericht-Modell ist für das Beispiel-Ergebnis vollständig.
- Das PDF hat Seiten und enthält die Schlüsselwerte und den Markenhinweis (Textextraktion).
- Das HTML enthält keine externen Verweise.
- Beide Sprachen sind vollständig.
- Fehlt der 3D-Cache, entsteht ein Bericht ohne Bilder mit Hinweis statt eines Fehlers.

## Akzeptanzkriterien

Der Task gilt erst als fertig, wenn alle Punkte erfüllt und durch Tests oder einen konkreten Befehl belegt sind:

- [x] `export.report` erzeugt PDF und HTML in Englisch und Deutsch
- [x] Der Bericht enthält Verlässlichkeits-Ampel, Grenzen der Aussagekraft und Markenhinweis
- [x] Ein MCP-Client kann den Bericht anfordern
- [x] Der Nutzer wurde über das Erreichen von Meilenstein 2 informiert

## Offene Fragen / Annahmen

- Keine

## Notizen zu diesem Task

Taskspezifische Informationen während der Arbeit hier festhalten. Taskübergreifendes gehört in `Roadmap/README.md`.

- Typst 0.15.1 (Apache-2.0) als Bibliothek: `typst`, `typst-pdf`, `typst-layout`; Lizenzprüfung danach 941 Pakete, 0 nicht erlaubt. Schrift: Libertinus Serif (OFL-1.1) aus dem Paket typst-assets 0.15.1 nach `assets/fonts/` kopiert (mit `OFL.txt`); `typst-assets` selbst wird nicht eingebunden (es enthält auch Schriften unter anderen Lizenzen).
- Belege:
  - `cargo test -p foamcraft-report` (Modell vollständig en/de, PDF hat Seiten und enthält Schlüsselwerte, Ampel, Grenzen und Markenhinweis per Textextraktion aus dem Satz, HTML ohne externe Verweise, ohne Bilder mit Hinweis, Vergleichsbericht, Zahlenformat).
  - `cargo test -p foamcraft-api --test report` (PDF + HTML en/de, Release v2606, Vergleich mit Laufnamen, nur gewählte Formate).
  - `cargo test -p foamcraft-mcp` (`export_report`, Sandbox auch für `logo_path`); echter stdio-Client: `export_report` am Demo-Projekt lieferte PDF und HTML mit 3 Ansichten.
  - Vitest `tests/app/report-dialog.test.tsx`.
  - Demo-Projekt: deutscher Vergleichsbericht 30 m/s vs. 20 m/s, 8 Seiten, in 3,5 s (PDF selbst nicht im Repository – keine Ergebnisdaten in Git; Nachweise `Roadmap/Nachweise/task20_bericht_*.png`, gerendert mit `tools/windows/pdf-page.ps1`).
- Gesamtprüfung vor Meilenstein 2: `cargo test --workspace` 323 bestanden, 0 fehlgeschlagen, 29 ignoriert (Runtime/Validierung/Leistung); Vitest 76; Clippy, Lint, Prettier, Typen und Lizenzprüfung sauber.
- Meilenstein 2 am 2026-11-21 per Datei an den Nutzer gemeldet (Abnahme steht aus).

## Abschluss-Checkliste

Reihenfolge und Details stehen in `AGENTS.md`, Abschnitt "Abschluss- und Archivierungs-Workflow".

- [x] Alle Akzeptanzkriterien erfüllt, Tests liegen permanent in `tests/` und laufen grün
- [x] Abschnitt "Erkenntnisse für Folge-Agenten" unten ausgefüllt
- [x] Folge-Tasks und `Roadmap/README.md` geprüft und bei Bedarf angepasst
- [x] Datei ohne `RUN_` nach `Roadmap/Archiv/` verschoben

## Erkenntnisse für Folge-Agenten

Vor der Archivierung vom Agenten auszufüllen. Schreibe so, dass ein Agent, der diesen Task nie gesehen hat, davon bei späteren Tasks profitiert:

- **Umgesetzt (und Abweichungen vom Plan):** Crate `foamcraft-report` (Modell, SVG-Diagramm, HTML, Typst-PDF mit eigener Vorlage `data/report.typ`), Befehl `export.report` (`foamcraft_api::report`) mit Standardansichten über `export::render_view_png`, MCP-Werkzeug `export_report`, Dialog "Bericht", `foamcraft_runtime::OPENFOAM_RELEASE`, `runs::summary_json`/`run_name`/`compare` öffentlich, `tools/windows/pdf-page.ps1`.
- **Learnings & Stolpersteine:** Typst 0.15: `PagedDocument` liegt in `typst-layout`; `FileId::new(RootedPath::new(VirtualRoot::Project, VirtualPath::new("/x")?))`; Text für Prüfungen aus den Frames (`FrameItem::Text`) statt aus dem PDF. Inhalte nur als JSON-Daten an Typst geben (kein Markup), dann können Projektnamen nichts kaputt machen. Deutsche Zahlen: erst englisch formatieren, dann Punkt und Komma tauschen (sonst doppelte Vertauschung bei Tausendern). Vorschau von PDFs ist in dieser Umgebung nur über `Windows.Data.Pdf` möglich (`pdftoppm` fehlt).
- **Wichtige Entscheidungen und ihre Gründe:** Eigene Vorlage und eigenes HTML (keine Anlehnung an fremde Reports). Nur eine Schrift (Libertinus Serif, OFL) eingebettet. Markenhinweis immer auch im englischen Originalwortlaut. Fehlender Ergebnis-Cache oder fehlende GPU → Bericht ohne Bilder mit Hinweis.
- **Auswirkungen auf Folge-Tasks:** Task_21 (Beispielprojekte) kann Berichte als Teil der Beispiele zeigen. Task_32 (Installer) muss `assets/fonts/OFL.txt` mit ausliefern. Neue Kennzahlen brauchen Kurzbezeichnungen in `app/src/i18n/*.json` (`figures.label`), die auch der Bericht nutzt.
- **Hinweise zu Tests und Befehlen:** `cargo test -p foamcraft-report`, `cargo test -p foamcraft-api --test report`. PDF ansehen: `powershell -ExecutionPolicy Bypass -File tools\windows\pdf-page.ps1 -Pdf <pdf> -Out <prefix>`.
