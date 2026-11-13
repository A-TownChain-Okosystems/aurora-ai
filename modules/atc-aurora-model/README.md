# atc-aurora-model

M-05 `model` der Aurora-Referenzimplementierung (Blueprint ATC-AI-LLM-001-B,
Abschnitt 3.5).

## Stand

Dieser Schnitt enthaelt Konfiguration und Parameterhaushalt: welche Tensoren das
Referenzmodell besitzt, wie viele Elemente sie zusammen haben und welche
Konfigurationen unzulaessig sind. Es gibt **keinen** Vorwaertslauf, keine
Attention, keine Gewichte und keine Zuteilung. Wer hier einen Zustand erwartet,
findet keinen.

Die Zahl wird abgeleitet, nicht gesetzt:

```
Embedding        4096 x 256            1 048 576
Attention        6 x 1024 x 256        1 572 864
SwiGLU           6 x 2112 x 256        3 244 032
Block-Normen     6 x 2 x 256               3 072
Finale Norm            256                   256
                                      ----------
gesamt                                 5 868 800
```

## Vorwaertslauf

`forward(&cfg, &weights, &tokens, &mut budget)` rechnet Tokens zu Logits `(T, vocab)`.
Der Pfad ist: Einbettung ziehen, je Block RMSNorm -> Q/K/V -> RoPE -> kausale
Multi-Head-Attention (`j <= i`, sonst `-inf`) -> Ausgabeprojektion -> Restpfad,
dann RMSNorm -> SwiGLU (`wgu` haelt Gate und Up in einer Matrix) -> Down ->
Restpfad. Am Ende abschliessende Norm und gebundener Ausgabekopf.

`Weights::from_seed(&cfg, seed)` erzeugt Gewichte deterministisch (splitmix64,
kein `rand`, keine Betriebssystemzufall). `Weights::param_count()` muss
`param_count(&cfg)` entsprechen; ein Test haelt diese Invariante fuer die kleine
Konfiguration und fuer Ref-1 fest.

Gerechnet wird ausschliesslich mit `atc-aurora-tensor`; dieses Crate enthaelt
keine eigene Tensor-, Norm- oder Attention-Mathematik, nur die Verdrahtung.

### Artefakt schreiben und lesen

```rust
let mut bytes = Vec::new();
let digest = artifact::write(&cfg, &weights, &mut bytes)?;   // Anker fuer spaeter

let opts = artifact::ReadOptions::strict(1 << 30);           // max_bytes ist Pflicht
let a = artifact::read(&bytes, artifact::TrustAnchor::ExpectedDigest(digest),
                       &opts, &mut budget, Some(&cfg))?;
assert!(a.trustworthy());
let w = a.to_weights()?;                                     // die einzige Vollkopie
```

`read` prueft in der Reihenfolge des Vertrags: `max_bytes` (vor dem Hashen),
Kennung, Version, Flags, reservierte Felder, Typ, Layout, Architektur,
Dateilaenge, Datenmenge gegen Budget, Tabelle (Namen, Formen, Laengen,
lueckenlose Offsets), Digest, Anker, optionale Architekturpruefung, optionale
Endlichkeitspruefung. Ein Strukturfehler wird immer vor einem Ankerfehler
gemeldet.

Zwei Dinge, die leicht falsch verstanden werden: der Digest belegt **Integritaet,
nicht Authentizitaet** (ohne externen Anker ist `trustworthy = false`), und
`tensor_bytes` liefert Bytes, keine `f32`-Sicht — in `unsafe`-freiem Rust gibt es
keine ausgerichtete Sicht, deshalb dekodiert `tensor_f32` bewusst je Tensor.

Grenzen: initialisierte, nicht trainierte Gewichte. Die Tests pruefen Rechnung
und Grenzen (B-19 Kausalitaet, B-20 512 Token mit endlichen Werten), nicht
Sprachqualitaet. Kein KV-Cache, kein Batching, kein Sampler.

## Artefakt (B-21)

Der Vertrag ist spezifiziert **und umgesetzt**:
[`ATC-AI-LLM-002`](../../docs/specs/ATC-AI-LLM-002-ATCM-ARTIFACT-CONTRACT.md) legt
Byte-Layout (96-Byte-Header, 64-Byte-Tabelleneinträge), die kanonische
Tensor-Reihenfolge, den Digest über die ganze Datei mit genulltem Digest-Feld,
die fail-closed Lesereihenfolge, Größengrenzen vor Allokation, die
Fehlertaxonomie und 28 adversariale Parser-Tests fest. Kern des
Vertrauensmodells: ein gültiger Digest belegt **Integrität, nicht
Authentizität** — ohne externen Anker ist ein Artefakt nicht vertrauenswürdig.

`ROPE_BASE` ist mit 10 000 **normativ** festgelegt (`ATC-AI-LLM-001` 1.5.0, K6
entschieden am 11.11.2026); andere Basen gelten nur mit einer neuen
Architekturversion. `NORM_EPS` (1e-8) ist weiterhin Implementierungsparameter.
Beide stehen im Artefaktformat als Felder, damit ein Modell seine Numerik selbst
deklariert: der Leser prüft sie gegen das erwartete Profil und lehnt Abweichungen
ab, statt still falsch zu rechnen.

## B-18: entschieden und belegt

B-18 ist am 11.11.2026 mit **A + F1** geschlossen: Sollwert **5 868 800**, K5 bleibt
(`d_ff` 704), die abschliessende Norm zaehlt mit
(`P = L(4d^2 + 3 d d_ff + 2d) + V d + d`). Der Test
`b_18_parameterzahl_der_referenz_ist_abgeleitet` prueft die Zerlegung gegen genau diese
Summe, `b_18_historischer_stand_ist_dokumentiert` haelt fest, woher die alte Zahl kommt.

Die aeltere Zahl **5 771 776** aus dem Blueprint ist der Stand **vor** der
K5-Entscheidung und wird nicht geglaettet. Die Ableitung aus der eingefrorenen
Konfiguration (K4 Vokabular 4096, K5 `d_ff` 704, sechs Bloecke, geteilte Einbettung,
keine Bias-Terme) ergibt 5 868 800, ohne die abschliessende Norm 5 868 544. Differenz
zur alten Zahl: **97 024** beziehungsweise **96 768** Elemente.

Die Differenz laesst sich zerlegen: 97 024 = 379 x 256, ohne die abschliessende Norm
96 768 = 6 x 63 Zeilen. Sie entspricht damit genau der abschliessenden Norm plus 63
Zeilen je Block, also `d_ff` 683 statt 704. Mit `d_ff` 683 und ohne abschliessende Norm
ergibt die Ableitung exakt 5 771 776, und 683 ist gerade aufgerundetes 8/3 x 256. Der
Sollwert ist damit belegt die Zahl **vor** K5: der Blueprint nennt in §6 selbst `d_ff`
683 oder 704 als offenen Punkt, und 683 gegen 704 sind genau 96 768 Parameter, also
1,68 Prozent. Die Modellspezifikation nennt in Abschnitt 7.1.1 nach K5 5 868 544 aus
der Formel `P = L(4d^2 + 3 d d_ff + 2d) + V d`; diese Formel zaehlt die abschliessende
Norm nicht mit, mit ihr sind es 5 868 800, und so rechnet dieses Crate.

Entscheidungsvorlage mit Rechenweg, Suchraum, zwei exakten Aufloesungen und drei
Optionen: [`docs/decisions/AD-B18-PARAMETERHAUSHALT.md`](../../docs/decisions/AD-B18-PARAMETERHAUSHALT.md).

## Digest-Autorität (AD-FT1)

Die SHA-256-Umsetzung liegt seit dem 13.11.2026 nicht mehr in diesem Crate, sondern in
`atc-aurora-digest` (`ATC-AI-DIGEST-001`). `artifact::sha256` ist ein Re-Export; die
Streaming-API bleibt unverändert. Cross-Crate-Nachweis:
`tests/digest_vectors.rs` prüft die Fassade gegen denselben unabhängigen Orakel-Vektor
wie der Tokenizer.
