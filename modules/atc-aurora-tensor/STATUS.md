# STATUS: atc-aurora-tensor

| Dimension | Status | Beleg |
|---|---|---|
| Implementierung | `IMPLEMENTATION_COMPLETE` für P2 | M-01, M-02 und M-03 implementiert und baubar |
| Funktionale Prüfung | `TESTED` (lokal) | B-01 bis B-13 grün, 53 Tests, `cargo test` am 11.11.2026 |
| Performance-Referenz | `PENDING` | T2.6 nicht Teil dieses Crates, global `BLOCKED_EXTERNAL` |
| Gesamtabnahme | `PENDING_ACCEPTANCE` | T2.6 fehlt (K3) |

`CI_VERIFIED` wird ausdrücklich nicht beansprucht: push-getriggerte Workflows
scheitern derzeit am unbestätigten GitHub-Konto. `TESTED` heißt hier lokal
ausgeführt, nicht in CI bestätigt.

## P2-Bedingungen nach Blueprint Kapitel 4

| Bedingung | Erfüllt | Beleg |
|---|---|---|
| `P2_IMPLEMENTATION_COMPLETE` | ja | M-01 bis M-03 im Workspace baubar, `cargo build` und Bare-Metal-`cargo check --target thumbv7em-none-eabihf` grün |
| `P2_FUNCTIONALLY_TESTED` | ja | B-01 bis B-13 grün, 53 Tests aus frischem Klon |
| `P2_PERFORMANCE_VERIFIED` | nein | T2.6 `BLOCKED_EXTERNAL`, keine qualifizierte Maschine (K3) |
| `P2_ACCEPTED` | nein | setzt alle drei vorgenannten voraus |

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
| B-08 | Kreuzentropie, Gradient exakt `softmax − target`, Zielwahrscheinlichkeit 0 | grün (`b_08_*`, zwei Tests) |
| B-09 | Matmul-Gradient gegen zentrale Finitedifferenz | grün (`b_09_*`) |
| B-10 | Kette Softmax und Kreuzentropie | grün (`b_10_*`, zwei Tests) |
| B-11 | RMSNorm, `ε = 1e-8`, normnahe und exakt null Zeile | grün (`b_11_*`, drei Tests) |
| B-12 | Zwei Rückwärtspässe auf dasselbe Blatt, Raute | grün (`b_12_*`, zwei Tests) |
| B-13 | Zusammengesetztes Netz, zwei Schichten, alle Parameter | grün (`b_13_*`) |

B-08 bis B-13 sind aus dem Contract abgeleitet, nicht aus dem Verhalten einer
Vorlage: die Vorlage hatte keinen Rechengraphen. Referenz ist überall die zentrale
Finitedifferenz in f64 mit `h = 1e-5`, gebildet im Test.

## Messwerte gegen die Toleranzen

| Prüfung | Grenze | gemessen |
|---|---|---|
| B-09 dA / dB | 1e-6 | 4,6e-8 / 4,1e-8 |
| B-10 Kette / Softmax-JVP | 1e-6 | 1,3e-7 / 1,8e-7 |
| B-11 dx / dw | 1e-5 | 1,6e-6 / 2,4e-7 |
| B-13 w0 / W1 / W2 | 1e-3 (f32-Pfad) | 4,6e-8 / 5,0e-6 / 4,3e-7 |

B-13 liegt gemessen auch unter der strengeren 1e-4-Grenze, die der Contract für
einen f64-Pfad nennt; geprüft wird die f32-Bedingung, weil das Crate f32 als
einzigen Puffer führt.

## Wirksamkeit der Autograd-Tests

Fünf Mutationen am Rückwärtspfad, jede von den erwarteten Tests erkannt:

| Mutation | erkannt von |
|---|---|
| `matmul`-Gradient ohne Transponierung des zweiten Faktors | B-09, B-12, B-13 |
| Softmax-Gradient aus dem Eingang statt aus dem Ausgang | B-10 Softmax-JVP |
| Kreuzentropie-Gradient mit `1/n` statt Summe | B-08, B-10 |
| RMSNorm ohne den zweiten Term der Ableitung | B-11 |
| Akkumulation überschreibt statt addiert | B-12 Raute |

Ehrlich dazu: die Akkumulations-Mutation wird nur von der Raute erkannt, nicht vom
Test der zwei Rückwärtspässe, weil dieser die Addition im dauerhaften
Parameterspeicher prüft und nicht die Verteilung im Pass. Beide Tests bleiben.

Drei Mutationen am Operationssatz aus Phase A bleiben gültig (Matmul ohne letzten
Summanden, Softmax ohne Maximum-Subtraktion, Transponierung ohne Indexvertauschung).

## Zuständigkeitsgrenze

`abhaengigkeitsregel_nur_libm` liest das eigene `Cargo.toml` und prüft bei jedem
Testlauf: als Abhängigkeit ist ausschließlich `libm` zulässig, und kein
Kernel-Crate, keine zweite Tensor-Bibliothek (torch, candle, burn) darf im
Manifest auftauchen. Damit ist die Grenze aus AD-026 und AD-028 nicht nur
dokumentiert, sondern bricht den Testlauf, wenn sie verletzt wird.

## Vertrag der Autograd-Seite

`AUTOGRAD-CONTRACT.md` hält fest, welche zehn Operationen eine Ableitung haben und
warum die übrigen keine bekommen, welche Ableitung verbindlich ist, welche
Speicherlebensdauer der Graph verlangt und wo der Contract schweigt (Festlegungen
F1 bis F5).

## Offen

- **K6**: LayerNorm, GELU, Dropout und Einbettungs-Lookup nennt der P2-Absatz,
  die API-Fläche und die 24 Testfälle nennen sie nicht. Entscheidung des Owners:
  nachziehen mit Testfällen oder den P2-Absatz auf die API-Fläche einengen.
- **T2.6 / K3**: Durchsatzmessung braucht die qualifizierte Maschine.
- SIMD und Ausrichtung: bewusst nicht Teil von P2. Der Kern bleibt die einzige
  semantische Implementierung.
