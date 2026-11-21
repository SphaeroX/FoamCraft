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

- [x] Ein Windkanal-Lauf lässt sich in der App starten, verfolgen und abbrechen (Bildschirmfoto in den Notizen)
- [x] Nach einem Neustart der App während eines Laufs zeigt der Monitor den Lauf wieder an
- [x] Der Vollständigkeitstest der Fehlertexte läuft grün

## Offene Fragen / Annahmen

- Keine

## Notizen zu diesem Task

Taskspezifische Informationen während der Arbeit hier festhalten. Taskübergreifendes gehört in `Roadmap/README.md`.

- Abnahme 2026-11-21 mit dem Demo-Projekt (519 195 Zellen, Entwurf, 24 Kerne): Start, Stufen, Live-Kurven und Restzeit (`Roadmap/Nachweise/task11_lauf_monitor.png`); App während des Laufs beendet und neu gestartet → Monitor zeigt den Lauf wieder ("Läuft", `task11_wiederverbunden.png`), Ergebniskarte Cd 0,779 / Cl 0,411, Grund "stabil", Hinweis y+ < 30 (`task11_ergebnis.png`); zweiter Lauf nach ~25 s abgebrochen → "Abgebrochen", Container beendet (`task11_abgebrochen.png`).
- Benachrichtigung am Ende: Taskleisten-Hinweis (`request_user_attention`) statt System-Benachrichtigung, ohne neue Abhängigkeit.

## Abschluss-Checkliste

Reihenfolge und Details stehen in `AGENTS.md`, Abschnitt "Abschluss- und Archivierungs-Workflow".

- [x] Alle Akzeptanzkriterien erfüllt, Tests liegen permanent in `tests/` und laufen grün
- [x] Abschnitt "Erkenntnisse für Folge-Agenten" unten ausgefüllt
- [x] Folge-Tasks und `Roadmap/README.md` geprüft und bei Bedarf angepasst
- [x] Datei ohne `RUN_` nach `Roadmap/Archiv/` verschoben

## Erkenntnisse für Folge-Agenten

- **Umgesetzt (und Abweichungen vom Plan):** `RunPanel` + `LineChart` (Schritt "Rechnen"), `subscribeApiEvents` in `app/src/api.ts`, Befehle `run.active`, `run.attach`, `run.log`, `error.describe`, Restzeit-Schätzer `remaining_seconds`, `solve.json` je Solve-Job, Wachhalten/Taskleisten-Hinweis im Tauri-Bridge (`app/src-tauri/src/api.rs`). Abweichung: keine OS-Benachrichtigung (siehe Notizen).
- **Learnings & Stolpersteine:** `api_call` sperrt die Session für die Dauer eines Befehls – lange Befehle (`run.start` ohne Netz vernetzt erst blockierend) halten andere Aufrufe auf; Ereignisse laufen trotzdem. Nach `Stop-Process foamcraft-app` beendet sich `tauri dev` mit Fehler; einfach `npm run app:dev` neu starten. Beim Wiederverbinden zeigen die Kurven erst ab dem Wiederverbinden (die Wiederholung aus `job.log` sendet keine Ereignisse); die Dauer im Ergebnis zählt dann nur die verfolgten Stufen.
- **Wichtige Entscheidungen und ihre Gründe:** Diagramme als eigenes SVG (keine Lizenzprüfung, klein). Texte zu Codes nur aus dem Fehlerkatalog über `error.describe` (eine Quelle). Wachhalten ohne Abhängigkeit über `SetThreadExecutionState` pro Fortschrittsereignis.
- **Auswirkungen auf Folge-Tasks:** Task_12/13 (Abnahme M1) können den Schritt "Rechnen" nutzen; Task_14 hängt die 3D-Ergebnisse an die Ergebniskarte bzw. `run.result` an. Task_17 (Kennzahlen-Tafel) kann `WindTunnelResult.history` und `parts` zeigen. Die Ergebniskarte zeigt noch keine Teile und kein y+-Detail.
- **Hinweise zu Tests und Befehlen:** `npx vitest run tests/app/run-panel.test.tsx` (7 Tests, simulierter Ereignisstrom), `cargo test -p foamcraft-api --test solve --test error_catalog`, `cargo test -p foamcraft-sim --test solve` (Restzeit). Bedienung per Skript: `tools/windows/app-ui.ps1 -Steps "click:x,y;wait:ms;wheel:x,y,delta"` und `app-shot.ps1`; Schritt "Rechnen" liegt bei (365,62), "Rechnung starten" bei (93,272) im 1296×839-Fenster.
