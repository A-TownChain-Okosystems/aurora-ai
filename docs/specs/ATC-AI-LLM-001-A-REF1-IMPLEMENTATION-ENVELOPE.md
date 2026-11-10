---
document_id: ATC-AI-LLM-001-A
title: Implementierungsgrenzen Ref-1 (Anhang zu ATC-AI-LLM-001)
version: 1.0.0
status: proposed
owner: A-TownChain-Okosystems
created: 2026-11-10
updated: 2026-11-10
parent: ATC-AI-LLM-001 v1.1.0
layer: L2 (aurora-ai)
security_class: S3
evidence_gate: AURORA-001
---

# Implementierungsgrenzen Ref-1

Anhang zur Spezifikation `ATC-AI-LLM-001`. Dieser Anhang beantwortet die Frage,
die vor der ersten Zeile Produktivcode beantwortet sein muss: was genau ist mit
Ref-1 auf realer Hardware in realer Zeit nachweisbar, und wo verläuft die Grenze
zur Skalierung.

**Status: SPECIFIED, 0 Zeilen Produktivcode.** Alle Zahlen sind berechnet, nicht
gemessen. Die Messung ist Austrittskriterium T2.6 aus der Spezifikation.

---

## 1. Zweck und Geltung

Der Anhang ist verbindlich für P1 bis P4. Er legt fest:

- das korrigierte Rechenmodell (Abschnitt 2 und 3),
- das Speichermodell als eigentliche Grenze (Abschnitt 4),
- den exakten Ref-1-Rahmen mit Schrittbudget (Abschnitt 5),
- das Determinismusverfahren (Abschnitt 6),
- die numerischen Toleranzen und ihre Herleitung (Abschnitt 7),
- die öffentliche API-Fläche, damit keine zweite Implementierung entsteht
  (Abschnitt 8),
- die Abgrenzung zur bestehenden KAI-OS-Struktur (Abschnitt 9),
- was in P1 bis P4 nicht gebaut wird (Abschnitt 10).

Er ersetzt keine Aussage der Spezifikation, sondern präzisiert sie. Bei
Widerspruch gilt die Spezifikation, und der Widerspruch ist zu melden.

---

## 2. Korrektur C1: Die 6ND-Näherung unterschätzt den Aufwand

Die Spezifikation rechnet mit `FLOPs ≈ 6·N·D`. Das ist die Standardnäherung und
für kurze Trainingssequenzen brauchbar. Sie lässt den Attention-Anteil weg, der
mit der Sequenzlänge quadratisch wächst. Genauer gilt je Token:

```text
FLOPs je Token ≈ 6·N + 12·L·d·T
```

Dabei ist `L` die Schichtzahl, `d` die Modellbreite und `T` die Trainings-
Sequenzlänge. Der Faktor 12 enthält Vorwärts- und Rückwärtspass der beiden
Attention-Matrixprodukte.

| Modell | N | T | 6N je Token | Attention-Anteil | Aufschlag |
|---|---|---|---|---|---|
| Ref-1 | 5,77 Mio | 512 | 3,46e7 | 9,44e6 | +27,3 % |
| 0.5B | 517,8 Mio | 2048 | 3,11e9 | 7,55e8 | +24,3 % |
| 3B | 2.939 Mio | 4096 | 1,76e10 | 4,53e9 | +25,7 % |
| 7B | 6.610 Mio | 4096 | 3,97e10 | 6,44e9 | +16,2 % |

Konsequenz: Die Wanduhrzeiten der Spezifikation (Abschnitt 7.3) sind um 16 bis
27 Prozent zu niedrig. Sie werden hier nicht stillschweigend ersetzt, sondern als
Näherung gekennzeichnet und in Abschnitt 5 für Ref-1 exakt gerechnet.

---

## 3. Korrektur C1b: Warum 3B und 7B nicht proportional skalieren

Der Einwand ist berechtigt, die Begründung liegt aber woanders als vermutet. Die
FLOPs skalieren sehr wohl proportional: weil `D = 20·N` gesetzt ist, gilt
`FLOPs ∝ N²`, also Faktor 32,3 von 0.5B zu 3B und Faktor 163 von 0.5B zu 7B. Die
Rechnung in der Spezifikation ist insoweit konsistent.

Nicht proportional skaliert der **Durchsatz**. Vier Gründe:

1. **Auslastung sinkt mit Modellgröße.** Bei 0.5B dominiert Matrixmultiplikation
   mit gutem Datenwiederverbrauch. Bei 7B wird der Schritt speicherbandbreiten-
   gebunden, weil Gewichte und Aktivierungen nicht mehr in den Cache passen. Die
   effektive Leistung je Karte fällt, sie steigt nicht.
2. **Modellparallelität kostet.** 7B passt nicht in eine Karte, also braucht es
   Tensor- oder Pipeline-Parallelität, und jeder Schritt bekommt Kommunikation
   dazu. Der Wirkungsgrad fällt mit der Zahl der Karten unter 1.
3. **Kleinere Batchgröße je Karte.** Aus Speichergründen sinkt die Batchgröße je
   Karte, was den Wirkungsgrad weiter senkt.
4. **Der Attention-Anteil steigt mit T**, siehe Abschnitt 2, und T wächst mit der
   Zielgröße (512, 2048, 4096).

Deshalb gilt: Die FLOPs-Zahlen dürfen als Verhältnis verwendet werden, die
Wanduhrzeiten nicht. Für Terminaussagen ist die Speichergrenze aus Abschnitt 4
die härtere und belastbarere Zahl.

---

## 4. Korrektur C2: Der Speicher ist die bindende Grenze, nicht die Rechenzeit

### 4.1 Zustandsspeicher des Trainings

AdamW braucht je Parameter: Gewicht (BF16), Gradient (BF16), fp32-Mastergewicht,
zwei Momente (fp32). Das sind 2 + 2 + 4 + 4 + 4 = 16 Byte je Parameter.

| Modell | Parameter | Zustand | Bewertung |
|---|---|---|---|
| Ref-1 | 5,87 Mio | 93,9 MB | unkritisch, auch in f32 vollständig |
| 0.5B | 517,8 Mio | 8,3 GB | eine 24-GB-Karte reicht für den Zustand |
| 3B | 2.939 Mio | 47,0 GB | eine 24-GB-Karte reicht nicht, eine 80-GB-Karte knapp |
| 7B | 6.610 Mio | 105,8 GB | zwei 80-GB-Karten nur mit Zustandsaufteilung |

### 4.2 Aktivierungsspeicher

`A ≈ c · B · T · L · d`, mit `c ≈ 48 Byte` je Aktivierungselement in f32 ohne
Wiederberechnung und `c ≈ 8 Byte` mit Gradient-Checkpointing.

| Modell | B | T | ohne Wiederberechnung | mit Checkpointing |
|---|---|---|---|---|
| Ref-1 | 8 | 512 | 0,30 GB | 0,05 GB |
| 0.5B | 8 | 2048 | 24,2 GB | 4,0 GB |
| 3B | 8 | 4096 | 145,0 GB | 24,2 GB |
| 7B | 8 | 4096 | 206,2 GB | 34,4 GB |

### 4.3 Ergebnis: Was die drei Hardwareprofile aus D4 tatsächlich tragen

Die Profile aus D4 sind Planungsziele. Gegen die Zahlen oben gilt konkret:

| Profil | Ref-1 | 0.5B | 3B | 7B |
|---|---|---|---|---|
| CPU, 64 GB RAM | ja | nein (Rechenzeit, nicht Speicher) | nein | nein |
| GPU, 24 GB VRAM | ja | ja, nur mit Checkpointing und kleiner Batch | nein (47 GB Zustand) | nein |
| GPU-Workstation, 48 bis 80 GB | ja | ja, komfortabel | nur mit Zustandsaufteilung und Checkpointing | nein (105,8 GB Zustand) |
| mehrere 80-GB-Karten | nicht nötig | nicht nötig | 2 Karten bei Zustandsaufteilung, 4 realistisch | 2 Karten technisch, 4 realistisch |

Damit ist D4 nicht widerlegt, sondern geschärft: Das Profil "24 GB VRAM" ist ein
Ref-1- und 0.5B-Profil, kein 3B-Profil. Wer 3B trainieren will, braucht
Zustandsaufteilung, nicht nur mehr Rechenzeit.

### 4.4 Inferenzspeicher

KV-Cache je Token bei 8 KV-Köpfen, Kopfdimension 128, BF16:
`2 · L · 8 · 128 · 2 Byte`. Ref-1: 24 KiB je Token, bei 512 Kontext 12 MB. 7B:
128 KiB je Token, bei 4096 Kontext 0,54 GB je Sequenz. Für Ref-1 ist Inferenz
damit speicherseitig trivial, was P8 zu einer reinen Korrektheitsaufgabe macht.

---

## 5. Der exakte Ref-1-Rahmen

### 5.1 Konfiguration, festgeschrieben

| Feld | Wert | Begründung |
|---|---|---|
| `d_model` | 256 | klein genug für CPU, groß genug für echte Attention |
| Schichten | 6 | mindestens 4 nötig, damit Restpfad und Norm zusammenwirken |
| Heads / KV-Heads | 8 / 8 | MHA, damit der GQA-Pfad nicht zusätzlich geprüft werden muss |
| `d_head` | 32 | 256 / 8 |
| `d_ff` | 704 | K5, entschieden 11.11.2026: 2.816 Byte je Zeile sind exakt 44 Cache-Lines und exakt 88 AVX2-Vektoren. 683 ergäbe 2.732 Byte, also 42,69 Cache-Lines und 85,375 Vektoren. Preis: 96.768 Parameter, 1,68 Prozent. |
| Vokabular | 4096 | byte-level BPE mit Byte-Rückfall |
| Kontext `T` | 512 | Attention-Anteil bleibt bei 27 Prozent, Rechnung bleibt beherrschbar |
| Norm | RMSNorm, Pre-Norm, ε = 1e-5 | numerisch stabiler als Post-Norm bei kleiner Tiefe |
| Position | RoPE, θ = 10000 | keine gelernten Positionsparameter |
| MLP | SwiGLU, keine Bias-Terme | drei Matrizen, gated |
| Einbettung | gebunden (Ein- und Ausgabe teilen Gewichte) | spart 1,05 Mio Parameter |
| Präzision | f32 durchgehend | Voraussetzung für den Gradiententest |
| Dropout | 0,0 in der Referenz, optional im Training | 0,0 macht den Gradiententest exakt |

Parameterrechnung, nachvollziehbar:
`6 · (4·256² + 3·256·704 + 2·256) + 4096·256 = 4.819.968 + 1.048.576 = 5.868.544`

### 5.2 Schrittbudget und Lernnachweise

FLOPs je Token bei `T = 512`: `6·5.868.544 + 12·6·256·512 = 4,46e7`.
Bei `B = 8` sind das 4.096 Token je Schritt und `1,83e11` FLOPs je Schritt.

Wanduhr je Schritt bei angenommener CPU-Leistung:

| Leistung | je Schritt | 250 Schritte | 1.000 Schritte |
|---|---|---|---|
| 20 GFLOP/s | 9,0 s | 38 min | 2,5 h |
| 50 GFLOP/s | 3,6 s | 15 min | 1,0 h |

Die drei Lernnachweise aus Abschnitt 10 der Spezifikation, mit Budget:

| Stufe | Umfang | Token-Durchläufe | Schritte | Zeit bei 20 GFLOP/s |
|---|---|---|---|---|
| L1 Überanpassung | 100k Token, 10 Epochen | 1,0 Mio | 244 | 37 min |
| L2 Algorithmik | 50k Sequenzen Länge 33, `T=64`, 3 Epochen | 5,0 Mio | 1.208 | 2,4 h, Attention-Anteil dort nur 3 Prozent |
| L3 Sprachkorpus | 50 Mio Token, 1 Epoche | 50 Mio | 12.207 | etwa 30 h |

Damit ist die Aussage belastbar: Ref-1 kann auf CPU innerhalb eines Arbeitstags
bis L2 geführt werden. L3 ist ein Nachtlauf, kein Kurztest. Wer L3 schneller
will, braucht eine GPU, was D4 entscheidet und nicht dieses Dokument.

Speicherbedarf des gesamten Ref-1-Trainings: 93,9 MB Zustand (16 Byte je Parameter) plus höchstens 60 MB
Aktivierungen mit Checkpointing. Ref-1 läuft damit in jedem Profil aus D4,
einschließlich der 64-GB-CPU.

---

## 6. Determinismusverfahren

Deterministische Parallelität ist möglich, aber nur mit fester Zerlegung.

**Regel:** Eine Reduktion über `n` Elemente wird in eine Blockzahl `k = f(n)`
zerlegt, die ausschließlich von `n` abhängt, niemals von der Threadzahl. Jeder
Block wird seriell summiert. Die Blocksummen werden in Indexreihenfolge seriell
addiert.

Damit ist das Ergebnis unabhängig von der Zahl der Threads, und D1 ist erfüllt,
ohne auf Parallelität zu verzichten.

**Verboten:** atomares `f32`-Aufsummieren, threadabhängige Blockgrenzen,
Reduktion über eine `HashMap`, Sortierung ohne totalen Schlüssel.

**Prüfung:** Ein Test fährt dieselbe Operation mit 1, 2, 4 und 8 Threads und
verlangt bitidentische Ergebnisse. Dieser Test gehört zu T2.1.

---

## 7. Numerische Toleranzen, hergeleitet statt gesetzt

### 7.1 Warum der Gradiententest in f64 rechnen muss

Zentrale Finitedifferenz: `(f(x+h) − f(x−h)) / 2h`. Der Abbruchfehler ist
`O(h²)`, der Rundungsfehler `O(ε/h)`. Das Optimum liegt bei `h ≈ ε^(1/3)`.

- In f32: `ε = 1,2e-7`, also `h ≈ 5e-3`, erreichbare Genauigkeit etwa `1e-4`.
- In f64: `ε = 2,2e-16`, also `h ≈ 6e-6`, erreichbare Genauigkeit etwa `1e-10`.

Deshalb: Der numerische Gradient wird mit `h = 1e-5` und einer Verlustrechnung
in f64 gebildet. Nur so lässt sich ein f32-Autograd sinnvoll prüfen. Die
erreichbare Genauigkeit liegt dann bei etwa 1e-8, also weit unter jeder
Toleranz, die für f32 sinnvoll ist.

### 7.2 Toleranzen

| Prüfung | Referenz | Toleranz |
|---|---|---|
| Gradient je Einzeloperation | f64-Finitedifferenz | relative Abweichung unter 1e-6 |
| Gradient des zusammengesetzten Netzes | f64-Finitedifferenz | relative Abweichung unter 1e-4 |
| Autograd in f32 gegen f64 | f64-Autograd | relative Abweichung unter 1e-3 |

Die 1e-3 sind keine Bequemlichkeit, sondern die Größenordnung der f32-Summierung
über 512 Elemente: Der Rundungsfehler einer Summe wächst mit `√n · ε`, hier
`√512 · 1,2e-7 ≈ 2,7e-6` je Summe, und über mehrere Schichten entsprechend mehr.
Eine Toleranz unter 1e-3 wäre für f32 nicht belastbar.

### 7.3 Zwei Pflichtkonstruktionen für Stabilität

- **Softmax** immer mit Maximumssubtraktion. Test: Logits von ±1e5 liefern
  endliche Werte und Wahrscheinlichkeitssummen von 1,0 innerhalb 1e-6.
- **Kreuzentropie** als log-softmax, nicht als `log(softmax(x))`. Sonst entsteht
  bei kleinen Wahrscheinlichkeiten `log(0)`. Der Gradient vereinfacht sich dabei
  zu `softmax(x) − target`, was zusätzlich als Test prüfbar ist.

---

## 8. Öffentliche API-Fläche

Damit keine zweite Architektur entsteht, wird die Schnittstelle vor dem Code
festgelegt. Namen sind verbindlich, Signaturen dürfen in P2 nur erweitert, nicht
umbenannt werden.

```rust
// atc-aurora-tensor
pub struct Shape(Vec<usize>);
pub enum DType { F32, Bf16 }
pub struct Tensor { /* shape, dtype, storage */ }

pub fn matmul(a: &Tensor, b: &Tensor) -> Result<Tensor>;
pub fn add(a: &Tensor, b: &Tensor) -> Result<Tensor>;
pub fn mul(a: &Tensor, b: &Tensor) -> Result<Tensor>;
pub fn transpose(a: &Tensor, dim0: usize, dim1: usize) -> Result<Tensor>;
pub fn softmax(a: &Tensor, axis: usize) -> Result<Tensor>;
pub fn rms_norm(a: &Tensor, w: &Tensor, eps: f32) -> Result<Tensor>;
pub fn swiglu(a: &Tensor) -> Result<Tensor>;
pub fn cross_entropy(logits: &Tensor, targets: &[u32]) -> Result<(Tensor, Tensor)>;

pub trait Autograd { fn backward(&self, grad: &Tensor) -> Result<Vec<Tensor>>; }
pub struct Graph { /* tape, knots, seeds */ }

// atc-aurora-tokenizer
pub trait Tokenizer {
    fn encode(&self, text: &str) -> Vec<u32>;
    fn encode_bytes(&self, bytes: &[u8]) -> Vec<u32>;
    fn decode(&self, ids: &[u32]) -> Result<Vec<u8>>;
    fn vocab_size(&self) -> usize;
    fn digest(&self) -> [u8; 32];
}
pub fn train_bpe(corpus: &[u8], vocab_size: usize) -> Result<Vocab>;

// atc-aurora-model
pub trait Module { fn forward(&self, x: &Tensor, mask: Option<&Tensor>) -> Result<Tensor>; }
pub struct ModelConfig { /* Felder aus 5.1 */ }
pub fn read_artifact(bytes: &[u8]) -> Result<(ModelConfig, Vec<Tensor>)>;
pub fn write_artifact(cfg: &ModelConfig, params: &[Tensor], meta: &Meta) -> Result<Vec<u8>>;

// atc-aurora-train
pub trait Optimizer { fn step(&mut self, params: &mut [Tensor], grads: &[Tensor]) -> Result<()>; }
pub struct AdamW { /* lr, betas, eps, wd */ }
pub fn train_step(model: &mut dyn Module, batch: &Batch, opt: &mut dyn Optimizer) -> Result<f32>;
pub struct Checkpoint { /* params, moments, rng, data_cursor, step, config_hash */ }
```

Fehlertypen sind typisiert und werden nicht als Zeichenkette zurückgegeben:
`ShapeMismatch`, `DTypeMismatch`, `BudgetExceeded`, `InvalidArtifact`,
`DigestMismatch`, `UnknownFormatVersion`.

---

## 9. Abgrenzung zur bestehenden KAI-OS-Struktur

Das ist der Abschnitt, der die zweite Architektur verhindert.

### 9.1 Widerspruch zu AD-021, der entschieden werden muss

`AD-021` (Sprachstrategie, verbindlich) legt für Aurora AI fest: Rust Core plus
**Python AI-Layer mit PyTorch/ONNX/LLM und ROCm**. Die neuen Entscheidungen
D1 bis D5 verlangen dagegen einen Produktivpfad ohne externe KI-Frameworks.

Das ist ein echter Widerspruch zwischen einer bestehenden Architekturentscheidung
und der neuen Zielsetzung, kein Auslegungsspielraum. Er wird hier nicht still
aufgelöst.

Drei Wege:

1. **AD-021 wird ergänzt** (empfohlen): Python bleibt Referenz, Forschung und
   Validierung, PyTorch/ONNX sind dort erlaubt, aber nicht im Produktivpfad. Der
   Produktivpfad ist ausschließlich Rust und frameworkfrei. Damit bleibt die
   Trennung Rust Core / Python Layer erhalten und I3 aus der Spezifikation gilt.
2. Die neue Zielsetzung fällt teilweise: PyTorch darf in den Produktivpfad, dann
   ist I3 zu streichen und der Nachweis "ohne fremdes KI-Framework" entfällt.
3. Zwei getrennte Produkte: ein frameworkfreies Aurora LLM und ein separat
   benannter PyTorch-Pfad. Das erzeugt genau die zweite Architektur, die
   vermieden werden soll.

Ohne Entscheidung darf P2 nicht beginnen, weil jedes neue Crate sonst gegen eine
verbindliche Entscheidung verstößt.

### 9.2 Es gibt bereits zwei Kopien von aurora-ai

Gemessen am 2026-11-10 gegen `aurora-ai` main:

| Ort | Dateien | identisch | divergent | Bewertung |
|---|---|---|---|---|
| `a-townchain/ai/aurora-ai` | 474 | 472 | 2 | echte Zweitkopie, bereits gedriftet |
| `a-townchain-ecosystem/components/aurora-ai` | 467 | 467 | 0 | regelkonformer Mirror, `docs/` ist per Spiegelregel ausgenommen |

Die zwei divergenten Dateien in `a-townchain` sind
`modules/atc-aistudio/src/routes/notion.ts` und `modules/atc-aistudio/server.ts`.

Damit gilt für die neuen Crates verbindlich:

- Neue Crates entstehen **ausschließlich** in `aurora-ai` (AD-017: Produkt-Repo
  ist die Quelle).
- `a-townchain-ecosystem` erhält sie über `scripts/sync_modules.py --sync`,
  nicht durch Handkopie.
- Die Kopie in `a-townchain/ai/aurora-ai` ist **kein** Bauziel für Ref-1. Sie
  wird entweder über den Sync erneuert oder, nach eigener Entscheidung des
  Owners, entfernt. Sie darf jedenfalls nicht als zweite Quelle weiterleben.
- `a-townchain-os` enthält heute keine Aurora-Kopie (Top-Level: `kai-os`, `docs`,
  `packaging`, `tests`), also entsteht dort keine dritte Wahrheit.

### 9.3 Was nicht neu gebaut wird

- Kein zweiter Task-Scheduler. `atc-aurora-core` hat einen, und
  `ATC-DOC-AI-SSOT-004` verbietet einen zweiten ausdrücklich.
- Kein zweites Modellregister. `ModelHub` bleibt das Register; das native Modell
  wird ein Provider dahinter (Aufgabe #53).
- Kein zweiter Autorisierungspfad. Training und Inferenz berühren
  `authorize_action` nicht.
- Keine Kernel-Abhängigkeit. Ref-1 braucht kein Crate aus L1 oder L3.

---

## 10. Was in P1 bis P4 nicht gebaut wird

Verteiltes Training, GPU-Kernel, Quantisierung, BF16-Pfad, RLHF, Multimodalität,
Instruction Tuning, Modellmarkt, Tokenizer-Oberfläche, zweiter Scheduler,
zweites Modellregister, Ladepfad aus der Chain. Jeder dieser Punkte ist einer
späteren Phase zugeordnet und wird in P1 bis P4 nicht begonnen, auch nicht
"schon mal vorbereitet".

---

## 11. Offene Entscheidungen

| ID | Frage | Wirkung |
|---|---|---|
| K1 | Wird AD-021 um den frameworkfreien Produktivpfad ergänzt (Weg 1 aus 9.1)? | blockiert P2 |
| K2 | Wird `a-townchain/ai/aurora-ai` erneuert oder entfernt? | verhindert die dritte Wahrheit |
| K3 | Welche CPU ist die Referenz für die Durchsatzmessung T2.6? | macht aus Annahmen Messwerte |
| K5 | `d_ff = 683` oder 704? | **ENTSCHIEDEN 11.11.2026: 704**, siehe Spezifikation 7.1.1 |

---

*Dieser Anhang enthält 0 Zeilen Produktivcode. Alle Zahlen sind berechnet; die
zugrunde liegenden Annahmen sind in den Abschnitten 2 bis 5 offen benannt. Die
Messung ersetzt sie erst, wenn T2.6 vorliegt.*
