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

Grenzen: initialisierte, nicht trainierte Gewichte. Die Tests pruefen Rechnung
und Grenzen (B-19 Kausalitaet, B-20 512 Token mit endlichen Werten), nicht
Sprachqualitaet. Kein KV-Cache, kein Batching, kein Sampler.

## Artefakt (B-21)

Der Artefaktvertrag ist spezifiziert, aber **nicht implementiert**:
[`ATC-AI-LLM-002`](../../docs/specs/ATC-AI-LLM-002-ATCM-ARTIFACT-CONTRACT.md) legt
Byte-Layout (96-Byte-Header, 64-Byte-Tabelleneinträge), die kanonische
Tensor-Reihenfolge, den Digest über die ganze Datei mit genulltem Digest-Feld,
die fail-closed Lesereihenfolge, Größengrenzen vor Allokation, die
Fehlertaxonomie und 28 adversariale Parser-Tests fest. Kern des
Vertrauensmodells: ein gültiger Digest belegt **Integrität, nicht
Authentizität** — ohne externen Anker ist ein Artefakt nicht vertrauenswürdig.

`ROPE_BASE` und `NORM_EPS` sind heute explizite Implementierungsparameter des
Crates (10 000 und 1e-8) und **noch nicht normativ**. Sie stehen deshalb im
Artefaktformat als Felder, damit ein Modell seine eigene Numerik deklariert; die
normative Festlegung ist offener Punkt K6.

## Befund zu B-18

Das Blueprint nennt fuer B-18 den Sollwert **5 771 776**. Die Ableitung aus der
eingefrorenen Konfiguration (K4 Vokabular 4096, K5 `d_ff` 704, sechs Bloecke,
geteilte Einbettung, keine Bias-Terme) ergibt **5 868 800**, ohne die
abschliessende Norm **5 868 544**.

Differenz: **97 024** beziehungsweise **96 768** Elemente. Das ist keine
Rundung und keine Frage der Zaehlweise, sondern eine offene Entscheidung: der
Sollwert und die Architektur passen nicht zueinander. Der Test
`b_18_blueprint_sollwert_weicht_von_der_ableitung_ab` haelt die Abweichung fest,
damit sie nicht stillschweigend verschwindet.

Bis der Owner entscheidet, gilt: die Implementierung folgt der Architektur, nicht
dem Sollwert. Ein Test, der 5 771 776 erzwingen wuerde, waere entweder falsch
oder wuerde eine unbekannte Architekturaenderung verstecken.

Die Differenz laesst sich zerlegen: 97 024 = 379 x 256, ohne die abschliessende
Norm 96 768 = 6 x 63 Zeilen. Sie entspricht damit genau der abschliessenden Norm
plus 63 Zeilen je Block, also `d_ff` 683 statt 704. Mit `d_ff` 683 und ohne
abschliessende Norm ergibt die Ableitung exakt 5 771 776, und 683 ist gerade
aufgerundetes 8/3 x 256. Der Sollwert ist damit belegt die Zahl **vor** der K5-Entscheidung: der Blueprint
nennt in §6 selbst `d_ff` 683 oder 704 als offenen Punkt, und 683 gegen 704 sind
genau 96 768 Parameter, also 1,68 Prozent. Die Modellspezifikation nennt in
Abschnitt 7.1.1 nach K5 5 868 544 aus der Formel
`P = L(4d^2 + 3 d d_ff + 2d) + V d`. Diese Formel zaehlt die abschliessende Norm
allerdings nicht mit; mit ihr sind es 5 868 800, und so rechnet dieses Crate.

Entscheidungsvorlage mit Rechenweg, Suchraum, zwei exakten Aufloesungen und drei
Optionen: [`docs/decisions/AD-B18-PARAMETERHAUSHALT.md`](../../docs/decisions/AD-B18-PARAMETERHAUSHALT.md).
