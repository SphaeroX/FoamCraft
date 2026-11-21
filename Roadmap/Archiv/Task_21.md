# Task_21: Einrichtungsassistent und Beispielprojekte

> ## ⚠️ PFLICHT – BEVOR DU IRGENDETWAS ANDERES TUST
>
> 1. **Lies als Allererstes die Datei `AGENTS.md` im übergeordneten Ordner `Roadmap/`** (von hier: `../AGENTS.md`) und befolge sie strikt.
> 2. **Deine allererste Aktion** ist, diese Datei in `RUN_Task_21.md` umzubenennen, um den Task als laufend zu markieren. Das passiert **noch bevor irgendein Code geschrieben** wird.

## Ziel

Beim ersten Start führt FoamCraft den Nutzer durch die Einrichtung: Docker, Speicher und GPU werden geprüft, das Image wird bereitgestellt, sinnvolle Standardwerte werden gesetzt. Beispielprojekte zeigen beide Grundfälle.

## Kontext & Abhängigkeiten

- **Voraussetzung:** Task_13 ist archiviert (beide Grundfälle); Task_14 bis Task_20 liegen davor.
- **Relevantes Vorwissen:** Fehlercodes des Runners aus Task_02. Beim Anlegen der Roadmap war auf dem Referenzrechner der Docker-Dienst aus und keine WSL-Distribution vorhanden; genau diesen Zustand muss der Assistent gut erklären. Systemänderungen (Docker installieren, WSL aktivieren) führt der Nutzer selbst aus.

## Umfang

**Gehört dazu:**

- Assistent beim ersten Start: Docker installiert, Dienst läuft, WSL2-Backend, freier Speicherplatz, dem Container zugeteilter Arbeitsspeicher, GPU
- Bereitstellen des Runtime-Images mit Fortschritt
- Hardware-Profil mit Standardwerten für Kerne und Zellbudget
- Schritt-für-Schritt-Hilfen mit Verweisen für alles, was der Nutzer selbst tun muss
- Systemprüfung jederzeit erneut aufrufbar, Diagnosebericht zum Kopieren
- Beispielprojekte, selbst modelliert und CC0: ein Körper im Windkanal, ein Rohrverteiler
- Kurze geführte Tour durch die Arbeitsschritte, leere Zustände mit Handlungsaufforderung
- Bedienbarkeit: Tastatur, Kontrast, helles und dunkles Erscheinungsbild

**Gehört nicht dazu:**

- Installer (Task_36)
- Verwaltete WSL-Umgebung (Task_40)

## Umsetzungsschritte

1. Tests zuerst für die Prüfer mit simulierten Umgebungen (Docker fehlt, Dienst aus, wenig Arbeitsspeicher, keine GPU).
2. Prüfer und Hardware-Profil umsetzen.
3. Assistenten in der Oberfläche umsetzen (Vitest zuerst).
4. Beispielgeometrie modellieren und die Beispielprojekte anlegen.
5. Geführte Tour und leere Zustände umsetzen.
6. Tastaturbedienung, Kontrast und Erscheinungsbild prüfen und nachbessern.

## Tests

Alle Tests dieses Tasks werden permanent in `tests/` im Hauptverzeichnis des Projekts abgelegt (siehe `AGENTS.md`).

- Prüfer liefern für jede simulierte Umgebung die richtigen Schritte und Texte in beiden Sprachen.
- Das Hardware-Profil ergibt für Beispielrechner die erwarteten Standardwerte.
- Die Beispielprojekte entsprechen dem Projektschema; ihre Geometrie besteht die Diagnose.
- Vitest für den Ablauf des Assistenten.
- `runtime_`: Das Windkanal-Beispiel läuft in Entwurfsqualität durch.

## Akzeptanzkriterien

Der Task gilt erst als fertig, wenn alle Punkte erfüllt und durch Tests oder einen konkreten Befehl belegt sind:

- [x] Auf einem Rechner mit gestopptem Docker-Dienst erklärt der Assistent den Zustand und erkennt, wenn er behoben ist
- [x] Beide Beispielprojekte lassen sich öffnen und rechnen
- [x] Die Systemprüfung erzeugt einen kopierbaren Diagnosebericht

## Offene Fragen / Annahmen

- Annahme: Bis Task_36 wird das Image lokal gebaut; danach lädt der Assistent es aus der Registry.

## Notizen zu diesem Task

Taskspezifische Informationen während der Arbeit hier festhalten. Taskübergreifendes gehört in `Roadmap/README.md`.

- Belege:
  - `cargo test -p foamcraft-api --test system` (Docker fehlt, Dienst gestoppt → nach Start erkannt, alter und neuer Docker-Wortlaut, Hyper-V/wenig Speicher/kein Image/wenig Platz/keine GPU, Hardware-Profil für drei Beispielrechner, Diagnosebericht ohne Benutzerordner).
  - `cargo test -p foamcraft-api --test samples` (Liste en/de, Schema, Diagnose, dicht mit 4 Verschlüssen) und `-- --ignored runtime_`: beide Beispiele rechnen in Entwurfsqualität durch (250 s zusammen).
  - Vitest `tests/app/setup-assistant.test.tsx` (Assistent, Bereitstellung mit Fortschritt, Diagnose kopieren, Tour, Beispiele auf der Startseite).
  - Live (2026-11-21): App mit frischem Datenordner und nicht erreichbarem Docker-Endpunkt (`DOCKER_HOST=npipe:////./pipe/foamcraft_no_docker`, simuliert den gestoppten Dienst, ohne Docker anzufassen) → "Docker-Dienst läuft nicht" mit Schritt-für-Schritt-Hilfe inkl. WSL (`Roadmap/Nachweise/task21_assistent_docker_gestoppt.png`); normaler Start → "Bereit für Simulationen" mit den echten Hinweisen (Hyper-V-Backend, 1,9 GiB) (`task21_assistent_bereit.png`); Diagnosebericht in die Zwischenablage kopiert und geprüft; danach Tour und Beispielprojekte auf der Startseite (`task21_tour_und_beispiele.png`). Helles Erscheinungsbild im eingebauten Browser mit emuliertem hellen Systemmodus geprüft.
- Gefundener Fehler: Docker 28+ meldet einen gestoppten Dienst als "failed to connect to the docker API … if the daemon is running"; bisher als `runtime.command_failed` eingeordnet. Jetzt `runtime.daemon_not_running` (Test).
- "Erkennt, wenn behoben" ist live nur über zwei Starts belegt (die Umgebungsvariable lässt sich im laufenden Prozess nicht ändern); im Ablauf selbst belegen es der Rust-Test (gleiche Prüfung erst gestoppt, dann laufend) und der Vitest-Test ("Erneut prüfen").

## Abschluss-Checkliste

Reihenfolge und Details stehen in `AGENTS.md`, Abschnitt "Abschluss- und Archivierungs-Workflow".

- [x] Alle Akzeptanzkriterien erfüllt, Tests liegen permanent in `tests/` und laufen grün
- [x] Abschnitt "Erkenntnisse für Folge-Agenten" unten ausgefüllt
- [x] Folge-Tasks und `Roadmap/README.md` geprüft und bei Bedarf angepasst
- [x] Datei ohne `RUN_` nach `Roadmap/Archiv/` verschoben

## Erkenntnisse für Folge-Agenten

Vor der Archivierung vom Agenten auszufüllen. Schreibe so, dass ein Agent, der diesen Task nie gesehen hat, davon bei späteren Tasks profitiert:

- **Umgesetzt (und Abweichungen vom Plan):** `foamcraft_api::system` (Prüfschritte mit Texten/Hilfen, `Probe`, Hardware-Profil, Diagnose, Setup-Zustand, Image bereitstellen), `foamcraft_api::samples` mit `samples/` (eigene Gmsh-Skripte, CC0), Oberfläche `SetupAssistant`, `GuidedTour`, `SamplesSection`, Knöpfe "Tour" und "System", helles Erscheinungsbild und Fokusrahmen. `foamcraft_core::command::opened` ist jetzt öffentlich.
- **Learnings & Stolpersteine:** `sysinfo` 0.40 liefert `Result` bei `System::new` und `Disks::new_with_refreshed_list`; `Disks` ist per Deref eine Liste. Der bestehende Katalogtest verlangt zu jedem Eintrag einen Vorschlag – auch bei "alles gut" ("Nichts zu tun."). Pfade für die Windows-CLI aus Git-Bash mit `cygpath -m` umwandeln. Beim Neustart von `npm run app:dev` erst warten, bis der alte Vite-Server beendet ist (Port 1420).
- **Wichtige Entscheidungen und ihre Gründe:** Der Assistent ändert nichts am System (Docker/WSL installiert der Nutzer selbst); Hilfetexte mit Verweisen. Warnungen (Hyper-V, wenig Speicher) blockieren nicht. Diagnosebericht ohne Benutzerordner (Datenschutz). Beispiele als eingebettete Projektvorlagen, damit sie ohne Runtime-Container angelegt werden können.
- **Auswirkungen auf Folge-Tasks:** Task_36: `system.provide_image` auf das veröffentlichte Image umstellen, `samples/` und `assets/fonts/OFL.txt` ausliefern. Task_40 (WSL2-Runner) erweitert die Prüfschritte um die verwaltete WSL-Umgebung.
- **Hinweise zu Tests und Befehlen:** `cargo test -p foamcraft-api --test system --test samples`; echte Läufe der Beispiele: `cargo test -p foamcraft-api --test samples -- --ignored runtime_ --test-threads=1`. Gestoppten Docker simulieren: App mit `DOCKER_HOST=npipe:////./pipe/foamcraft_no_docker` starten.
