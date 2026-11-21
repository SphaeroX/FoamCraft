# Task_47: Sofort-Vorschau in der App

> ## ⚠️ PFLICHT – BEVOR DU IRGENDETWAS ANDERES TUST
>
> 1. **Lies als Allererstes die Datei `AGENTS.md` im übergeordneten Ordner `Roadmap/`** (von hier: `../AGENTS.md`) und befolge sie strikt.
> 2. **Deine allererste Aktion** ist, diese Datei in `RUN_Task_47.md` umzubenennen, um den Task als laufend zu markieren. Das passiert **noch bevor irgendein Code geschrieben** wird.

## Ziel

Der Nutzer sieht die Strömung wenige Sekunden nach dem Einrichten live in der 3D-Ansicht und kann Geschwindigkeit und Richtung ändern, während die Vorschau läuft. Ein Knopf übergibt das Szenario an die genaue OpenFOAM-Rechnung.

## Kontext & Abhängigkeiten

- **Voraussetzung:** Task_16 (Partikel, Schnitte auf Rastern) und Task_46 (GPU-Löser) sind archiviert.
- **Relevantes Vorwissen:** Der GPU-Löser und der Renderer nutzen dieselbe GPU; Felder sollen ohne Umweg über Dateien in die Ansicht gelangen. Die Vorschau ist eine Schätzung und muss als solche erkennbar sein.

## Umfang

**Gehört dazu:**

- Modus "Vorschau" neben "Genau": startet den GPU-Löser direkt aus dem Szenario, ohne Container
- Felder des Lösers als gemeinsame GPU-Texturen im Renderer; Partikel und Schnitte live während der Rechnung
- Live-Kennzahlen (gleitend gemittelt) mit der Kennzeichnung "Schätzung"
- Änderungen von Geschwindigkeit und Richtung wirken ohne Neustart
- Regler zwischen Auflösung und Bildrate
- Genauigkeitshinweis und Knopf "Jetzt genau rechnen"
- Hinweis, wenn die GPU ungeeignet ist
- MCP-Werkzeug für die Vorschau
- Anzeige des Unterschieds zwischen Vorschau und genauer Rechnung, wenn beide vorliegen

**Gehört nicht dazu:**

- Ersatz der OpenFOAM-Rechnung durch die Vorschau

## Umsetzungsschritte

1. Tests zuerst für die Kopplung (Offscreen-Bild nach einer festen Schrittzahl, wiederholbar) und die Glättung der Kennzahlen.
2. Gemeinsame Texturen zwischen Löser und Renderer umsetzen.
3. Modus, Live-Kennzahlen und Regler in der Oberfläche umsetzen (Vitest zuerst).
4. Änderungen im laufenden Betrieb umsetzen.
5. Übergabe an die genaue Rechnung und die Vergleichsanzeige umsetzen.
6. MCP-Werkzeug ergänzen.
7. Leistung und Abweichung zur OpenFOAM-Rechnung für die Vergleichsfälle messen und dokumentieren.

## Tests

Alle Tests dieses Tasks werden permanent in `tests/` im Hauptverzeichnis des Projekts abgelegt (siehe `AGENTS.md`).

- Kopplung: Das Offscreen-Bild nach N Schritten ist wiederholbar.
- Eine Parameteränderung wirkt, ohne dass das Feld neu initialisiert wird.
- Glättung der Kennzahlen an synthetischen Signalen.
- Vitest: Umschalten des Modus, Genauigkeitshinweis ist sichtbar.
- Abweichung der Vorschau zur OpenFOAM-Rechnung für die Vergleichsfälle innerhalb von 30 %; die Messwerte stehen in der Doku.

## Akzeptanzkriterien

Der Task gilt erst als fertig, wenn alle Punkte erfüllt und durch Tests oder einen konkreten Befehl belegt sind:

- [ ] In der App läuft die Vorschau für beide Grundfälle live mit Partikeln (Bildrate in den Notizen)
- [ ] Geschwindigkeit und Richtung lassen sich während der Vorschau ändern
- [ ] "Jetzt genau rechnen" startet die OpenFOAM-Rechnung mit demselben Szenario
- [ ] Die Vorschau ist in Ansicht, Kennzahlen und Report als Schätzung gekennzeichnet

## Offene Fragen / Annahmen

- Die Toleranz von 30 % zur OpenFOAM-Rechnung ist eine Annahme und wird hier mit Messwerten bestätigt oder begründet angepasst.

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
