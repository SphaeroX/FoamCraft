# Task_11: Lauf-Monitor in der App

> ## ⚠️ PFLICHT – BEVOR DU IRGENDETWAS ANDERES TUST
>
> 1. **Lies als Allererstes die Datei `AGENTS.md` im übergeordneten Ordner `Roadmap/`** (von hier: `../AGENTS.md`) und befolge sie strikt.
> 2. **Deine allererste Aktion** ist, diese Datei in `RUN_Task_11.md` umzubenennen, um den Task als laufend zu markieren. Das passiert **noch bevor irgendein Code geschrieben** wird.

## Ziel

Der Nutzer startet einen Lauf in der App und verfolgt ihn: Stufen, Live-Kurven, Restzeit, Abbruch. Schlägt etwas fehl, bekommt er eine Erklärung in Klartext mit einem Vorschlag, was zu tun ist.

## Kontext & Abhängigkeiten

- **Voraussetzung:** Task_10 ist archiviert.
- **Relevantes Vorwissen:** Fehlercodes aus Task_02, Task_03, Task_09 und Task_10; `docs/code-style.md` (kein fest verdrahteter Text).

## Umfang

**Gehört dazu:**

- Schritt "Rechnen": Zusammenfassung vor dem Start (Zellbudget, geschätzte Dauer, Kerne), Stufen-Zeitleiste, Live-Diagramme für Residuen und Beiwerte, Restzeit, Abbruch, aufklappbares Log, Abschlusskarte mit Kennzahlen
- Klartext-Diagnose mit Handlungsvorschlag für jeden bekannten Fehlercode, in beiden Sprachen
- Wiederverbinden mit einem laufenden Job nach einem Neustart der App
- Rechner während des Laufs wach halten; Benachrichtigung am Ende

**Gehört nicht dazu:**

- 3D-Ergebnisansicht (Task_14)
- Kennzahlen-Tafel und Laufvergleich (Task_17)

## Umsetzungsschritte

1. Vitest zuerst: Diagramme und Zeitleiste mit einem simulierten Ereignisstrom.
2. Schritt "Rechnen" umsetzen.
3. Test zuerst für den Restzeit-Schätzer (Rust, synthetische Verläufe), dann umsetzen.
4. Register der Fehlercodes mit Texten und Vorschlägen füllen; Vollständigkeitstest schreiben.
5. Wiederverbinden, Wachhalten und Benachrichtigung umsetzen.
6. Mit einem echten Lauf abnehmen und das Ergebnis dokumentieren.

## Tests

Alle Tests dieses Tasks werden permanent in `tests/` im Hauptverzeichnis des Projekts abgelegt (siehe `AGENTS.md`).

- Vitest: Diagramme zeigen eingehende Werte; Zustände (läuft, fertig, Fehler, abgebrochen) werden richtig dargestellt; Abbruch ruft `run.cancel`.
- Vollständigkeitstest: Jeder Fehlercode im Register hat englischen und deutschen Text und einen Vorschlag.
- Restzeit-Schätzer mit synthetischen Verläufen.
- Wiederverbinden mit einem laufenden Job (Fake-Runner).

## Akzeptanzkriterien

Der Task gilt erst als fertig, wenn alle Punkte erfüllt und durch Tests oder einen konkreten Befehl belegt sind:

- [ ] Ein Windkanal-Lauf lässt sich in der App starten, verfolgen und abbrechen (Bildschirmfoto in den Notizen)
- [ ] Nach einem Neustart der App während eines Laufs zeigt der Monitor den Lauf wieder an
- [ ] Der Vollständigkeitstest der Fehlertexte läuft grün

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
