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

- Rotierende Lüfterblätter, Drall, Kennlinien (Task_43 behandelt rotierende Teile)
- Wärme (Task_42)
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

- [x] Ein Szenario mit einer kleinen runden Quelle, die auf das Modell bläst, rechnet durch und zeigt den Strahl in den Ergebnissen
- [x] Zwei Quellen in einem Szenario funktionieren
- [x] Die Quelle lässt sich in der Ansicht platzieren und ausrichten
- [x] Die Validierung liegt in der Toleranz

## Offene Fragen / Annahmen

- Annahme: Die Umgebung ruht. Eine Quelle zusätzlich zu einer Grundströmung (Windkanal plus Strahl) ist eine spätere Erweiterung, wenn der Nutzer sie braucht.
- Offen: Welche Randbedingung an den offenen Rändern in v2606 am stabilsten läuft, klärt Schritt 1.

## Notizen zu diesem Task

Taskspezifische Informationen während der Arbeit hier festhalten. Taskübergreifendes gehört in `Roadmap/README.md`.
- 2026-11-21, Stand der Umsetzung: Quellen im Manifest (`jets`, je Szenario), `scenario.set_jets`/`move_jet`/`rotate_jet`, `foamcraft_sim::jets` (Gebiet, gestufte Verfeinerung, Fallumbau), Löser-Stufe ohne potentialFoam bei Quellen, Hinweis `run.jet_reference`, Quellen in der Ansicht (Scheibe mit Pfeil, Griffe), Eingabe im Szenario-Schritt, MCP, Doku. Schnelle Tests grün (99 Rust-Testgruppen, 142 Vitest).
- Echte Läufe: Windkanal-Beispiel mit einer Quelle (Ø 80 mm, 20 m/s): stabil nach 463 Iterationen, 538 000 Zellen, 0,57 N; Strahl im Mittelschnitt sichtbar (`Roadmap/Nachweise/task31_strahl_mittelschnitt.png`). Stolpersteine unterwegs: Verfeinerung bis in den Nachlauf → Netz zu groß (Exit 137); potentialFoam stürzt mit Quellen ab → wird übersprungen. `runtime_one_and_two_sources_blowing_at_a_model` bestanden (eine Quelle: stabil, 548 Iterationen, 0,85 N; zwei Quellen: stabil, 411 Iterationen, 0,63 N).
- **Validierung außerhalb der Toleranz (angehalten, Nutzer informiert):** `validation_round_free_jet_decays_like_the_literature` (d = 0,1 m, 10 m/s, 484 513 Zellen, stabil). Achsgeschwindigkeit zwischen 15 d und 30 d: 2,57 / 2,29 / 2,09 / 1,99 / 2,03 / 2,25 / 2,68 / 3,28 / 4,03 / 4,77 / 5,31 / 5,51 / 5,33 / 4,86 / 4,30 m/s – sie steigt stromab wieder an statt mit 1/x abzufallen. Angepasstes B = −4,7 (Toleranz 4,0 … 7,5, Literatur 5,8 … 6,1). Vermutung: Der Strahl verläuft nicht gerade auf der Abtastlinie (Ablenkung durch die nahen offenen Ränder: nur 10 d Abstand seitlich/oben/unten, oder ein schräger Strahl durch die grobe gestufte Verfeinerung); die Linie schneidet dann Rand und Kern. Toleranz nicht verändert.

- **2026-11-21, Diagnose der Validierung – Ursache gefunden und behoben:** Der Lauf wurde zu früh beendet, der Strahl selbst ist in Ordnung.
  - Befund: Der Würfel steht 1,5 m neben dem Strahl, seine Beiwerte sind ab der ersten Iteration praktisch null (c_D = −3·10⁻⁷). Die Konvergenz-Automatik (`assess`: Band ≤ 1 % von max(|c_D|, 0,1) über ein Fenster, frühestens nach zwei Fenstern) meldete deshalb bei Entwurfsqualität (Fenster 60) nach **120 Iterationen** „stabil"; der Löser hörte bei 125 auf (Restwerte: Ux 1,4·10⁻², Uy/Uz 4,7·10⁻², p 4,5·10⁻³). Zu diesem Zeitpunkt läuft die Front des anfahrenden Strahls noch durch das Gebiet: Achsgeschwindigkeit bei 125 Iterationen 2,28 m/s bei 17 d, 5,82 m/s bei 24 d (der „Buckel" aus der ersten Messung), B = −8,5.
  - Nachweis (derselbe Fall, von Hand im Volume weitergerechnet): Achse nach 250 Iterationen noch mit Buckel (B = 8,2), nach 500 und 750 Iterationen monoton und deckungsgleich (15 d: 2,74 m/s, 20 d: 2,15, 30 d: 1,51; B = 5,06 bzw. 5,02, x0 = −3,3 d). Der Strahl braucht also rund 450 Iterationen, weit unter der Grenze von 900.
  - Die anderen Vermutungen sind widerlegt: Querprofile bei 10/20/30 d (y und z) sind symmetrisch, das Maximum liegt auf der Abtastlinie (y = 1,50, z = 0,01), Quergeschwindigkeit auf der Achse < 0,05 m/s – keine Ablenkung zu den offenen Rändern. Halbwertsradius bei 30 d 0,41 m, Aufweitung 0,12 (Literatur 0,094; die bekannte Schwäche der Zwei-Gleichungs-Modelle). Fall stimmt mit dem Plan überein: Zwangszylinder (−3,05 … −3,00; 1,5; 0,01), Radius 0,05, 5 Zellen lang, U an der Quelle 9,7 … 10,0 m/s; Gebiet x −4,01 … 0,37, y −0,10 … 2,56, z −1,04 … 1,07; Hintergrundzelle 78 mm (56 × 34 × 27), Stufen 3/2/1 (10/5/2,5 Zellen über d). Die Abtastung im Test ist richtig und bleibt unverändert, ebenso die Toleranz.
  - Behebung (TDD): Mit Quellen beobachtet der Lauf zusätzlich die **mittlere Geschwindigkeit im Gebiet**. `apply_jets` schreibt zwei Funktionsobjekte vor die Kräfte in `system/controlDict` (`jetSpeed`: `mag` von U; `jetFlow`: `volFieldValue`, `volAverage`, `log true`), `FieldValueLog` liest auch `volFieldValue`, und `foamcraft_sim::solve::assess_with_flow` meldet erst „stabil"/„schwankend", wenn die Kräfte so beurteilt sind **und** die mittlere Geschwindigkeit im 1-%-Band ihres Fensters liegt (`Watch` in `crates/foamcraft-api/src/solve.rs`, Merkmal `sources` im gespeicherten Lauf-Setup). Für die Geschwindigkeit zählt nur das strenge Band: Das Kriterium „Fenstermittel stimmen überein" hätte in der Messreihe schon bei 245 Iterationen angeschlagen (B noch 8,2). Erreicht die Geschwindigkeit das Band nicht, endet der Lauf an der Iterationsgrenze mit „Grenze erreicht". Gemessene Reihe (Gebietsmittel |U|): 0,174 bei 125, 0,123 bei 250, 0,1191 bei 450, 0,1213 bei 700; Band erfüllt bei 466.
  - Neue Tests: `tests/sim/convergence.rs` (4: Kräfte nahe null halten einen Lauf mit sich entwickelnder Strömung nicht an; Urteil bleibt bei den Kräften; schwingende/driftende Strömung läuft zur Grenze; NaN = divergiert), `tests/sim/jets.rs` (2: Funktionsobjekte im Fall, Lesen der Log-Zeilen von v2606), `tests/api/solve.rs` (1: `run.start` mit Quelle gegen die Fake-Engine entscheidet erst nach dem Einschwingen). Schnelle Suite, `cargo fmt`, `cargo clippy` grün.
  - **Validierung bestanden:** `validation_round_free_jet_decays_like_the_literature` – stabil nach 469 Iterationen, 484 513 Zellen, Achse 15 d … 30 d: 2,74 / 2,60 / 2,47 / 2,36 / 2,25 / 2,16 / 2,07 / 1,99 / 1,91 / 1,84 / 1,78 / 1,72 / 1,66 / 1,61 / 1,56 / 1,51 m/s, **B = 5,05** (Toleranz 4,0 … 7,5), x0 = −3,4 d. Dauer 10 min, davon gut 4 min für die Ergebnis-Abtastung (`simpleFoam -postProcess`, ein Kern).
  - `runtime_one_and_two_sources_blowing_at_a_model` nach der Änderung erneut bestanden, die Läufe dauern aber länger: eine Quelle „schwankend" nach 749 Iterationen, 0,84 N (vorher „stabil" nach 548, 0,85 N); zwei Quellen „stabil" nach 878 Iterationen, 0,66 N (vorher 411, 0,63 N). Die mittlere Geschwindigkeit änderte sich dort bis etwa Iteration 700 noch deutlich (eine Quelle: 0,044 m/s bei 200, 0,101 bei 450, 0,072 bei 700), die Kraft driftete mit (c_D 0,1565 bei 500, 0,1545 bei 700) – das frühere „stabil" war also ebenfalls etwas zu früh. Offen: 878 liegt knapp unter der Grenze von 900 (Entwurf); ähnliche Fälle können künftig mit „Grenze erreicht" enden. Ob die Grenze für Läufe mit Quellen höher liegen soll, ist nicht entschieden.
  - Offen (nicht Teil der Toleranz, aber für „Lüfter bläst aus der Nähe auf ein Modell" wichtig): Der Strahlkern endet zu früh. Achsgeschwindigkeit 9,9 m/s bei 1,8 d, 8,3 bei 3 d, 6,4 bei 5 d, 3,8 bei 10 d; ein realer Strahl hat bis etwa 5 d noch fast U0 und bei 10 d rund 0,6 U0. Daher der virtuelle Ursprung von −3,4 d (Literatur etwa +4 d) und im Fernfeld 25 bis 35 % zu wenig Geschwindigkeit. Versuch mit längerer Kernverfeinerung (10 Zellen über d bis 6 d, Radius 1 d; 500 529 Zellen): 9,2 m/s bei 3 d, 6,8 bei 5 d, B = 5,04 – nur wenig besser, also überwiegend Modell und scharfes Rechteckprofil der Zwangsbedingung (k auf der Achse schon bei 2,4 d rund 2,3 m²/s², Turbulenzgrad etwa 12 %), nicht das Netz. Die Verfeinerung wurde deshalb nicht geändert.
  - Für Folgeläufe: Die Löser-Stufe rechnet mit 24 Prozessen etwa 0,25 … 0,5 s je Iteration; von Hand im Container braucht `mpirun` als root `OMPI_ALLOW_RUN_AS_ROOT=1` und `OMPI_ALLOW_RUN_AS_ROOT_CONFIRM=1` (setzt sonst der Agent).

- 2026-11-21, Schritt 1 (Machbarkeit, von Hand im Container am Netz des Windkanal-Beispiels, 538 000 Zellen): Offene Ränder mit `U` `pressureInletOutletVelocity` und `p` `totalPressure` (p0 0), k/ω `inletOutlet`. Die Quelle als **Geschwindigkeits-Zwangsbedingung** statt einer Einlassfläche: `vectorFixedValueConstraint` mit `selectionMode geometric` und `source cylinder` (in v2606 vorhanden) – kein Gehäuse, keine neuen Patches, keine Änderung an snappyHexMesh-Regionen. Erster Versuch mit den Windkanal-Einstellungen (SIMPLEC, Relaxation 0,9) divergierte nach wenigen Iterationen (Ux-Löser 1e24, Gleitkomma-Ausnahme). Mit SIMPLE (`consistent false`), Relaxation U 0,5, k/ω 0,5, p 0,3 stabil über 150 Iterationen: Achsgeschwindigkeit 19,6 m/s von der Quelle bis kurz vor das Modell, Staupunkt am Modell. Im groben Netz lagen nur 8 Zellen in der Quelle – Verfeinerung entlang des Strahls ist Pflicht.
- Entscheidung: Ein Freistrahl ist ein Windkanal-Szenario mit Quellen (Feld `jets` des aktiven Szenarios im Manifest, mitgeführt in der Szenarienliste), keine dritte `Scenario`-Art: `WindTunnel` wird in einem bestehenden Test als Struktur-Literal gebaut, und so bleiben alle Zweige für Netz, Lauf, Kennzahlen und Bericht gültig. Mit Quellen ruht die Umgebung, die Bezugsgeschwindigkeit ist die größte Strahlgeschwindigkeit.

## Abschluss-Checkliste

Reihenfolge und Details stehen in `AGENTS.md`, Abschnitt "Abschluss- und Archivierungs-Workflow".

- [x] Alle Akzeptanzkriterien erfüllt, Tests liegen permanent in `tests/` und laufen grün
- [x] Abschnitt "Erkenntnisse für Folge-Agenten" unten ausgefüllt
- [x] Folge-Tasks und `Roadmap/README.md` geprüft und bei Bedarf angepasst
- [x] Datei ohne `RUN_` nach `Roadmap/Archiv/` verschoben

## Erkenntnisse für Folge-Agenten


Vor der Archivierung vom Agenten auszufüllen. Schreibe so, dass ein Agent, der diesen Task nie gesehen hat, davon bei späteren Tasks profitiert:

- **Umgesetzt (und Abweichungen vom Plan):** Quellen (`manifest.jets`, je Szenario) mit `scenario.set_jets`, `scenario.move_jet`, `scenario.rotate_jet` (auch MCP), `foamcraft_sim::jets` (`to_flow`, `extend_plan` mit Vergröberung des Grundnetzes, `apply_jets`, `check_jets`), Löser-Stufe ohne potentialFoam bei `system/fvOptions`, Abbruch bei Quellen erst, wenn auch die mittlere Geschwindigkeit ruhig ist (`assess_with_flow`, vom Diagnose-Agenten), Hinweis `run.jet_reference`, Quellen in der Ansicht (`jet_marker`, `jet:N`, Griffe), `JetsSection` im Szenario-Schritt. Abweichungen: keine eigene Szenario-Art "Freistrahl", sondern Windkanal mit Quellen; Quelle als Geschwindigkeits-Zwangsbedingung statt Einlassfläche auf einem Gehäuse; Rechteck mit flächengleichem Kreis angenähert.
- **Learnings & Stolpersteine:** (1) SIMPLEC mit 0,9 divergiert beim Start aus der Ruhe; SIMPLE 0,5/0,3 ist stabil. (2) potentialFoam scheitert bei ruhender Luft mit nur offenen Rändern. (3) Verfeinerung entlang des ganzen Strahls sprengt den Docker-Speicher (Exit 137) – gestuft und nur bis zur Modellmitte verfeinern; ferne Quellen vergröbern das Grundnetz. (4) **Ursache der fehlgeschlagenen Validierung:** Der Abbruch schaute nur auf Kraftbeiwerte; ein Modell abseits des Strahls hat c_D ≈ 0 und meldete nach 120 Iterationen "stabil", während die Strahlfront noch durchs Gebiet lief (Buckel in der Achsgeschwindigkeit, B = −4,7). Mit der zusätzlichen Geschwindigkeits-Bedingung: B = 5,05, monotoner Abfall. (5) Zwei gleichzeitige Testläufe auf der GPU haben einmal den Export-Test hängen lassen; einzeln und im vollen Lauf ohne Konkurrenz grün.
- **Wichtige Entscheidungen und ihre Gründe:** Zwangsbedingung statt Patch: keine neuen Patches, keine Gehäuse-Geometrie, beliebige Lage und Richtung. Toleranz der Validierung (4,0 ≤ B ≤ 7,5) vor dem ersten Lauf festgelegt und nicht verändert.
- **Auswirkungen auf Folge-Tasks:** Task_37 (Leistung): Läufe mit Quellen brauchen mehr Iterationen (bis ~880 bei Entwurf, Grenze 900) – eine eigene Iterationsgrenze für Quellen prüfen; das Abtasten der Ergebnisse nach dem Lauf braucht >4 min auf einem Kern. Task_43 (rotierende Teile) kann die Zwangsbedingung als Ausgangspunkt nehmen. Offen bleibt: Der Strahlkern endet zu früh (Achse 6,4 m/s bei 5 d statt ~U0) – Turbulenzmodell mit Rechteckprofil, nicht das Netz; für "Lüfter aus der Nähe" ist die Geschwindigkeit am Modell dadurch eher zu niedrig.
- **Hinweise zu Tests und Befehlen:** `cargo test -p foamcraft-sim --test jets`, `--test scenario_commands`; echte Läufe: `CARGO_TARGET_DIR=target/agent cargo test -p foamcraft-api --test jet_runtime -- --ignored --nocapture --test-threads 1` (runtime ~15 min, validation ~10 min). Nie zwei GPU-Testläufe gleichzeitig starten.
