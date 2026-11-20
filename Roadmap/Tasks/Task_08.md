# Task_08: Windkanal einrichten

> ## ⚠️ PFLICHT – BEVOR DU IRGENDETWAS ANDERES TUST
>
> 1. **Lies als Allererstes die Datei `AGENTS.md` im übergeordneten Ordner `Roadmap/`** (von hier: `../AGENTS.md`) und befolge sie strikt.
> 2. **Deine allererste Aktion** ist, diese Datei in `RUN_Task_08.md` umzubenennen, um den Task als laufend zu markieren. Das passiert **noch bevor irgendein Code geschrieben** wird.

## Ziel

Der Nutzer richtet eine Außenumströmung ein: Anströmgeschwindigkeit, Richtung, Boden, Medium und Qualitätsstufe. Tunnelgröße und Bezugsgrößen berechnet FoamCraft selbst.

## Kontext & Abhängigkeiten

- **Voraussetzung:** Task_06 ist archiviert (Task_07 liegt in der Reihenfolge davor und ist ebenfalls archiviert).
- **Relevantes Vorwissen:** `docs/architecture.md`, Regel 4 (physikalische Eingaben, erzeugte Dictionaries). Referenzrechner: 64 GB Arbeitsspeicher, 24 Kerne (8 P-Kerne, 16 E-Kerne).

## Umfang

**Gehört dazu:**

- Szenario "Windkanal" im Projekt, Befehl `scenario.set_wind_tunnel`
- Medien-Vorgaben (Luft und Wasser bei einer Temperatur ergeben Dichte und Viskosität) und frei definierbare Medien
- Boden: kein Boden, fester Boden; bewegter Boden als Option vorgesehen, gerechnet wird er in Task_29
- Automatische Tunnelgröße aus den Abmessungen des Modells nach einer dokumentierten Regel
- Bezugsfläche (projizierte Stirnfläche in Anströmrichtung) und Bezugslänge
- Reynolds-Zahl mit Einordnung in Klartext
- Qualitätsstufen (Entwurf, Standard, Fein) mit Zellbudget aus Arbeitsspeicher und Kernen, grobe Zeitschätzung
- Ansicht: Tunnel als Box, Strömungspfeil, Boden
- Warnungen: Mach-Zahl über 0,3 liegt außerhalb des Gültigkeitsbereichs; winziges oder riesiges Modell deutet auf eine falsche Einheit

**Gehört nicht dazu:**

- Vernetzung (Task_09) und Rechnung (Task_10)

## Umsetzungsschritte

1. Tests zuerst für die Stirnfläche (Kugel, Würfel, gedrehter Würfel gegen die analytischen Werte).
2. Szenario-Modell und Befehl umsetzen.
3. Regeln für Tunnelgröße und Zellbudget festlegen, dokumentieren und testen.
4. Reynolds-Zahl, Mach-Warnung und Einheiten-Warnung umsetzen.
5. Tunnel, Pfeil und Boden in der Ansicht darstellen.
6. Schritt "Szenario" in der Oberfläche umsetzen (Vitest zuerst).

## Tests

Alle Tests dieses Tasks werden permanent in `tests/` im Hauptverzeichnis des Projekts abgelegt (siehe `AGENTS.md`).

- Stirnfläche innerhalb von 1 % der analytischen Werte.
- Tunnelmaße folgen der Regel; Zellbudget für 64 GB und für 8 GB Arbeitsspeicher.
- Reynolds-Zahl, Mach-Warnung, Einheiten-Warnung.
- Das Szenario übersteht Speichern und Öffnen.
- Vitest: Das Formular prüft Eingaben und zeigt Warnungen; Golden Image mit Tunnel.

## Akzeptanzkriterien

Der Task gilt erst als fertig, wenn alle Punkte erfüllt und durch Tests oder einen konkreten Befehl belegt sind:

- [ ] `scenario.set_wind_tunnel` speichert ein vollständiges Szenario und liefert Bezugsfläche, Reynolds-Zahl und Zellbudget
- [ ] Die Regeln für Tunnelgröße und Zellbudget stehen in `docs/` und sind durch Tests belegt
- [ ] Der Tunnel ist in der App sichtbar und folgt Änderungen der Richtung

## Offene Fragen / Annahmen

- Annahme: Tunnelgröße nach üblichen Richtwerten (etwa 5 Modelllängen davor, 15 dahinter, 5 seitlich und darüber). Task_10 prüft sie am Vergleichsfall.
- Annahme: Faustwert für den Speicherbedarf je Million Zellen; in Task_09 und Task_23 nachmessen.

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
