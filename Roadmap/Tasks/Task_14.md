# Task_14: Oberflächenfelder und Volumenraster

> ## ⚠️ PFLICHT – BEVOR DU IRGENDETWAS ANDERES TUST
>
> 1. **Lies als Allererstes die Datei `AGENTS.md` im übergeordneten Ordner `Roadmap/`** (von hier: `../AGENTS.md`) und befolge sie strikt.
> 2. **Deine allererste Aktion** ist, diese Datei in `RUN_Task_14.md` umzubenennen, um den Task als laufend zu markieren. Das passiert **noch bevor irgendein Code geschrieben** wird.

## Ziel

Nach einem Lauf sieht der Nutzer das Modell eingefärbt nach Druck, Druckbeiwert und Wandschubspannung und fragt Werte per Klick ab. Zusätzlich liegen die Volumendaten als 3D-Raster bereit, auf denen Schnitte, Isoflächen und Partikel aufbauen.

## Kontext & Abhängigkeiten

- **Voraussetzung:** Task_10 und Task_13 sind archiviert.
- **Relevantes Vorwissen:** `docs/architecture.md`, Regel 3: OpenFOAM bereitet auf, Rust liest, die GPU rechnet die Darstellung; kein VTK. Der Agent aus Task_02 ist der einzige Zugriff auf Falldateien. Referenz-GPU: 12 GB.

## Umfang

**Gehört dazu:**

- Extraktion im Runtime: Wandflächen mit Feldern (Druck, Druckbeiwert, Wandschubspannung, y+) als kompakte Binärdaten; der Weg (Function Objects oder direktes Lesen) wird begründet gewählt
- Volumenfelder (Geschwindigkeit, Druck, Gesamtdruck, Turbulenzgröße) auf ein mehrstufiges regelmäßiges Raster abgetastet: grob für das ganze Gebiet, fein um den Körper; Maske für Festkörper
- Übertragung zum Host und Cache im Projekt mit Größenbegrenzung
- Renderer: Feld auf der Oberfläche mit Farbskala (gleichmäßig wahrnehmbar, für Farbfehlsichtige geeignet), Legende mit Einheit, Bereich automatisch oder von Hand, Wert am Mauspunkt
- Befehle `results.list_fields`, `results.probe`, `view.set_surface_field`
- Grundgerüst des Schritts "Ergebnisse" in der Oberfläche

**Gehört nicht dazu:**

- Schnitte und Isoflächen (Task_15), Partikel (Task_16), Kennzahlen-Tafel (Task_17)

## Umsetzungsschritte

1. Kleine OpenFOAM-Felddateien aufzeichnen (ASCII und binär) und Tests für den Leser schreiben.
2. Leser und Extraktion der Wandflächen umsetzen.
3. Test zuerst für die Abtastung: Ein analytisches Feld auf einem Testnetz wird mit begrenztem Fehler abgetastet.
4. Mehrstufiges Raster, Maske, Übertragung und Cache umsetzen.
5. Farbskala, Legende und Wert-Abfrage im Renderer umsetzen (Golden Images).
6. Befehle und den Schritt "Ergebnisse" ergänzen.
7. Speicherbudget für die Raster festlegen und dokumentieren.

## Tests

Alle Tests dieses Tasks werden permanent in `tests/` im Hauptverzeichnis des Projekts abgelegt (siehe `AGENTS.md`).

- Leser: ASCII und binär, einheitliche und nicht einheitliche Felder.
- Abtastfehler für das analytische Feld unter der festgelegten Schwelle; Maske stimmt für einen Würfel.
- Cache übersteht Speichern und Öffnen; das Speicherbudget wird eingehalten.
- Golden Image des eingefärbten Würfels.
- `validation_` am Würfelfall: Der Druckbeiwert am Staupunkt liegt bei 1 (Toleranz 0,1).

## Akzeptanzkriterien

Der Task gilt erst als fertig, wenn alle Punkte erfüllt und durch Tests oder einen konkreten Befehl belegt sind:

- [ ] Nach einem Lauf ist das Modell in der App nach Druck eingefärbt, mit Legende
- [ ] `results.probe` liefert für einen Punkt auf der Oberfläche den Feldwert mit Einheit
- [ ] Die Volumenraster liegen im Cache und bleiben unter dem dokumentierten Speicherbudget

## Offene Fragen / Annahmen

- Annahme: drei Rasterstufen mit begrenzter Auflösung. Sehr feine Details werden damit ungenau dargestellt; die App weist darauf hin.

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
