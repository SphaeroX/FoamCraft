# Task_26: WSL2-Runner ohne Docker Desktop

> ## ⚠️ PFLICHT – BEVOR DU IRGENDETWAS ANDERES TUST
>
> 1. **Lies als Allererstes die Datei `AGENTS.md` im übergeordneten Ordner `Roadmap/`** (von hier: `../AGENTS.md`) und befolge sie strikt.
> 2. **Deine allererste Aktion** ist, diese Datei in `RUN_Task_26.md` umzubenennen, um den Task als laufend zu markieren. Das passiert **noch bevor irgendein Code geschrieben** wird.

## Ziel

Unter Windows kann FoamCraft OpenFOAM in einer selbst verwalteten WSL2-Umgebung betreiben. Docker Desktop ist dann nicht mehr nötig.

## Kontext & Abhängigkeiten

- **Voraussetzung:** Task_21 (Einrichtungsassistent) und Task_22 (Release-Pipeline) sind archiviert; Task_23 bis Task_25 liegen davor.
- **Relevantes Vorwissen:** Runner-Schnittstelle aus Task_02. Das Aktivieren von WSL braucht Administratorrechte und einen Neustart; das macht der Nutzer selbst, FoamCraft leitet nur an.

## Umfang

**Gehört dazu:**

- Zweite Runner-Implementierung: eine von FoamCraft verwaltete WSL2-Distribution (Import eines Root-Dateisystems aus dem Runtime-Image, eigener Name, Aktualisieren, Entfernen)
- Befehle über `wsl.exe`, Agent in der Distribution, GPU in WSL
- Assistent: Wahl zwischen Docker und der integrierten Umgebung, Prüfung, ob WSL2 aktiv ist, Hinweise zu den Ressourcen
- Die Release-Pipeline erzeugt das Root-Dateisystem als Artefakt
- Gemeinsame Vertragstests für alle Runner

**Gehört nicht dazu:**

- Änderungen an Systemeinstellungen durch FoamCraft
- Unterstützung für WSL1

## Umsetzungsschritte

1. Vertragstests für die Runner-Schnittstelle schreiben, die Docker-, Fake- und WSL-Runner gleichermaßen bestehen müssen.
2. Erzeugung des Root-Dateisystems aus dem Image umsetzen.
3. WSL-Runner umsetzen (Import, Start, Ausführung, Aktualisieren, Entfernen).
4. Fehlerfälle umsetzen (WSL nicht aktiv, Distribution beschädigt).
5. Assistent und Einstellungen erweitern.
6. Release-Pipeline um das Artefakt ergänzen.

## Tests

Alle Tests dieses Tasks werden permanent in `tests/` im Hauptverzeichnis des Projekts abgelegt (siehe `AGENTS.md`).

- Vertragstests laufen für alle Runner.
- Fehlerfälle liefern Fehlercodes mit Text in beiden Sprachen.
- `runtime_` unter Windows: Import, ein Lauf des Referenzfalls, Aktualisieren, Entfernen.

## Akzeptanzkriterien

Der Task gilt erst als fertig, wenn alle Punkte erfüllt und durch Tests oder einen konkreten Befehl belegt sind:

- [ ] Der Referenzfall rechnet über den WSL-Runner ohne laufendes Docker Desktop
- [ ] Die verwaltete Distribution lässt sich über die App rückstandsfrei entfernen
- [ ] Die Vertragstests laufen für Docker- und WSL-Runner grün

## Offene Fragen / Annahmen

- Die Größe des Root-Dateisystems und der Weg der Auslieferung (im Installer oder Download beim ersten Start) werden hier entschieden.

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
