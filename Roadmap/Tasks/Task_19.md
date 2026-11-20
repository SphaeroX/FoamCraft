# Task_19: Bild- und Videoexport

> ## ⚠️ PFLICHT – BEVOR DU IRGENDETWAS ANDERES TUST
>
> 1. **Lies als Allererstes die Datei `AGENTS.md` im übergeordneten Ordner `Roadmap/`** (von hier: `../AGENTS.md`) und befolge sie strikt.
> 2. **Deine allererste Aktion** ist, diese Datei in `RUN_Task_19.md` umzubenennen, um den Task als laufend zu markieren. Das passiert **noch bevor irgendein Code geschrieben** wird.

## Ziel

Der Nutzer exportiert Bilder bis 4K und Videos mit Kamerafahrt und Partikel-Animation. Der Export rendert Bild für Bild, unabhängig von der Echtzeit, und ist wiederholbar.

## Kontext & Abhängigkeiten

- **Voraussetzung:** Task_16 ist archiviert (wiederholbare Partikel-Animation); Task_17 und Task_18 liegen davor.
- **Relevantes Vorwissen:** `docs/architecture.md`, offene Entscheidung "Video encoder"; `docs/licensing.md` (nur erlaubte Lizenzen, Patentlage beachten).

## Umfang

**Gehört dazu:**

- Bildexport als PNG bis 4K, optional mit transparentem Hintergrund, Legende ein oder aus
- Kamerafahrten: Drehteller, vorbereitete Fahrten, eigene Schlüsselbilder mit weicher Interpolation
- Zeitachse, die Kamerafahrt und Partikel-Animation koppelt
- Offscreen-Rendering Bild für Bild und Kodierung als Job mit Fortschritt und Abbruch
- Entscheidung für einen lizenzsauberen Encoder und ein Containerformat, festgehalten in `docs/licensing.md`
- Befehle `export.image` und `export.video`, Voreinstellungen für Auflösung, Bildrate und Dauer
- Exportdialog mit Vorschau; die Werkzeuge des MCP-Servers um den Export ergänzen

**Gehört nicht dazu:**

- Zeitabhängige Felder im Video (Task_27)
- GIF-Export

## Umsetzungsschritte

1. Encoder-Kandidaten vergleichen (Lizenz, Patente, Abspielbarkeit unter Windows, Geschwindigkeit) und entscheiden.
2. Tests zuerst für Bildexport und Kamera-Interpolation.
3. Bildexport und Kamerafahrten umsetzen.
4. Zeitachse und Bild-für-Bild-Rendering umsetzen.
5. Kodierung als Job mit Abbruch umsetzen.
6. Befehle, MCP-Werkzeuge und Exportdialog ergänzen.

## Tests

Alle Tests dieses Tasks werden permanent in `tests/` im Hauptverzeichnis des Projekts abgelegt (siehe `AGENTS.md`).

- Bildexport: Größe und Transparenz stimmen.
- Kamera-Interpolation trifft die Schlüsselbilder exakt und verläuft stetig.
- Video: Container lesbar, Bildzahl und Dauer stimmen.
- Zwei Exporte mit gleichen Einstellungen liefern identische Einzelbilder.
- Ein Abbruch entfernt die unvollständige Datei.

## Akzeptanzkriterien

Der Task gilt erst als fertig, wenn alle Punkte erfüllt und durch Tests oder einen konkreten Befehl belegt sind:

- [ ] `export.video` erzeugt aus einem Ergebnis ein Video, das der Standard-Player unter Windows abspielt
- [ ] `export.image` erzeugt ein 4K-Bild mit transparentem Hintergrund
- [ ] Die Encoder-Entscheidung steht mit Begründung in `docs/licensing.md` und `docs/architecture.md`

## Offene Fragen / Annahmen

- Offene Entscheidung "Video encoder" fällt in diesem Task.

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
