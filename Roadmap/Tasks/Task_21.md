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

- Installer (Task_22)
- Verwaltete WSL-Umgebung (Task_26)

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

- [ ] Auf einem Rechner mit gestopptem Docker-Dienst erklärt der Assistent den Zustand und erkennt, wenn er behoben ist
- [ ] Beide Beispielprojekte lassen sich öffnen und rechnen
- [ ] Die Systemprüfung erzeugt einen kopierbaren Diagnosebericht

## Offene Fragen / Annahmen

- Annahme: Bis Task_22 wird das Image lokal gebaut; danach lädt der Assistent es aus der Registry.

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
