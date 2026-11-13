---
document_id: ATC-AI-LLM-001
title: Aurora Foundation Model (Aurora LLM): P0 Architektur und Spezifikation
version: 1.6.0
status: accepted
owner: A-TownChain-Okosystems
created: 2026-11-10
updated: 2026-11-13
standard: atc-standards
layer: L2 (aurora-ai)
security_class: S3
evidence_gate: AURORA-001
---

# Aurora Foundation Model (Aurora LLM)

**Status: SPECIFIED. Implementierung 0 von 11 Phasen.**

Dieses Dokument ist P0 des Entwicklungsfahrplans. Es legt Architektur, Numerik,
Determinismus, Artefaktformat, Phasengrenzen und Abnahmekriterien fest. Es
enthält keinen Produktivcode und behauptet keine Implementierung.


## Revision 1.6.0 vom 2026-11-13: Zielprofil Aurora-1B als Zielzeile (nicht normativ)

Der Owner hat am 13.11.2026 bestätigt: B-18 bleibt geschlossen (A + F1), das
1B-Parameterprofil gilt mit **1 036 077 056** Parametern, `d_ff` = 5632 bleibt, und
`V` = 4096 ist **Zielwert**, kein Normwert. Das Profil wird als Zielzeile neben
Abschnitt 7.2 geführt (neuer Abschnitt 7.2.1).

Das ist **keine** rückwirkende normative Spezifikation des bestehenden Ref-1-Modells:
K4 (`V` = 4096 für Ref-1), K5 (`d_ff` = 704) und K6 (`rope_base` = 10 000) bleiben
unverändert, und keine Zahl in den Abschnitten 7.1 und 7.2 ändert sich. Kein Code,
keine Parameterzählung und kein Test wurde angefasst.

## Revision 1.5.0 vom 2026-11-11: `rope_base` normativ festgelegt (K6 entschieden)

K6 ist entschieden (Owner, 11.11.2026, Option A der Vorlage
`docs/decisions/AD-K6-ROPE-BASE.md`):

- **Für das bestehende Aurora-Modellprofil ist `rope_base = 10 000` Normwert.**
- Das ATCM-Artefakt speichert weiterhin den tatsächlich verwendeten Wert
  (`ATC-AI-LLM-002` §3), damit ein Modell seine Numerik selbst deklariert.
- Der Leser prüft den Artefaktwert gegen das erwartete Architekturprofil und
  lehnt Abweichungen ab (`ArchMismatch`).
- Andere RoPE-Konfigurationen werden **nur** über eine neue Architekturversion
  eingeführt, niemals durch stillschweigende Änderung am bestehenden Profil.

Kein Gewicht, kein Byte und kein Test ändert sich dadurch: der Wert galt bereits,
er ist ab jetzt normativ statt implizit.

## Revision 1.4.0 vom 2026-11-11 (F1): die abschliessende Norm gehört in die Formel

Der Owner hat am 11.11.2026 zwei Entscheidungen bestätigt, **A** und **F1**.

**F1 — Formel korrigiert.** Die Parameterformel lautet ab hier verbindlich

    P = L·(4d² + 3·d·d_ff + 2d) + V·d + d

Der letzte Term `d` sind die Gewichte der abschliessenden RMSNorm vor dem
Ausgabekopf. Die bisherige Formel zählte die beiden Normen **innerhalb** eines
Blocks (`2d`) mit, die eine Norm **ausserhalb** der Blöcke aber nicht, obwohl
eine Pre-Norm-Architektur sie braucht. Das war ein Fehlbetrag von `d` = 256
Elementen, kein Architekturunterschied. Die abschliessende Norm bleibt
Bestandteil der Architektur; es wird nichts entfernt.

**A — Sollwert nachgezogen.** B-18 nennt bisher 5 771 776. Diese Zahl ist der
Parameterstand **vor** der K5-Entscheidung: sie entspricht `d_ff` = 683 ohne
abschliessende Norm. K5 hat `d_ff` = 704 festgelegt (96 768 Parameter, 1,68
Prozent, Cache-Line-Ausrichtung), die Formel ergibt danach 5 868 544 und mit der
abschliessenden Norm, also nach F1, **5 868 800**. B-18 wird auf 5 868 800
nachgezogen; K5 bleibt unverändert bestehen.

Drei Zahlen, die nicht verwechselt werden dürfen:

| Zahl | Bedeutung |
|---|---|
| 5 771 776 | Stand **vor** K5: `d_ff` 683, ohne abschliessende Norm. Historisch, keine gültige Anforderung mehr |
| 5 868 544 | Stand **nach** K5 (`d_ff` 704), ohne abschliessende Norm. Nur als Zwischenrechnung gültig |
| **5 868 800** | Stand **nach** K5 **mit** abschliessender Norm. **Verbindlicher Sollwert** |

Die historische Rechnung wird nicht gelöscht: sie steht im Entscheidungsdokument
`docs/decisions/AD-B18-PARAMETERHAUSHALT.md` (Fassung 0.3.0) und in der
Anmerkung zu B-18 im Blueprint `ATC-AI-LLM-001-B`. Ein Test in
`modules/atc-aurora-model/src/tests.rs` hält die Differenz 97 024 = 96 768 + 256
fest, damit die Herkunft nachprüfbar bleibt und nicht als Rechenfehler
wiederkommt.

Auswirkung auf andere Dokumente: `ATC-AI-LLM-001-B` §3.5 B-18 (Sollwert) und die
Register- und Statusangaben in `modules/atc-aurora-model`. Kein anderer
Parameter, keine andere Formel, keine Phase ändert sich.

---

## Revision 1.1.0 vom 2026-11-10

Die fünf offenen Entscheidungen sind durch den Owner angenommen (D1 bis D5,
Abschnitt 14). Damit ist dieses Dokument nicht mehr Vorschlag, sondern
verbindliche Entwicklungsbasis für P1 bis P4. Präzisiert wird es durch den
Anhang `ATC-AI-LLM-001-A` (Implementierungsgrenzen Ref-1), der das Rechenmodell
korrigiert, das Speichermodell als eigentliche Grenze nachweist und vier neue
Entscheidungen K1 bis K4 stellt.

Geltungsregel aus `AURORA-001`: Architektur ist nicht Implementierung, nicht
Test, nicht CI-Evidenz. Jede Statusaussage in diesem Dokument benennt, welcher
der Stufen `MISSING`, `ARCHITECTURE_ONLY`, `SPECIFIED`, `IMPLEMENTED`,
`TESTED`, `CI_VERIFIED`, `INTEGRATED`, `E2E_VERIFIED` sie angehört.

---

## 1. Zweck

Aurora LLM ist ein Foundation Model, das vollständig im eigenen Ökosystem
entsteht: zufällige Initialisierung der Gewichte, eigene Architektur, eigener
Tokenizer, eigene Trainingspipeline, eigene Inferenz-Engine. Es ist keine
Integration von Llama, GPT, Qwen, Mistral oder deren Gewichten.

### 1.1 Verbindliche Invarianten

| ID | Invariante | Prüfbar durch |
|---|---|---|
| I1 | Die Blockchain liegt nicht im Berechnungspfad des Modells. Kein Tensor, kein Gradient, kein Tokenisierungsschritt hängt von Chain-Zustand, Konsens oder ATC-VM ab. | Abhängigkeitsgraph der Crates, kein `atc-*`-Chain-Crate als Dependency von `atc-aurora-tensor`, `-tokenizer`, `-model`, `-train` |
| I2 | Aurora bleibt eine eigenständige Domäne (AD-021, AD-026 Layer L2) und wird weder ATC-VM noch A-TownChain untergeordnet. | Crate-Abhängigkeitsrichtung |
| I3 | Keine verpflichtende Abhängigkeit von externen KI-Frameworks. Kein PyTorch, TensorFlow, ONNX Runtime, candle, tch, burn, llama.cpp, tokenizers, safetensors im Produktivpfad. | `cargo tree` der neuen Crates, Review je Pull Request |
| I4 | Modellausführung liegt außerhalb des ShivaCore-Kernel-TCB. | Kein Kernel-Symbol, kein Syscall, keine Kernel-Crate als Dependency |
| I5 | Modellgewichte und Modelloutput tragen keine Ausführungsautorität. | AURORA-001 Autoritätsmodell, `authorize_action`-Pfad unverändert |
| I6 | Jedes Artefakt ist digest-gebunden und reproduzierbar. | Manifest mit Inhaltsdigest, Prüftest je Ladepfad |
| I7 | Eine erfolgreiche Kompilierung gilt nicht als Funktionsnachweis. | Abnahmekriterien in Abschnitt 10 |

### 1.2 Ausdrücklich nicht Teil dieses Dokuments

Reinforcement Learning from Human Feedback (ATC-33), Multimodalität (ATC-52),
verteiltes Training über Knoten (ATC-25), Modellmarkt (ATC-29), Instruction
Tuning (P7) und Quantisierung (P9) werden hier nur als spätere Phasen geführt,
nicht spezifiziert.

---

## 2. Ausgangslage: verifizierte Bestandsaufnahme

Methode: frischer Klon von `A-TownChain-Okosystems/aurora-ai`, Commit
`b4858f350fcc8db4d48c1cb20b2f116be53c41fe` (2026-11-06), vollständige
Dateiliste über die Git-Tree-API (527 Einträge), Stichwortsuche über alle
Rust-Quellen.

### 2.1 Was existiert

| Größe | Wert |
|---|---|
| Rust-Quellzeilen gesamt | 3.173 |
| Rust-Dateien | 31 |
| Rust-Crates | 5 (`atc-aurora-core`, `-agents`, `-memory`, `-localization`, `sdk`) |
| Testfunktionen (`#[test]`) | 14, verteilt auf 11 Dateien |
| Python-Implementierung | 4 `__init__.py` mit Docstring, kein Code |
| Modellartefakte (`*.gguf`, `*.safetensors`) in der gesamten Organisation | 0 |

Der vorhandene Rust-Code ist Orchestrierung: Agentenregister, Aufgabenplanung,
Capability-Policy, Freigabe, Audit, Kontextfenster, Vektorspeicher,
Lokalisierung. Das ist wertvoll und bleibt unangetastet.

### 2.2 Was nicht existiert

Stichwortsuche über alle `*.rs` des Repositories, Trefferzahl je Begriff:

```text
tensor 0 · matmul 0 · attention 0 · softmax 0 · transformer 0
tokenizer 0 · vocab 0 · autograd 0 · gradient 0 · backward 0
optimizer 0 · AdamW 0 · checkpoint 0 · logits 0 · rms_norm 0
bf16 0 · f16 0 · embedding 1 (nur im Kommentar von vector_store.rs)
```

Es gibt keine Tensoroperation, keine automatische Differenzierung, keinen
Tokenizer, keine Transformer-Schicht, keine Trainingsschleife und keine
Inferenz. Das Repository enthält keine einzige Zeile Modellmathematik.

### 2.3 Drei Befunde, die vor dem Bau korrigiert werden müssen

**B-1: `ModelHub::inference` liefert eine Zeichenkette, kein Modell.**

`modules/atc-aurora-core/src/model_hub.rs` gibt bei jedem Aufruf
`"[<modell>: <40 Zeichen Prompt> → response (N chars)]"` zurück. Es gibt keinen
Provider, kein Netz, keine Gewichte. Die vorhandenen Tests prüfen nur die Form
dieser Platzhalterantwort und die UTF-8-Grenze bei 40 Zeichen.

**B-2: Der Modellname `shiva-1.0` hat kein Artefakt.**

`LlmRouter` und `ModelHub` führen `shiva-1.0` als Standardmodell mit
Kontextfenster 8192. In der gesamten Organisation existiert keine Gewichtsdatei
und kein Manifest zu diesem Namen. Der Name ist heute eine Zeichenkette, keine
Modellreferenz.

**B-3: `atc-aurora-runtime` hat keinen Code und eine widersprüchliche
Selbstdokumentation.**

Das Verzeichnis enthält kein `Cargo.toml` und keine `.rs`-Datei. Es liegen fünf
`.atc`-Dateien mit je etwa 950 Byte Spezifikationsnotizen. `STATUS.md` behauptet
gleichzeitig "0 `.atc` Dateien (geplant: 5)", während fünf Dateien vorhanden
sind. Der Widerspruch ist ein Dokumentationsfehler, kein Codefehler.

Zusätzlich: Von 31 Rust-Dateien haben 11 Tests. `task_scheduler.rs` (16 KB),
`agent_registry.rs` (13 KB), `capability_policy.rs` (7 KB), `result_collector.rs`
(7 KB), `approval.rs`, `audit.rs`, `event.rs` und `state.rs` haben keinen
einzigen Test, obwohl die CI sie baut.

### 2.4 Einordnung

Die Aussage "Aurora AI hat ein Modell" ist heute nicht belegbar. Die Aussage
"Aurora AI hat ein sauberes Orchestrierungsgerüst ohne Modell" ist belegbar. Der
Unterschied ist der Gegenstand dieses Dokuments.

Die bestehende P0-Aufgabe `#53` (Platzhalter-Inferenz durch einen versionierten
Provider-Adapter ersetzen) bleibt gültig und wird durch dieses Dokument nicht
verdrängt: `#53` definiert den Provider-Vertrag, das native Modell wird später
ein Provider hinter genau diesem Vertrag. Beides getrennt zu halten verhindert,
dass ein echter Modellpfad und ein Platzhalter denselben Status bekommen.

---

## 3. Was "native Implementierung" konkret bedeutet

"Native" ist prüfbar zu definieren, sonst ist es eine Behauptung.

### 3.1 Erlaubt

- Rust-Standardbibliothek.
- Reine Datenformate: `serde`, `serde_json`.
- Kleine Hilfskisten ohne Rechenkern: `half` (BF16-Typ), `sha2` oder `blake3`
  für Digests, `rand` mit festgelegter Bitstrom-Semantik.
- Eigene SIMD-Intrinsics über `std::arch` (P9, hinter identischer Semantik).

### 3.2 Nicht erlaubt im Produktivpfad

BLAS, LAPACK, oneDNN, cuBLAS, rocBLAS, MKL, candle, burn, tch, `tokenizers`,
`safetensors`, ONNX Runtime, llama.cpp.

### 3.3 Konsequenz, die benannt werden muss

Ein eigener Matrixkern erreicht in P2 nicht die Geschwindigkeit von BLAS. Das
ist eine bewusste Entscheidung: Korrektheit und Determinismus vor Durchsatz. Ab
P9 dürfen beschleunigte Kerne hinzukommen, aber nur als austauschbare
Implementierung derselben Semantik mit Toleranztest gegen den Referenzpfad
(Konzept aus `docs/ATC_AI_COMPUTE_BACKEND_CONTRACT.md`, Abschnitt 3 und 6).

---

## 4. Numerik- und Determinismusvertrag

Das ist der Teil, der später am teuersten wird, wenn er jetzt unscharf bleibt.

### 4.1 Deterministizitätsstufen

| Stufe | Bedeutung | Gilt für |
|---|---|---|
| D0 `BITEXACT_LOCAL` | Gleiches Binary, gleiche Maschine, gleiche Threadzahl: bitidentisches Ergebnis | Referenzpfad, alle Tests |
| D1 `BITEXACT_SERIAL` | Ein Thread, feste Reduktionsreihenfolge: bitidentisch über gleiche Zielarchitektur | Training, Checkpoint-Wiederaufnahme |
| D2 `TOLERANT` | Verschiedene Architekturen oder Backends: innerhalb dokumentierter Toleranz | Beschleuniger ab P9 |

Verbindlich: Der Trainingspfad muss mindestens D1 erfüllen. D0 gilt für den
Referenzpfad und die Testmatrix. D2 ist ausschließlich für Beschleuniger
zulässig und braucht je Arbeitslast eine begründete Toleranz.

### 4.2 Verbotene Konstrukte im Trainingspfad

- Nichtdeterministische Parallelreduktion (atomares `f32`-Aufsummieren über
  Threads).
- Schnittstellen, die `f32`-Additionen umsortieren (`fast-math`, freie
  Neuzuordnung).
- Zeit-, Adress- oder Hashmap-Iterationsreihenfolge als Eingang in die
  Berechnung. Der vorhandene `VectorStore` iteriert eine `HashMap`; das ist für
  eine Suche ohne Relevanz, für Training wäre es ein Fehler.
- Zufall aus Systemzeit statt aus einem explizit gespeicherten Seed-Zustand.

### 4.3 Zahlenformat

| Phase | Speicher | Rechnung | Akkumulation |
|---|---|---|---|
| P2 bis P4 Referenz | f32 | f32 | f32, feste Reihenfolge |
| P4 Produktion | BF16 Gewichte, f32 Mastergewichte | BF16 oder f32 je Backend | immer f32 |
| P9 Quantisierung | int8 oder int4 | dequantisiert f32 | f32 |

Begründung: Der numerische Gradiententest (Abschnitt 9, T2.4) ist in BF16 nicht
sinnvoll, weil die Finitedifferenz kleiner ist als die Quantisierungsstufe. Die
Korrektheit wird deshalb in f32 nachgewiesen, BF16 kommt erst, wenn der
Referenzpfad grün ist.

---

## 5. Architektur

### 5.1 Crate-Zuschnitt

```text
modules/atc-aurora-tensor/      Tensortyp, Speicher, Operationen, Autograd   (P2)
modules/atc-aurora-tokenizer/   ATC Tokenizer: BPE, Vokabular, Korpus-Codec   (P1)
modules/atc-aurora-model/       Transformer-Definition, Artefaktformat        (P3)
modules/atc-aurora-train/       Optimierer, Trainingsschleife, Checkpoints   (P4/P5)
modules/atc-aurora-runtime/     Inferenz, KV-Cache, Sampler                  (P8, heute leer)
```

Bestehende Crates bleiben unverändert in ihrer Rolle: `atc-aurora-core` bleibt
Orchestrierung und Autorisierung.

### 5.2 Abhängigkeitsrichtung

```text
atc-aurora-tokenizer     (keine Abhängigkeit zu Tensor)
        │
atc-aurora-tensor  ◄──  atc-aurora-model  ◄──  atc-aurora-train
                              ▲
                              └──────────────  atc-aurora-runtime
                                                        ▲
atc-aurora-core ──(nur über Provider-Vertrag, nicht über Tensoren)──┘
```

Regeln:

- `atc-aurora-core` importiert keine Tensoren. Sonst wandert Modellmathematik in
  die Autorisierungsschicht und der SSOT-Schnitt aus
  `docs/AURORA_RUNTIME_EXECUTION_SSOT_DECISION.md` bricht.
- Kein Zyklus. `tensor` kennt kein `model`, `model` kennt kein `train`.
- Kein neues Crate importiert ein Chain-Crate (Invariante I1).

### 5.3 Bezug zu bestehenden Standards

| Standard | Bezug |
|---|---|
| ATC-22 HAL | CPU/GPU/NPU-Security-HAL, Backend-Grenze ab P9 |
| ATC-25 Tensor Compute | Tensor-Chunking, Integrität, Verteilung; hier nur referenziert, ab P9 relevant |
| ATC-26 XAI | Transparenzpflichten, ab P7 |
| ATC-27 AI Model Auditing | Auditierbarkeit der Gewichte und Läufe |
| ATC-33 AI Feedback RLHF | P7 und später |
| ATC-39 AI Model Versioning | Manifest, Versionierung, Deployment |
| ATC-DOC-AI-HW-001 | Compute-Backend-Vertrag, Konformitätstests |

---

## 6. Artefaktformat ATC Model Artifact v1 (`.atcm`)

Ein eigenes Format, kein GGUF. Begründung: GGUF ist ein Fremdformat mit eigener
Versionierung und wird von Fremdwerkzeugen gepflegt; ein eigenes Format erlaubt
Digest-Bindung, Versionsprüfung und Ablehnung unbekannter Felder nach eigenem
Vertrag.

```text
Header
  magic           "ATCM"              4 Byte
  format_version  u16                 = 1
  flags           u16
  reserved        u32                 = 0
  manifest_len    u64
  manifest_sha256 32 Byte
Manifest (UTF-8 JSON, kanonisch serialisiert)
  model_id, model_version
  arch: decoder_only_transformer
  config: d_model, n_layers, n_heads, n_kv_heads, d_ff, vocab_size,
          max_context, rope_theta, norm_eps, tie_embeddings, mlp_kind
  dtype, byte_order
  tokenizer: id, version, vocab_sha256, merge_table_sha256
  tensors: [{name, shape, dtype, offset, length, sha256}]
  training: {step, tokens_seen, seed, dataset_manifest_sha256}
  produced_by: {crate, crate_version, source_commit}
Tensor-Block (ausgerichtet, Little Endian, row major)
```

Verbindlich:

- Das Laden lehnt unbekannte `format_version` ab. Kein stilles Weiterlaufen.
- Jeder Tensor wird gegen `sha256` geprüft, bevor er benutzt wird.
- Die kanonische JSON-Serialisierung ist festgelegt (Schlüssel sortiert, keine
  Leerzeichen, keine Fließkommazahlen im Manifest außer als String). Sonst ist
  der Manifest-Digest nicht reproduzierbar.
- Ein Modell ohne Tokenizer-Referenz ist ungültig. Text ohne Tokenizer ist nicht
  reproduzierbar.

---

## 7. Modellkonfigurationen

Alle Parameterzahlen sind aus der Formel
`P = L·(4d² + 3·d·d_ff + 2d) + V·d + d` berechnet (Pre-Norm, SwiGLU mit drei
Matrizen, gebundene Ein- und Ausgabeeinbettung, abschliessende RMSNorm vor dem
Ausgabekopf), nicht gemessen. Der letzte Term `d` ist die abschliessende Norm;
er fehlte bis Revision 1.4.0 (Entscheidung F1, 11.11.2026).

### 7.1 Aurora-Ref-1, das eigentliche erste Ziel

| Feld | Wert |
|---|---|
| Rolle | Referenzmodell für Korrektheit und Lernnachweis, kein Produktmodell |
| Parameter | 5.868.800 (5,87 Mio) |
| `d_model` / Schichten / Köpfe | 256 / 6 / 8 (KV-Köpfe = 8) |
| `d_ff` | 704 |
| Vokabular | 4.096 |
| Kontext | 512 |
| Präzision | f32 |
| Norm | RMSNorm, Pre-Norm |
| Position | RoPE, θ = 10 000 (Normwert, K6 entschieden 11.11.2026) |
| Aktivierung | SwiGLU |
| Bias | keine |

Begründung: klein genug, um auf CPU in Minuten bis Stunden zu trainieren, groß
genug, dass Attention und Restpfad echt geprüft werden, und exakt
finite-differenz-prüfbar.

### 7.1.1 Verbindliche Modellkonfiguration, einzige Quelle

Die folgenden Werte sind mit K4 und K5 vom 11.11.2026 beschlossen und ab hier
verbindlich. Ein Crate darf keinen eigenen Default für einen dieser Werte
definieren. Gelesen wird aus dem Artefaktmanifest; eine Konstante mit
abweichendem Wert ist ein Konfigurationsfehler, kein zulässiger Default.

| Feld | Wert | Herkunft |
|---|---|---|
| Parameter, exakt | 5.868.800 | aus der Formel, nicht gemessen; F1 (abschliessende Norm) entschieden 11.11.2026 |
| `d_model` | 256 | Abschnitt 7.1 |
| Schichten `L` | 6 | Abschnitt 7.1 |
| Köpfe / KV-Köpfe | 8 / 8 | Abschnitt 7.1 |
| `d_ff` | 704 | **K5, entschieden 11.11.2026** |
| Vokabular `V` | 4096 | **K4, entschieden 11.11.2026** |
| Kontext `T` | 512 | Abschnitt 7.1 |
| Präzision | f32 | D1 |

Begründung K5, `d_ff = 704`: eine Zeile mit 704 f32 sind 2.816 Byte, also exakt
44 Cache-Lines zu 64 Byte und exakt 88 AVX2-Vektoren; Rest 0. Mit 683 sind es
2.732 Byte, also 42,69 Cache-Lines und 85,375 Vektoren, und jede Zeile überquert
eine Cache-Line. Preis sind 96.768 Parameter, also 1,68 Prozent. Die
Parameterzahl war abgeleitet, die Ausrichtung ist eine Eigenschaft, die man
nachträglich nicht mehr bekommt.

Begründung K4, `V = 4096`: der Ausgabekopf kostet 2·d·V je Token, bei d = 256
also 2,10 MFLOP bei 4096 gegen 4,19 MFLOP bei 8192. Das sind 18,2 gegen 30,7
Prozent der Vorwärts-FLOPs, weil der Rumpf bei dieser Modellgröße klein bleibt.
Je Byte Text ist der Unterschied fast neutral (4,44 gegen 4,01 MFLOP), aber bei
8192 geht fast ein Drittel der Rechnung in den Kopf statt in den Rumpf, und
seltene Merges brauchen mehr Korpus.

Falsifikator für K4: misst P1 auf dem tatsächlichen Zielkorpus weniger als 2,2
Byte je Token, wird die Entscheidung neu bewertet. Maßgeblich ist der
P1-Zielkorpus, kein künstlicher Testkorpus.

### 7.2 Zielgrößen

| Parameter | Aurora-0.5B | Aurora-3B | Aurora-7B |
|---|---|---|---|
| Parameter (berechnet) | 517,8 Mio | 2.939 Mrd | 6.610 Mrd |
| `d_model` / Schichten | 1280 / 24 | 2560 / 36 | 4096 / 32 |
| `d_ff` | 3456 | 6912 | 11008 |
| Vokabular | 32.768 | 32.768 | 32.768 |
| Kontextziel | 4K | 8K | 16K |
| Präzision | BF16 mit f32-Master | BF16 mit f32-Master | BF16 mit f32-Master |
| Bereitstellung | lokal, CPU oder eine GPU | GPU | GPU oder verteilt |

Diese Zahlen sind Entwicklungsziele. Sie sind keine Zusage erreichbarer
Modellqualität.

### 7.2.1 Zielprofil Aurora-1B (Zielzeile, nicht normativ)

Ein eigenes Profil neben den Größen aus 7.2, nicht deren Ersatz. Es leitet sich aus
derselben Formel ab und ändert Ref-1 nicht.

| Parameter | Zielwert | Herkunft |
|---|---|---|
| Parameter (Formel aus 7.1) | **1 036 077 056** | nachgerechnet mit den Werten dieser Tabelle |
| Transformer-Blöcke `L` | 20 | Zielprofil |
| `d_model` | 2048 | Zielprofil |
| Attention-Köpfe / Kopfbreite | 16 / 128 | Zielprofil, 16 · 128 = `d_model` |
| `d_ff` | 5632 | Zielprofil, 8 · 704, also Vielfaches der Cache-Line-Ausrichtung aus K5 |
| Vokabular `V` | 4096 als **Zielwert** | entscheidet der P1-Tokenizer-Benchmark, nicht diese Zeile |
| Kontextziel, Präzision, Bereitstellung | offen | entscheiden P1 und der Rechenpfad |

Zerlegung, nachgerechnet: Einbettung 8 388 608; je Block 51 384 320
(4d² = 16 777 216, 3·d·d_ff = 34 603 008, 2d = 4 096), zwanzig Blöcke
1 027 686 400; abschließende Norm 2 048. Summe 1 036 077 056, also 1,036 Milliarden.
`V` = 4096 trägt davon 8 388 608 bei, 0,81 Prozent: die Vokabulargröße ist im
Parameterhaushalt fast kostenlos und trotzdem die falsche Frage an dieser Stelle.

### 7.3 Rechenbedarf, damit die Skalierung eine Entscheidung bleibt und keine Hoffnung

Formel: Trainingsaufwand `FLOPs ≈ 6·N·D`, Datenmenge nach Chinchilla mit 20
Token je Parameter. Wanduhrzeit bei angenommener effektiver Leistung.

| Modell | Token | FLOPs | CPU 0,05 TF/s | 1 GPU 100 TF/s | 1 GPU 300 TF/s |
|---|---|---|---|---|---|
| Ref-1 | 0,1 Mrd | 4,0e15 | 22 h | unter 1 min | unter 1 min |
| 0.5B | 10,4 Mrd | 3,2e19 | 7.448 d | 3,7 d | 29,8 h |
| 3B | 58,8 Mrd | 1,0e21 | 239.907 d | 120 d | 40 d |
| 7B | 132,2 Mrd | 5,2e21 | 1,2 Mio d | 607 d | 202 d |

Diese Tabelle rechnet mit der Näherung `6·N·D` und lässt den Attention-Anteil
weg. Der Anhang `ATC-AI-LLM-001-A` korrigiert das: je Token kommt `12·L·d·T`
hinzu, also 16 bis 27 Prozent mehr Aufwand, und der Speicher erweist sich als die
härtere Grenze als die Rechenzeit. Die Zahlen hier bleiben als Verhältnis
gültig, nicht als Wanduhrzeit.

Die Durchsatzwerte sind Annahmen, keine Messungen. Der erste echte Messwert
entsteht als Austrittskriterium von P2 (Abschnitt 9, T2.6). Die Aussage, die
sich daraus schon heute sicher ableiten lässt: 0.5B ist auf einer GPU ein
Wochenprojekt, 3B und 7B sind ohne mehrere GPUs nicht erreichbar. Die
Sandbox dieser Umgebung hat keine GPU.

---

## 8. Phasenplan

Jede Phase hat ein prüfbares Austrittskriterium. Eine Phase ohne erfülltes
Kriterium wird nicht als abgeschlossen geführt, unabhängig davon, wie viel Code
entstanden ist.

### P0 Spezifikation (dieses Dokument)

Austritt: dieses Dokument ist reviewt und angenommen. Ergebnis:
`ATC-AI-LLM-001`, Status `SPECIFIED`.

### P1 ATC Tokenizer

Byte-Level-BPE, eigene Implementierung.

- Kodierung ist verlustfrei: `decode(encode(b)) == b` für beliebige Bytefolgen,
  auch für ungültiges UTF-8.
- Training deterministisch: Paarhäufigkeiten, Tie-Break nach Häufigkeit
  absteigend und Paar-Tupel lexikografisch aufsteigend, festgeschrieben im Code
  und im Test. Ohne festen Tie-Break ist BPE-Training nicht reproduzierbar.
- Vokabular und Merge-Tabelle sind digest-gebunden.
- Normalisierung: keine in v1. Bewusste Einschränkung, siehe Abschnitt 14.

Austritt: 20 Testvektoren inklusive Grenzfällen (leere Eingabe, einzelnes Byte,
Emoji, 4-Byte-Sequenz, 1 MB Zufallsbytes), Rundlauf-Eigenschaft über 10.000
Zufallsfolgen, identisches Vokabular bei zwei Läufen aus derselben Quelle.

### P2 Tensor-Engine und Autograd

Vorwärts und rückwärts für: Matmul, Addition, Skalarprodukt, Transponierung,
Softmax mit Maximumssubtraktion, Layer- und RMS-Norm, GELU und SwiGLU, Dropout
(nur Training, deterministisch aus Seed), Kreuzentropie, Einbettungs-Lookup.

Austritt, fünf Tore:

- T2.1 Formen: jede Operation lehnt unpassende Dimensionen typisiert ab, kein
  Panik, kein stilles Senden.
- T2.2 Algebra: Assoziativität, Distributivität, Transponierungsidentität gegen
  von Hand gerechnete Kleinstbeispiele.
- T2.3 Numerik: Softmax mit Logits von ±1e5 liefert keine `NaN` und keine
  `Inf`; Summe der Wahrscheinlichkeiten 1,0 innerhalb 1e-6.
- T2.4 Gradiententest: analytischer Gradient gegen zentrale Finitedifferenz für
  jede Operation und für ein zusammengesetztes Mini-Netz, relative Abweichung
  unter 1e-5 bei `f64`-Referenz, unter 1e-3 bei `f32`.
- T2.5 Speicher: ein Budget von N Bytes wird nie überschritten, geprüft durch
  Instrumentierung des Allokators, nicht durch Beobachtung des RSS.
- T2.6 Durchsatz: Messung der effektiven FLOPs pro Sekunde für die Größen aus
  Abschnitt 7.3, damit die Zahlen dort durch Messwerte ersetzt werden.

### P3 Transformer und Aurora-Ref-1

Austritt: Ref-1 lässt sich instanziieren, ein Vorwärtslauf über 512 Token
liefert endliche Logits, die Kausalmaske ist beweisbar korrekt (Test: Änderung
von Position t darf Logits an Position unter t nicht verändern, an Position ab t
schon), und die Parameterzahl entspricht der Formel auf ±0,1 Prozent.

### P4 Trainings-Engine

AdamW mit entkoppeltem Weight Decay, globales Gradient-Clipping über die
Norm, Warmup und Cosinus-Verlauf, Checkpoint mit vollständigem Zustand.

Austritt:

- Verlust sinkt auf einem festen Mini-Korpus monoton über 200 Schritte
  (Toleranz: höchstens zwei Ausreißer).
- Checkpoint-Wiederaufnahme ist bitidentisch zu einem ununterbrochenen Lauf
  gleicher Länge. Das ist der strengste Test der Phase und deckt RNG-, Momente-
  und Datenzeiger-Fehler auf.
- Ein Lauf mit abweichendem Seed erzeugt andere Gewichte, ein Lauf mit gleichem
  Seed bitidentische.

### P5 Datenpipeline

Sharded Binärformat mit Digest je Shard, Manifest, feste Token-Ströme,
deterministische Stichprobenziehung aus Seed.

Austritt: zwei Läufe über dasselbe Manifest liefern identische Tokenfolgen;
manipulierter Shard wird abgelehnt; Lücke im Manifest wird erkannt.

### P6 Pretraining

Austritt: die drei Stufen aus Abschnitt 10 sind nachgewiesen, mit Zahlen.

### P7 Fine-Tuning

Instruction Tuning auf einem eigenen, lizenzklaren Datensatz. Austritt:
Formatkonformität der Ausgaben über einen festen Prüfsatz, keine Verschlechterung
des Basismodells über einen festen Referenzverlust hinaus.

### P8 Inferenz-Runtime

KV-Cache, Sampler mit festgelegter Temperatur-, Top-k- und Top-p-Semantik,
Kontextverwaltung.

Austritt: identische Ausgabe bei gleichem Seed und gleicher Sampler-Konfiguration
über zwei Prozesse; Cache-Wiederverwendung liefert bitidentische Logits wie ein
Lauf ohne Cache über den vollständigen Kontext.

### P9 Optimierung

Quantisierung und Beschleuniger, jeweils hinter dem Backend-Vertrag, mit
Toleranztest gegen den Referenzpfad.

Austritt: jeder beschleunigte Pfad liefert innerhalb der dokumentierten Toleranz
dasselbe Ergebnis wie der Referenzpfad, und ein fehlender Beschleuniger fällt
sauber auf CPU zurück (Konformitätstest 8 aus ATC-DOC-AI-HW-001).

### P10 Integration in KAI-OS

Blockiert bis das ATC Model ABI existiert (Abschnitt 12). Austritt: Modell läuft
als Nutzerraumdienst über GlobusOS, ohne Kernel-Autorität, mit Audit-Eintrag je
Inferenz.

---

## 9. Testmatrix

| ID | Test | Phase | Erwartung |
|---|---|---|---|
| T1.1 | Rundlauf beliebiger Bytes | P1 | identisch |
| T1.2 | Zwei Tokenizer-Läufe, gleiche Quelle | P1 | identisches Vokabular und gleicher Digest |
| T1.3 | Leere Eingabe, 1 Byte, Emoji, 1 MB Zufall | P1 | kein Panik, verlustfrei |
| T2.1 | Falsche Dimensionen je Operation | P2 | typisierter Fehler |
| T2.2 | Algebraische Identitäten | P2 | exakt in f64, tolerant in f32 |
| T2.3 | Softmax mit Logits ±1e5 | P2 | endlich, Summe 1,0 |
| T2.4 | Finitedifferenz gegen Autograd | P2 | relative Abweichung unter Toleranz |
| T2.5 | Speicherbudget | P2 | Budget nie überschritten |
| T2.6 | Durchsatzmessung | P2 | Messwert dokumentiert |
| T3.1 | Kausalmaske | P3 | Zukunft beeinflusst Vergangenheit nicht |
| T3.2 | Parameterzahl | P3 | Formel ±0,1 Prozent |
| T3.3 | 512 Token vorwärts | P3 | endliche Logits |
| T4.1 | Verlustkurve | P4 | monoton fallend |
| T4.2 | Checkpoint-Wiederaufnahme | P4 | bitidentisch zum Durchlauf |
| T4.3 | Seed-Reproduzierbarkeit | P4 | gleich bitidentisch, ungleich verschieden |
| T4.4 | Gradientenexplosion | P4 | Clipping greift, kein `NaN` |
| T5.1 | Datenreproduzierbarkeit | P5 | identische Tokenfolge |
| T5.2 | Manipulierter Shard | P5 | abgelehnt |
| T6.1 | Überanpassung | P6 | Verlust unter 0,1 nats |
| T6.2 | Algorithmische Aufgabe | P6 | über 95 Prozent Genauigkeit |
| T6.3 | Sprachverlust gegen Baseline | P6 | unter Unigramm-Baseline |
| T8.1 | Sampler-Reproduzierbarkeit | P8 | identische Ausgabe |
| T8.2 | KV-Cache-Äquivalenz | P8 | bitidentische Logits |
| T9.1 | Backend-Toleranz | P9 | innerhalb dokumentierter Toleranz |
| T9.2 | Rückfall auf CPU | P9 | sauberer Pfad |
| T10.1 | Isolation | P10 | keine Kernel-Autorität |
| T10.2 | Audit je Inferenz | P10 | Eintrag vorhanden |

---

## 10. Abnahmekriterium "Lernfortschritt"

Das Kriterium des Auftrags lautet: nachweisbarer Lernfortschritt eines Modells
mit zufällig initialisierten Gewichten. Das ist in drei Stufen zu messen, weil
die Stufen unterschiedliche Fehler aufdecken.

### L1 Überanpassung

Ein Korpus von einer Million Token, Ref-1, Training bis der Trainingsverlust
unter 0,1 nats liegt, dann Reproduktion der Trainingsanfänge.

Beweist: Vorwärts- und Rückwärtspfad, Optimierer, Datenzufuhr arbeiten zusammen.
Deckt nicht ab: ob Attention etwas anderes tut als bigram-Martingale.

### L2 Algorithmische Aufgabe

Eine synthetische Aufgabe mit eindeutiger richtiger Antwort, zum Beispiel
Umkehrung einer Folge über einem kleinen Alphabet oder fortgesetzte Addition
modulo 10. Ausgangsgenauigkeit ist der Zufallswert `1/V`, Zielwert über 95
Prozent auf einer zurückgehaltenen Menge gleicher Verteilung.

Beweist: Attention trägt Information über Positionen, Gradienten fließen durch
den Restpfad, die Maske ist korrekt. Ein Modell mit toter Attention oder falscher
Maske scheitert hier, besteht aber L1.

### L3 Sprachkorpus

Natürlicher Text, Bericht des Validierungsverlusts gegen drei gemessene
Baselines: uniform `ln(32768) = 10,4`, Unigramm und Bigramm über denselben
Korpus. Erfolg ist nicht "Verlust ist gefallen", sondern "Verlust liegt unter
der stärksten Baseline".

### Was diese drei Stufen nicht belegen

Sie belegen keine Sprachfähigkeit, keine Anweisungsbefolgung, keine Wahrhaftigkeit
und keine Sicherheit. Ein Modell, das L1 bis L3 besteht, ist ein funktionierendes
Trainingssystem, kein nutzbares Sprachmodell. Diese Trennung steht hier, damit
sie später nicht verwischt wird.

---

## 11. Fehlende Module

| Modul | Zustand heute | Ziel |
|---|---|---|
| Tensor-Engine | MISSING | `atc-aurora-tensor` |
| Autograd | MISSING | in `atc-aurora-tensor` |
| ATC Tokenizer | MISSING | `atc-aurora-tokenizer` |
| Transformer | MISSING | `atc-aurora-model` |
| Trainingsschleife | MISSING | `atc-aurora-train` |
| Optimierer | MISSING | in `atc-aurora-train` |
| Checkpoint-Format | MISSING | in `atc-aurora-model` |
| Artefaktformat `.atcm` | MISSING | in `atc-aurora-model` |
| Inferenz und KV-Cache | MISSING | `atc-aurora-runtime` (heute nur `.atc`-Notizen) |
| Datenpipeline | MISSING | in `atc-aurora-train` |
| ATC Model ABI | MISSING, nicht in diesem Repository | Standard in `atc-standards` |
| CPU/GPU/NPU-Security-HAL | SPECIFIED (ATC-22), nicht implementiert | ab P9 |

---

## 12. Blocker und Risiken

### R1 Rechenhardware (blockierend ab 0.5B)

Es gibt in dieser Umgebung keine GPU. 0.5B braucht nach Abschnitt 7.3 mindestens
drei bis vier GPU-Tage, 3B und 7B brauchen mehrere GPUs über Wochen. Ohne
verbindliche Hardwarezusage bleiben 0.5B und größer Entwicklungsziele ohne
Termin. Ref-1 und die gesamte Korrektheitsarbeit sind davon nicht betroffen.

### R2 Trainingskorpus (blockierend ab P5)

Im Ökosystem existiert kein Trainingskorpus. Beschaffung, Lizenz, Herkunft und
Zulässigkeit müssen vor P5 geklärt sein, sonst ist das Ergebnis nicht
veröffentlichbar. ATC-27 und ATC-39 verlangen nachvollziehbare Herkunft.

### R3 ATC Model ABI fehlt (blockierend für P10)

Der Auftrag nennt eine native ATC Model ABI als Schnittstelle zwischen Modell und
Laufzeit. Sie existiert nicht. Sie gehört als Standard nach `atc-standards`, nicht
in dieses Repository, sonst entsteht eine zweite Wahrheit. Bis dahin bleibt P10
gesperrt, P1 bis P9 sind nicht betroffen.

### R4 Kein Modell-Provider-Vertrag (offen, Aufgabe #53)

Solange `ModelHub::inference` ein Platzhalter ist, kann ein echtes Modell nicht
sauber angebunden werden. #53 ist damit nicht nur eine Aufräumaufgabe, sondern
Vorbedingung für jede belastbare Inferenzaussage.

### R5 Durchsatz des eigenen Matrixkerns

Ein eigener Kern ohne BLAS ist langsam. Das ist akzeptiert, aber es bedeutet,
dass die Durchsatzmessung aus T2.6 früh kommen muss, weil sie entscheidet, ob
Ref-1 auf CPU in Minuten oder Stunden trainiert.

---

## 13. Erste konkrete Schritte

In dieser Reihenfolge, jeder Schritt einzeln prüfbar:

1. K1 entscheiden (Anhang 9.1). Ohne diese Entscheidung verstößt jedes neue
   Crate gegen AD-021.
2. `atc-aurora-tensor` anlegen: `Tensor`-Typ, `f32`-Speicher, Formprüfung,
   Matmul, Addition. Erster Test: T2.1.
3. Autograd-Gerüst mit numerischem Gradiententest für Matmul und Addition
   (T2.4 in kleinster Form). Erst danach weitere Operationen.
4. Softmax mit Maximumssubtraktion und Stabilitätstest (T2.3).
5. Restliche Operationen bis T2.1 bis T2.6 vollständig sind.
6. Parallel dazu, unabhängig: `atc-aurora-tokenizer` mit Rundlauf- und
   Determinismustests (T1.1 bis T1.3).

Der Schritt, der nicht übersprungen werden darf, ist 3. Ein Autograd ohne
Gradiententest ist eine Behauptung.

---

## 14. Entscheidungen

### 14.1 Angenommen am 2026-11-10

| ID | Entscheidung | Ergebnis |
|---|---|---|
| D1 | Ref-1 als verbindliches erstes Modell, vor jeder Skalierung | **angenommen**, 5,77 Mio Parameter, f32, CPU, zufällige Initialisierung, Loss-Nachweis und reproduzierbare Läufe als Austritt |
| D2 | Keine implizite Unicode-Normalisierung im nativen Tokenizer | **angenommen**, UTF-8 als Eingabeformat, keine NFC- oder NFKC-Umwandlung, verlustfreie Kodierung, versioniertes Vokabular |
| D3 | Eigener, kuratierter Korpus mit nachvollziehbarer Herkunft und Lizenz | **angenommen**, je Datensatz Quelle, Lizenz, Abrufdatum, SHA-256, Sprachzuordnung, Qualitätsbewertung, Duplikatprüfung, Ausschlusskriterien; keine ungeprüften Scrapes |
| D4 | Drei verbindliche Hardwareprofile (CPU 64 GB, GPU 24 GB, Workstation 48 bis 80 GB) | **angenommen als Planungsziele**, geschärft durch den Anhang: 24 GB tragen Ref-1 und 0.5B mit Checkpointing, nicht 3B |
| D5 | ATC Model ABI als eigener normativer Standard | **angenommen**, Zuschnitt: Modellarchitektur und Implementierung in `aurora-ai`, normative ABI in `atc-standards`, Hardware- und Sicherheitsintegration in `globus-os`, Entwicklerschnittstelle in `atc-sdk`. Das ABI ist Schnittstellenstandard, keine zweite Inferenz-Engine |

Damit ist der Kern der Zielsetzung bestätigt: Aurora LLM muss ohne fremde
Modellgewichte, ohne externe Inferenz-Engine und ohne fremdes KI-Framework ein
eigenes Modell trainieren und ausführen. Skalierung folgt erst nach dem
vollständigen Ref-1-Nachweis.

### 14.2 Entscheidungsstand, Stand 11.11.2026

| ID | Frage | Stand |
|---|---|---|
| K1 | Wird AD-021 um den frameworkfreien Produktivpfad ergänzt? | **GESCHLOSSEN** durch AD-052 vom 11.11.2026 |
| K2 | Wird die Zweitkopie `a-townchain/ai/aurora-ai` erneuert oder entfernt? | **GESCHLOSSEN**, entfernt am 11.11.2026 |
| K4 | `d_ff = 683` oder 704? | **ENTSCHIEDEN: 704**, Abschnitt 7.1.1 |
| K5 | Vokabular 4096 oder 8192? | **ENTSCHIEDEN: 4096**, Abschnitt 7.1.1 |

Die Nummernfolge K4 und K5 weicht von der früheren Zählung ab, weil K4 in
Version 1.1.0 die Frage nach `d_ff` trug. Verbindlich ist die Benennung in
dieser Tabelle.

### 14.3 K3 entschieden: First Qualified Hardware

Beschlossen am 11.11.2026. Es wird keine CPU auf Verdacht benannt. Eine nicht
vorhandene oder nicht verfügbare Workstation als Referenz einzutragen wäre
methodisch falsch, weil sie einen Maßstab behauptet, der nie gemessen wurde.

**Referenzregel:** Die erste Maschine, die das vollständige Qualifikationsprofil
erfüllt und für T2.6 exklusiv messen darf, wird die verbindliche CPU-Referenz.
Nicht die erste verfügbare CPU wird Referenz, sondern die erste qualifizierte.

| Kriterium | Vorgabe |
|---|---|
| Architektur | x86-64 |
| SIMD-Minimum | AVX2 |
| Erweiterung | AVX-512, sofern vorhanden |
| NUMA | ein Sockel |
| Arbeitsspeicher | mindestens 1 GB frei für Ref-1 |
| Zugriff | exklusiv während der Messung |
| Takt | stabil, kein Throttling |
| Umgebung | native Ausführung |
| Ausgeschlossen | Shared Container, CI-Läufer |

**Messprotokoll:** verbindlich nach `ATC-AI-LLM-001-C` (T2.6-Messprotokoll).
Kurzform: ein Thread, CPU-Affinität und Pinning, Fingerprint aus `/proc/cpuinfo`,
fünf Aufwärmläufe, drei unabhängige Messläufe, Median als Referenzwert, Angabe in
Token pro Sekunde und Byte pro Sekunde. Zusätzlich zu erfassen: CPU-Modell und
Microcode, Kernelversion, Rust-Version und Compilerflags, unterstützte
SIMD-Erweiterungen, Speicher- und NUMA-Konfiguration, Frequenz und Temperatur
während der Messung. Fehlt eines dieser Felder, ist die Messung nicht
reproduzierbar und zählt nicht als Evidenz.

**Governance, Wechsel der Referenz:** Spätere Hardware ersetzt die Referenzwerte
nicht stillschweigend. Ein Wechsel erfordert einen neuen CPU-Fingerprint, ein
neues Messprotokoll, einen Vergleich gegen die bisherige Referenz und eine
dokumentierte Freigabe. Bis dahin bleiben alle Durchsatzzahlen relativ zur
qualifizierten Referenz.

**Status:** noch keine Maschine qualifiziert. Bis zur ersten qualifizierten
Messung bleibt jede Durchsatzzahl eine Annahme und darf nicht als Messwert
zitiert werden.

## 15. Verhältnis zu bestehenden Aussagen im Ökosystem

- Das Modell heißt `Aurora LLM`. Der Name `shiva-1.0` bleibt eine Zeichenkette
  in `ModelHub` und wird erst zu einer Modellreferenz, wenn ein Artefakt mit
  Digest existiert. Bis dahin darf er nicht als existierendes Modell zitiert
  werden.
- Aurora AI bleibt eigenständige Domäne, Layer L2, ohne Kernel- und ohne
  Konsensautorität.
- Die A-TownChain kann optional Identität, Integrität, Herkunft und Abrechnung
  bereitstellen. Sie ist nicht Teil der Berechnung (Invariante I1).

---

Anhang: `docs/specs/ATC-AI-LLM-001-A-REF1-IMPLEMENTATION-ENVELOPE.md`
(Implementierungsgrenzen Ref-1, Rechen- und Speichermodell, API-Fläche,
Abgrenzung zur bestehenden Struktur).

*Dieses Dokument ist Spezifikation, kein Nachweis. Es enthält 0 Zeilen
Produktivcode und keinen einzigen Messwert aus einem eigenen Modelllauf. Die
einzigen belastbaren Zahlen sind die Bestandsaufnahme in Abschnitt 2, belegt
gegen Commit `b4858f350fcc8db4d48c1cb20b2f116be53c41fe`, und die aus der
Parameterformel berechneten Größen in Abschnitt 7, die als berechnet und nicht
als gemessen gekennzeichnet sind.*
