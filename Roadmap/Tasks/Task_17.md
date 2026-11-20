# Task_17: Kennzahlen und Laufvergleich

> ## ⚠️ PFLICHT – BEVOR DU IRGENDETWAS ANDERES TUST
>
> 1. **Lies als Allererstes die Datei `AGENTS.md` im übergeordneten Ordner `Roadmap/`** (von hier: `../AGENTS.md`) und befolge sie strikt.
> 2. **Deine allererste Aktion** ist, diese Datei in `RUN_Task_17.md` umzubenennen, um den Task als laufend zu markieren. Das passiert **noch bevor irgendein Code geschrieben** wird.

## Ziel

Der Nutzer sieht alle Kennzahlen eines Laufs mit Erklärung und einer Einschätzung der Verlässlichkeit, verwaltet mehrere Läufe und vergleicht zwei davon direkt.

## Kontext & Abhängigkeiten

- **Voraussetzung:** Task_14 ist archiviert (Task_15 und Task_16 liegen davor).
- **Relevantes Vorwissen:** Ergebnisobjekte aus Task_10 und Task_13. Die Antwort von `results.summary` wird später vom MCP-Server (Task_18) und vom Report (Task_20) genutzt und muss stabil bleiben.

## Umfang

**Gehört dazu:**

- Kennzahlen-Tafel Außenströmung: Kräfte gesamt und je Teil (Druck- und Reibungsanteil), Beiwerte, Momente, Widerstandsfläche, Leistungsbedarf
- Kennzahlen-Tafel Innenströmung: Druckverlust, Durchfluss je Öffnung, Verlustbeiwert
- Erklärtext in Klartext je Kennzahl, in beiden Sprachen
- Verlässlichkeits-Ampel (Konvergenz, Netzqualität, y+, Massenbilanz) mit Begründung
- Läufe verwalten: Liste, umbenennen, als Variante duplizieren, löschen mit Speicherfreigabe
- Vergleich zweier Läufe: Tabelle mit absoluter und relativer Differenz, geteilte Ansicht mit gekoppelter Kamera, Differenzfeld bei gleicher Geometrie
- Befehle `results.summary` und `results.compare`; Export der Kennzahlen als CSV und JSON

**Gehört nicht dazu:**

- Report (Task_20)

## Umsetzungsschritte

1. Tests zuerst für die abgeleiteten Kennzahlen (gegen Handrechnung) und die Ampel-Logik.
2. `results.summary` mit stabilem Schema umsetzen (Snapshot).
3. Lauf-Verwaltung umsetzen.
4. `results.compare` und die geteilte Ansicht umsetzen.
5. Tafel, Vergleichstabelle und Export in der Oberfläche umsetzen (Vitest zuerst).

## Tests

Alle Tests dieses Tasks werden permanent in `tests/` im Hauptverzeichnis des Projekts abgelegt (siehe `AGENTS.md`).

- Widerstandsfläche, Leistungsbedarf und Verlustbeiwert gegen Handrechnung.
- Ampel-Logik für gute, grenzwertige und schlechte Läufe.
- Differenzen im Vergleich; Löschen gibt Speicher frei und lässt das Projekt konsistent.
- Snapshot des Schemas von `results.summary`.
- Vitest für Tafel und Vergleichstabelle.

## Akzeptanzkriterien

Der Task gilt erst als fertig, wenn alle Punkte erfüllt und durch Tests oder einen konkreten Befehl belegt sind:

- [ ] `results.summary` liefert alle Kennzahlen mit Einheit und Ampel
- [ ] Zwei Läufe lassen sich in der App nebeneinander mit Differenz vergleichen
- [ ] Jede Kennzahl hat einen Erklärtext in beiden Sprachen

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
