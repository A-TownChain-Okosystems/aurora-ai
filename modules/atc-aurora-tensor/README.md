# atc-aurora-tensor

Nativer Tensor-Kern der Aurora-Referenzimplementierung. Eigentümer der
KI-Rechenoperationen in Layer L2.

| Feld | Wert |
|---|---|
| Module | M-01 `tensor-core`, M-02 `tensor-ops`, M-03 `autograd` |
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
P2-Testfälle. Der Autograd-Teil ist **keine** Migration: die Vorlage hatte keinen
Rechengraphen.

## Umfang

| Operation | Rang | Hinweis |
|---|---|---|
| `add`, `mul` | beliebig | gleiche Formen erforderlich, kein Broadcasting (B-02) |
| `scale`, `relu`, `sigmoid` | beliebig | elementweise |
| `matmul` | 2 | Innenmaße müssen passen (B-01) |
| `transpose` | 2 | |
| `softmax` | 1, 2 | mit Maximum-Subtraktion, zeilenweise |
| `rms_norm` | 1, 2 | Pre-Norm der Ref-1-Konfiguration, `eps > 0` erzwungen |
| `swiglu` | 2 | `silu(x[:h]) · x[h:]`, letzte Achse gerade |
| `cross_entropy` | 2 | Log-Softmax, Summe über Zeilen, liefert Verlust und Gradient |
| `dot`, `sum`, `mean` | beliebig | ohne Zuteilung |

Konstruktion: `new`, `zeros`, `from_vec`, `scalar`, `from_le_bytes` (B-04).
Formen werden mit geprüfter Multiplikation berechnet, ein Überlauf ist
`SizeOverflow` und keine stille Verkürzung.

## Autograd (M-03)

`Graph` besitzt seine Werte und verweist über Indizes, nicht über Referenzen.
Zehn Operationen haben eine Ableitung, alle anderen nicht; es gibt keinen
generischen Rückfall. Der Rückwärtspass ist transaktional: schlägt er fehl, ist
das Budget auf dem Stand vor dem Aufruf und kein Gradient übernommen.

```rust
let mut g = Graph::new();
let x = g.parameter(input);              // Blatt mit Gradientenpflicht
let w = g.leaf(weights);                 // Konstante
let h = g.matmul(x, w, &mut budget)?;
let loss = g.cross_entropy(h, &targets, &mut budget)?;
g.backward(loss, None, &mut budget)?;    // zweiter Aufruf addiert (B-12)
let dx = g.grad(x);
```

Der Abgleich, welche Operationen eine Ableitung brauchen, welche Ableitung
verbindlich ist und welche Speicherlebensdauer der Graph verlangt, steht in
[`AUTOGRAD-CONTRACT.md`](AUTOGRAD-CONTRACT.md).

## Tests

`cargo test` im Crate, 53 Tests, Stand 11.11.2026 alle grün:

| Gruppe | Anzahl |
|---|---|
| Migrationsbelege `mig_*` aus der Vorlage | 12 |
| Vertragstests B-01 bis B-13 | 27 |
| Grenzfälle M-01/M-02 (Überlauf, Datentyp, Softmax-Stabilität, Budgetfreigabe) | 7 |
| Governance (Budget vor Zuteilung für alle Operationen, nur `libm` als Abhängigkeit) | 2 |
| Autograd: Budget, Determinismus, Fehlerfälle | 5 |

Zusätzlich ein echter Bare-Metal-Build statt nur `cargo check`, weil `cargo test`
mit std übersetzt und Fehler ohne std verdeckt:

```
cargo check --lib --target thumbv7em-none-eabihf
```

## Nicht enthalten

Transformer und Modell (M-05), Training (M-06), SIMD-Intrinsics, GPU-Pfade,
Gradient-Checkpointing. Jeder Punkt hat eine eigene Phase. LayerNorm, GELU,
Dropout und Einbettungs-Lookup sind im P2-Absatz genannt, aber nicht in der
API-Fläche und in keinem Testfall; sie sind bewusst nicht gebaut und als K6
offen (siehe `AUTOGRAD-CONTRACT.md` Abschnitt 7).

## Lizenz

Apache-2.0, Copyright (c) 2026 Michael Wroblewski / ShivaCore /
A-TownChain-Okosystems. All Rights Reserved.
