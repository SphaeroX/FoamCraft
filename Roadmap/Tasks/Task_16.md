# Task_16: Stromlinien und Partikel-Animation

> ## ⚠️ PFLICHT – BEVOR DU IRGENDETWAS ANDERES TUST
>
> 1. **Lies als Allererstes die Datei `AGENTS.md` im übergeordneten Ordner `Roadmap/`** (von hier: `../AGENTS.md`) und befolge sie strikt.
> 2. **Deine allererste Aktion** ist, diese Datei in `RUN_Task_16.md` umzubenennen, um den Task als laufend zu markieren. Das passiert **noch bevor irgendein Code geschrieben** wird.

## Ziel

Die Strömung wird mit animierten Partikeln und Stromlinien sichtbar, in Echtzeit und mit frei setzbaren Startbereichen. Das ist der Kern der anschaulichen Darstellung.

## Kontext & Abhängigkeiten

- **Voraussetzung:** Task_14 ist archiviert (Task_15 liegt davor).
- **Relevantes Vorwissen:** Referenz-GPU: RTX 4080 Laptop. Die Animation muss wiederholbar sein, weil Task_19 daraus Videos rendert.

## Umfang

**Gehört dazu:**

- Partikel-Advektion auf der GPU im Geschwindigkeitsraster, mit Festkörpermaske
- Startbereiche: Fläche vor dem Modell, Einlassflächen, frei platzierbare Linie oder Fläche, nahe der Oberfläche
- Darstellung: Punkte mit Schweif, Stromlinien-Bänder, Rauch-Optik
- Färbung nach Geschwindigkeit, Druck oder Verweilzeit
- Steuerung: Tempo, Pause, Dichte
- Wiederholbarkeit über einen festen Startwert
- Befehle und Voreinstellungen ("Windkanal-Rauch", "Innenströmung")

**Gehört nicht dazu:**

- Zeitabhängige Felder (Task_27)
- Videoexport (Task_19)

## Umsetzungsschritte

1. Tests zuerst für den Integrator (gleichförmiges Feld, Starrkörperwirbel, Maske).
2. Advektion auf der GPU umsetzen.
3. Startbereiche umsetzen.
4. Darstellungsarten und Färbung umsetzen (Golden Images mit festem Startwert).
5. Steuerung, Befehle und Voreinstellungen ergänzen.
6. Leistung auf der Referenz-GPU messen und dokumentieren.

## Tests

Alle Tests dieses Tasks werden permanent in `tests/` im Hauptverzeichnis des Projekts abgelegt (siehe `AGENTS.md`).

- Im gleichförmigen Feld legt ein Partikel Geschwindigkeit mal Zeit zurück; im Starrkörperwirbel bleibt der Radius innerhalb von 1 %.
- Kein Partikel dringt in die Maske ein.
- Gleicher Startwert ergibt das gleiche Bild.
- Golden Images der Darstellungsarten.
- Leistungsmessung als ignorierter Test mit dokumentiertem Ergebnis.

## Akzeptanzkriterien

Der Task gilt erst als fertig, wenn alle Punkte erfüllt und durch Tests oder einen konkreten Befehl belegt sind:

- [ ] In der App laufen Partikel flüssig durch das Ergebnis eines Windkanal- und eines Innenströmungs-Laufs (Messwert in den Notizen)
- [ ] Startbereiche lassen sich setzen und verschieben
- [ ] Zwei Läufe der Animation mit gleichem Startwert sind bildgleich

## Offene Fragen / Annahmen

- Annahme: Zielwert 60 Bilder pro Sekunde bei mindestens 200 000 Partikeln auf der Referenz-GPU; hier nachmessen.

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
