# Task_31: Lokale Strömungsquellen – Düse und Lüfter

> ## ⚠️ PFLICHT – BEVOR DU IRGENDETWAS ANDERES TUST
>
> 1. **Lies als Allererstes die Datei `AGENTS.md` im übergeordneten Ordner `Roadmap/`** (von hier: `../AGENTS.md`) und befolge sie strikt.
> 2. **Deine allererste Aktion** ist, diese Datei in `RUN_Task_31.md` umzubenennen, um den Task als laufend zu markieren. Das passiert **noch bevor irgendein Code geschrieben** wird.

## Ziel

Statt über die ganze Einlasswand kann die Luft aus einer oder mehreren begrenzten Quellen kommen: eine runde oder rechteckige Öffnung mit Lage, Größe, Richtung und Geschwindigkeit, wie bei einem Lüfter oder einer Düse, die auf das Modell bläst.

## Kontext & Abhängigkeiten

- **Voraussetzung:** Task_30 ist archiviert (mehrere Szenarien; eine Strahl-Variante ist ein Szenario wie jedes andere).
- **Herkunft:** Rückmeldung des Nutzers (2026-11-21): "Bei Lüftern wollen wir nur den kleinen Bereich abbilden, wo die Strömung rauskommt … dass wir einen kleinen Punkt oder eine Fläche haben, wo die Luft rauskommt, nicht immer die ganze Wand … vielleicht auch zwei Windquellen."
- **Relevantes Vorwissen:** Windkanal-Fall in `foamcraft_sim::case` (Ränder `inlet`, `outlet`, `sides`, `ground`; Strömung im Fall immer +X), Tunnelregel in `foamcraft_sim::tunnel`, Abbruch über stabile Kraftbeiwerte (`foamcraft_sim::solve`), Bezugswerte (Stirnfläche, Staudruck aus der Anströmgeschwindigkeit). Bei einem Freistrahl gibt es keine gleichmäßige Anströmung: Beiwerte wie C_D verlieren ihre übliche Bedeutung, Kräfte in Newton bleiben aussagekräftig.

## Umfang

**Gehört dazu:**

- Neues Szenario "Freistrahl" (`scenario.set_jets`): ruhende Umgebung, eine bis vier Quellen mit Form (Kreis, Rechteck), Mittelpunkt, Größe, Richtung, Geschwindigkeit; Medium, Boden und Qualität wie beim Windkanal
- Rechengebiet: Kasten um Modell und Quellen mit offenen Rändern (Luft kann ein- und ausströmen); Quelle als Einlassfläche auf einer kleinen Wandscheibe (Lüftergehäuse) im Raum
- Netz: Verfeinerung entlang der Strahlen bis zum Modell
- Abbruch über stabile Kräfte; Ergebnis mit Kräften in Newton, Kraftbeiwerte nur mit ausdrücklich genanntem Bezug (Strahlgeschwindigkeit, Stirnfläche) und Hinweis
- Quellen in der Ansicht: sichtbar als Scheibe mit Pfeil, verschieben und ausrichten mit den Griffen aus Task_25; Zahlenfelder im Szenario-Schritt
- Vorlagen aus Task_28 funktionieren (Rauchfäden starten an den Quellen)
- Plausibilität: Quelle im Modell oder vom Modell weg gerichtet ergibt eine Warnung
- Validierung: runder Freistrahl ohne Modell, Abfall der Achsgeschwindigkeit gegen den Literaturwert

**Gehört nicht dazu:**

- Rotierende Lüfterblätter, Drall, Kennlinien (Task_39 behandelt rotierende Teile)
- Wärme (Task_38)
- Quellen im Innenströmungs-Szenario (dort gibt es Einlässe)

## Umsetzungsschritte

1. Machbarkeit im Runtime-Container klären: Randbedingungen für offene Ränder und Strahl-Einlass, Stabilität in Entwurfsqualität; Ergebnis in den Notizen festhalten.
2. Tests zuerst: Gebietsregel, Fallvorlage als Snapshot, Validierung der Eingaben (Rust).
3. Szenario, Befehle, Fallvorlage und Netzplan umsetzen.
4. Lauf-Logik: Bezugswerte, Abbruch, Ergebnis und Kennzahlen (`results.summary` mit eigener Art).
5. Ansicht und Oberfläche: Quellen zeigen, ziehen, eingeben.
6. `runtime_`-Lauf am Beispielkörper; `validation_` Freistrahl.
7. `docs/architecture.md` (Regeln, Grenzen), Fehlerkatalog, MCP.

## Umsetzungshinweise

Aus der Sichtung des Codes am 2026-11-21. Dieser Task ist der riskanteste der Phase: neue Physik, echte Läufe, eine Validierung. Die Hinweise zu OpenFOAM sind Ausgangspunkte für Schritt 1, keine geprüften Ergebnisse.

- **Schritt 1 ernst nehmen:** Erst im Container von Hand einen kleinen Fall zum Laufen bringen, dann Code schreiben. Dateien kommen per `docker cp` in ein Volume (so macht es `prepare_job`); Bind-Mounts neuer Ordner lösen auf dem Referenzrechner einen Freigabe-Dialog aus, der unbeaufsichtigt hängen bleibt. OpenFOAM-Befehle im Container immer in einer Login-Shell (`exec_openfoam` im Runtime-Crate zeigt wie).
- **Wie der heutige Fall aussieht:** Vorlagen in `crates/foamcraft-core/templates/wind_tunnel/` (Platzhalter `{{…}}`), gefüllt von `foamcraft_sim::case::template_values`/`write_wind_tunnel_case`. Ränder: `inlet` (feste Geschwindigkeit), `outlet` (Druck 0, `inletOutlet`), `sides` (`slip`), `ground`, Modell-Patches `model_<gruppe>` in der Patch-Gruppe `model`. Der Fall liegt immer im Strömungssystem (Strömung +X).
- **Vorschlag für die Ränder des Freistrahls** (in Schritt 1 prüfen): Kasten mit sechs offenen Flächen als ein Patch; dort `U`: `pressureInletOutletVelocity`, `p`: `totalPressure` mit `p0` 0, Turbulenzgrößen `inletOutlet`. Quelle: `U` `fixedValue` in Strahlrichtung, `p` `zeroGradient`, Turbulenz aus etwa 5 % Intensität und einer Länge von rund 7 % des Durchmessers.
- **Woher die Einlassfläche kommt:** Der einfachste Weg nutzt, was es gibt: Das "Lüftergehäuse" als kleine geschlossene Scheibe (Zylinder oder Quader, wenige Millimeter dick) erzeugen und wie ein Modell an snappyHexMesh geben, mit zwei Regionen: die Stirnfläche in Strahlrichtung als `jet_<n>`, der Rest als Wand. Regionen werden heute schon zu Patches (`ModelSurface.regions`). Zwei Dinge sind dabei neu: Der Patch der Stirnfläche muss vom Typ `patch` sein (in `snappyHexMeshDict` bei `refinementSurfaces` über `patchInfo`), und Gehäuse und Strahl dürfen nicht in der Patch-Gruppe `model` landen, sonst zählen ihre Kräfte zum Modell.
- **Anfangslösung:** Die Löser-Stufe ruft immer erst `potentialFoam` (`stage_script` in `crates/foamcraft-core/src/job.rs`) und bricht ab, wenn es scheitert. Mit ruhender Umgebung und nur offenen Rändern ist das Potentialproblem fragwürdig. Vermutlich braucht der Freistrahl eine Variante ohne diesen Schritt (Start aus der Ruhe). `stage_script` kennt heute nur Stufe und Kernzahl; die Skripte entstehen auf dem Host und gehen als Text an den Agenten, ein neues Image ist dafür nicht nötig. Wird dagegen der Agent (`crates/foamcraft-agent`) geändert: Image neu bauen (`cargo run -p foamcraft-cli -- runtime build`).
- **Rechengebiet und Netz:** eigene Regel statt `tunnel_box`: Hüllbox um Modell und Quellen, rundum einige Modelllängen Abstand, hinter dem Modell in Strahlrichtung mehr. Verfeinerung als Zylinder entlang jeder Strahlachse von der Quelle bis hinter das Modell, Radius wächst mit dem Abstand (ein freier Strahl öffnet sich mit etwa 10 bis 12° Halbwinkel). Das Zellbudget ist auf dem Referenzrechner klein (rund 0,7 Mio. Zellen bei 1,9 GiB): über den Durchmesser der Quelle trotzdem mindestens 10 Zellen, sonst stimmt der Strahl von Anfang an nicht.
- **Neue Szenario-Art überall nachziehen:** `Scenario` ist eine Aufzählung mit `kind`. Eine dritte Variante braucht einen Zweig an jeder Stelle, die unterscheidet: `foamcraft_sim::commands` (`scenario.info`), `mesh.rs`/`mesh_internal.rs`, `solve.rs` (`Kind`, `Watch`, Auswertung, `extract_for`), `scene.rs` (Tunnel zeichnen), `summary.rs`, Bericht (Abschnitt Aufbau), `ScenarioPanel`. Mit `grep "Scenario::"` alle finden; der Compiler meldet fehlende Zweige nur bei `match` ohne `_`.
- **Abbruch und Kennzahlen:** Die Konvergenz-Automatik (`Watch` in `solve.rs`) beobachtet die Kraftbeiwerte aus `forceCoeffs`; die Bezugsgeschwindigkeit dort ist dann die Strahlgeschwindigkeit. Stationäre Freistrahlen pendeln gern: Der Ausgang "schwankend" (Mittel über drei Fenster) existiert schon und ist hier der Normalfall, kein Fehler. In `results.summary` die Kräfte in Newton nach vorn stellen; neue Kennzahlen brauchen `figure.<id>` im Katalog und `figures.label.<id>` in beiden Sprachdateien (wird auch im Bericht benutzt).
- **Validierung ehrlich ansetzen:** Für den runden turbulenten Freistrahl gilt im Fernfeld `U_Achse/U_0 = B·d/(x − x_0)` mit `B` um 5,8 bis 6,1 (Hussein, Capp und George 1994; Pope, "Turbulent Flows", Kapitel 5). Zwei-Gleichungs-Modelle treffen den runden Strahl bekannt schlecht (zu starke Aufweitung, die "round jet anomaly"). Die Toleranz deshalb vor dem ersten Lauf mit Quelle festlegen und in den Test schreiben; danach wird sie nicht mehr geweitet (Root-`AGENTS.md`). Liegt das Ergebnis außerhalb: anhalten und dem Nutzer berichten, was gemessen wurde.
- **Ansicht:** Quellen als Scheibe mit Pfeil, platzieren und ausrichten mit den Griffen aus Task_25 (Verschieben entlang der Achsen, Drehen um zwei Achsen). Befehl am Ende des Zugs ist `scenario.set_jets` mit dem ganzen Szenario, wie beim Windpfeil in Task_26.
- **Plausibilität:** "Quelle im Modell" mit `foamcraft_sim::mesh::inside_any` prüfen; "bläst am Modell vorbei" über den Strahl von der Quelle gegen die Hüllkugel.
- **Zu groß für einen Zug?** Fall und Lauf ohne Oberfläche (Befehle, Tests, echter Lauf, Validierung) sind für sich schon ein vollständiger Stand. Reicht die Kraft nicht für alles, dem Nutzer vorschlagen, Ansicht und Oberfläche als eigenen Task abzutrennen.

## Tests

Alle Tests dieses Tasks werden permanent in `tests/` im Hauptverzeichnis des Projekts abgelegt (siehe `AGENTS.md`).

- Rust: Gebiet umschließt Modell und Quellen mit Abstand; Quelle im Modell wird abgelehnt.
- Rust: Fallvorlage (Ränder, Anfangswerte, Verfeinerung) als Snapshot.
- Rust: Kennzahlen eines Freistrahl-Laufs nennen den Bezug; Texte in beiden Sprachen.
- Vitest: Quellen anlegen, ändern, löschen.
- `runtime_`: Beispielkörper mit einer Quelle in Entwurfsqualität rechnet durch; mit zwei Quellen ebenso.
- `validation_`: Achsgeschwindigkeit eines runden turbulenten Freistrahls (Abfall ~ 1/x im Fernfeld) innerhalb der im Test genannten Toleranz mit Quelle.

## Akzeptanzkriterien

Der Task gilt erst als fertig, wenn alle Punkte erfüllt und durch Tests oder einen konkreten Befehl belegt sind:

- [ ] Ein Szenario mit einer kleinen runden Quelle, die auf das Modell bläst, rechnet durch und zeigt den Strahl in den Ergebnissen
- [ ] Zwei Quellen in einem Szenario funktionieren
- [ ] Die Quelle lässt sich in der Ansicht platzieren und ausrichten
- [ ] Die Validierung liegt in der Toleranz

## Offene Fragen / Annahmen

- Annahme: Die Umgebung ruht. Eine Quelle zusätzlich zu einer Grundströmung (Windkanal plus Strahl) ist eine spätere Erweiterung, wenn der Nutzer sie braucht.
- Offen: Welche Randbedingung an den offenen Rändern in v2606 am stabilsten läuft, klärt Schritt 1.

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
