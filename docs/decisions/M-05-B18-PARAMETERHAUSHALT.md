# M-05 · Parameterhaushalt: Architekturentscheidung und Klärungsbedarf B-18

**Datum:** 11.11.2026 · **Entscheidung:** Owner · **Betrifft:** M-05 `atc-aurora-model`, Testfall B-18
**Einstufung von B-18:** `INCONSISTENT / REQUIRES RECONCILIATION`

## 1. Entscheidung

Die aus K4/K5 abgeleitete Architektur ist maßgeblich. Der B-18-Sollwert von 5 771 776 wird **nicht**
als Implementierungsvorgabe übernommen. Ein Sollwert darf keine Architekturdefinition überschreiben,
solange seine eigene Herleitung nicht nachgewiesen ist.

| Gegenstand | Entscheidung |
|---|---|
| K4/K5 eingefrorene Konfiguration | Architektur-SSOT |
| `param_count` | maßgebliche Berechnung |
| 5 868 800 | abgeleiteter Gesamtwert, einschließlich abschließender Normierung |
| 5 868 544 | abgeleiteter Wert ohne abschließende Normierung |
| 5 771 776 | ungeklärter historischer Referenzwert |
| B-18-Test | Abweichung dokumentieren |
| Unbekannte Architekturänderung | nicht zulässig |
| Anpassung der Implementierung an B-18 | abgelehnt |

B-18 wird ausdrücklich **nicht** als veraltet bezeichnet. Dafür fehlt der Nachweis der ursprünglichen
Herleitung. Bis dahin gilt `INCONSISTENT / REQUIRES RECONCILIATION`.

## 2. Zahlenlage

| Komponente | Parameter |
|---|---|
| Embedding (geteilt, 4096 × 256) | 1 048 576 |
| Attention (6 × 1024 × 256) | 1 572 864 |
| SwiGLU (6 × 2112 × 256) | 3 244 032 |
| Block-Normen (6 × 2 × 256) | 3 072 |
| Abschließende Norm | 256 |
| **Gesamt** | **5 868 800** |

Die Summe stimmt exakt. Ohne abschließende Normierung: 5 868 544.
Abweichung zum B-18-Sollwert: 97 024 beziehungsweise 96 768 Elemente.
Die abschließende Normierung erklärt 256 Parameter, nicht die verbleibenden 96 768.
Das ist eine strukturelle Abweichung, kein Rundungsfehler.

## 3. Verbindliche Maßnahmen

1. **Herkunft B-18 untersuchen:** ursprüngliche Formel, Architekturversion, Referenz-Commit feststellen.
2. **Parameterrechnung aufschlüsseln:** Embedding, Attention, Feed-Forward, Normierung und weitere
   Positionen separat ausweisen (im Code als `Breakdown`, in §2 dieses Dokuments).
3. **Referenz korrigieren**, falls B-18 veraltet ist: Sollwert samt Herleitung aktualisieren.
   Erst nach Nachweis, nicht vorher.
4. **Keine Architekturänderung erzwingen.** K4/K5 bleiben unverändert, solange kein dokumentierter
   Architekturentscheid etwas anderes festlegt.
5. **Regressionstest erhalten.** Die bekannte Abweichung bleibt reproduzierbar, bis die Referenzfrage
   geklärt ist (`b_18_blueprint_sollwert_weicht_von_der_ableitung_ab`).

## 4. Freigabestatus

| Kriterium | Status |
|---|---|
| Implementierung | `IMPLEMENTED` — Parameterhaushalt, unverändert |
| Unit-Tests | `TESTED` — 8 Tests, aus frischem Klon reproduziert |
| Clippy / Formatierung | sauber (`-D warnings`, `cargo fmt --check`) |
| Register-Invarianten | halten — 500 Dateien, 58 Verzeichnisse |
| Mirror | `IN_SYNC` |
| B-18-Abgleich | `INCONSISTENT / REQUIRES RECONCILIATION` — dokumentiert und getestet |
| `CI_VERIFIED` | **offen** — Actions starten nicht bei unverifizierter Commit-Herkunft |

Statusaufwertung allein durch einen erfolgreichen Push findet nicht statt. Push heißt veröffentlicht,
frischer Klon heißt reproduziert, Tests heißen `TESTED`, `CI_VERIFIED` verlangt commitgebundene
Actions-Evidenz am exakten HEAD, `VERIFIED` den vollständigen Evidenznachweis.

## 5. Grenze der Aussage

M-05 implementiert derzeit eine mathematische Grundlage, noch kein Foundation Model. Diese Grenze
bleibt auch in öffentlichen Projektbeschreibungen erhalten. Nicht behauptet und nicht vorhanden:
Modellinferenz, Attention-Berechnung, Gewichtsspeicherung, Training, Modellartefakt-Erzeugung.

## 6. Nächste Verifikationsstufe

M-05 erhält die nächste Verifikationsstufe erst nach Auflösung der Referenzabweichung. Keine weiteren
M-05-Funktionserweiterungen vorziehen; zuerst Dokumentation veröffentlichen, den historischen
B-18-Widerspruch sauber auflösen und die Reproduzierbarkeit absichern.

Grundsatz: Architektur bestimmt die Implementierung, nicht ein ungeklärter historischer Zahlenwert.
