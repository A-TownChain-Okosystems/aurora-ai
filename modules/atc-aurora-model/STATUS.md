# Status atc-aurora-model (M-05)

| Bereich | Status | Beleg |
|---|---|---|
| Parameterhaushalt | `IMPLEMENTED` | `Config`, `Breakdown`, `param_count`, gepruefte Arithmetik |
| Lokale Pruefung | `TESTED` | 8 Tests, `cargo test` gruen, clippy `-D warnings` sauber, fmt sauber |
| Vorwaertslauf (B-19, B-20) | **offen** | braucht Attention, RoPE, Einbettung, SwiGLU-Verdrahtung |
| Artefakt (B-21) | **offen** | braucht Format, Digest und Zustandsschreiben |
| `CI_VERIFIED` | **offen** | Actions starten nicht, solange die Commit-Herkunft unverifiziert ist |
| `E2E_VERIFIED` | **offen** | kein Lauf im Gesamtsystem |

## B-18: entschieden, aber nicht aufgeloest

Einstufung: **`INCONSISTENT / REQUIRES RECONCILIATION`** (Owner-Entscheidung 11.11.2026,
`docs/decisions/M-05-B18-PARAMETERHAUSHALT.md`).

Massgeblich ist die aus K4/K5 abgeleitete Architektur. Der Sollwert 5 771 776 wird nicht als
Implementierungsvorgabe uebernommen, aber auch nicht als veraltet bezeichnet: seine Herleitung ist
nicht nachgewiesen.

| Position | Elemente |
|---|---|
| Embedding (geteilt, 4096 x 256) | 1 048 576 |
| Attention (6 x 1024 x 256) | 1 572 864 |
| SwiGLU (6 x 2112 x 256) | 3 244 032 |
| Block-Normen (6 x 2 x 256) | 3 072 |
| Abschliessende Norm | 256 |
| **Gesamt** | **5 868 800** |

Ohne abschliessende Norm 5 868 544. Abweichung zu B-18: 97 024 beziehungsweise 96 768 Elemente.
Die Norm erklaert 256 davon, die restlichen 96 768 nicht. Der Regressionstest
`b_18_blueprint_sollwert_weicht_von_der_ableitung_ab` haelt die Abweichung reproduzierbar fest.

Offen bis zur Aufloesung: urspruengliche Formel, Architekturversion und Referenz-Commit von B-18.
Erst danach erhaelt M-05 die naechste Verifikationsstufe.

## Statusleiter

Push erfolgreich = veroeffentlicht. Frischer Klon erfolgreich = reproduziert. Tests erfolgreich =
`TESTED`. Actions-Evidenz am exakten HEAD = `CI_VERIFIED`. Vollstaendiger Evidenznachweis =
`VERIFIED`. Keine Aufwertung durch einen Push allein. Gesamtstatus jetzt: `IMPLEMENTED` / `TESTED`.

## Nicht enthalten

Attention, Kausalmaske, RoPE, Gewichte, Initialisierung, Dateiformat, Training.
Diese Teile gehoeren zu M-05 (Rest), M-06 und zur Artefaktschicht; sie sind hier
weder angefangen noch vorgetaeuscht.
