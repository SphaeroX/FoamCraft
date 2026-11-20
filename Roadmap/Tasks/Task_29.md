# Task_29: Rotierende Teile und bewegter Boden

> ## ⚠️ PFLICHT – BEVOR DU IRGENDETWAS ANDERES TUST
>
> 1. **Lies als Allererstes die Datei `AGENTS.md` im übergeordneten Ordner `Roadmap/`** (von hier: `../AGENTS.md`) und befolge sie strikt.
> 2. **Deine allererste Aktion** ist, diese Datei in `RUN_Task_29.md` umzubenennen, um den Task als laufend zu markieren. Das passiert **noch bevor irgendein Code geschrieben** wird.

## Ziel

Der Nutzer markiert Teile als rotierend (Lüfter, Propeller, Räder) und schaltet im Windkanal einen bewegten Boden ein. Ergebnisse sind Schub, Drehmoment und Leistung.

## Kontext & Abhängigkeiten

- **Voraussetzung:** Task_10 ist archiviert (Außenströmung); die Teile-Auswahl aus Task_06 und das Szenario aus Task_08 sind vorhanden.
- **Relevantes Vorwissen:** Der bewegte Boden ist in Task_08 als Option vorgesehen. Die Vernetzung aus Task_09 muss Zellzonen für rotierende Bereiche erzeugen können.

## Umfang

**Gehört dazu:**

- Teil als rotierend markieren: Achse (Vorschlag aus der Geometrie, anpassbar) und Drehzahl
- Zwei Modelle: rotierende Wand (Räder, glatte Rotationskörper) und rotierendes Bezugssystem in einer Zone (Lüfter, Propeller); die Zone entsteht automatisch um das Teil
- Bewegter Boden im Windkanal und passende Drehzahl der Räder aus Fahrgeschwindigkeit und Radius
- Ergebnisse: Schub, Drehmoment, Leistung, Wirkungsgrad; beim Lüfter Volumenstrom und Druckaufbau
- Klartext-Hinweis zu den Grenzen des Modells (der Rotor steht im Bild still)

**Gehört nicht dazu:**

- Echte Drehung mit gleitenden Netzen

## Umsetzungsschritte

1. Testkörper erzeugen (Zylinder, einfacher Propeller).
2. Tests zuerst für den Achsvorschlag, die Umfangsgeschwindigkeit und die Drehzahl aus der Fahrgeschwindigkeit.
3. Rotierende Wand und bewegten Boden umsetzen.
4. Zonen-Erzeugung in der Vernetzung und das rotierende Bezugssystem umsetzen.
5. Auswertung (Schub, Drehmoment, Leistung) umsetzen.
6. Oberfläche, Kennzahlen und Report erweitern.

## Tests

Alle Tests dieses Tasks werden permanent in `tests/` im Hauptverzeichnis des Projekts abgelegt (siehe `AGENTS.md`).

- Achsvorschlag für Zylinder und Propeller; Umfangsgeschwindigkeit; Drehzahl aus Fahrgeschwindigkeit.
- Snapshot der Zonen-Erzeugung.
- `runtime_`: Ein rotierender Zylinder im Querstrom erzeugt Auftrieb mit dem richtigen Vorzeichen; der Propeller liefert Schub in Achsrichtung.
- Leistung gleich Drehmoment mal Winkelgeschwindigkeit.

## Akzeptanzkriterien

Der Task gilt erst als fertig, wenn alle Punkte erfüllt und durch Tests oder einen konkreten Befehl belegt sind:

- [ ] Ein Teil lässt sich in der App als rotierend markieren und rechnen
- [ ] Der bewegte Boden ist im Windkanal wählbar
- [ ] Schub, Drehmoment und Leistung stehen in den Kennzahlen

## Offene Fragen / Annahmen

- Echte Drehung mit gleitenden Netzen ist ein Wunsch für später.

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
