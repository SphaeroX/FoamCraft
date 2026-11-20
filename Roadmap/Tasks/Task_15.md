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

- [ ] Eine Schnittebene lässt sich in der App ziehen und zeigt das gewählte Feld
- [ ] Isoflächen für Gesamtdruck und Q-Kriterium sind einblendbar
- [ ] Eine gespeicherte Szene wird nach dem Öffnen des Projekts gleich dargestellt

## Offene Fragen / Annahmen

- Keine

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
