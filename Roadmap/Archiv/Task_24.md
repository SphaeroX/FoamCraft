# Task_24: Ansichtswürfel und Standardansichten

> ## ⚠️ PFLICHT – BEVOR DU IRGENDETWAS ANDERES TUST
>
> 1. **Lies als Allererstes die Datei `AGENTS.md` im übergeordneten Ordner `Roadmap/`** (von hier: `../AGENTS.md`) und befolge sie strikt.
> 2. **Deine allererste Aktion** ist, diese Datei in `RUN_Task_24.md` umzubenennen, um den Task als laufend zu markieren. Das passiert **noch bevor irgendein Code geschrieben** wird.

## Ziel

In einer Ecke der 3D-Ansicht sitzt ein kleiner Würfel, der die Blickrichtung zeigt. Ein Klick auf eine Seite, Kante oder Ecke dreht die Ansicht dorthin; ein Knopf stellt die Standardansicht wieder her.

## Kontext & Abhängigkeiten

- **Voraussetzung:** Task_23 ist archiviert.
- **Herkunft:** Rückmeldung des Nutzers (2026-11-21): "ein kleiner Würfel, wo wir direkt auf Front, Left, Right oder die Ecken klicken können … oder die Standardansicht".
- **Relevantes Vorwissen:** `foamcraft_render::camera::OrbitCamera` (Gieren, Nicken, Abstand; Welt Z oben), `viewport_camera` in `app/src-tauri/src/main.rs`, `app/src/viewport-input.ts`. Die Ansicht wird nativ unter der Web-View gezeichnet; der Würfel selbst ist ein Element der Oberfläche, das die Kamera spiegelt.
- **Unabhängigkeit:** Aussehen und Beschriftung selbst gestalten, kein Nachbau des Würfels eines anderen Produkts (`docs/licensing.md`).

## Umfang

**Gehört dazu:**

- Standardansichten: vorn, hinten, links, rechts, oben, unten, acht Ecken, zwölf Kanten, "Standard" (isometrisch) und "Einpassen"
- Ansichtswürfel in der Oberfläche, folgt der Kamera beim Drehen
- Sanfter Übergang zwischen Ansichten (kurze Kamerafahrt)
- Dieselben Ansichten als Parameter von `view.render` und für MCP
- Tastatur: Würfelflächen sind erreichbare Schaltflächen mit Namen

**Gehört nicht dazu:**

- Orthografische Projektion
- Gespeicherte eigene Ansichten

## Umsetzungsschritte

1. Tests zuerst: Kamerawinkel je Standardansicht, Übergang endet exakt auf der Zielansicht (Rust); Würfel-Komponente (Vitest).
2. Standardansichten im Renderer umsetzen; Kamerazustand an die Oberfläche melden.
3. Würfel-Komponente bauen, in die Ansicht setzen.
4. `view.render` um benannte Ansichten erweitern.
5. Live prüfen und Nachweis ablegen.

## Umsetzungshinweise

Aus der Sichtung des Codes am 2026-11-21. Hinweise, keine Vorschrift; wo der Code inzwischen anders aussieht, gilt der Code.

- **Kamera-Mathematik:** `OrbitCamera` (`crates/foamcraft-render/src/camera.rs`) kennt nur Gieren und Nicken; das Auge liegt bei `target + (cos p·cos y, cos p·sin y, sin p)·distance`. Eine Standardansicht ist also nur ein Paar (Gieren, Nicken):
  - vorn (Auge bei −X, Blick nach +X): Gieren 180°, Nicken 0; hinten: 0°; links (Auge bei +Y): 90°; rechts (Auge bei −Y): −90°
  - oben/unten: Nicken ±(90° − 0,001 rad). Der Wert ist schon als `MAX_PITCH` begrenzt, exakt 90° gibt es nicht. Gieren dabei 180° lassen, dann steht "vorn" im Bild unten.
  - Ecken: Gieren 45° + k·90°, Nicken ±35,264° (atan(1/√2)); Kanten: Nicken 0 mit Gieren 45° + k·90°, oder Nicken ±45° mit Gieren k·90°
  - Standard: `OrbitCamera::default()` (Gieren −60°, Nicken 25°), danach `fit`
- **Wohin damit:** eine Funktion `named_view(name) -> Option<(f32, f32)>` in `camera.rs` und Tests in `tests/render/camera.rs`. Namen als feste englische Kennungen (`front`, `top_front_right`, …), übersetzt wird nur in der Oberfläche.
- **Übergang:** `camera_path::interpolate` nimmt beim Gieren nicht den kürzeren Weg. Vor dem Übergang das Ziel-Gieren auf ±180° um das aktuelle normalisieren, sonst dreht die Ansicht einmal außen herum. Den Takt gibt es schon: der 16-ms-Thread in `app/src-tauri/src/main.rs` (Partikel). Er überspringt die Runde heute mit `continue`, wenn keine Partikel laufen; für den Übergang die Schleife so umbauen, dass sie auch dann zeichnet, wenn ein Übergang aktiv ist (z. B. `ViewState.transition: Option<…>`).
- **Kamera an die Oberfläche melden:** `viewport_camera` gibt heute nichts zurück. Gieren und Nicken zurückgeben (und in `native.ts` durchreichen), dann folgt der Würfel beim Drehen mit der Maus ohne Zusatzaufruf. Während eines Übergangs aus dem Thread ein Tauri-Ereignis senden (`handle.emit("viewport://camera", …)`), sonst steht der Würfel still, bis die Maus sich bewegt.
- **Würfel drehen ohne Raten:** nicht mit `rotateX/rotateZ` herumprobieren. In TypeScript dieselbe Rechnung wie die Kamera machen (vorwärts = −Offset, rechts = vorwärts × Z, oben = rechts × vorwärts), daraus eine 3×3-Matrix bilden und als `matrix3d(…)` setzen. Das ist eine reine Funktion und lässt sich testen: Bei "vorn" muss die Normale der −X-Fläche zum Betrachter zeigen, bei "oben" die der +Z-Fläche. CSS hat y nach unten: das Vorzeichen der Oben-Achse umdrehen.
- **Komponente:** Flächen, Kanten und Ecken als echte `<button>` mit `aria-label` aus den Übersetzungen (26 Treffer plus "Standard"). Der Würfel sitzt über der transparenten Eingabefläche; `ViewportInput` reagiert nur auf Ereignisse, deren Ziel die Fläche selbst ist, also stört der Würfel die Kamerabedienung nicht.
- **Platz:** oben rechts unter der Werkzeugleiste. In den Schritten Geometrie und Ergebnisse liegt dort das rechte Panel (300 px); `measurePanels()` in `app/src/view-insets.ts` kennt die Breiten schon.
- **`view.render`:** `RenderParams` in `crates/foamcraft-api/src/view.rs` um `view: Option<String>` erweitern; unbekannter Name → neuer Fehlercode in `crates/foamcraft-core/data/errors.json` (en und de, jeweils mit `fix`; `tests/api/error_catalog.rs` prüft das). Das MCP-Werkzeug `view_render` hat ein eigenes, handgeschriebenes Schema in `crates/foamcraft-mcp/src/lib.rs` (bei "view_render"): dort ebenfalls ergänzen und in `render()` durchreichen.
- **Golden-Image:** neue Datei in `tests/golden/` mit `FOAMCRAFT_UPDATE_GOLDEN=1` erzeugen; die vorhandenen Bilder nicht anfassen.
- **Live prüfen:** Fenster mit `tools/windows/app-print.ps1` aufnehmen (ohne Fokus, ohne Maus). Klicks auf den Würfel lassen sich so nicht auslösen; arbeitet der Nutzer am Rechner, ihn bitten, einmal auf eine Würfelseite zu klicken, statt `app-ui.ps1` zu benutzen.

## Tests

Alle Tests dieses Tasks werden permanent in `tests/` im Hauptverzeichnis des Projekts abgelegt (siehe `AGENTS.md`).

- Rust: Jede Standardansicht blickt aus der erwarteten Richtung; "oben" und "unten" bleiben stabil (kein Umklappen).
- Rust: Der Übergang interpoliert auf dem kürzeren Weg und endet auf der Zielansicht.
- Vitest: Klick auf eine Fläche, Kante oder Ecke sendet die passende Ansicht; der Würfel dreht sich mit der gemeldeten Kamera; Schaltflächen haben Namen in beiden Sprachen.
- Golden-Image: eine Standardansicht des Testwürfels.

## Akzeptanzkriterien

Der Task gilt erst als fertig, wenn alle Punkte erfüllt und durch Tests oder einen konkreten Befehl belegt sind:

- [x] Klick auf eine Würfelseite zeigt das Modell genau aus dieser Richtung
- [x] Der Würfel dreht sich mit, wenn man die Ansicht mit der Maus dreht
- [x] "Standard" stellt die Anfangsansicht her und passt das Modell ein
- [x] `view.render` nimmt eine benannte Ansicht an

## Offene Fragen / Annahmen

- Annahme: "Vorn" blickt entgegen der Standard-Anströmung, also aus −X in Richtung +X; so sieht man beim Windkanal die angeströmte Seite.

## Notizen zu diesem Task

Taskspezifische Informationen während der Arbeit hier festhalten. Taskübergreifendes gehört in `Roadmap/README.md`.

- 2026-11-21: Umgesetzt wie in den Umsetzungshinweisen. Belege: `tests/render/camera.rs` (Richtung je Ansicht, kein Umklappen, kürzerer Weg, Ende exakt), `tests/app/view-cube.test.tsx` (Klick sendet Ansicht, Würfel folgt der Kamera, Namen in beiden Sprachen), `tests/api/view.rs` und `tests/mcp/tools.rs` (benannte Ansicht, `view.unknown_view`), Golden-Image `tests/golden/view_top_front_right.png`.
- Live geprüft (`tools/windows/app-print.ps1`, ohne Maus): Der Würfel sitzt oben rechts und zeigt in der Standardansicht "Rechts", "Hinten" und "Oben" – das stimmt, denn die Standardkamera (Gieren −60°) blickt von +X/−Y. Nachweis: `Roadmap/Nachweise/task24_ansichtswuerfel.png`.
- Live-Klick: Der Nutzer hat den Würfel am 2026-11-21 in der laufenden App selbst getestet und bestätigt, dass er funktioniert (Klick auf Würfelflächen, Kamerafahrt).
- Test-Korrektur während der Arbeit (eigener, neuer Test): Von unten gesehen liegt "vorn" oben im Bild (wie beim Kippen aus der Vorderansicht); die erste Fassung des Tests hatte für "unten" fälschlich dieselbe Richtung wie für "oben" erwartet.

## Abschluss-Checkliste

Reihenfolge und Details stehen in `AGENTS.md`, Abschnitt "Abschluss- und Archivierungs-Workflow".

- [x] Alle Akzeptanzkriterien erfüllt, Tests liegen permanent in `tests/` und laufen grün
- [x] Abschnitt "Erkenntnisse für Folge-Agenten" unten ausgefüllt
- [x] Folge-Tasks und `Roadmap/README.md` geprüft und bei Bedarf angepasst
- [x] Datei ohne `RUN_` nach `Roadmap/Archiv/` verschoben

## Erkenntnisse für Folge-Agenten


Vor der Archivierung vom Agenten auszufüllen. Schreibe so, dass ein Agent, der diesen Task nie gesehen hat, davon bei späteren Tasks profitiert:

- **Umgesetzt (und Abweichungen vom Plan):** `named_view`, `VIEW_NAMES` (27 Namen) und `CameraTransition` in `crates/foamcraft-render/src/camera.rs`; Kamera-Aktion `{kind: "view", name}`, `viewport_camera` gibt `{yaw, pitch}` zurück, der 16-ms-Takt zeichnet Übergänge und sendet `viewport://camera`; `ViewCube` (`app/src/components/ViewCube.tsx`, Mathematik in `app/src/view-cube.ts`) mit 26 Schaltflächen plus "Standard"; `view.render` und MCP `view_render` nehmen `view`. Jede benannte Ansicht endet eingepasst (nicht nur "Standard"), damit das Modell aus jeder Richtung ganz zu sehen ist.
- **Learnings & Stolpersteine:** Die Welt ist rechtshändig, CSS (x rechts, y unten, z zum Betrachter) linkshändig. Die Würfelmatrix (Welt → Bildschirm) und die Lage jedes Elements (Text-rechts, Text-unten, Normale) haben beide Determinante −1; ihr Produkt ist eine echte Drehung, deshalb ist die Schrift nicht gespiegelt. Nicht mit `rotateX/rotateY` probieren.
- **Wichtige Entscheidungen und ihre Gründe:** Würfel als reines DOM/CSS-3D-Element über der Ansicht (keine zweite GPU-Szene); Übergang 0,35 s mit Smoothstep; Mausbewegung bricht einen laufenden Übergang ab.
- **Auswirkungen auf Folge-Tasks:** Task_25–28 können `viewport://camera` und den Rückgabewert von `viewport_camera` nutzen, um Griffe in der Oberfläche an die Kamera zu koppeln. Der Würfel sitzt in `.view-cube-anchor` (oben rechts, rechts um die Breite des rechten Panels verschoben); neue Elemente oben rechts müssen ihm ausweichen. Task_29 (neues Erscheinungsbild) gestaltet die Farben des Würfels über `styles.css` (`.view-cube-*`).
- **Hinweise zu Tests und Befehlen:** `cargo test -p foamcraft-render --test camera`, `npx vitest run tests/app/view-cube.test.tsx`, Golden: `FOAMCRAFT_UPDATE_GOLDEN=1 cargo test -p foamcraft-render --test golden named_corner` (nur für ein neues Bild). Die App mit freiem Port starten: `npx tauri dev --config <json>` mit `build.devUrl` und `beforeDevCommand: "npx vite --port <p> --strictPort"` (Port 1420 war von einem fremden Node-Prozess belegt).
