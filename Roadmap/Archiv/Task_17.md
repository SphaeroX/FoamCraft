# Task_17: Kennzahlen und Laufvergleich

> ## ⚠️ PFLICHT – BEVOR DU IRGENDETWAS ANDERES TUST
>
> 1. **Lies als Allererstes die Datei `AGENTS.md` im übergeordneten Ordner `Roadmap/`** (von hier: `../AGENTS.md`) und befolge sie strikt.
> 2. **Deine allererste Aktion** ist, diese Datei in `RUN_Task_17.md` umzubenennen, um den Task als laufend zu markieren. Das passiert **noch bevor irgendein Code geschrieben** wird.

## Ziel

Der Nutzer sieht alle Kennzahlen eines Laufs mit Erklärung und einer Einschätzung der Verlässlichkeit, verwaltet mehrere Läufe und vergleicht zwei davon direkt.

## Kontext & Abhängigkeiten

- **Voraussetzung:** Task_14 ist archiviert (Task_15 und Task_16 liegen davor).
- **Relevantes Vorwissen:** Ergebnisobjekte aus Task_10 und Task_13. Die Antwort von `results.summary` wird später vom MCP-Server (Task_18) und vom Report (Task_20) genutzt und muss stabil bleiben.

## Umfang

**Gehört dazu:**

- Kennzahlen-Tafel Außenströmung: Kräfte gesamt und je Teil (Druck- und Reibungsanteil), Beiwerte, Momente, Widerstandsfläche, Leistungsbedarf
- Kennzahlen-Tafel Innenströmung: Druckverlust, Durchfluss je Öffnung, Verlustbeiwert
- Erklärtext in Klartext je Kennzahl, in beiden Sprachen
- Verlässlichkeits-Ampel (Konvergenz, Netzqualität, y+, Massenbilanz) mit Begründung
- Läufe verwalten: Liste, umbenennen, als Variante duplizieren, löschen mit Speicherfreigabe
- Vergleich zweier Läufe: Tabelle mit absoluter und relativer Differenz, geteilte Ansicht mit gekoppelter Kamera, Differenzfeld bei gleicher Geometrie
- Befehle `results.summary` und `results.compare`; Export der Kennzahlen als CSV und JSON

**Gehört nicht dazu:**

- Report (Task_20)

## Umsetzungsschritte

1. Tests zuerst für die abgeleiteten Kennzahlen (gegen Handrechnung) und die Ampel-Logik.
2. `results.summary` mit stabilem Schema umsetzen (Snapshot).
3. Lauf-Verwaltung umsetzen.
4. `results.compare` und die geteilte Ansicht umsetzen.
5. Tafel, Vergleichstabelle und Export in der Oberfläche umsetzen (Vitest zuerst).

## Tests

Alle Tests dieses Tasks werden permanent in `tests/` im Hauptverzeichnis des Projekts abgelegt (siehe `AGENTS.md`).

- Widerstandsfläche, Leistungsbedarf und Verlustbeiwert gegen Handrechnung.
- Ampel-Logik für gute, grenzwertige und schlechte Läufe.
- Differenzen im Vergleich; Löschen gibt Speicher frei und lässt das Projekt konsistent.
- Snapshot des Schemas von `results.summary`.
- Vitest für Tafel und Vergleichstabelle.

## Akzeptanzkriterien

Der Task gilt erst als fertig, wenn alle Punkte erfüllt und durch Tests oder einen konkreten Befehl belegt sind:

- [x] `results.summary` liefert alle Kennzahlen mit Einheit und Ampel
- [x] Zwei Läufe lassen sich in der App nebeneinander mit Differenz vergleichen
- [x] Jede Kennzahl hat einen Erklärtext in beiden Sprachen

## Offene Fragen / Annahmen

- Keine

## Notizen zu diesem Task

Taskspezifische Informationen während der Arbeit hier festhalten. Taskübergreifendes gehört in `Roadmap/README.md`.

- Belege der Akzeptanzkriterien:
  - `results.summary`: `cargo test -p foamcraft-sim --test summary` (Handrechnung, Ampel, Schema-Snapshot, Katalog-Vollständigkeit) und `cargo test -p foamcraft-api --test runs` (Kennzahlen mit en/de-Text, Ampel grün).
  - Vergleich: `cargo test -p foamcraft-api --test runs` (Differenzen, Export, Löschen mit Speicherfreigabe, Variante), `--test view` (`view.set_compare` split/difference, `results.geometry_differs`), `cargo test -p foamcraft-render --test split` (jede Hälfte = Szene allein in halber Breite), Vitest `tests/app/key-figures-panel.test.tsx` und `view-insets.test.ts`.
  - App-Abnahme (2026-11-21) am Projekt "Demo" mit zwei echten Läufen (20 m/s und 30 m/s, gleiches Netz): Nachweise `Roadmap/Nachweise/task17_kennzahlen_ampel.png`, `task17_vergleichstabelle.png`, `task17_vergleich_nebeneinander.png`, `task17_differenz_am_modell.png`. Widerstand +126 % (Quadrat der Geschwindigkeit: +125 %), Leistung +239 % (dritte Potenz: +238 %).
- Der ältere 20-m/s-Lauf stammt von vor Task_17 und hat keine Druck-/Reibungsaufteilung; in Tafel und Vergleich steht dort "–".
- Der zweite Demo-Lauf wurde per CLI gestartet (`scenario.set_wind_tunnel` mit 30 m/s, dann `run.start {"wait": true}`, ~65 s Rechenzeit).

## Abschluss-Checkliste

Reihenfolge und Details stehen in `AGENTS.md`, Abschnitt "Abschluss- und Archivierungs-Workflow".

- [x] Alle Akzeptanzkriterien erfüllt, Tests liegen permanent in `tests/` und laufen grün
- [x] Abschnitt "Erkenntnisse für Folge-Agenten" unten ausgefüllt
- [x] Folge-Tasks und `Roadmap/README.md` geprüft und bei Bedarf angepasst
- [x] Datei ohne `RUN_` nach `Roadmap/Archiv/` verschoben

## Erkenntnisse für Folge-Agenten

Vor der Archivierung vom Agenten auszufüllen. Schreibe so, dass ein Agent, der diesen Task nie gesehen hat, davon bei späteren Tasks profitiert:

- **Umgesetzt (und Abweichungen vom Plan):** `foamcraft_sim::summary` (Kennzahlen, abgeleitete Werte, Ampel, Schema 1), Druck-/Reibungsanteil aus den `forceCoeffs`-Spalten (`drag_split` im gespeicherten Ergebnis), `foamcraft_api::runs` mit `results.summary`, `results.compare`, `results.export` (CSV/JSON), `runs.list/rename/duplicate/delete`; `view.set_compare` (split/difference) mit `Renderer::render_split_gpu` und `foamcraft_sim::results::difference_field`; Oberfläche `KeyFiguresPanel` (rechte Seitenleiste im Schritt "Ergebnisse"). Zusätzlich: Die geteilte Ansicht nutzt nur den Bereich zwischen den Seitenleisten (`viewport_insets`), und die Netzkanten der Vorschau werden ausgeblendet, solange ein Ergebnisfeld das Modell färbt (sonst sind die Farben kaum zu sehen).
- **Learnings & Stolpersteine:** `npm run format` hat den JSON-Schema-Snapshot umformatiert → `tests/data/snapshots/` steht jetzt in `.prettierignore`. Erklärtexte fallen ohne Katalogeintrag stillschweigend auf den Code zurück; deshalb prüft `every_figure_and_reason_is_explained_in_both_languages` den Katalog direkt. Eine Variante (`runs.duplicate`) kopiert nur `geometry/` und das Manifest ohne Läufe, Netz und Ansicht.
- **Wichtige Entscheidungen und ihre Gründe:** Kennzahl-Ids sind stabil, Erklärungen kommen aus dem Fehler-/Textkatalog (`figure.<id>`, `reliability.*`), kurze UI-Bezeichnungen aus `figures.label.<id>` in en/de.json. Ampel = schlechteste Prüfung (Konvergenz, Netz, y+ bzw. Massenbilanz). Differenz nur bei identischer Oberfläche (gleiche Punkte und Dreiecke), sonst `results.geometry_differs` – bei anderer Geometrie bleibt die Nebeneinander-Ansicht. In der geteilten Ansicht ist Picking aus.
- **Auswirkungen auf Folge-Tasks:** Task_18 (MCP) soll `results.summary`, `results.compare` und `view.render` (rendert bei aktivem Split beide Läufe nebeneinander) als kuratierte Werkzeuge anbieten. Task_20 (Report) liest `results.summary` (Schema 1) inklusive der en/de-Texte und der Ampel-Begründungen. Manifest ist jetzt Schema 11 (`run_names`, `view.compare`).
- **Hinweise zu Tests und Befehlen:** `cargo test -p foamcraft-api --test runs --test view`, `cargo test -p foamcraft-sim --test summary --test results`, `cargo test -p foamcraft-render --test split`, `npx vitest run tests/app/key-figures-panel.test.tsx`. Schema-Snapshot nur mit begründeter Änderung und `FOAMCRAFT_UPDATE_SNAPSHOTS=1` neu schreiben.
