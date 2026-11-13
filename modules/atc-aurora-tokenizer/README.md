# atc-aurora-tokenizer (M-04)

Byte-Level-BPE für Aurora. Eigene Implementierung, keine Fremdbibliothek im Pfad
(`ATC-AI-LLM-001` §3.2: `tokenizers` ist gebannt).

## Warum Byte-Level

Das Vokabular beginnt bei den 256 Byte-Werten. Jede weitere ID entsteht durch eine
Paarung. Damit ist die Zerlegung **verlustfrei per Konstruktion**: `decode` setzt
die Bytes der Token zusammen, `encode` partitioniert die Eingabe. Ungültiges UTF-8
ist kein Sonderfall, sondern eine Bytefolge wie jede andere. Es gibt in v1 **keine
Normalisierung** (`ATC-AI-LLM-001` §8 P1).

## Determinismus

Zwei Läufe auf identischer Quelle müssen dasselbe Vokabular und denselben Digest
ergeben (B-15). Das ist keine Eigenschaft, die sich ergibt, sondern eine, die
festgeschrieben wird:

- Paarhäufigkeiten werden in einer `BTreeMap` gezählt, also in fester Reihenfolge.
- Bei gleicher Häufigkeit gewinnt die lexikografisch kleinste Paarung: aufsteigend
  iterieren, nur bei echt größerem Zähler ersetzen.
- Eine Paarung wird nur übernommen, wenn sie **zweimal** vorkommt. Sonst bricht das
  Training typisiert ab (`TargetUnreachable`), statt ein Vokabular mit
  Einmal-Paarungen aufzufüllen.

## Artefakt

`canonical()` erzeugt eine zeilenweise ASCII-Form ohne Zeitstempel und ohne Pfad;
`digest_hex()` ist der SHA-256 darüber. `from_canonical(bytes, digest)` prüft den
Digest **vor** dem Parsen (fail-closed). Eine manipulierte Merge-Tabelle passiert
diese Prüfung nicht (B-17).

## Stand

| Punkt | Wert |
|---|---|
| Tests | 15 grün, `cargo test` auf stable und nightly |
| clippy / fmt | `-D warnings` sauber, `fmt --check` sauber |
| Abhängigkeiten | keine |
| `CI_VERIFIED` | offen, Actions startet mit `startup_failure` |
| Vokabular der Referenz | **noch nicht trainiert**, es gibt keinen Korpus |

## Offene Punkte

- **K-T1 Artefaktformat:** die kanonische Form ist implementiert, aber nicht als
  eigener Vertrag spezifiziert (anders als `ATC-AI-LLM-002` für Gewichte). Vor dem
  ersten eingefrorenen Vokabular braucht sie einen Vertrag mit Testfällen.
- **F-T1 Digest-Doppelung:** `atc-aurora-model/src/artifact/sha256.rs` enthält eine
  zweite SHA-256-Umsetzung. Vereinheitlichung ist eine Architekturentscheidung
  (gemeinsames Digest-Crate oder Umzug), kein Nebenprodukt von P1.
- **Zielvokabular:** 4096 ist der Zielwert aus K4. Der Benchmark auf dem
  P1-Zielkorpus entscheidet ihn, nicht diese Bibliothek.
