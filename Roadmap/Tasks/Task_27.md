# Task_27: Zeitabhängige Rechnung mit Zeitleiste

> ## ⚠️ PFLICHT – BEVOR DU IRGENDETWAS ANDERES TUST
>
> 1. **Lies als Allererstes die Datei `AGENTS.md` im übergeordneten Ordner `Roadmap/`** (von hier: `../AGENTS.md`) und befolge sie strikt.
> 2. **Deine allererste Aktion** ist, diese Datei in `RUN_Task_27.md` umzubenennen, um den Task als laufend zu markieren. Das passiert **noch bevor irgendein Code geschrieben** wird.

## Ziel

Der Nutzer rechnet eine Strömung zeitabhängig, spielt den Verlauf in der 3D-Ansicht ab und sieht Kräfte über der Zeit. Damit werden Wirbelablösung und schwankende Lasten sichtbar.

## Kontext & Abhängigkeiten

- **Voraussetzung:** Task_19 ist archiviert (Zeitachse, Videoexport); Task_14 bis Task_16 liefern Raster und Partikel.
- **Relevantes Vorwissen:** Erkenntnisse aus Task_10 (Konvergenz), Task_14 (Speicherbudget der Raster) und Task_16 (Partikel). Zeitabhängige Rechnungen dauern deutlich länger; Speicherbedarf der Zeitreihen ist das Hauptrisiko.

## Umfang

**Gehört dazu:**

- Rechenart "zeitabhängig" für beide Szenarien mit pimpleFoam, Start aus der stationären Lösung
- Adaptiver Zeitschritt über eine Ziel-Courant-Zahl; Vorschlag für die Dauer aus Durchströmzeiten
- Turbulenz: URANS mit k-ω SST; eine detailreichere Variante (DDES) als Option mit Hinweis auf den Netzbedarf
- Kräfte über der Zeit, Mittelwert, Schwankung, dominante Frequenz
- Zeitreihen der Volumenraster mit Speicherbudget und Kompression
- Ansicht: Zeitleiste, Abspielen der Felder, Partikel im zeitabhängigen Feld
- Videoexport der Zeitreihe
- Monitor: Fortschritt in Simulationszeit, Courant-Zahl; Abbruch mit nutzbarem Teilergebnis

**Gehört nicht dazu:**

- Freie Wasseroberfläche (Task_31)
- Bewegte Netze

## Umsetzungsschritte

1. Tests zuerst für Zeitschritt- und Dauer-Vorschläge und für die Statistik (Mittelwert, Schwankung, Frequenz) an synthetischen Signalen.
2. Fallvorlagen und Ablauf (stationär, dann zeitabhängig) umsetzen.
3. Zeitreihen-Cache mit Budget umsetzen.
4. Zeitleiste und Abspielen im Renderer umsetzen; Partikel an zeitabhängige Felder anpassen.
5. Monitor und Videoexport erweitern.
6. Validierungstest schreiben.

## Tests

Alle Tests dieses Tasks werden permanent in `tests/` im Hauptverzeichnis des Projekts abgelegt (siehe `AGENTS.md`).

- Statistik an synthetischen Signalen; Vorschläge für Zeitschritt und Dauer.
- Der Zeitreihen-Cache hält das Budget ein; die Zeitleiste interpoliert korrekt.
- `validation_`: Zylinderumströmung bei kleiner Reynolds-Zahl, Strouhal-Zahl innerhalb von 10 % des Literaturwerts.
- Golden Images einzelner Zeitpunkte.

## Akzeptanzkriterien

Der Task gilt erst als fertig, wenn alle Punkte erfüllt und durch Tests oder einen konkreten Befehl belegt sind:

- [ ] Ein zeitabhängiger Lauf ist für beide Szenarien wählbar und abspielbar
- [ ] Kräfte über der Zeit mit Mittelwert und Frequenz stehen in den Kennzahlen
- [ ] Der Validierungstest läuft grün
- [ ] Ein Video der Zeitreihe lässt sich exportieren

## Offene Fragen / Annahmen

- Annahme: Zeitreihen werden nur für ein Teilgebiet und in begrenzter Zahl gespeichert; die Grenzen werden hier festgelegt.
- Ob DDES auf einem Laptop sinnvoll nutzbar ist, wird hier gemessen und ehrlich dokumentiert.

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
