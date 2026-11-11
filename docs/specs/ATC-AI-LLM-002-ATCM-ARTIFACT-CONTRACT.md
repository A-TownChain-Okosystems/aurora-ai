---
document_id: ATC-AI-LLM-002
title: Artefaktvertrag ATC Model Artifact v1 (.atcm) — Layout, Digest, Vertrauensmodell, adversarial getestete Parser-Grenzen
version: 1.1.0
status: proposed
owner: A-TownChain-Okosystems
created: 2026-11-11
updated: 2026-11-11
parent: ATC-AI-LLM-001 v1.4.0, Blueprint ATC-AI-LLM-001-B (B-21)
layer: L2 (aurora-ai)
security_class: S3
evidence_gate: AURORA-001
---

# Artefaktvertrag ATC Model Artifact v1 (`.atcm`)

**Status: SPECIFIED, Implementierung 0 Zeilen.** Dieses Dokument legt den
Vertrag fest, nicht seine Umsetzung. Es enthält keine Behauptung, dass ein
Leser oder Schreiber existiert.

## Revision 1.1.0 vom 2026-11-11 — Gate-Prüfung G1 bis G5

Die Fassung 1.0.0 wurde am 11.11.2026 gegen den Vertragstext selbst geprüft
(Protokoll: `docs/audits/2026-11-11-ATC-AI-LLM-002-GATE-REVIEW.md`). Sechs
Befunde sind eingearbeitet:

| Befund | Änderung |
|---|---|
| F-01 | Header sind **104** Byte, nicht 96 (Feldsumme 4+2+2+44+4+4+4+8+32). Alle vier Textstellen korrigiert |
| F-02 | Nicht gebundene Einbettung braucht `lm_head`; `tensor_count` und kanonische Folge entsprechend erweitert |
| F-03 | Grenze „Elemente je Tensor ≤ 2^28" ergänzt |
| F-04 | Felder sind nicht ausgerichtet zu lesen; Alignment-Annahme ausdrücklich verboten |
| F-05 | Neuer Abschnitt 8.1 (Ressourcenerschöpfung: zwei Grenzen, keine Vollkopie, linearer Aufwand, Zuteilungsreihenfolge, keine Ratenbegrenzung) und konsistente Grenzen (`tensor_count` ≤ 8194 zu `n_layers` ≤ 1024) |
| F-06 | Präzedenz: Strukturfehler immer vor Ankerfehlern |

Testfälle steigen von 28 auf 34 (AT-29 bis AT-34).

---

## 0. Warum dieses Dokument vor der Implementierung steht

B-21 verlangt „Artefakt schreiben, lesen, prüfen". Das ist zu wenig für einen
Vertrag, den später Validatoren, Knoten und Trainingsläufe teilen. Ein
Dateiformat ist eine Schnittstelle mit drei Eigenschaften, die man nachträglich
nicht mehr reparieren kann: **Byte-Layout**, **Prüfregeln beim Lesen** und
**Vertrauensmodell**. Deshalb steht hier zuerst der Vertrag, danach der Code.

Verbindliche Vorgabe des Owners vom 11.11.2026: erst vollständig spezifizieren,
dann implementieren, dann mit adversarialen Parser-Tests absichern. Keine
Trainingspipeline vor einem stabilen Artefaktvertrag.

## 1. Geltung und Nicht-Ziele

Gültig für Ref-1 und alle Modelle derselben Architekturfamilie (Pre-Norm,
SwiGLU, RoPE, gebundene Einbettung, f32). Kennung `ATCM`, `format_version` 1.

Nicht Teil dieses Dokuments: Optimiererzustand, Trainingsfortschritt
(Checkpoints sind ein eigenes Format, M-06), Kompression, Verschlüsselung,
Signaturverfahren (Abschnitt 12 nennt die Lücke, entscheidet sie aber nicht),
KV-Cache, Quantisierung.

## 2. Vertrauensmodell: Integrität ist nicht Authentizität

Der Digest dieses Formats ist eine **kryptografische Prüfsumme über den Inhalt**.
Er beantwortet genau eine Frage: *wurden diese Bytes seit dem Schreiben
verändert?* Er beantwortet **nicht**, ob sie von einem vertrauenswürdigen
Erzeuger stammen — wer ein Artefakt manipulieren kann, kann den Digest neu
berechnen. Ein gültiger Digest ist deshalb **kein** Vertrauensanker.

Daraus folgen drei Regeln, die der Leser einhalten muss:

1. **R1 — Kein Vertrauen ohne externen Anker.** Ein Artefakt wird nur geladen,
   wenn der Aufrufer den erwarteten Digest aus einer vom Artefakt
   unabhängigen Quelle mitbringt (`expected_digest`), oder wenn eine
   Signatur über den Digest mit einem vertrauenswürdigen Schlüssel geprüft
   wurde. Ohne Anker ist das Ergebnis „formal gültig, nicht vertrauenswürdig"
   und muss als solches im Rückgabewert erscheinen, nicht als „geladen".
2. **R2 — Prüfen vor Zuteilen.** Alle Längen, Offsets und Produkte werden aus
   den Bytes **gegen** die Grenzen dieses Dokuments geprüft, bevor ein Puffer
   allokiert wird. Eine Datei, die „zu groß ist", wird abgelehnt, ohne dass ihr
   Umfang alloziert wurde.
3. **R3 — Keine ungeprüften Offsets.** Kein Feld wird als Adresse benutzt, das
   nicht vorher gegen Dateilänge, Tensorzahl und laufende Summe geprüft wurde.
   Es gibt keinen Pfad, in dem ein Offset aus der Datei direkt in eine
   Indexoperation geht.

## 3. Byte-Layout

Little Endian, feste Breiten, keine Ausrichtungslücken. Der Header ist genau
**104 Byte**. Die Feldbreiten der Tabelle summieren sich auf
4+2+2+44+4+4+4+8+32 = 104; die frühere Angabe „96 Byte" in Fassung 1.0.0 war
ein Rechenfehler (Befund F-01 der Gate-Prüfung, siehe
`docs/audits/2026-11-11-ATC-AI-LLM-002-GATE-REVIEW.md`).

| Offset | Länge | Feld | Typ | Regel |
|---|---|---|---|---|
| 0 | 4 | `magic` | Bytes | exakt `41 54 43 4D` (`ATCM`) |
| 4 | 2 | `format_version` | u16 | muss `1` sein, sonst `UnsupportedVersion` |
| 6 | 2 | `flags` | u16 | Bit 0 = gebundene Einbettung; alle anderen Bits müssen 0 sein |
| 8 | 4 | `d_model` | u32 | > 0, ≤ 2^20 |
| 12 | 4 | `n_layers` | u32 | > 0, ≤ 1024 |
| 16 | 4 | `n_heads` | u32 | > 0, teilt `d_model` |
| 20 | 4 | `n_kv_heads` | u32 | > 0, teilt `n_heads` |
| 24 | 4 | `d_head` | u32 | > 0, `n_heads * d_head == d_model` |
| 28 | 4 | `d_ff` | u32 | > 0, ≤ 2^20 |
| 32 | 4 | `vocab` | u32 | > 0, ≤ 2^20 |
| 36 | 4 | `max_seq` | u32 | > 0, ≤ 2^20 |
| 40 | 4 | `tensor_count` | u32 | == `2 + 8 * n_layers + (gebunden ? 0 : 1)`, ≤ 8194 |
| 44 | 4 | `dtype` | u32 | `0` = f32; alles andere `UnsupportedDtype` |
| 48 | 4 | `layout` | u32 | `0` = zeilenweise, dicht, little endian; sonst `UnsupportedLayout` |
| 52 | 4 | `rope_base` | f32-Bits | endlich, > 0; heute 10 000 (Abschnitt 11, K6) |
| 56 | 4 | `norm_eps` | f32-Bits | endlich, > 0; heute 1e-8 |
| 60 | 4 | `reserved` | u32 | muss 0 sein, sonst `ReservedNotZero` |
| 64 | 8 | `tensor_data_bytes` | u64 | Summe aller Tensor-Bytes, ≤ 2^30 |
| 72 | 32 | `digest` | 32 Byte | SHA-256 nach Abschnitt 6 |

`rope_base` und `norm_eps` stehen **im Artefakt**, nicht nur im Code: beides
ändert die Semantik der Gewichte. Ein Artefakt, das seine eigene Numerik nicht
deklariert, wäre nicht reproduzierbar.

## 4. Tensor-Tabelle

Unmittelbar nach dem Header, `tensor_count` Einträge zu je **64 Byte**:

| Offset im Eintrag | Länge | Feld | Typ | Regel |
|---|---|---|---|---|
| 0 | 4 | `name_len` | u32 | 1..=32 |
| 4 | 32 | `name` | ASCII | `[a-z0-9_.]`, Rest `0x00`, exakt `name_len` Zeichen |
| 36 | 4 | `rank` | u32 | 1 oder 2 |
| 40 | 4 | `dim0` | u32 | > 0 |
| 44 | 4 | `dim1` | u32 | > 0 bei `rank` 2, sonst `1` |
| 48 | 8 | `offset` | u64 | relativ zum Beginn des Tensor-Datenbereichs |
| 56 | 8 | `byte_len` | u64 | == `dim0 * dim1 * 4` (bei `rank` 1: `dim0 * 4`) |

Der Datenbereich beginnt bei `104 + 64 * tensor_count`. Diese Zahl ist durch 8
teilbar; es gibt **keine** Auffüllung zwischen Tabelle und Daten. Die Dateilänge
ist exakt `104 + 64 * tensor_count + tensor_data_bytes`; zusätzliche Bytes am
Ende sind `TrailingBytes`, eine zu kurze Datei ist `LengthMismatch`.

**Ausrichtung (Befund F-04):** Felder sind **nicht** ausgerichtet zu lesen. Weder
die 64-Byte-Einträge noch die `u64`-Felder darin dürfen als ausgerichtet
angenommen werden; gelesen wird byteweise (Little-Endian-Konvertierung je Feld),
und es gibt keine Regel, die eine Ausrichtung erzwingt.

## 5. Kanonische Reihenfolge (deterministisch)

Die Reihenfolge ist Teil des Vertrags, nicht Konvention: sie ist der Grund, warum
zwei Läufe mit identischen Gewichten identische Bytes ergeben. Erlaubt ist genau
diese Folge, `b` von `0` bis `n_layers - 1`:

```
tok_emb
lm_head            (nur wenn flags.Bit0 = 0, also nicht gebundene Einbettung)
für jeden Block b:
  block.{b}.attn_norm
  block.{b}.wq
  block.{b}.wk
  block.{b}.wv
  block.{b}.wo
  block.{b}.mlp_norm
  block.{b}.wgu
  block.{b}.wd
final_norm
```

`lm_head` hat die Form `(d_model, vocab)` und ist der Ausgabekopf; bei
gebundener Einbettung (Bit 0 gesetzt) existiert er **nicht**, der Kopf ist dann
`tok_emb`. Die frühere Fassung 1.0.0 nannte nur `tok_emb` und hätte ein Modell
mit `tied_embedding = false` — das der Parameterhaushalt als zulässig führt —
nicht darstellen können (Befund F-02).

`{b}` ist dezimal ohne führende Nullen. Der Schreibpfad **muss** in dieser Folge
schreiben; der Lesepfad **muss** jede Abweichung als `TensorOrderInvalid`
ablehnen, auch wenn Name und Form sonst stimmen. Damit ist ein erneutes
Schreiben eines gelesenen Artefakts byte-identisch.

`offset` folgt zwingend aus der laufenden Summe der `byte_len` in genau dieser
Reihenfolge: `offset[0] = 0`, `offset[i] = offset[i-1] + byte_len[i-1]`,
`offset[letzter] + byte_len[letzter] == tensor_data_bytes`. Lücken, Überlappungen
und Umordnungen sind Fehler (`OffsetNotContiguous`), keine zulässigen Varianten.

## 6. Digest

`digest` ist SHA-256 über **die gesamte Datei**, wobei die 32 Byte des
Digest-Feldes (Offset 72..104) als **Nullbytes** behandelt werden. Header,
Tabelle und Daten sind damit in einer Prüfsumme gebunden; ein geänderter
Architekturparameter oder ein umbenannter Tensor fällt auf, nicht nur ein
geänderter Zahlenwert.

Regeln:

- Beim Schreiben wird der Digest zuletzt berechnet: Feld nullen, hashen,
  Ergebnis eintragen.
- Beim Lesen wird zuerst die Struktur geprüft, dann der Digest. Ein
  `DigestMismatch` ist ein eigener Fehler, kein `InvalidArtifact`.
- Der Digest ist reproduzierbar: identische Gewichte, identische Konfiguration,
  identische `format_version` und identischer Encoder ergeben identische Bytes.
  Keine Zeitstempel, keine Hostnamen, keine Pfade, keine Zufallswerte im
  Artefakt.

## 7. Lese-Reihenfolge (fail-closed)

Der Leser prüft in genau dieser Reihenfolge und bricht beim ersten Fehler ab:

1. Mindestlänge 104 Byte, sonst `Truncated`.
2. `magic`, `format_version`, `flags`, `reserved`, `dtype`, `layout`.
3. Architekturparameter gegen die Grenzen aus Abschnitt 3 und die
   Konsistenzregeln (`n_heads * d_head == d_model`, `n_heads % n_kv_heads == 0`,
   `tensor_count == 2 + 8 * n_layers + (gebunden ? 0 : 1)`).
4. Dateilänge gegen `104 + 64 * tensor_count + tensor_data_bytes`, sonst
   `LengthMismatch` (zu kurz) beziehungsweise `TrailingBytes` (zu lang).
5. `tensor_data_bytes` gegen die harte Grenze 2^30 **vor** jeder Zuteilung,
   danach gegen das übergebene `Budget`.
6. Tabelleneinträge: Namen, Formen, `byte_len`, `offset`-Kontiguität.
7. Digest.
8. Optionale Ankerprüfung: `expected_digest`, falls übergeben.
9. Optionale Architekturprüfung gegen eine erwartete `Config` (`ArchMismatch` je
   abweichendem Feld).
10. Erst jetzt werden Puffer zerteilt; jeder Tensor wird als Sicht auf den
    geprüften Datenbereich gebildet.

Punkt 10 ist die Regel aus R3: kein Tensor entsteht, bevor nicht alle seine
Grenzen geprüft sind.

**Präzedenz (Befund F-06):** Strukturfehler werden **immer** vor Ankerfehlern
gemeldet. Ist eine Datei strukturell falsch und der erwartete Digest ebenfalls,
antwortet der Leser mit dem Strukturfehler. Damit verrät ein abgelehnter
Lesevorgang nichts über den erwarteten Inhalt, und eine Fehlermeldung ist immer
die Ursache, nicht ihre Folge.

## 8. Größengrenzen (vor Allokation)

| Grenze | Wert | Zweck |
|---|---|---|
| `tensor_data_bytes` | ≤ 2^30 (1 GiB) | eine Datei kann keinen Knoten aushungern |
| Dateilänge insgesamt | ≤ `104 + 64·8194 + 2^30` | Obergrenze auch für den Puffer, den der Aufrufer liefert |
| `tensor_count` | ≤ 8194 (= `2 + 8·1024`) | Tabelle bleibt klein und linear prüfbar; passt zu `n_layers` ≤ 1024 |
| einzelne Dimension | ≤ 2^20 | verhindert Produktüberläufe |
| Elemente je Tensor | ≤ 2^28 | begrenzt `byte_len` auf ≤ 1 GiB je Tensor |
| `name_len` | 1..=32 | feste Feldbreite |
| `byte_len` je Tensor | == Formprodukt · 4 | keine deklarierte Lücke |

Alle Produkte werden mit geprüfter Arithmetik gebildet (`checked_mul`); ein
Überlauf ist `ShapeOverflow` und keine umgebrochene Zahl.

### 8.1 Ressourcenerschöpfungsschutz (Befund F-05, verbindlich)

Ein Format, das nur „nicht zu groß" sagt, ist kein Schutz. Diese Regeln sind
Teil des Vertrags, nicht der Implementierung überlassen:

1. **Zwei Grenzen, nicht eine.** Der Aufrufer übergibt `max_bytes` (Grenze für
   die Dateilänge) **und** ein `Budget` (Grenze für zusätzlich belegten
   Speicher). Die harte Grenze aus der Tabelle gilt immer; `max_bytes` darf nur
   strenger sein. Eine Datei über `max_bytes` wird abgelehnt, **bevor** sie
   gehasht wird — Hashing kostet CPU proportional zur Länge und ist sonst ein
   Erschöpfungsvektor bei wiederholten Versuchen.
2. **Kein Vollkopie.** `read` darf die übergebenen Bytes nicht duplizieren. Der
   zusätzliche Speicher ist O(Tensoren) Sichten, nicht O(Modellgröße); wer die
   Gewichte verändern will, kopiert bewusst außerhalb des Lesers.
3. **Linearer Aufwand, keine Rekursion.** Der Leser ist ein einzelner Durchlauf:
   Header, Tabelle, Digest. Es gibt keine quadratischen Vergleiche (Namen werden
   in einem festen 32-Byte-Fenster verglichen), keine Rekursion und keine
   Allokation je Tabelleneintrag.
4. **Prüfen vor Zuteilen, Zuteilen vor Rechnen.** Die Reihenfolge aus Abschnitt 7
   ist bindend: keine Allokation, bevor Länge, `tensor_data_bytes` und
   `tensor_count` geprüft sind; kein Tensor, bevor seine Grenzen geprüft sind.
5. **Wiederholte Aufrufe sind Aufrufersache.** Der Vertrag begrenzt einen
   Lesevorgang. Ratenbegrenzung über viele Aufrufe gehört zum Aufrufer und wird
   hier ausdrücklich **nicht** zugesagt.
6. **Grenzen müssen zueinander passen.** `n_layers` ≤ 1024 und
   `tensor_count` ≤ 8194 sind so gewählt, dass die Formel
   `2 + 8·n_layers` für jeden zulässigen Wert innerhalb der Obergrenze liegt.
   Die Fassung 1.0.0 hatte `n_layers` ≤ 1024 gegen `tensor_count` ≤ 8192
   gestellt; bei 1024 Schichten wären es 8194 Tensoren gewesen (Befund F-05e).

## 9. Fehlertaxonomie

| Fehler | Bedeutung |
|---|---|
| `Truncated { at }` | Datei endet vor dem angeforderten Feld |
| `BadMagic` | Kennung ist nicht `ATCM` |
| `UnsupportedVersion { got, expected }` | Formatversion nicht 1 |
| `UnsupportedDtype { got }` | `dtype` unbekannt |
| `UnsupportedLayout { got }` | `layout` unbekannt |
| `UnknownFlag { bits }` | reserviertes Flag gesetzt |
| `ReservedNotZero` | `reserved` nicht 0 |
| `TensorCountMismatch { got, expected }` | Tabelle passt nicht zur Architektur |
| `InvalidName { index }` | Zeichen außerhalb `[a-z0-9_.]`, Länge falsch, keine NUL-Auffüllung |
| `TensorOrderInvalid { index }` | Reihenfolge nicht kanonisch |
| `ShapeInvalid { index }` | Dimension 0 oder Produktüberlauf |
| `ByteLenMismatch { index, declared, expected }` | `byte_len` ≠ Formprodukt · 4 |
| `OffsetNotContiguous { index }` | Lücke, Überlappung oder falsche laufende Summe |
| `LengthMismatch { file, expected }` | Datei zu kurz |
| `TrailingBytes { extra }` | Bytes hinter dem Datenbereich |
| `TooLarge { requested, limit }` | über der harten Grenze |
| `BudgetExceeded { requested, limit }` | über dem Aufruferbudget |
| `DigestMismatch { expected, actual }` | Prüfsumme stimmt nicht |
| `TrustAnchorMissing` | kein `expected_digest`, kein Signaturnachweis (Regel R1) |
| `ArchMismatch { field }` | Parameter weicht von der erwarteten Konfiguration ab |
| `ValueNotFinite { index }` | ein Gewicht ist `NaN` oder `inf` (optional, `strict`) |

`ValueNotFinite` ist bewusst als **eigener, optional strenger** Fehler geführt:
Endlichkeit ist eine Eigenschaft der Gewichte, keine Eigenschaft des Containers.

## 10. API-Vertrag (verbindlich für die Implementierung)

```
write(cfg: &Config, weights: &Weights, out: &mut Vec<u8>) -> Result<[u8; 32], WriteError>
read(bytes: &[u8], anchor: TrustAnchor, max_bytes: usize, budget: &mut Budget,
     expected_cfg: Option<&Config>) -> Result<Artifact, ReadError>
Artifact { config: Config, weights: Weights, digest: [u8; 32], trustworthy: bool }
TrustAnchor = { ExpectedDigest([u8;32]) | SignatureVerified | None }
```

- `write` liefert den berechneten Digest zurück, damit der Aufrufer ihn
  außerhalb der Datei ablegen kann (das ist der Anker für R1).
- `max_bytes` ist die Aufrufergrenze für die Dateilänge (Abschnitt 8.1, Regel 1);
  sie wird vor dem Hashen geprüft.
- `read` verweigert **nicht** das Lesen ohne Anker, aber setzt dann
  `trustworthy = false` und meldet `TrustAnchorMissing` nur im strengen Modus.
  Ein Aufrufer, der `trustworthy` ignoriert, hat die Regel verletzt, nicht der
  Leser.
- `read` gibt Gewichte als Sichten auf `bytes` zurück (kein Kopieren des
  gesamten Modells); die Lebensdauer ist an `bytes` gebunden.

## 11. Adversariale Testfälle (verbindlich)

Zusätzlich zu B-21 (schreiben, lesen, Digest stimmt; manipulierter Tensor →
`DigestMismatch`; unbekannte `format_version` → `InvalidArtifact`-Klasse) sind
diese Fälle verpflichtend. Jeder Fall ist eine gezielte Mutation eines gültigen
Artefakts; erwartet wird der genannte Fehler, **kein Panik, keine Allokation
über der Grenze**.

| ID | Mutation | Erwartung |
|---|---|---|
| AT-01 | Datei um 1 Byte gekürzt | `Truncated` |
| AT-02 | Datei am Ende um 1 Byte verlängert | `TrailingBytes` |
| AT-03 | `magic` verändert | `BadMagic` |
| AT-04 | `format_version` = 2 | `UnsupportedVersion` |
| AT-05 | `format_version` = 0 | `UnsupportedVersion` |
| AT-06 | `tensor_count` + 1 | `TensorCountMismatch` |
| AT-07 | `tensor_count` = 2^31 | `TensorCountMismatch` (vor jeder Zuteilung) |
| AT-08 | `tensor_data_bytes` = 2^40 | `TooLarge` |
| AT-09 | `d_model` geändert, Rest konsistent | `DigestMismatch` |
| AT-10 | `rope_base` = 0 | `InvalidArgument` (bei Prüfung der Architektur) |
| AT-11 | `norm_eps` = 0 | `InvalidArgument` |
| AT-12 | `reserved` = 1 | `ReservedNotZero` |
| AT-13 | `flags` Bit 5 gesetzt | `UnknownFlag` |
| AT-14 | `dtype` = 1 | `UnsupportedDtype` |
| AT-15 | `layout` = 1 | `UnsupportedLayout` |
| AT-16 | zwei Tensoren vertauscht (Tabelle umgeordnet, Offsetkette intakt) | `TensorOrderInvalid` |
| AT-17 | `offset` eines Tensors + 4 | `OffsetNotContiguous` |
| AT-18 | `byte_len` eines Tensors halbiert | `ByteLenMismatch` |
| AT-19 | `dim0` = 0 | `ShapeInvalid` |
| AT-20 | `dim0` = 2^20, `dim1` = 2^20 (Produktüberlauf in u32) | `ShapeInvalid` |
| AT-21 | `name` mit Großbuchstaben | `InvalidName` |
| AT-22 | `name` mit NUL mitten im Namen | `InvalidName` |
| AT-23 | ein Gewicht auf `NaN` gesetzt, Digest neu berechnet, strenger Modus | `ValueNotFinite` |
| AT-24 | ein Gewicht verändert, Digest **nicht** neu berechnet | `DigestMismatch` |
| AT-25 | gültiges Artefakt ohne `expected_digest`, strenger Modus | `TrustAnchorMissing`, `trustworthy = false` |
| AT-26 | Artefakt gegen abweichende `Config` gelesen | `ArchMismatch { field }` |
| AT-27 | Budget kleiner als `tensor_data_bytes` | `BudgetExceeded` |
| AT-29 | `flags` Bit 0 = 0 (nicht gebunden), Tabelle ohne `lm_head` | `TensorCountMismatch` |
| AT-30 | `flags` Bit 0 = 0 mit `lm_head` in falscher Position | `TensorOrderInvalid` |
| AT-31 | `flags` Bit 0 = 0, `lm_head` vorhanden, aber Form `(vocab, d_model)` | `ShapeInvalid` |
| AT-32 | zwei Einträge mit identischem Namen in kanonischen Positionen | `TensorOrderInvalid` |
| AT-33 | `offset` des letzten Tensors = 2^63 | `OffsetNotContiguous` (kein `usize`-Überlauf) |
| AT-34 | `max_bytes` kleiner als die Dateilänge | `TooLarge`, **ohne** dass gehasht wurde |
| AT-28 | 1000 zufällige Einzelbit-Mutationen | jeder Fall endet in einem typisierten Fehler oder einem erfolgreichen Lesevorgang mit korrektem Digest; **nie** Panik, **nie** Überallokation |

AT-28 ist der einzige Fall mit Zufall: er prüft, dass der Parser an **keiner**
Byteposition ungeprüft liest. Der Zufallsstrom ist festgelegt (splitmix64 mit
Seed), damit der Lauf reproduzierbar bleibt.

## 12. Versionierung und Kompatibilität

- `format_version` erhöht sich nur bei einer Änderung, die ein Leser der
  bisherigen Version nicht korrekt verarbeiten kann. Ein unbekannter Wert ist
  ein Fehler, **kein** Best-Effort-Lesen.
- Neue Flags dürfen nur eingeführt werden, wenn ein Leser sie ablehnen kann
  (`UnknownFlag`), nicht stillschweigend ignorieren.
- Ein Artefakt deklariert seine Architektur selbst. Ein Leser darf fehlende
  Parameter nicht mit Defaults auffüllen; fehlt etwas, ist es ein anderes
  Format.
- Upgrade-Regeln für Gewichte (Umschreiben von `d_ff`, Vokabularwechsel) sind
  **nicht** Teil von `format_version` 1 und brauchen ein eigenes Dokument.

## 13. Offene Punkte

| ID | Punkt | Wirkung |
|---|---|---|
| K6 | `rope_base` normativ festlegen (heute Implementierungsparameter 10 000) | ändert die Bedeutung trainierter Gewichte, deshalb im Artefakt deklariert |
| K7 | Signaturverfahren für Authentizität (R1) | ohne ihn bleibt der Digest reine Integritätsprüfung |
| K8 | Kompression oder Quantisierung als zweite Formatversion | würde `dtype` und `layout` erweitern |

## 14. Abnahme von B-21

B-21 gilt erst als erfüllt, wenn:

1. `write` → `read` → identische Gewichte (bitweise) für Ref-1 nachgewiesen ist,
2. der Digest über einen zweiten Lauf identisch ist (Determinismus),
3. alle Fälle AT-01 bis AT-27 typisiert abgelehnt werden,
4. AT-28 (1000 Mutationen) ohne Panik und ohne Überallokation durchläuft,
5. `ValueNotFinite` im strengen Modus greift und im lockeren Modus nicht,
6. ein gelesenes Artefakt erneut geschrieben byte-identische Bytes ergibt,
7. ein Modell mit `tied_embedding = false` geschrieben und gelesen wird
   (`lm_head`, AT-29 bis AT-31),
8. `max_bytes` greift vor dem Hashen (AT-34) und die zusätzliche Belegung
   unabhängig von der Dateigröße bleibt (Abschnitt 8.1, Regel 2).

Punkt 6 ist der eigentliche Beweis der Kanonizität: er verbindet Abschnitt 5 mit
Abschnitt 6.

---

*Dieses Dokument enthält 0 Zeilen Produktivcode. Es legt Layout, Prüfregeln und
Vertrauensmodell fest. Keine Implementierung, kein Test ist ausgeführt.*
