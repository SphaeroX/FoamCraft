# Task_29: Oberfläche als Assistent mit neuem Erscheinungsbild

> ## ⚠️ PFLICHT – BEVOR DU IRGENDETWAS ANDERES TUST
>
> 1. **Lies als Allererstes die Datei `AGENTS.md` im übergeordneten Ordner `Roadmap/`** (von hier: `../AGENTS.md`) und befolge sie strikt.
> 2. **Deine allererste Aktion** ist, diese Datei in `RUN_Task_29.md` umzubenennen, um den Task als laufend zu markieren. Das passiert **noch bevor irgendein Code geschrieben** wird.

## Ziel

FoamCraft führt wie ein Assistent durch die Simulation: Jeder Schritt sagt in einem Satz, was zu tun ist, zeigt nur das Nötige und bietet "Weiter". Fachliches (Flächengruppen, Nachbarn, Rollen, Kennwerte) bleibt erreichbar, steht aber nicht im Weg. Das Erscheinungsbild wird ruhiger und hochwertiger.

## Kontext & Abhängigkeiten

- **Voraussetzung:** Task_28 ist archiviert; alle Bedienelemente der Tasks 22 bis 28 existieren.
- **Herkunft:** Rückmeldung des Nutzers (2026-11-21): "Flächengruppen, Nachbarn hinzufügen, Auswahl aufheben ist alles sehr komplex. Ich möchte es wirklich so wie einen Assistenten haben, damit ein Laie eine CFD-Simulation machen kann … die UI schöner … wie bei airshaper.com, damit es einfach ist."
- **Unabhängigkeit (bindend):** Die Root-`AGENTS.md` und `docs/licensing.md` verbieten, Layouts, Texte oder Grafiken anderer CFD-Produkte zu übernehmen. Der genannte Dienst ist ein Maßstab für Einfachheit, keine Vorlage: eigenes Layout, eigene Farben, eigene Texte und Symbole.
- **Relevantes Vorwissen:** `app/src/App.tsx` (Werkzeugleiste mit zehn Knöpfen, vier Schritte), `app/src/styles.css`, Panels je Schritt, `GuidedTour`. Skill `frontend-design` für die Gestaltung nutzen.

## Umfang

**Gehört dazu:**

- Gestaltungssystem: Farb-, Abstands- und Schrift-Variablen, Karten, Knöpfe (primär/sekundär/dezent), Felder, Symbole (selbst gezeichnet, als SVG im Repository); hell und dunkel
- Schrittleiste mit Nummern und Zustand (offen, bereit, erledigt, Achtung); "Weiter" und "Zurück" in jedem Schritt; ein Schritt nennt, was noch fehlt
- Zwei Stufen: "Einfach" (Standard) und "Experte". Einfach zeigt je Schritt nur die Hauptaufgabe; Flächen, Gruppen, Teile, Rollen, Netz-Kennwerte und Protokolle liegen hinter "Experte" oder in zugeklappten Abschnitten
- Geometrie-Schritt einfach: Modell laden, Größe/Einheit bestätigen, ausrichten (Griffe aus Task_25), "Auf den Boden setzen"
- Werkzeugleiste entrümpelt: Projektname, Schritte, Export/Bericht; alles Weitere (Sprache, System, Agenten, Tour, Über) in einem Menü
- Startseite neu: große Karten für Beispiel, neues Projekt, zuletzt geöffnet
- Leere Zustände und Fehlermeldungen mit klarer nächster Handlung
- Bedienbarkeit: Tastatur, sichtbarer Fokus, Kontrast AA, verkleinertes Fenster

**Gehört nicht dazu:**

- Neue Funktionen
- Änderungen an Befehlen

## Umsetzungsschritte

1. Gestaltungsrichtung festlegen (Skill `frontend-design`), Variablen und Grundbausteine anlegen.
2. Tests zuerst für Schrittleiste (Zustände, Weiter/Zurück), Stufen Einfach/Experte, Menü (Vitest).
3. Schritt für Schritt umbauen: Startseite, Geometrie, Szenario, Rechnen, Ergebnisse.
4. Kontrast, Tastatur und kleines Fenster prüfen (Skill `design:accessibility-review`).
5. Live prüfen in hell und dunkel, Nachweise ablegen; Tour-Texte anpassen.
6. `docs/architecture.md` (Oberfläche) und `docs/code-style.md` (Gestaltungssystem) ergänzen.

## Umsetzungshinweise

Aus der Sichtung des Codes am 2026-11-21.

- **Das Wichtigste zuerst: bestehende Tests bleiben unverändert.** Jede Komponente hat Tests, die Elemente über Rolle und englischen Namen finden (`getByRole("button", { name: "Start run" })`, `getByLabelText("Project name")`, …). Umbauen heißt also: Aussehen, Anordnung und Hülle ändern, zugängliche Namen und Rollen behalten. Vor dem Umbau einer Komponente ihren Test lesen. Wo ein alter Test dem neuen Aufbau wirklich im Weg steht: anhalten, Grund nennen, den Nutzer fragen.
- **Einfach/Experte ohne Testbruch:** Die Tests rendern Komponenten einzeln, ohne umgebenden Anbieter, und erwarten alle Bedienelemente. Die Stufe deshalb als React-Kontext anlegen, dessen Vorgabewert "Experte" ist; nur `App` setzt den Anbieter mit "Einfach" als Standard. So sehen die alten Tests weiter alles, und neue Tests prüfen "Einfach" mit Anbieter. Speichern wie die Sprache (`app/src/i18n/i18n.tsx`, `localStorage`, eigener Schlüssel wie `foamcraft.mode`).
- **Schritt-Zustände als reine Funktion:** z. B. `stepStates({models, watertight, scenario, run, hasResult})` in einer eigenen Datei mit Tests. Die Eingaben gibt es schon: `geometry.list`/`geometry.info` (Modelle, `diagnosis.watertight`), `scenario.info` (Fehlercode `scenario.none`), `RunMonitor` (`useRunMonitor`), `results.list_fields` (`results.none`). `App` zählt bei jedem `project.changed` schon `revision` hoch; daran die Abfragen hängen.
- **`App.tsx` zerlegen:** Die Werkzeugleiste hat zehn Knöpfe in einer Zeile und bricht bei 1280 px um. Eigene Komponenten für Leiste, Schrittleiste und Menü; `App` ist bisher nicht getestet, die neuen Teile bekommen Tests. Dabei den bekannten Fehler beheben: Beim Laden einmal `project.info` rufen und ein offenes Projekt übernehmen (heute zeigt die App nach einem Neuladen der Seite die Startseite, obwohl die Sitzung ein Projekt offen hat).
- **Klassen, die Logik tragen:** `measurePanels()` in `app/src/view-insets.ts` misst die Seitenpanels über ihre Klassen, damit der geteilte Vergleich weiß, wie viel Platz frei ist (`tests/app/view-insets.test.ts`). Panels dürfen anders aussehen, müssen aber messbar bleiben; nach dem Umbau den Vergleich "Nebeneinander" einmal ansehen.
- **Gestaltungssystem:** Alles liegt in einer Datei, `app/src/styles.css` (rund 780 Zeilen), mit wenigen Variablen auf `:root` und einem `@media (prefers-color-scheme: light)`-Block. Erst die Variablen ausbauen (Flächen, Text, Akzent, Zustände, Abstände, Radien, Schriftgrößen), dann die Regeln darauf umstellen; harte Farbwerte in Regeln (es gibt einige, z. B. bei `.run-state`, `.series-*`) durch Variablen ersetzen. In getrennte Dateien aufteilen ist in Ordnung (Vite bündelt CSS-Importe).
- **Nichts von außen laden:** Die App ist "local only", und die CSP in `app/src-tauri/tauri.conf.json` erlaubt nur eigene Dateien. Keine Web-Schriften per URL, keine Symbol-Dienste. Eine Schrift nur als Datei im Repository und nur mit erlaubter Lizenz (`docs/licensing.md`; OFL ist erlaubt, Lizenztext mit ablegen wie bei `assets/fonts/OFL.txt`). Symbole selbst als Inline-SVG zeichnen; eine Symbol-Bibliothek wäre eine neue Abhängigkeit, die erst der Nutzer freigeben müsste.
- **Die 3D-Ansicht gehört zum Bild:** Die Seite ist durchsichtig, darunter zeichnet der native Renderer mit festem dunklem Hintergrund (`BACKGROUND` in `crates/foamcraft-render/src/scene.rs`), auch im hellen Erscheinungsbild. Panels müssen auf diesem Hintergrund funktionieren. Den Renderer-Hintergrund zu ändern würde alle Golden-Images betreffen: nicht in diesem Task.
- **Eigenständigkeit:** Der Nutzer nennt einen fremden Dienst als Maßstab. Nicht dessen Seite öffnen und nachbauen. Aus dem Wunsch folgt: wenige Entscheidungen je Schritt, klare Hauptaktion, verständliche Sprache.
- **Begriffe:** In "Einfach" Fachwörter vermeiden oder mit einem Halbsatz erklären (Reynolds-Zahl, y+, Grenzschichten). Texte kommen nur aus `app/src/i18n/en.json` und `de.json`; `tests/app/i18n.test.ts` schlägt fehl, wenn ein Schlüssel in einer Sprache fehlt. Viele Schlüssel auf einmal ergänzt man am sichersten mit einem kleinen Skript, das beide Dateien als JSON liest und schreibt (kein Suchen und Ersetzen im Text).
- **Aussehen prüfen, ohne den Nutzer zu stören:** Der Vite-Server der App lässt sich im eingebauten Browser öffnen (`npm run dev` im Ordner `app`, Port in `app/vite.config.ts`; ist 1420 belegt, `npx vite --port <frei>`). Außerhalb von Tauri schlagen Befehle fehl, das Layout der Startseite und die Themen (hell/dunkel über die Emulation des Browsers) sieht man trotzdem. Für echte Zustände die App starten und mit `tools/windows/app-print.ps1` aufnehmen.
- **Barrierefreiheit:** Fokusrahmen und Escape für Dialoge gibt es schon (Ende von `styles.css`, `SetupAssistant`). Kontrast für Text auf den halbtransparenten Panels gegen den dunkelsten und hellsten Hintergrund rechnen, nicht nur gegen die Panel-Farbe.
- **Umfang im Griff behalten:** Schritt für Schritt umbauen und nach jedem Schritt `npm test` laufen lassen; nicht alle Komponenten gleichzeitig anfassen.

## Tests

Alle Tests dieses Tasks werden permanent in `tests/` im Hauptverzeichnis des Projekts abgelegt (siehe `AGENTS.md`).

- Vitest: Schrittleiste zeigt je Schritt den richtigen Zustand; "Weiter" ist gesperrt, solange etwas fehlt, und nennt den Grund.
- Vitest: In "Einfach" sind Flächen- und Gruppenwerkzeuge nicht sichtbar, in "Experte" schon; die Wahl bleibt nach einem Neustart erhalten.
- Vitest: Das Menü enthält Sprache, System, Agenten, Tour und Über; alle bestehenden Dialoge öffnen weiterhin.
- Bestehende Tests der Panels bleiben grün (keine Abschwächung).

## Akzeptanzkriterien

Der Task gilt erst als fertig, wenn alle Punkte erfüllt und durch Tests oder einen konkreten Befehl belegt sind:

- [ ] Das Windkanal-Beispiel lässt sich in "Einfach" nur mit "Weiter" und dem Startknopf von der Geometrie bis zum Ergebnis führen
- [ ] In "Einfach" erscheint kein Fachbegriff ohne Erklärung in der ersten Ebene
- [ ] Alle bisherigen Funktionen sind in "Experte" erreichbar
- [ ] Kontrast AA in hell und dunkel; jeder Schritt ist mit der Tastatur bedienbar

## Offene Fragen / Annahmen

- Annahme: "Einfach" ist der Standard; die Wahl wird je Rechner gespeichert, nicht je Projekt.

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
