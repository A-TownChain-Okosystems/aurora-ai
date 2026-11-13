# Status atc-aurora-train (M-06)

| Bereich | Status | Beleg |
|---|---|---|
| Gradientenspeicher | `IMPLEMENTED` | `Gradients`, kanonische Parameterfolge, Formpruefung |
| Optimierer | `IMPLEMENTED` | `Adam` (AdamW), Bias-Korrektur, Zustandspruefung vor dem Schritt |
| Trainingsschleife | `IMPLEMENTED` | `Trainer::step` / `fit`, Mittelung im Graphen, Begrenzung optional |
| Bewertung | `IMPLEMENTED` | `evaluate`: Verlust, Perplexitaet, Trefferquote |
| Pruefpunkte | `IMPLEMENTED` | ATCM plus `AURST001`, externer Digest-Anker, Register nach Schritt |
| Lokale Pruefung | `TESTED` | 15 Tests gruen auf **stable 1.83.0** (Gegenprobe nightly 1.99 ebenfalls gruen), clippy `-D warnings` sauber, fmt sauber |
| `CI_VERIFIED` | **offen** | Actions starten nicht, solange die Commit-Herkunft unverifiziert ist |
| `E2E_VERIFIED` | **offen** | kein Lauf im Gesamtsystem, keine Laufzeitbindung (P8) |

## Was die Tests belegen

| Test | Aussage |
|---|---|
| `p4_adam_minimiert_eine_quadratische_funktion` | Adam konvergiert auf `2(w-1)` in 400 Schritten auf 1 |
| `p4_abgelehnter_schritt_verschiebt_den_zaehler_nicht` | Pruefung vor Zustandsaenderung |
| `p4_hyperparameter_werden_abgelehnt` | sieben unzulaessige Kombinationen typisiert abgelehnt |
| `p4_training_senkt_den_verlust` | Mittel der letzten fuenf Schritte unter 85 % der ersten fuenf |
| `p4_training_ist_deterministisch` | zwei Laeufe, gleiche Gewichte, gleicher Verlauf |
| `p4_ungueltiger_stapel_wird_abgelehnt` | leerer Stapel, Sequenz ohne Zielposition |
| `p4_gradientenbegrenzung_wird_angewendet` | gemeldete Norm ist die Norm vor der Begrenzung |
| `p4_bewertung_liefert_verlust_perplexitaet_und_trefferquote` | drei Kennzahlen, endlich, im Wertebereich |
| `p4_bewertung_lehnt_leere_und_kurze_faelle_ab` | Grenzen der Bewertung |
| `p4_pruefpunkt_kommt_bytegleich_zurueck` | Gewichte bytegleich, Schritt, Token, Momentzaehler |
| `p4_fortgesetzter_lauf_ist_derselbe_lauf` | gleicher naechster Schritt, gleiche Gewichte |
| `p4_manipuliertes_modell_wird_abgelehnt` | `ReadError::DigestMismatch` gegen den externen Anker |
| `p4_manipulierter_zustand_wird_abgelehnt` | eigene Pruefsumme des Zustands |
| `p4_zustand_mit_falscher_parameterzahl_wird_abgelehnt` | Zustand gehoert nicht zu diesen Gewichten |
| `p4_register_ordnet_nach_schritt` | Reihenfolge, `latest`, `get`, doppelter Schritt abgelehnt |

## Bewusste Grenzen

Der Verlust sinkt auf einem winzigen Korpus mit winziger Konfiguration. Das ist
ein Nachweis, dass die Kette rechnet und lernt, **kein** Nachweis von
Sprachqualitaet und **kein** Trainingslauf auf Ref-1. Ein Lauf auf Ref-1 braucht
Daten, eine Lernratenplanung und eine Laufzeitumgebung; nichts davon ist hier
vorgetaeuscht.

`CI_VERIFIED` bleibt offen: alle Workflow-Laeufe enden mit `startup_failure`
ohne Job, weil die Konto-E-Mail unverifiziert ist (F-T2).
