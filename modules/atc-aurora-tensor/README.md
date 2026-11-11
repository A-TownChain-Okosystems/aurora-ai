# atc-aurora-tensor

Nativer Tensor-Kern der Aurora-Referenzimplementierung. Eigentümer der
KI-Rechenoperationen in Layer L2.

| Feld | Wert |
|---|---|
| Module | M-01 `tensor-core`, M-02 `tensor-ops` |
| Datentyp | f32, verbindlich nach `ATC-AI-LLM-001` Abschnitt 7.1.1 |
| Umgebung | `no_std` mit `alloc`, `libm`, keine Kernel-Abhängigkeit (per Test geprüft) |
| Budget | Prüfung vor jeder Speicherzuteilung, `BudgetExceeded` |

## Herkunft

Der Kern ist aus `globus-os/modules/atc-shivacore/service_space/src/ai.rs`
extrahiert (Beschluss vom 11.11.2026, Option A). Damit gibt es genau eine
Tensorimplementierung im Ökosystem: diese. Der Service Space ist kein
Eigentümer von KI-Code mehr.

Die zwölf Tensor- und Operationstests der Vorlage sind als `mig_*` übertragen und
sichern die Semantik beim Umzug. Sie sind Migrationsbelege, kein Ersatz für die
P2-Testfälle.

## Umfang

| Operation | Rang | Hinweis |
|---|---|---|
| `add`, `mul` | beliebig | gleiche Formen erforderlich, kein Broadcasting (B-02) |
| `scale`, `relu`, `sigmoid` | beliebig | elementweise |
| `matmul` | 2 | Innenmaße müssen passen (B-01) |
| `transpose` | 2 | |
| `softmax` | 1, 2 | mit Maximum-Subtraktion, zeilenweise |
| `dot`, `sum`, `mean` | beliebig | ohne Zuteilung |

Konstruktion: `new`, `zeros`, `from_vec`, `scalar`, `from_le_bytes` (B-04).
Formen werden mit geprüfter Multiplikation berechnet, ein Überlauf ist
`SizeOverflow` und keine stille Verkürzung.

## Tests

`cargo test` im Crate, 37 Tests, Stand 11.11.2026 alle grün. Zusätzlich
`cargo check` für das `no_std`-Profil, weil `cargo test` mit std übersetzt und
Fehler ohne std verdeckt.

## Nicht enthalten

Autograd (M-03), Transformer und Modell (M-05), Training (M-06), SIMD-Intrinsics,
GPU-Pfade. Jeder Punkt hat eine eigene Phase.

## Lizenz

Apache-2.0, Copyright (c) 2026 Michael Wroblewski / ShivaCore /
A-TownChain-Okosystems. All Rights Reserved.
