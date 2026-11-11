# STATUS: atc-aurora-tensor

| Dimension | Status | Beleg |
|---|---|---|
| Implementierung | `IN_PROGRESS` | M-01 vollständig, M-02 in Arbeit |
| Funktionale Prüfung | `TESTED` (lokal) | 29 Tests grün, `cargo test` am 11.11.2026 |
| Performance-Referenz | `PENDING` | T2.6 nicht Teil dieses Crates |
| Gesamtabnahme | `PENDING_ACCEPTANCE` | M-03 offen |

`CI_VERIFIED` wird ausdrücklich nicht beansprucht: push-getriggerte Workflows
scheitern derzeit am unbestätigten GitHub-Konto. `TESTED` heißt hier lokal
ausgeführt, nicht in CI bestätigt.

## Testfälle aus P2

| Fall | Prüfung | Status |
|---|---|---|
| B-01 | Matmul mit unpassenden Formen | grün (`b_01_*`) |
| B-02 | Addition nicht kompatibler Formen | grün (`b_02_*`) |
| B-03 | Budgetüberschreitung vor der Zuteilung | grün (`b_03_*`) |
| B-04 | Konstruktion aus Little-Endian-Bytes | grün (`b_04_*`) |

## Offen

- B-05 bis B-13: gehören zu M-02 und M-03.
- M-03 Autograd: Rechengraph, Rückwärtspass, Gradientenakkumulation.
- SIMD und Ausrichtung: bewusst nicht Teil von P2, der Kern bleibt die einzige
  semantische Implementierung.
