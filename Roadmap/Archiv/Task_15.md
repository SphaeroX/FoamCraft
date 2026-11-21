# Task_15: Schnitte und Isoflächen

> ## ⚠️ PFLICHT – BEVOR DU IRGENDETWAS ANDERES TUST
>
> 1. **Lies als Allererstes die Datei `AGENTS.md` im übergeordneten Ordner `Roadmap/`** (von hier: `../AGENTS.md`) und befolge sie strikt.
> 2. **Deine allererste Aktion** ist, diese Datei in `RUN_Task_15.md` umzubenennen, um den Task als laufend zu markieren. Das passiert **noch bevor irgendein Code geschrieben** wird.

## Ziel

Der Nutzer zieht Schnittebenen durch die Strömung und blendet Isoflächen ein, um Ablösegebiete und Wirbel zu erkennen.

## Kontext & Abhängigkeiten

- **Voraussetzung:** Task_14 ist archiviert (Volumenraster, Farbskalen).
- **Relevantes Vorwissen:** Erkenntnisse aus Task_14 zu Rasterstufen und Speicherbudget.

## Umfang

**Gehört dazu:**

- Schnittebenen: achsparallel und frei, per Ziehen verschiebbar, mit Feldwahl (Geschwindigkeit, Druck, Gesamtdruck, Turbulenzgröße)
- Isoflächen auf der GPU für Geschwindigkeitsbetrag, Gesamtdruck und Q-Kriterium; Halbtransparenz
- Q-Kriterium aus dem Geschwindigkeitsgradienten auf dem Raster
- Modell entlang einer Ebene aufschneiden
- Befehle zum Anlegen und Ändern; der Ansichtszustand wird als Szene im Projekt gespeichert
- Bedienfeld in der Oberfläche

**Gehört nicht dazu:**

- Partikel und Stromlinien (Task_16)

## Umsetzungsschritte

1. Tests zuerst mit analytischen Feldern (Schnitt, Isofläche einer Kugelfunktion, Q-Kriterium).
2. Schnitt-Darstellung umsetzen.
3. Isoflächen auf der GPU umsetzen; das Verfahren begründen.
4. Q-Kriterium berechnen.
5. Befehle, Szenen-Speicherung und Bedienfeld umsetzen.

## Tests

Alle Tests dieses Tasks werden permanent in `tests/` im Hauptverzeichnis des Projekts abgelegt (siehe `AGENTS.md`).

- Golden Image: Schnitt durch ein analytisches Feld.
- Isofläche der Kugelfunktion: Radius innerhalb der Rasterauflösung.
- Q-Kriterium: positiv bei Starrkörperdrehung, null bei reiner Scherung.
- Befehle erzeugen ohne Fenster ein Bild; Vitest für das Bedienfeld.

## Akzeptanzkriterien

Der Task gilt erst als fertig, wenn alle Punkte erfüllt und durch Tests oder einen konkreten Befehl belegt sind:

- [x] Eine Schnittebene lässt sich in der App ziehen und zeigt das gewählte Feld
- [x] Isoflächen für Gesamtdruck und Q-Kriterium sind einblendbar
- [x] Eine gespeicherte Szene wird nach dem Öffnen des Projekts gleich dargestellt

## Offene Fragen / Annahmen

- Keine

## Notizen zu diesem Task

- Abnahme in der App (T-Stück "Rohr", 2026-11-21): Schnitt quer zu y mit Geschwindigkeit angelegt; Modell am Schnitt aufgeschnitten ("Andere Seite" nötig, damit die dem Betrachter zugewandte Hälfte verschwindet) → Schnitt sichtbar (`Roadmap/Nachweise/task15_schnitt_aufgeschnitten.png`); Isofläche Q = 200 1/s² zeigt das Wirbelgebiet am Abzweig. Nach Neuladen der Oberfläche und erneutem Öffnen des Projekts war die Szene (Schnitt, Aufschnitt) unverändert da.
- Abweichung "Isoflächen auf der GPU": Berechnung auf der CPU (Marching Tetrahedra), Darstellung auf der GPU – Begründung in `docs/architecture.md`.
- "Per Ziehen verschiebbar": über den Schieberegler je Schnitt; Ziehen direkt in der 3D-Ansicht kommt nicht in dieser Version.

Taskspezifische Informationen während der Arbeit hier festhalten. Taskübergreifendes gehört in `Roadmap/README.md`.

## Abschluss-Checkliste

Reihenfolge und Details stehen in `AGENTS.md`, Abschnitt "Abschluss- und Archivierungs-Workflow".

- [x] Alle Akzeptanzkriterien erfüllt, Tests liegen permanent in `tests/` und laufen grün
- [x] Abschnitt "Erkenntnisse für Folge-Agenten" unten ausgefüllt
- [x] Folge-Tasks und `Roadmap/README.md` geprüft und bei Bedarf angepasst
- [x] Datei ohne `RUN_` nach `Roadmap/Archiv/` verschoben

## Erkenntnisse für Folge-Agenten

- **Umgesetzt (und Abweichungen vom Plan):** `foamcraft_sim::visual` (Schnitt, Isofläche, Q-Kriterium, Geschwindigkeitsbetrag, Rasterstufen), `foamcraft_api::view` (Befehle, `flow_meshes`, `clip`, `view.render`), `foamcraft_api::scene` (Szenenaufbau aus `main.rs` hierher verschoben), Manifest Schema 9, `FlowViewSection` im Schritt "Ergebnisse", Golden Image `tests/golden/analytic_slice.png`. Abweichungen siehe Notizen.
- **Learnings & Stolpersteine:** Vite-Hot-Reload lädt bei manchen Änderungen die Oberfläche neu (Startseite); das Projekt bleibt im Backend offen – einfach erneut öffnen. Antworten der Befehle in Vitest-Mocks realistisch halten (z. B. `view.update_slice` gibt die Szene zurück), sonst verschwinden Elemente mitten im Test. Bei undurchsichtigen Modellen sind Schnitte im Inneren erst nach dem Aufschneiden sichtbar.
- **Wichtige Entscheidungen und ihre Gründe:** Szene als Teil des Manifests (Speichern/Öffnen, CLI/MCP gleich). Szenenaufbau in `foamcraft-api`, damit `view.render` ohne Fenster dasselbe Bild liefert. CPU-Isoflächen (Begründung in der Doku).
- **Auswirkungen auf Folge-Tasks:** Task_16 (Partikel, Stromlinien) nutzt `view::load_grids` (Raster mit `U_mag`, `Q`), `visual::sample_levels` und den Weg über `view.*`-Befehle + `flow_meshes` (Linien: `scene::polylines`). Bilder für Berichte (Task_18?) können `view.render` verwenden.
- **Hinweise zu Tests und Befehlen:** `cargo test -p foamcraft-sim --test visual`, `cargo test -p foamcraft-api --test view` (künstlicher Cache, kein Docker), `cargo test -p foamcraft-render --test golden`, `npx vitest run tests/app/flow-view-section.test.tsx`.
