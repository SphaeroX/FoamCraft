# Task_12: Öffnungen erkennen und verschließen

> ## ⚠️ PFLICHT – BEVOR DU IRGENDETWAS ANDERES TUST
>
> 1. **Lies als Allererstes die Datei `AGENTS.md` im übergeordneten Ordner `Roadmap/`** (von hier: `../AGENTS.md`) und befolge sie strikt.
> 2. **Deine allererste Aktion** ist, diese Datei in `RUN_Task_12.md` umzubenennen, um den Task als laufend zu markieren. Das passiert **noch bevor irgendein Code geschrieben** wird.

## Ziel

Für die Innenströmung klickt der Nutzer eine Öffnung an; FoamCraft verschließt sie mit einer Deckfläche und markiert sie als Einlass oder Auslass. Das funktioniert für Hohlkörper mit Öffnungen und für Modelle, die direkt das Fluidvolumen beschreiben.

## Kontext & Abhängigkeiten

- **Voraussetzung:** Task_06 ist archiviert (Flächen, Auswahl). Task_07 bis Task_11 liegen in der Reihenfolge davor.
- **Relevantes Vorwissen:** `docs/project-brief.md`: Verschließen von Öffnungen ist erlaubt, weitergehende CAD-Bearbeitung nicht.

## Umfang

**Gehört dazu:**

- Erkennung der Geometrieart: offene Schale mit Randschleifen, dickwandiger Hohlkörper mit Durchgängen, geschlossenes Fluidvolumen
- Randschleifen finden und ebene Deckflächen erzeugen, auch für nicht konvexe und ringförmige Öffnungen; Warnung bei nicht ebenen Schleifen
- Dickwandige Körper: Deckfläche an der Mündung einer Bohrung nach Klick auf Innenfläche oder Kante
- Rollen je Fläche: Einlass, Auslass, Wand
- Befehle `geometry.cap_opening` und `scenario.set_boundary_role`
- Punkt im Fluidraum für die spätere Vernetzung
- Dichtheitsprüfung nach dem Verschließen (Flutungstest auf einem Voxelgitter)
- Ansicht: Öffnungen hervorgehoben, Deckflächen halbtransparent, Pfeile für die Rolle

**Gehört nicht dazu:**

- Randwerte und Rechnung (Task_13)

## Umsetzungsschritte

1. Testkörper erzeugen: offenes Rohrstück, T-Stück, Gehäuse mit Wandstärke und zwei Bohrungen, Ringspalt, Rohr mit schräg-unebener Öffnung.
2. Tests zuerst für Randschleifen und Deckflächen, dann umsetzen.
3. Erkennung der Geometrieart und den Fall "dickwandig" umsetzen.
4. Rollen und Befehle umsetzen.
5. Flutungstest und Punkt im Fluidraum umsetzen.
6. Darstellung und Bedienung in der Oberfläche umsetzen.

## Tests

Alle Tests dieses Tasks werden permanent in `tests/` im Hauptverzeichnis des Projekts abgelegt (siehe `AGENTS.md`).

- Rohrstück: 2 Schleifen, 2 Deckflächen, Flächeninhalt innerhalb von 1 % von π·r².
- T-Stück: 3 Öffnungen; Gehäuse mit Bohrungen: 2 Öffnungen; Ringspalt wird korrekt gedeckelt.
- Nicht ebene Öffnung liefert eine Warnung.
- Der Flutungstest erkennt ein Leck, wenn eine Öffnung unverschlossen bleibt.
- Der Punkt im Fluidraum liegt im Inneren; Kennungen bleiben nach Speichern und Öffnen gleich.

## Akzeptanzkriterien

Der Task gilt erst als fertig, wenn alle Punkte erfüllt und durch Tests oder einen konkreten Befehl belegt sind:

- [ ] Alle Testkörper lassen sich per Befehl vollständig verschließen und bestehen den Flutungstest
- [ ] In der App wird eine Öffnung per Klick verschlossen und als Einlass oder Auslass markiert
- [ ] Warnungen und Fehler haben Texte in beiden Sprachen

## Offene Fragen / Annahmen

- Die genaue Bedienung bei dickwandigen Körpern (Klick auf Innenfläche oder Kante) wird hier festgelegt.

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
