# STATUS: atc-aurora-tensor

| Dimension | Status | Beleg |
|---|---|---|
| Implementierung | `IN_PROGRESS` | M-01 vollständig, M-02 Operationssatz vollständig, Autograd fehlt |
| Funktionale Prüfung | `TESTED` (lokal) | 36 Tests grün, `cargo test` am 11.11.2026 |
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
| B-05 | Matmul gegen handgerechnetes 2x2-Beispiel | grün (`b_05_*`, zwei Tests) |
| B-06 | Doppelte Transponierung und Reihenfolgeidentität | grün (`b_06_*`, zwei Tests) |
| B-07 | Softmax mit Logits ±1e5 | grün (`b_07_*`, zwei Tests) |

B-05 bis B-07 sind aus dem Contract abgeleitet, nicht aus dem Verhalten der
Vorlage. B-05 führt die f64-Referenz im Test, weil das Crate f32 als einzigen
Puffer führt; der f32-Pfad wird gegen diese Referenz mit 1e-6 relativ geprüft.

## Wirksamkeit der Tests

Drei Mutationen am Operationssatz, jede von den Contract-Tests erkannt:

| Mutation | erkannt von |
|---|---|
| Matmul lässt den letzten Summanden weg | B-05 (beide), Migrationstest |
| Softmax ohne Maximum-Subtraktion | B-07 (beide), Stabilitätstest |
| Transponierung ohne Indexvertauschung | B-06 Reihenfolge, Migrationstest |

Bemerkenswert: die doppelte Transponierung allein erkennt die dritte Mutation
nicht, weil zweimal dieselbe falsche Abbildung wieder beim Original landet. Erst
die Reihenfolgeidentität macht den Fehler sichtbar. Beide Tests bleiben.

## Offen

- B-08 Kreuzentropie: gehört zu M-02, verlangt aber die Gradientenseite. Wird
  zusammen mit M-03 abgeschlossen, nicht vorher.
- B-09 bis B-13: M-03 Autograd, Rechengraph, Rückwärtspass, Gradientenakkumulation.
- SIMD und Ausrichtung: bewusst nicht Teil von P2. Der Kern bleibt die einzige
  semantische Implementierung.
