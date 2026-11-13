---
document_id: ATC-AD-AI-P4
title: AD-P4 — Trainingskette und Inferenzpfad der Aurora-Referenz
version: 1.0.0
status: proposed
owner: A-TownChain-Okosystems
created: 2026-11-13
updated: 2026-11-13
standard: ATC-STD-MD-001
---

# AD-P4 — Trainingskette und Inferenzpfad der Aurora-Referenz

## 1. Frage

Der Blueprint `ATC-AI-LLM-001-B` fuehrt die Phasen P1 bis P8. Mit M-01 bis M-05
stehen Tensor-Kern, Autograd, Tokenizer, Vorwaertslauf und Artefakt. Offen war
die Kette, die aus einem Vorwaertslauf einen lernenden und einen inferenzfaehigen
Lauf macht:

```text
Tokenizer -> Autograd -> Backpropagation -> Optimizer -> Training
          -> Evaluation -> Registry -> Runtime-Inferenz
```

Diese Akte legt fest, **wie** die Kette geschlossen wird, ohne die
Eigentuemerschaft der Teile zu verletzen: eine Tensor-Engine (M-01/M-02), eine
Ableitung (M-03), ein Modell (M-05), eine Trainings-Engine (M-06).

## 2. Befund vor der Entscheidung

1. **Der Graph konnte die Ref-1-Architektur nicht ableiten.** `AUTOGRAD-CONTRACT.md`
   fuehrte zehn Operationen mit Ableitung; Einbettungs-Lookup, RoPE und
   Kopfzerlegung fehlten. Ohne sie gibt es keinen Rueckwaertspass durch das
   Modell, das M-05 gebaut hat. Die Luecke war im Contract als K6 vermerkt.
2. **Der Graph war nicht der Vorwaertslauf.** `forward` rechnet ohne Graph. Ein
   Trainingspfad daneben waere ein zweiter Vorwaertslauf mit eigener
   Fehlerwahrscheinlichkeit.
3. **Der Pruefsummenpfad war nicht dicht.** `Graph::cross_entropy` reservierte im
   Vorwaertspass einen Gradientenpuffer ueber das Budget und gab ihn nie zurueck;
   der Graph nahm beim Aufraeumen weniger zurueck, als er genommen hatte. Der
   Fehler war nicht sichtbar, solange niemand die Buchung prueft.
4. **Es gab keine Mittelung.** `cross_entropy` summiert ueber Zeilen (B-08). Ohne
   definierte Division haengt der Verlust von der Stapelgroesse ab, und zwei
   Laeufe sind nicht vergleichbar.

## 3. Entscheidung

### D1 — Drei Operationen mit Ableitung, sonst keine

Ergaenzt werden genau die Operationen, die der Ref-1-Baum braucht:

| Operation | Warum | Ableitung |
|---|---|---|
| `gather` (Einbettungs-Lookup) | Zeilen ziehen statt multiplizieren | Streuung auf die gelesenen Zeilen, mehrfach gelesen summiert |
| `slice_cols` (Spaltenblock) | Kopfzerlegung von `q`, `k`, `v`, `wo` | Block in sonst leerem Puffer |
| `rope` (Rotation je Kopf) | Positionskodierung vor der Attention | Rotation um `-angle`, keine vierte Winkelfunktion |

Kein `LayerNorm`, kein `GELU`, kein `Dropout`: die Referenz nutzt RMSNorm,
SwiGLU und Dropout 0,0. Eine Ableitung ohne Testfall waere eine Behauptung.
Jede der drei neuen Operationen ist gegen die zentrale Finitedifferenz in f64
geprueft (`p4_gather_*`, `p4_slice_*`, `p4_rope_*`).

### D2 — Ein Vorwaertslauf, zwei Auspraegungen, ein Abgleich

Der differenzierbare Pfad liegt in `atc-aurora-model::graph` und ist gegen
`forward` abgeglichen: derselbe Baum, dieselben Gewichte, Abweichung nur aus der
Summationsreihenfolge der kopfweisen Ausgabeprojektion (Schranke 1e-4 relativ,
Test `p4_graph_logits_stimmen_mit_forward_ueberein`, gebundene und nicht
gebundene Einbettung). Die Formeln selbst stehen weiterhin genau einmal, in den
Operationen von `atc-aurora-tensor`.

Verworfen: `forward` um einen optionalen Graph erweitern (haette jede
Berechnung im heissen Pfad mit Graphknoten belastet) und ein zweites Modell im
Trainings-Crate (haette die Verdrahtung verdoppelt).

### D3 — Mittelung im Graphen, AdamW als Optimierer

Der Verlust eines Stapels ist die Summe der Sequenzverluste geteilt durch die
Zahl der Zielpositionen; der Faktor wird als `scale`-Knoten vor dem
Rueckwaertspass angewendet. Optimierer ist Adam mit entkoppelter
Gewichtsdaempfung (`w - lr*wd*w`), weil eine L2-Strafe im Gradienten mit der
Momentenschaetzung wechselwirkt. Voreinstellung: `beta1` 0,9, `beta2` 0,999,
`eps` 1e-8, keine Daempfung, keine Begrenzung.

### D4 — Pruefpunkt aus zwei Teilen mit zwei Pruefsummen

Das Modell bleibt ein ATCM-Artefakt (`ATC-AI-LLM-002`) mit dem Digest als
**externem** Anker im Pruefpunkt. Der Trainingszustand ist ein eigenes Format
`AURST001` (Little-Endian, feste Feldreihenfolge, Laengen vor Inhalt) mit eigener
Pruefsumme aus `atc-aurora-digest`. Ohne die Momente des Optimierers ist ein
fortgesetzter Lauf kein fortgesetzter Lauf; der Test
`p4_fortgesetzter_lauf_ist_derselbe_lauf` prueft genau das.

### D5 — Inferenz als Greedy-Dekodierung im Modell

`atc_aurora_model::decode::generate` ist `argmax` ueber die letzte Logitzeile,
je Schritt ein vollstaendiger Vorwaertslauf, ohne KV-Cache. Das ist bewusst die
kleinste inferenzfaehige Form und nicht die spaetere Laufzeit: KV-Cache,
Batching und Sampling gehoeren zu P8/`atc-aurora-runtime`. Der Nutzen ist der
Nachweis, dass der inferenzfaehige Pfad derselbe `forward` ist, den B-19 und
B-20 abdecken.

### D6 — Produktionsbasis ist Stable, kein Nightly (Nachtrag 13.11.2026)

Beim Verifizieren des P4-Stands zeigte sich, dass vier von fuenf Crates mit
Stable 1.83.0 nicht bauen: `unsigned_is_multiple_of` (Model, Tensor, Tokenizer)
und `slice_as_chunks` (Model) sind bis 1.87 instabile Bibliotheksfunktionen.
Der Eigentuemer hat entschieden: **Stable ist die regulaere Produktionsbasis**,
Nightly nur fuer ausdruecklich begruendete Entwicklungsfunktionen. Damit ist
Variante 1 (Nightly festschreiben) verworfen.

Umgesetzt, semantisch identisch:

| Stelle | vorher | nachher |
|---|---|---|
| `model::config` | `n_heads.is_multiple_of(n_kv_heads)` | `n_heads % n_kv_heads != 0` (Nullpruefung steht davor) |
| `model::artifact::read` | `bytes.as_chunks::<4>().0` | `bytes.chunks_exact(4)` (gleiche vollstaendige Vierergruppen) |
| `tensor::autograd::rope` | `d_head.is_multiple_of(2)`, `cols.is_multiple_of(d_head)` | `d_head % 2 != 0`, `cols % d_head != 0` (`d_head == 0` steht davor) |
| `tokenizer::bpe::parse_hex` | `bytes.len().is_multiple_of(2)` | `bytes.len() % 2 != 0` |

Dazu zwei Clippy-Befunde, die nur mit dem Stable-Linter auftreten (der
Nightly-Linter 1.99 kennt sie nicht mehr): `format!` je Byte in zwei
Digest-Tests (jetzt `write!` in einen `String`, gleiche Ausgabe) und eine
entbehrliche Lifetime in `train::state::Cursor`. Keine dieser Aenderungen
beruehrt Modellarchitektur, Tensorsemantik, Gradientenrechnung oder das
Pruefpunktformat.

Konsequenz: kein `rust-toolchain.toml`, der CI-Job bleibt auf `stable`, die
dokumentierte und geprueft untere Grenze ist **1.83.0**. Die fuenf Crates sind
dort gruen (185 Tests).

Offener Punkt, nicht Teil dieser Entscheidung: `atc-aurora-core` zieht
`getrandom 0.4.3`, das Cargo mit Edition-2024-Unterstuetzung verlangt; mit
Cargo 1.83 bricht schon der Download ab. Das betrifft nicht die P4-Kette, muss
aber vor einer Stable-CI ueber alle acht Crates entschieden werden.

## 4. Konsequenzen

- Ein Rueckwaertspass durch das Ref-1-Modell ist moeglich und numerisch belegt:
  Richtungsableitung gegen Finitedifferenz des **unabhaengigen** Vorwaertslaufs
  (`p4_graph_gradient_gegen_finitedifferenz_der_richtung`).
- Der Budgetbefund aus §2.3 ist behoben; `p4_graph_gibt_budget_zurueck` haelt ihn
  geschlossen.
- Die Kette ist lokal gruen: Tensor 61, Model 70 (1 ignoriert), Training 15,
  Digest 5, Tokenizer 34 Tests — mit Stable 1.83.0 und mit Nightly 1.99.
- **Nicht** belegt: Sprachqualitaet, ein Lauf auf Ref-1, Laufzeitbindung. Der
  Verlust sinkt auf einem winzigen Korpus; das ist ein Rechennachweis.
- `CI_VERIFIED` bleibt offen (F-T2, unverifizierte Konto-E-Mail).

## 5. Offene Punkte fuer den Eigentuemer

1. **K6-Rest:** Soll der P2-Absatz des Blueprints auf die tatsaechliche
   Operationsflaeche eingeengt werden (RMSNorm/SwiGlu/kein Dropout), damit
   LayerNorm, GELU und Dropout nicht dauerhaft als Luecke gelesen werden?
2. **Trainingsvertrag:** Soll die Kette einen eigenen normativen Vertrag
   bekommen (`ATC-AI-LLM-005`), oder bleibt sie bis P8 unter dem Blueprint?
3. **Lernratenplanung und Datenaufbereitung:** Beides ist nicht Teil dieser Akte;
   ohne beides ist ein Lauf auf Ref-1 nicht sinnvoll.
