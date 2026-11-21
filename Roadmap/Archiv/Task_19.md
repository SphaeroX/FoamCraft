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
- Exportdialog mit Vorschau; die Werkzeuge des MCP-Servers um den Export ergänzen (Eintrag in `CURATED` in `crates/foamcraft-mcp/src/lib.rs`, Test in `tests/mcp/tools.rs`, Tabelle in `docs/mcp.md`; Zielpfade als `path`, damit die Sandbox greift)

**Gehört nicht dazu:**

- Zeitabhängige Felder im Video (Task_37)
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

- [x] `export.video` erzeugt aus einem Ergebnis ein Video, das der Standard-Player unter Windows abspielt
- [x] `export.image` erzeugt ein 4K-Bild mit transparentem Hintergrund
- [x] Die Encoder-Entscheidung steht mit Begründung in `docs/licensing.md` und `docs/architecture.md`

## Offene Fragen / Annahmen

- Offene Entscheidung "Video encoder" fällt in diesem Task. → Entschieden: Motion-JPEG (eigener Baseline-Encoder, 4:2:0) in AVI; Begründung in `docs/licensing.md`.

## Notizen zu diesem Task

Taskspezifische Informationen während der Arbeit hier festhalten. Taskübergreifendes gehört in `Roadmap/README.md`.

- Encoder-Vergleich: H.264/H.265 (Patentlizenzen; OpenH264 deckt nur Ciscos Binärdateien), VP9 (libvpx, C-Bibliothek; Wiedergabe unter Windows nur mit Store-Erweiterung), AV1 (rav1e groß/langsam, ebenfalls Erweiterung nötig), Motion-JPEG/AVI (patentfrei, reiner Rust-Code, spielt ohne Zusatz). Gewählt: MJPEG/AVI.
- Stolperstein: Der JPEG-Encoder der `image`-Crate schreibt 4:4:4 (Kommentar sagt 4:2:2). Der MJPEG-Decoder von Media Foundation zeigt solche Bilder mit falschen Farben (links grau, rechts farbig). `jpeg-encoder` hätte 4:2:0, steht aber unter IJG (nicht erlaubt). Lösung: eigener Baseline-Encoder nach T.81 Anhang K mit 4:2:0 (`crates/foamcraft-render/src/jpeg.rs`); `image` nur noch als Test-Abhängigkeit zum Dekodieren.
- Belege:
  - Tests: `cargo test -p foamcraft-render --test camera_path --test video --test jpeg`, `cargo test -p foamcraft-api --test export` (4K transparent, Legende an/aus, 20 Bilder bei 10 fps × 2 s, zwei Exporte identisch, Abbruch entfernt Datei, Vorschau), `cargo test -p foamcraft-mcp` (Export-Werkzeuge, Sandbox), Vitest `tests/app/export-dialog.test.tsx`.
  - Demo-Projekt (2026-11-21): `export.image` 4K transparent in 1 s (3840 × 2160, Ecken Alpha 0, Modell 255; `Roadmap/Nachweise/task19_4k_transparent.png`); `export.video` Full HD, 30 fps, 8 s, Drehteller mit Rauch-Partikeln in 12,7 s (240 Bilder, 130 MB). Windows/Media Foundation: Dauer 8 s, 1920 × 1080, MJPG, Bild korrekt dekodiert (`tools/windows/video-check.ps1`); im Standard-Player "Medienwiedergabe" geöffnet und abgespielt (`task19_video_im_windows_player.png`, `task19_video_wiedergabe_laeuft.png`). Exportdialog mit Vorschau: `task19_exportdialog.png`.
- Hinweis: Der neue Media Player zeigt beim Öffnen aus dem Hintergrund nur sein Startbild, bis das Fenster einen Klick bekommt – das ist kein Dateifehler.

## Abschluss-Checkliste

Reihenfolge und Details stehen in `AGENTS.md`, Abschnitt "Abschluss- und Archivierungs-Workflow".

- [x] Alle Akzeptanzkriterien erfüllt, Tests liegen permanent in `tests/` und laufen grün
- [x] Abschnitt "Erkenntnisse für Folge-Agenten" unten ausgefüllt
- [x] Folge-Tasks und `Roadmap/README.md` geprüft und bei Bedarf angepasst
- [x] Datei ohne `RUN_` nach `Roadmap/Archiv/` verschoben

## Erkenntnisse für Folge-Agenten

Vor der Archivierung vom Agenten auszufüllen. Schreibe so, dass ein Agent, der diesen Task nie gesehen hat, davon bei späteren Tasks profitiert:

- **Umgesetzt (und Abweichungen vom Plan):** `foamcraft_render::camera_path` (Schlüsselbilder, Hermite/Catmull-Rom, Presets), `foamcraft_render::jpeg` (eigener Baseline-Encoder 4:2:0), `foamcraft_render::video` (AVI-Schreiber, `read_frames`), `foamcraft_render::overlay` (Legende mit eingebauter 5 × 7-Schrift), `foamcraft_api::export` (`export.image`, `export.video`, `export.status`, `export.cancel`, `export.preview`), `foamcraft_api::scene::legend`, MCP-Werkzeuge `export_image`, `export_video`, `export_status`, `export_cancel`, Exportdialog in der App (Knopf "Export"), `tools/windows/video-check.ps1`. Abweichung: eigener JPEG-Encoder statt Bibliothek (siehe Notizen).
- **Learnings & Stolpersteine:** Media Foundation braucht unterabgetastete Chroma bei MJPEG. Clippy verlangt `as_chunks` statt `chunks_exact` mit fester Größe. Partikel im Video laufen mit `setup.dt · 60 / fps` je Bild, damit das Tempo dem der App entspricht.
- **Wichtige Entscheidungen und ihre Gründe:** MJPEG/AVI wegen Lizenz/Patenten und Abspielbarkeit ohne Erweiterungen; große Dateien (≈ 15 MB/s Full HD mit Partikeln) und 1-GiB-Grenze bewusst in Kauf genommen. Transparenz über Hintergrund-Alpha 0 (Geometrie ist deckend, kein Blending). Legende wird auf das fertige Bild gezeichnet (keine Schriftdateien).
- **Auswirkungen auf Folge-Tasks:** Task_20 (Report) rendert Standardansichten mit `export.image` (Parameter `camera` = ein `CameraKey`) und kann `foamcraft_render::overlay::format_value` für Zahlen nutzen. Task_37 (zeitabhängig) erweitert die Zeitachse in `foamcraft_api::export::Timeline`. Ein komprimierter Codec kann später als Option hinzukommen, ohne die Befehle zu ändern.
- **Hinweise zu Tests und Befehlen:** Export-Tests brauchen eine GPU (ohne GPU: SKIPPED). Prüfen einer Videodatei mit Windows: `powershell -ExecutionPolicy Bypass -File tools\windows\video-check.ps1 -Video <avi> -Frame <bmp>`.
