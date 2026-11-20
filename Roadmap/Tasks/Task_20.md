# Task_20: Report als PDF und HTML

> ## ⚠️ PFLICHT – BEVOR DU IRGENDETWAS ANDERES TUST
>
> 1. **Lies als Allererstes die Datei `AGENTS.md` im übergeordneten Ordner `Roadmap/`** (von hier: `../AGENTS.md`) und befolge sie strikt.
> 2. **Deine allererste Aktion** ist, diese Datei in `RUN_Task_20.md` umzubenennen, um den Task als laufend zu markieren. Das passiert **noch bevor irgendein Code geschrieben** wird.

## Ziel

Der Nutzer exportiert zu einem Lauf einen verständlichen Bericht als PDF und als eigenständige HTML-Datei: Aufbau, Netz, Konvergenz, Kennzahlen, Bilder und die Grenzen der Aussagekraft. Danach ist Meilenstein 2 erreicht.

## Kontext & Abhängigkeiten

- **Voraussetzung:** Task_17 (Kennzahlen, Ampel) und Task_19 (Bildexport) sind archiviert.
- **Relevantes Vorwissen:** `docs/architecture.md` (PDF mit Typst), `docs/licensing.md` (eigenes Layout, nicht an fremde Reports anlehnen; Markenhinweis im Bericht; Schriften nur mit erlaubter Lizenz).

## Umfang

**Gehört dazu:**

- Bericht-Modell aus Projekt und Lauf
- Abschnitte: Deckblatt, Zusammenfassung in Klartext, Aufbau (Geometrie, Maße, Szenario, Medium), Netz, Konvergenz mit Diagrammen, Kennzahlen mit Erklärung, automatisch gerenderte Standardansichten, Verlässlichkeits-Ampel, Grenzen der Aussagekraft, Versionsangaben, Markenhinweis
- PDF über eine eigene Typst-Vorlage mit eingebetteter freier Schrift; HTML als eine Datei ohne externe Verweise
- Englisch und Deutsch
- Vergleichsbericht für zwei Läufe
- Befehl `export.report`, MCP-Werkzeug, Berichtsdialog (Abschnitte wählen, Titel, Autor, optionales Logo)

**Gehört nicht dazu:**

- Frei gestaltbare Vorlagen durch den Nutzer

## Umsetzungsschritte

1. Tests zuerst für das Bericht-Modell aus einem Beispiel-Ergebnis.
2. Bericht-Modell und Standardansichten umsetzen.
3. Typst-Vorlage und PDF-Erzeugung umsetzen.
4. HTML-Erzeugung umsetzen.
5. Vergleichsbericht umsetzen.
6. Befehl, MCP-Werkzeug und Dialog ergänzen.
7. Meilenstein 2: Ergebnisse, MCP, Video und Report dem Nutzer zur Abnahme melden.

## Tests

Alle Tests dieses Tasks werden permanent in `tests/` im Hauptverzeichnis des Projekts abgelegt (siehe `AGENTS.md`).

- Das Bericht-Modell ist für das Beispiel-Ergebnis vollständig.
- Das PDF hat Seiten und enthält die Schlüsselwerte und den Markenhinweis (Textextraktion).
- Das HTML enthält keine externen Verweise.
- Beide Sprachen sind vollständig.
- Fehlt der 3D-Cache, entsteht ein Bericht ohne Bilder mit Hinweis statt eines Fehlers.

## Akzeptanzkriterien

Der Task gilt erst als fertig, wenn alle Punkte erfüllt und durch Tests oder einen konkreten Befehl belegt sind:

- [ ] `export.report` erzeugt PDF und HTML in Englisch und Deutsch
- [ ] Der Bericht enthält Verlässlichkeits-Ampel, Grenzen der Aussagekraft und Markenhinweis
- [ ] Ein MCP-Client kann den Bericht anfordern
- [ ] Der Nutzer wurde über das Erreichen von Meilenstein 2 informiert

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
