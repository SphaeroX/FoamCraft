# Task_24: GPU-Linearlöser (experimentell)

> ## ⚠️ PFLICHT – BEVOR DU IRGENDETWAS ANDERES TUST
>
> 1. **Lies als Allererstes die Datei `AGENTS.md` im übergeordneten Ordner `Roadmap/`** (von hier: `../AGENTS.md`) und befolge sie strikt.
> 2. **Deine allererste Aktion** ist, diese Datei in `RUN_Task_24.md` umzubenennen, um den Task als laufend zu markieren. Das passiert **noch bevor irgendein Code geschrieben** wird.

## Ziel

Der Drucklöser von OpenFOAM lässt sich auf die GPU auslagern. Ob und ab welcher Netzgröße das auf dem Referenzrechner schneller ist, steht gemessen in der Doku; die App empfiehlt die Option nur dort, wo sie gewinnt.

## Kontext & Abhängigkeiten

- **Voraussetzung:** Task_23 ist archiviert (Messung und beste CPU-Werte).
- **Relevantes Vorwissen:** `docs/architecture.md`, offene Entscheidung "GPU linear solver"; `docs/licensing.md`, Abschnitt "GPU runtime variant" (offene Lizenzfrage zu CUDA-Bestandteilen). Stand bei Anlage der Roadmap: Es gibt keine offizielle GPU-Version von OpenFOAM, nur die Auslagerung des linearen Lösers über Ginkgo (OGL) oder AmgX (petsc4Foam). Referenz-GPU: 12 GB.

## Umfang

**Gehört dazu:**

- Recherche des aktuellen Stands für die gepinnte OpenFOAM-Version und Wahl des Plugins
- Klärung der Lizenzfrage; vor der Veröffentlichung eines GPU-Images entscheidet der Nutzer
- GPU-Variante des Runtime-Images
- Schalter "GPU-Beschleunigung (experimentell)"
- Prüfung des GPU-Speichers vor dem Start, automatischer Rückfall auf die CPU mit Hinweis
- Messung gegen die besten CPU-Werte bei mehreren Netzgrößen
- Empfehlungsregel in der App
- Gleichheit der Ergebnisse: Kennzahlen innerhalb einer Toleranz zur CPU-Rechnung

**Gehört nicht dazu:**

- Eigener GPU-Löser (Task_32)
- GPUs anderer Hersteller, sofern das gewählte Plugin sie nicht ohne Mehraufwand unterstützt

## Umsetzungsschritte

1. Stand recherchieren (Repositories und Doku der Plugins lesen), Plugin wählen und begründen.
2. Lizenzfrage klären und in `docs/licensing.md` festhalten.
3. Tests zuerst für die Entscheidungslogik (Speicherprüfung, Rückfall).
4. GPU-Image bauen.
5. Schalter, Löser-Vorlage und Rückfall umsetzen.
6. Messreihe fahren und in `docs/performance.md` eintragen.
7. Empfehlungsregel aus der Messung ableiten.

## Tests

Alle Tests dieses Tasks werden permanent in `tests/` im Hauptverzeichnis des Projekts abgelegt (siehe `AGENTS.md`).

- Entscheidungslogik mit simulierten GPUs (zu wenig Speicher, Fehler beim Start).
- Snapshot der Löser-Einstellungen für die GPU.
- `runtime_` (braucht eine GPU, sonst als übersprungen melden): Ein Fall rechnet mit GPU-Löser, die Kennzahlen liegen innerhalb von 1 % der CPU-Rechnung.

## Akzeptanzkriterien

Der Task gilt erst als fertig, wenn alle Punkte erfüllt und durch Tests oder einen konkreten Befehl belegt sind:

- [ ] Der Schalter rechnet auf dem Referenzrechner mit GPU-Löser und fällt bei einem Fehler auf die CPU zurück
- [ ] `docs/performance.md` enthält die Messreihe GPU gegen CPU, auch wenn die GPU langsamer ist
- [ ] Der Schalter ist standardmäßig aus, wenn die Messung keinen Gewinn zeigt
- [ ] Die Lizenzfrage ist beantwortet oder dem Nutzer zur Entscheidung vorgelegt

## Offene Fragen / Annahmen

- Offene Entscheidung "GPU linear solver" fällt in diesem Task.
- Der erwartbare Gewinn ist begrenzt; ein ehrlicher Befund ist das Ziel, kein bestimmter Faktor.

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
