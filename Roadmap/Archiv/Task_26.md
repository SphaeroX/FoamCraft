# Task_26: Anströmung in der Ansicht einstellen

> ## ⚠️ PFLICHT – BEVOR DU IRGENDETWAS ANDERES TUST
>
> 1. **Lies als Allererstes die Datei `AGENTS.md` im übergeordneten Ordner `Roadmap/`** (von hier: `../AGENTS.md`) und befolge sie strikt.
> 2. **Deine allererste Aktion** ist, diese Datei in `RUN_Task_26.md` umzubenennen, um den Task als laufend zu markieren. Das passiert **noch bevor irgendein Code geschrieben** wird.

## Ziel

Die Richtung, aus der der Wind auf das Modell trifft, stellt man sichtbar ein: am orangen Pfeil in der 3D-Ansicht ziehen oder an einer Windrose im Szenario-Schritt drehen. Änderungen am Szenario wirken sofort, ohne "Übernehmen".

## Kontext & Abhängigkeiten

- **Voraussetzung:** Task_25 ist archiviert (Griffe in der Ansicht mit Vorschau und einem Befehl am Ende).
- **Herkunft:** Rückmeldung des Nutzers (2026-11-21): "über den orangenen Pfeil … visuell einstellen, wie der Wind auf das Objekt schießt"; allgemein mehr Klick-Elemente, das Technische darf bleiben.
- **Relevantes Vorwissen:** `scenario.set_wind_tunnel` (`direction_deg`: 0 = nach +X, 90 = nach +Y), Tunnel und Pfeil in `foamcraft_render::scene::tunnel`, `ScenarioPanel.tsx`. Der Pfeil sitzt heute an der Einlasswand des Tunnels, weit weg vom Modell und klein.

## Umfang

**Gehört dazu:**

- Deutlicher Windpfeil nahe am Modell, in der Bodenebene um das Modell drehbar (Griff aus Task_25), Einrasten auf 5°
- Windrose im Szenario-Schritt: Kreis mit Modell-Umriss von oben, Pfeil per Klick oder Ziehen setzen; Zahlenfeld bleibt daneben
- Szenario-Eingaben wirken sofort (entprellt), Fehler erscheinen am Feld; ob der Knopf "Übernehmen" ganz entfällt, hängt an zwei bestehenden Tests (siehe Umsetzungshinweise)
- Schnellwahl für die Geschwindigkeit (typische Werte als Knöpfe) neben dem Zahlenfeld
- Boden als Auswahl mit Bild-Symbolen statt Aufklappliste
- Kennwerte (Reynolds, Mach, Zellbudget) in einem zugeklappten Abschnitt "Details"

**Gehört nicht dazu:**

- Anströmung von oben oder unten (die Richtung bleibt in der Bodenebene)
- Lokale Strömungsquellen (Task_31)

## Umsetzungsschritte

1. Tests zuerst: Winkel aus einem Zug um die Hochachse (Rust); Windrose und sofortiges Übernehmen (Vitest).
2. Windpfeil neu platzieren und als Griff anbieten.
3. Windrose bauen, Szenario-Schritt auf sofortiges Übernehmen umstellen.
4. Schnellwahl, Boden-Auswahl, Details-Abschnitt.
5. Live prüfen und Nachweise ablegen.

## Umsetzungshinweise

Aus der Sichtung des Codes am 2026-11-21.

- **Achtung, bestehende Tests:** `tests/app/scenario-panel.test.tsx` klickt in zwei Tests auf den Knopf "Apply". Bestehende Tests dürfen nicht geändert werden. Deshalb: sofortiges Übernehmen einbauen und den Knopf vorerst behalten (er übernimmt dann ohne Wartezeit). Das Akzeptanzkriterium "kein Übernehmen-Knopf" ist erst erfüllbar, wenn der Nutzer der Anpassung dieser beiden Tests zustimmt: anhalten, Grund nennen, fragen. Stimmt er nicht zu, den Knopf dezent lassen und das Kriterium in den Notizen als abgewandelt festhalten.
- **Sofortiges Übernehmen:** Ein `useEffect` über die Eingaben mit etwa 400 ms Verzögerung, der dieselbe Funktion wie der Knopf ruft. Zwei Fallen: Beim Laden eines gespeicherten Szenarios darf er nicht feuern (Merker "vom Nutzer geändert"), und jede Antwort darf die Eingabefelder nicht überschreiben, während getippt wird. In Vitest mit `vi.useFakeTimers()` prüfen, dass schnelles Tippen genau einen Aufruf ergibt.
- **Folge für das Netz:** Jede Änderung am Szenario macht das Netz ungültig (Task_23). Sofortiges Übernehmen ist deshalb unkritisch für die Rechenzeit: vernetzt wird erst beim Start.
- **Windpfeil in der Ansicht:** Der Pfeil in `foamcraft_render::scene::tunnel` sitzt an der Einlasswand und ist Teil des Golden-Images `tests/golden/wind_tunnel.png`. Diese Funktion nicht verändern. Stattdessen einen zweiten, großen Pfeil als Griff (`handle:wind`, Technik aus Task_25) anlegen: auf der Anströmseite im Abstand von etwa 1,5 Hüllkugel-Radien vom Modellmittelpunkt, in Höhe der Modellmitte, Spitze zum Modell.
- **Winkel aus dem Zug:** Mausstrahl mit der waagerechten Ebene durch die Modellmitte schneiden; der Griff liegt stromauf, also `Richtung = atan2(my − y, mx − x)` vom Trefferpunkt zur Mitte, in Grad, auf 5° gerundet (Umschalt: frei). `direction_deg` ist die Richtung, in die der Wind strömt (0 = nach +X, 90 = nach +Y), siehe `foamcraft_sim::flow_direction`.
- **Vorschau:** Pfeil und Tunnelrahmen mit der Vorschau-Matrix aus Task_25 um die Hochachse durch die Modellmitte drehen. Dafür muss `ProjectScene` sagen, welche Netze zum Tunnel gehören (sie werden am Ende von `project_scene` angehängt). Der Tunnel ist um die Modell-Hüllbox in Strömungsrichtung gebaut (`tunnel_box`), bei schräger Anströmung ändern sich seine Maße leicht; für die Vorschau reicht die Drehung, nach dem Befehl wird er exakt neu gebaut.
- **Befehl am Ende:** `scenario.set_wind_tunnel` verlangt das ganze Szenario (`deny_unknown_fields`, alle Felder Pflicht). `viewport_handle_end` liest das aktuelle Szenario aus dem Manifest und ersetzt nur `direction_deg`.
- **Windrose:** reines SVG, keine Bibliothek (wie `LineChart.tsx`). Winkel aus Klick oder Ziehen mit `atan2` relativ zur Mitte; SVG hat y nach unten, also das Vorzeichen drehen, damit +Y oben ist wie in der Draufsicht. Tastatur: Pfeiltasten ±5°, `role="slider"` mit `aria-valuenow`. Der Modell-Umriss von oben ist Zugabe: die Hüllbox aus `geometry.info` (`bounds_m`) genügt.
- **Schnellwahl Geschwindigkeit:** Werte in SI speichern und über `toDisplaySpeed` anzeigen (`app/src/units.ts`); bei imperialer Anzeige sind es dieselben Geschwindigkeiten in mph.
- **Boden als Bild-Auswahl:** `scenario.groundMoving` ist als "in einer späteren Version" beschriftet (kommt mit Task_43); die Option gesperrt anzeigen, nicht entfernen. `getByLabelText("Ground")` mit `fireEvent.change(... value: "fixed")` steht im bestehenden Test: Ein `<select>` mit diesem Namen muss es weiter geben, zum Beispiel als zugängliche Grundlage unter den Bild-Knöpfen.

## Tests

Alle Tests dieses Tasks werden permanent in `tests/` im Hauptverzeichnis des Projekts abgelegt (siehe `AGENTS.md`).

- Rust: Der Zug am Windpfeil ergibt den erwarteten Winkel, mit und ohne Einrasten; der Pfeil zeigt in Strömungsrichtung.
- Golden-Image: Windpfeil am Modell bei 0° und 30°.
- Vitest: Klick in die Windrose setzt die Richtung und ruft `scenario.set_wind_tunnel`; Tippen im Zahlenfeld ruft den Befehl entprellt genau einmal; ungültige Geschwindigkeit zeigt den Fehler und ruft nichts.

## Akzeptanzkriterien

Der Task gilt erst als fertig, wenn alle Punkte erfüllt und durch Tests oder einen konkreten Befehl belegt sind:

- [x] Ziehen am Windpfeil dreht Pfeil und Tunnel; beim Loslassen ist die Richtung gespeichert (ein Undo-Schritt)
- [x] Die Windrose zeigt dieselbe Richtung und lässt sich per Klick setzen
- [x] Kennwerte folgen den Eingaben ohne Klick auf "Übernehmen" (den Knopf selbst nur mit Zustimmung des Nutzers entfernen, weil bestehende Tests ihn benutzen)

## Offene Fragen / Annahmen

- Annahme: Angezeigt wird die Richtung, in die der Wind strömt (wie bisher); die Windrose beschriftet "Wind kommt von" und "strömt nach", damit es keine Verwechslung gibt.

## Notizen zu diesem Task

Taskspezifische Informationen während der Arbeit hier festhalten. Taskübergreifendes gehört in `Roadmap/README.md`.

- 2026-11-21: **Zustimmung des Nutzers** eingeholt (Frage im Chat): Der Knopf "Übernehmen" entfällt, die zwei Tests in `tests/app/scenario-panel.test.tsx` wurden angepasst – nur der Klick auf "Apply" ist entfallen; der Test der ungültigen Geschwindigkeit wartet jetzt 600 ms und prüft, dass trotzdem nichts gesendet wurde. Alle übrigen Erwartungen sind unverändert.
- Abweichung vom Hinweis: Statt im App-Code das ganze Szenario aus dem Manifest zu lesen, gibt es den neuen, rückgängig machbaren Befehl `scenario.set_wind_direction {direction_deg}` (speichert im Bereich −180…180, neuer Fehlercode `scenario.not_wind_tunnel`, auch als MCP-Werkzeug). Der Zug am Windpfeil endet mit genau diesem Befehl.
- Belege: `tests/render/handles.rs` (Winkel aus dem Zug aus mehreren Punkten, keine Antwort bei Blick von der Kante, Pfeil liegt stromauf und zeigt stromab, Golden-Images `wind_arrow_0.png` und `wind_arrow_30.png`), `tests/sim/commands.rs` (Befehl, Undo, Fehler ohne Windkanal), `tests/api/view.rs` (Szene kennt Wind und Tunnelnetze), `tests/app/wind-rose.test.tsx` (Winkel aus Klick, Schieberegler mit Pfeiltasten, Klick in die Rose ruft `scenario.set_wind_tunnel` mit 90°, schnelles Tippen ergibt genau einen Aufruf, Laden eines gespeicherten Szenarios sendet nichts, Schnellwahl und Boden-Bildknöpfe, kein "Apply").
- **Nicht selbst live geprüft:** Ziehen des Windpfeils im Fenster und das Aussehen der Windrose (der Szenario-Schritt lässt sich ohne Mausklick nicht öffnen; der Nutzer arbeitet am Rechner). Die Logik ist durch Tests belegt; Nutzer wurde gebeten, kurz zu schauen.

## Abschluss-Checkliste

Reihenfolge und Details stehen in `AGENTS.md`, Abschnitt "Abschluss- und Archivierungs-Workflow".

- [x] Alle Akzeptanzkriterien erfüllt, Tests liegen permanent in `tests/` und laufen grün
- [x] Abschnitt "Erkenntnisse für Folge-Agenten" unten ausgefüllt
- [x] Folge-Tasks und `Roadmap/README.md` geprüft und bei Bedarf angepasst
- [x] Datei ohne `RUN_` nach `Roadmap/Archiv/` verschoben

## Erkenntnisse für Folge-Agenten


Vor der Archivierung vom Agenten auszufüllen. Schreibe so, dass ein Agent, der diesen Task nie gesehen hat, davon bei späteren Tasks profitiert:

- **Umgesetzt (und Abweichungen vom Plan):** Windpfeil als Griff `handle:wind` (`wind_arrow_mesh`, `wind_direction_deg`, `flow_vector` in `foamcraft_render::handles`), sichtbar im Szenario-Schritt (`viewport_show_wind`); `ProjectScene.wind` und `ProjectScene.tunnel_meshes`; Vorschau dreht Tunnel und Pfeil; Ende → `scenario.set_wind_direction`. `WindRose.tsx` (SVG, `role="slider"`, Pfeiltasten ±5°, Umriss der Modelle aus `geometry.info`), `ScenarioPanel` übernimmt 400 ms nach der letzten Eingabe, Schnellwahl 18/50/100/130 km/h (in SI gespeichert), Boden als Bildknöpfe (`role="radio"`, "bewegter Boden" gesperrt) plus die bisherige Auswahlliste "Ground", Kennwerte im zugeklappten Abschnitt "Details". `ScenarioPanel` nimmt `revision` und liest das Szenario nach Änderungen von außen neu, solange der Nutzer nicht gerade tippt.
- **Learnings & Stolpersteine:** (1) Zwei Elemente mit demselben zugänglichen Namen ("Ground") brechen `getByLabelText`; die Bildknopf-Gruppe trägt deshalb keinen eigenen Namen (die `legend` beschriftet sie sichtbar). (2) Die ESLint-Konfiguration kennt `react-hooks/exhaustive-deps` nicht; ein Disable-Kommentar dafür ist ein Lint-Fehler. (3) Entprellen testen: erst mit echten Timern auf das geladene Szenario warten, dann `vi.useFakeTimers()` und `vi.advanceTimersByTime` in `act`.
- **Wichtige Entscheidungen und ihre Gründe:** Ein eigener Befehl für die Windrichtung statt das ganze Szenario aus dem App-Code zu bauen: kleiner, rückgängig machbar, für MCP/CLI gleich nutzbar. Die Windrose zeigt mit der Pfeilspitze, wohin der Wind strömt; darunter stehen "kommt von" und "strömt nach".
- **Auswirkungen auf Folge-Tasks:** Task_27 (Schnitte ziehen) kann denselben Weg gehen: eigener Schlüssel `handle:slice-…`, Zweig in `begin_drag`/`drag_to`/`end_drag` (`DragKind` in `app/src-tauri/src/main.rs`), Vorschau über `set_model_transform`. Task_29 (Assistent) kann die Szenario-Teile (Windrose, Schnellwahl, Bodenknöpfe) wiederverwenden. Task_31: Quellen brauchen analog einen Befehl, der nur ihre Lage ändert.
- **Hinweise zu Tests und Befehlen:** `cargo test -p foamcraft-sim --test scenario_commands`, `cargo test -p foamcraft-render --test handles`, `npx vitest run tests/app/wind-rose.test.tsx tests/app/scenario-panel.test.tsx`.
