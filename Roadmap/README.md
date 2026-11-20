# FoamCraft – Roadmap

Projektübergreifendes Gedächtnis. Agenten lesen diese Datei vor jedem Task und aktualisieren sie, wenn sich etwas ergibt, das über einen einzelnen Task hinausgeht. Regeln für die Arbeit: [AGENTS.md](AGENTS.md).

## Projektziel

FoamCraft ist eine kostenlose, unabhängige Open-Source-Desktop-App (GPL-3.0-or-later), mit der auch Laien Strömungssimulationen mit OpenFOAM durchführen. Der Weg ist immer derselbe: Datei laden, Windkanal wählen oder Öffnungen anklicken, rechnen, ansehen, exportieren. Niemand muss dafür ein OpenFOAM-Dictionary anfassen.

**Was die Software tut:** Sie lädt 3D-Modelle (STL, OBJ, 3MF, STEP, IGES), richtet Außenströmung im Windkanal oder Innenströmung mit angeklickten Ein- und Auslässen ein, vernetzt und rechnet automatisch mit OpenFOAM in einer verwalteten Laufzeitumgebung und erkennt selbst, wann das Ergebnis stabil ist. Ergebnisse zeigt sie in einer GPU-gerenderten 3D-Ansicht mit Schnitten, Isoflächen und animierten Partikeln, dazu Kennzahlen mit Erklärung. Bilder, Videos und Berichte (PDF, HTML) lassen sich exportieren. Ein MCP-Server gibt KI-Agenten dieselben Möglichkeiten wie der Oberfläche. Erweiterungen: zeitabhängige Rechnung, Wärme und Auftrieb, rotierende und poröse Teile, freie Wasseroberfläche, GPU-Linearlöser und ein eigener GPU-Löser für die Sofort-Vorschau.

**Zielgruppe:** Maker, Studierende und Ingenieure ohne CFD-Erfahrung auf Windows (zuerst), Linux und macOS; außerdem KI-Agenten über MCP. Die Oberfläche gibt es in Englisch und Deutsch.

**Nicht-Ziele:** CAD-Bearbeitung: Geometrie wird nur geladen, ausgerichtet, skaliert und an Öffnungen verschlossen. Wünsche des Nutzers für später, für die es in dieser Roadmap keine Tasks gibt: Cloud-Dienst mit Konten, automatische Formoptimierung, Überschall und Verbrennung. In einzelnen Tasks zusätzlich ausgeklammert und dort begründet: Wärmeleitung in Festkörpern und Strahlung (Task_28), echte Drehung mit gleitenden Netzen (Task_29), frei bewegliche Körper im Wasser (Task_31).

## Aktueller Status

Stand: 2026-11-21 – Roadmap angelegt, noch kein Task begonnen.

| Task | Titel | Status |
|---|---|---|
| 01 | Durchstich: App mit 3D-Fenster | offen |
| 02 | Runtime-Image und Docker-Runner | offen |
| 03 | Job-Pipeline mit Referenzfall | offen |
| 04 | Projektmodell und Befehls-API | offen |
| 05 | Import von STL, OBJ und 3MF mit Diagnose | offen |
| 06 | Flächen erkennen und anklicken | offen |
| 07 | Import von STEP und IGES | offen |
| 08 | Windkanal einrichten | offen |
| 09 | Automatische Vernetzung für die Außenströmung | offen |
| 10 | Stationäre Außenströmung mit Kräften | offen |
| 11 | Lauf-Monitor in der App | offen |
| 12 | Öffnungen erkennen und verschließen | offen |
| 13 | Innenströmung rechnen | offen |
| 14 | Oberflächenfelder und Volumenraster | offen |
| 15 | Schnitte und Isoflächen | offen |
| 16 | Stromlinien und Partikel-Animation | offen |
| 17 | Kennzahlen und Laufvergleich | offen |
| 18 | MCP-Server | offen |
| 19 | Bild- und Videoexport | offen |
| 20 | Report als PDF und HTML | offen |
| 21 | Einrichtungsassistent und Beispielprojekte | offen |
| 22 | Installer und Release-Pipeline | offen |
| 23 | Leistungsmessung und CPU-Tuning | offen |
| 24 | GPU-Linearlöser (experimentell) | offen |
| 25 | Lösungsadaptive Netzverfeinerung | offen |
| 26 | WSL2-Runner ohne Docker Desktop | offen |
| 27 | Zeitabhängige Rechnung mit Zeitleiste | offen |
| 28 | Wärme und Auftrieb | offen |
| 29 | Rotierende Teile und bewegter Boden | offen |
| 30 | Poröse Bauteile | offen |
| 31 | Freie Wasseroberfläche | offen |
| 32 | GPU-Strömungslöser als Kern | offen |
| 33 | Sofort-Vorschau in der App | offen |

Status ist `offen`, `läuft` (Datei trägt das Präfix `RUN_`) oder `erledigt` (Datei liegt in `Archiv/`). Die Tabelle wird beim Archivieren eines Tasks aktualisiert.

## Taskübergreifende Entwicklungsinfos

Globale Informationen, Architekturentscheidungen und Wissen, das für das gesamte Projekt gilt. Wird laufend ergänzt; jeden Eintrag mit dem Task kennzeichnen, aus dem er stammt.

Alle Einträge dieses Abschnitts stammen, soweit nicht anders vermerkt, aus dem Interview vom 2026-11-20/21.

### Phasen und Meilensteine

| Phase | Tasks | Inhalt | Meilenstein am Ende |
|---|---|---|---|
| 1 | 01–04 | Fundament: App mit 3D-Fenster, Runtime, Job-Pipeline, Projektmodell und Befehls-API | – |
| 2 | 05–13 | Geometrie und beide Grundfälle (Windkanal, Innenströmung) | M1: beide Grundfälle von der Datei bis zur Kennzahl (Task_13) |
| 3 | 14–20 | Ergebnisse, Animation, MCP, Video, Report | M2: Ergebnisse, MCP und Export vollständig (Task_20) |
| 4 | 21–22 | Einrichtungsassistent, Installer, Release | M3: erste öffentliche Version (Task_22) |
| 5 | 23–26 | Leistung, GPU-Linearlöser, adaptive Verfeinerung, WSL-Runner | – |
| 6 | 27–31 | Physik: zeitabhängig, Wärme, rotierend, porös, freie Wasseroberfläche | – |
| 7 | 32–33 | Eigener GPU-Löser und Sofort-Vorschau | – |

An den Meilensteinen meldet der Agent dem Nutzer den Stand zur Abnahme (steht im jeweiligen Task).

### Architektur in Kürze

Maßgeblich ist `docs/architecture.md`; hier nur das Nötigste für die Einordnung:

- Kern in Rust (ein Cargo-Workspace), kein Python. Oberfläche mit Tauri 2, TypeScript und React in `app/`.
- Eigener Renderer in Rust auf wgpu, der unter der transparenten Web-View zeichnet und auch offscreen rendert. Derselbe Renderer bedient Ansicht, MCP-Bilder, Videoexport und die Sofort-Vorschau.
- OpenFOAM (OpenCFD-Linie, gepinnt) läuft im Container hinter einer Runner-Schnittstelle. Ein Linux-Build des Kerns (Agent) im Container ist der einzige Zugriff auf Falldateien.
- Jede Aktion ist ein Befehl mit Schema. Oberfläche, CLI und MCP sind gleichberechtigte Nutzer der Befehls-API. Alles muss ohne Fenster funktionieren.
- Auswertung ohne VTK: OpenFOAM bereitet auf, Rust liest und rastert das Volumen in 3D-Texturen, die GPU berechnet Schnitte, Isoflächen und Partikel.
- Der Nutzer hat sich im Interview ausdrücklich für "alles in Rust und TypeScript" entschieden und gegen eine Python-Engine. Diese Entscheidung nicht ohne Rückfrage umkehren.

### Offene Entscheidungen und wo sie fallen

| Entscheidung | Task |
|---|---|
| Web-View über wgpu oder Ausweichweg | Task_01 |
| Genaue OpenFOAM-Version; Datei-Zugriff über Volume mit Agent oder Bind-Mount | Task_02 |
| Technik für STEP und IGES | Task_07 |
| Brücke zwischen CLI/MCP und laufender App | Task_18 |
| Video-Encoder und Containerformat | Task_19 |
| Beitragspolitik (DCO oder keine), Signatur-Zertifikate, Repository und Registry | Task_22 |
| GPU-Plugin (Ginkgo/OGL oder AmgX) und Lizenzfrage der CUDA-Bestandteile | Task_24 |
| Auslieferung des Root-Dateisystems für WSL | Task_26 |

### Externe Voraussetzungen

- Docker Desktop mit WSL2-Backend muss für alle `runtime_`-Tests laufen (ab Task_02).
- Für Task_22 entscheidet der Nutzer über öffentliches Repository, Registry und Signatur. Veröffentlicht wird nur mit seiner ausdrücklichen Zustimmung.
- Testgeometrie wird selbst erzeugt (CC0). STEP-Testdateien kann der Nutzer mit SolidCraft erstellen.

### Lizenz und Unabhängigkeit

- Projektlizenz GPL-3.0-or-later; Abhängigkeiten nur von der Erlaubnisliste in `docs/licensing.md`.
- FoamCraft ist unabhängig. Keine Namen, Texte, Layouts, Grafiken oder Beispielmodelle anderer CFD-Produkte übernehmen. "OpenFOAM" nur beschreibend verwenden, immer mit dem Markenhinweis.
- Code aus FluidX3D oder anderen Quellen mit Nutzungsbeschränkung darf nicht verwendet werden (wichtig für Task_32).

### Referenzrechner

Windows 11, Intel i9-13900HX (8 P-Kerne, 16 E-Kerne), 64 GB Arbeitsspeicher, NVIDIA RTX 4080 Laptop mit 12 GB. Leistungs- und Speicherangaben in den Tasks beziehen sich darauf.

### Erkenntnisse aus abgeschlossenen Tasks

Noch keine. Hier trägt jeder Agent beim Archivieren taskübergreifende Erkenntnisse ein, mit Angabe des Tasks.

## Auffälligkeiten & Details

Dinge, die global noch zu beachten sind: Besonderheiten, bekannte Einschränkungen, kleinere Details. Wird laufend ergänzt.

- **Docker beim Anlegen der Roadmap (2026-11-21):** Docker 29 ist installiert, der Dienst lief aber nicht, und es gab keine WSL-Distribution. Vor Task_02 den Nutzer bitten, Docker Desktop mit WSL2-Backend zu starten.
- **Root-`Cargo.toml` fehlt absichtlich:** Ein leerer Workspace lässt `cargo test` scheitern. Task_01 legt sie an; bis dahin laufen nur die npm-Befehle.
- **GPU bei OpenFOAM:** Es gibt keine offizielle GPU-Version, nur die Auslagerung des linearen Lösers. Der Gewinn ist begrenzt und wird in Task_24 gemessen, nicht versprochen. Der große Sprung kommt erst mit dem eigenen GPU-Löser (Task_32, Task_33).
- **Hybrid-CPU:** P- und E-Kerne verhalten sich bei OpenFOAM unterschiedlich. Bis Task_23 gilt "physische Kerne" als Standard, danach die gemessene Regel.
- **Linux und macOS:** Bis Task_22 wird nur unter Windows entwickelt und geprüft. WebGPU in System-Web-Views ist unter Linux unzuverlässig, deshalb der native Renderer.
- **Prettier ignoriert `Roadmap/`**, damit das Regelwerk und die Task-Dateien nicht umformatiert werden.
- **Validierungstests:** Toleranzen stehen mit Quelle im Test und werden nie aufgeweitet, um Grün zu erreichen. Gelingt eine Toleranz nicht, anhalten und dem Nutzer berichten.
- **Reihenfolge:** Die Tasks werden streng numerisch abgearbeitet, auch wenn ein Task laut "Voraussetzung" nur von einem früheren abhängt.
