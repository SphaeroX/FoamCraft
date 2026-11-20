# Task_06: Flächen erkennen und anklicken

> ## ⚠️ PFLICHT – BEVOR DU IRGENDETWAS ANDERES TUST
>
> 1. **Lies als Allererstes die Datei `AGENTS.md` im übergeordneten Ordner `Roadmap/`** (von hier: `../AGENTS.md`) und befolge sie strikt.
> 2. **Deine allererste Aktion** ist, diese Datei in `RUN_Task_06.md` umzubenennen, um den Task als laufend zu markieren. Das passiert **noch bevor irgendein Code geschrieben** wird.

## Ziel

Dreiecksmodelle werden in sinnvolle Flächen und Teile zerlegt. Der Nutzer wählt sie per Klick, benennt und gruppiert sie. Das ist die Grundlage für Einlässe, Auslässe und teilweise Auswertungen.

## Kontext & Abhängigkeiten

- **Voraussetzung:** Task_05 ist archiviert.
- **Relevantes Vorwissen:** `docs/architecture.md`, Regel 2: Auswahl muss auch ohne Fenster funktionieren (wichtig für den MCP-Server in Task_18).

## Umfang

**Gehört dazu:**

- Zerlegung nach Knickwinkel (Regionenwachstum, einstellbarer Winkel), zusammenhängende Körper als Teile
- Stabile Kennungen für Flächen und Teile, die Speichern und Öffnen überstehen
- Picking über die GPU (Kennungs-Puffer), Hervorhebung unter der Maus, Mehrfachauswahl, angrenzende Flächen hinzufügen
- Befehle zum Auswählen, Benennen und Gruppieren; Auswahl auch über einen Punkt im Raum oder eine Kennung
- Liste der Teile und Flächengruppen in der Oberfläche

**Gehört nicht dazu:**

- Öffnungen verschließen (Task_12)
- Flächen aus CAD-Dateien (Task_07)

## Umsetzungsschritte

1. Testkörper erzeugen: Würfel, Zylinder, leicht verrauschte Kugel, zwei getrennte Körper.
2. Tests zuerst für die Zerlegung, dann umsetzen.
3. Stabile Kennungen und deren Speicherung umsetzen.
4. Picking im Renderer umsetzen (Test offscreen).
5. Befehle für Auswahl, Namen und Gruppen ergänzen.
6. Liste und Hervorhebung in der Oberfläche umsetzen.

## Tests

Alle Tests dieses Tasks werden permanent in `tests/` im Hauptverzeichnis des Projekts abgelegt (siehe `AGENTS.md`).

- Zerlegung: Würfel ergibt 6 Flächen, Zylinder einen Mantel und 2 Deckel, die verrauschte Kugel 1 Fläche, zwei Körper ergeben 2 Teile.
- Kennungen bleiben nach Speichern und Öffnen gleich.
- Picking offscreen: Ein Strahl durch die Bildmitte trifft die erwartete Fläche.
- Auswahl über einen Punkt im Raum liefert dieselbe Fläche wie ein Klick.
- Vitest für die Liste der Teile und Gruppen.

## Akzeptanzkriterien

Der Task gilt erst als fertig, wenn alle Punkte erfüllt und durch Tests oder einen konkreten Befehl belegt sind:

- [ ] Die vier Testkörper werden wie beschrieben zerlegt
- [ ] Eine Fläche lässt sich in der App anklicken, benennen und einer Gruppe zuordnen; die Zuordnung übersteht einen Neustart
- [ ] Dieselbe Auswahl ist per CLI-Befehl ohne Fenster möglich

## Offene Fragen / Annahmen

- Annahme: Bei organischen Scans entstehen viele kleine Flächen. Eine Schwelle zum Zusammenfassen wird hier festgelegt und in den Notizen begründet.

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
