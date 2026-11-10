---
document_id: ATC-AI-LLM-001-C
title: T2.6-Messprotokoll, Durchsatz der Referenzimplementierung
version: 1.0.0
status: accepted
owner: A-TownChain-Okosystems
created: 2026-11-11
updated: 2026-11-11
parent: ATC-AI-LLM-001 v1.3.0, Abschnitt 14.3
layer: L2 (aurora-ai)
security_class: S3
evidence_gate: AURORA-001
---

# T2.6-Messprotokoll

Verbindliches Verfahren für die Durchsatzmessung der Referenzimplementierung. Die
Referenzregel steht in `ATC-AI-LLM-001` Abschnitt 14.3: die erste Maschine, die
das Qualifikationsprofil vollständig erfüllt und exklusiv messen darf, wird die
verbindliche CPU-Referenz (First Qualified Hardware).

Eine Messung ohne vollständig ausgefülltes Protokoll ist keine Evidenz. Sie darf
weder zitiert noch als Vergleichsbasis verwendet werden.

---

## 1. Qualifikation der Maschine

Alle Punkte müssen erfüllt sein. Ein einzelner Fehlpunkt disqualifiziert.

| Kriterium | Vorgabe | Erfüllt |
|---|---|---|
| Architektur | x86-64 | ☐ |
| SIMD-Minimum | AVX2 | ☐ |
| Erweiterung | AVX-512, sofern vorhanden (dokumentieren, nicht zwingend) | ☐ |
| NUMA | ein Sockel | ☐ |
| Arbeitsspeicher | mindestens 1 GB frei für Ref-1 (93,9 MB Zustand, höchstens 60 MB Aktivierungen) | ☐ |
| Zugriff | exklusiv während der Messung, kein anderer Nutzer, kein anderer Prozess mit vergleichbarer Last | ☐ |
| Takt | stabil, kein Throttling während der Messung | ☐ |
| Umgebung | native Ausführung, kein Shared Container | ☐ |
| Ausgeschlossen | CI-Läufer, geteilte Sandbox | ☐ |

## 2. Umgebungsangaben, Pflichtfelder

Ohne diese Angaben ist die Messung nicht reproduzierbar.

| Feld | Erfassung | Wert |
|---|---|---|
| CPU-Modell | Bezeichnung aus `/proc/cpuinfo` | |
| CPU-Fingerprint | `model name`, `cpu family`, `model`, `stepping`, `microcode` | |
| Kerne und Threads | physisch / logisch | |
| Basistakt und maximaler Takt | laut Herstellerangabe und `cpufreq` | |
| Microcode-Stand | aus `/proc/cpuinfo` | |
| Kernelversion | `uname -r` | |
| Distribution | `/etc/os-release` | |
| Rust-Version | `rustc --version` | |
| Compilerflags | vollständige Flags des Messbuilds, Profil, `target-cpu` | |
| SIMD-Erweiterungen | aus `/proc/cpuinfo` `flags`: AVX2, AVX-512F und Untergruppen | |
| Speicherkonfiguration | Größe, Kanäle, bestückte Bänke | |
| NUMA-Konfiguration | `numactl --hardware` | |
| Frequenz während der Messung | minimal / maximal / Median über alle Läufe | |
| Temperatur während der Messung | minimal / maximal | |
| Throttling | aufgetreten ja/nein, wenn ja: Dauer und Grund | |
| Hintergrundlast | was sonst auf der Maschine läuft | |

`/proc/cpuinfo` wird vollständig als Rohdatum abgelegt, nicht auszugsweise.

## 3. Messablauf

1. Build mit den dokumentierten Flags, keine nachträgliche Änderung zwischen den
   Läufen.
2. Prozess auf einen festen Kern pinnen (`taskset -c N`), ein Thread, kein
   Threadpool.
3. Fünf Aufwärmläufe. Sie werden nicht gewertet.
4. Drei unabhängige Messläufe mit identischem Eingabekorpus und identischer
   Sequenzlänge.
5. Ausgabe je Lauf: Token pro Sekunde, Byte pro Sekunde, Dauer, Frequenz- und
   Temperaturverlauf.
6. Referenzwert ist der Median der drei Läufe. Zusätzlich werden Minimum,
   Maximum und die relative Streuung angegeben.
7. Wird die Streuung größer als 5 Prozent, gilt die Messung als nicht stabil. Sie
   wird nicht als Referenz übernommen, sondern wiederholt.

## 4. Ergebnis

| Lauf | Token/s | Byte/s | Dauer (s) | Takt (MHz) | Temperatur (°C) |
|---|---|---|---|---|---|
| Aufwärm 1 bis 5 | nicht gewertet | | | | |
| 1 | | | | | |
| 2 | | | | | |
| 3 | | | | | |
| **Median** | | | | | |
| Streuung | | | | | |

Bezugsgrößen der Referenzimplementierung, damit der Messwert einordenbar ist:

| Größe | Wert | Herkunft |
|---|---|---|
| Parameter | 5.868.544 | ATC-AI-LLM-001 Abschnitt 7.1.1 |
| FLOPs je Token, Vorwärtspass | 11,74 MFLOP | berechnet |
| FLOPs je Token inklusive Rückwärtspass | 4,46e7 | Anhang A, Abschnitt 5.2 |
| FLOPs je Schritt bei `B = 8` | 1,83e11 | Anhang A, Abschnitt 5.2 |
| Trainingszustand | 93,9 MB | Anhang A, Abschnitt 6 |

## 5. Rohdaten und Ablage

- Vollständige Rohausgabe je Lauf, unverändert, inklusive Fehlerausgabe.
- `/proc/cpuinfo`, `numactl --hardware`, `uname -a` als eigene Dateien.
- Das Protokoll mit ausgefüllten Feldern, Datum und Uhrzeit in UTC.
- Ablage zusammen mit dem Messlauf, nicht als Nacherzählung. Eine Zusammenfassung
  ersetzt die Rohdaten nicht.
- Wird ein Feld nachträglich ergänzt, bleibt der ursprüngliche Stand sichtbar.

## 6. Governance

Die qualifizierte Maschine ist die Referenz. Spätere Hardware ersetzt ihre Werte
nicht stillschweigend. Ein Wechsel erfordert:

1. neuen CPU-Fingerprint nach Abschnitt 2,
2. ein neues Messprotokoll nach Abschnitt 3,
3. einen Vergleich gegen die bisherige Referenz, mindestens Token pro Sekunde und
   Byte pro Sekunde auf identischem Korpus,
4. eine dokumentierte Freigabe durch den Owner.

Bis zu dieser Freigabe sind alle Durchsatzzahlen relativ zur ersten qualifizierten
Referenz zu lesen und mit ihrem Fingerprint zu zitieren.

## 7. Nicht Teil dieses Protokolls

GPU-Messungen, verteilte Messungen, Messungen im Container, Messungen auf
CI-Läufern und jede Messung, deren Zweck es ist, eine Leistungszahl ohne
Qualifikationsprofil zu erzeugen. Ref-1 läuft auf CPU; alle anderen Messungen
gehören zu den späteren Stufen und zu D4.

---

*Vorlage. Kein Feld ist ausgefüllt. Es liegt keine qualifizierte Maschine vor.*
