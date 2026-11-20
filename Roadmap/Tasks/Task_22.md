# Task_22: Installer und Release-Pipeline

> ## ⚠️ PFLICHT – BEVOR DU IRGENDETWAS ANDERES TUST
>
> 1. **Lies als Allererstes die Datei `AGENTS.md` im übergeordneten Ordner `Roadmap/`** (von hier: `../AGENTS.md`) und befolge sie strikt.
> 2. **Deine allererste Aktion** ist, diese Datei in `RUN_Task_22.md` umzubenennen, um den Task als laufend zu markieren. Das passiert **noch bevor irgendein Code geschrieben** wird.

## Ziel

FoamCraft lässt sich als Installer für Windows, Linux und macOS bauen und veröffentlichen, mit vollständigem Lizenzbündel und einem veröffentlichten Runtime-Image. Danach ist Meilenstein 3 erreicht: die erste öffentliche Version.

## Kontext & Abhängigkeiten

- **Voraussetzung:** Task_21 ist archiviert. Extern, vom Nutzer zu entscheiden oder bereitzustellen: öffentliches GitHub-Repository (Name, Konto), Container-Registry, optional Signatur-Zertifikat für Windows und Apple-Entwicklerkonto, Beitragspolitik (DCO-Signatur oder keine).
- **Relevantes Vorwissen:** `docs/licensing.md` (Abschnitte OpenFOAM, Releases, Marken), `docs/project-brief.md` (offene Entscheidung zur Beitragspolitik). Veröffentlichen ist eine nach außen sichtbare Aktion: vor jedem Push in ein öffentliches Repository, jedem Release und jedem Image-Upload die ausdrückliche Zustimmung des Nutzers einholen.

## Umfang

**Gehört dazu:**

- CI: Tests, Lint und Lizenzprüfung auf Windows, Linux und macOS
- Bau der Installer (Windows, Linux, macOS)
- Bau und Veröffentlichung des Runtime-Images; der Image-Tag ist an die App-Version gekoppelt, die App lädt das passende Image
- Lizenzbündel: erzeugte Hinweise zu allen Rust- und npm-Abhängigkeiten, OpenFOAM-Quellverweis, Markenhinweis in Installer und About-Dialog
- Prüfung unter Linux und macOS, ob die 3D-Ansicht unter der Web-View funktioniert, sonst dokumentierter Ausweichweg
- Versionsschema, `CHANGELOG.md`, Release-Checkliste in `docs/`, `CONTRIBUTING.md`
- Nutzergesteuerter Hinweis auf eine neue Version in der App

**Gehört nicht dazu:**

- Automatische Updates, App-Stores

## Umsetzungsschritte

1. Die externen Punkte mit dem Nutzer klären (Repository, Registry, Signatur, Beitragspolitik).
2. Tests zuerst für die Skripte: Lizenzbündel vollständig, Versionen konsistent.
3. CI-Workflow für Tests und Lint einrichten.
4. Installer-Bau für die drei Plattformen einrichten.
5. Image-Bau und Kopplung an die App-Version umsetzen.
6. 3D-Ansicht unter Linux und macOS prüfen.
7. Doku, `CHANGELOG.md` und `CONTRIBUTING.md` schreiben.
8. Nach Zustimmung des Nutzers die erste Version veröffentlichen und Meilenstein 3 melden.

## Tests

Alle Tests dieses Tasks werden permanent in `tests/` im Hauptverzeichnis des Projekts abgelegt (siehe `AGENTS.md`).

- Skript-Test: Das Lizenzbündel enthält jede Abhängigkeit aus den Lockfiles.
- Skript-Test: Versionen in Workspace, `package.json`, Tauri-Konfiguration und Image-Tag stimmen überein.
- CI läuft auf den drei Plattformen grün (Nachweis: Workflow-Lauf).
- Installer-Rauchtest unter Windows: installieren, starten, `foamcraft --version`.

## Akzeptanzkriterien

Der Task gilt erst als fertig, wenn alle Punkte erfüllt und durch Tests oder einen konkreten Befehl belegt sind:

- [ ] Die CI baut Installer für Windows, Linux und macOS
- [ ] Jeder Installer enthält Lizenzbündel, OpenFOAM-Quellverweis und Markenhinweis
- [ ] Eine frisch installierte App lädt das zur Version passende Runtime-Image
- [ ] Veröffentlicht wurde nur mit ausdrücklicher Zustimmung des Nutzers

## Offene Fragen / Annahmen

- Ohne Signatur zeigen Windows und macOS Warnungen beim Installieren; der Nutzer entscheidet über Zertifikate.
- Annahme: Unter macOS gibt es keine GPU-Unterstützung für den Löser im Container.

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
