# atc-aurora-digest (M-06)

SHA-256 als **einzige** Prüfsummen-Autorität der Aurora-Implementierung (AD-FT1).
Kein `unsafe`, keine Abhängigkeit, `no_std` bis auf `alloc` für die Hex-Ausgabe.

## Warum ein eigenes Crate

Vor AD-FT1 gab es zwei Umsetzungen (`atc-aurora-model/src/artifact/sha256.rs` und im
Tokenizer). Sie waren bytegenau gleich, aber sie waren zwei Stellen, an denen ein
Algorithmus hätte abdriften können, an dem Artefakt-Identität hängt. Die
Zusammenführung geht **über** dieses Crate, nicht durch Unterordnung des Tokenizers
unter das Model: die Abhängigkeitsrichtung ist strikt Model → Digest und Tokenizer →
Digest, niemals Model ↔ Tokenizer.

## API

```rust
let mut h = Sha256::new();     // normativ: Streaming, konstanter Zustand
h.update(b"ab");
h.update(b"c");
let digest = h.finish();        // [u8; 32]

digest(b"abc");                 // ein Durchlauf, keine Eingabekopie
sha256_hex(b"abc");             // 64 Zeichen lowercase Hex
```

## Nachweis

| Prüfung | Beleg |
|---|---|
| 301 Längen gegen einen fremden Orakel-Vektor | `tests/vectors.rs`, `tests/vectors.txt` (erzeugt mit python hashlib) |
| FIPS-180-4-Beispiele | `tests::fips_180_4_beispiele` |
| Chunking ändert das Ergebnis nicht | `tests::streaming_ergibt_denselben_digest_wie_ein_stueck` (11 Schrittweiten) |
| beide Fassaden liefern dasselbe | `tests/digest_vectors.rs` in Model und Tokenizer |

Der Vektor ist bewusst **nicht** aus diesem Code erzeugt. Ein Vergleich zweier eigener
Umsetzungen wäre nur eine Kopplung, kein Nachweis der Richtigkeit.

## Grenzen

- Keine Signatur, kein HMAC, kein HKDF. Das ist `ATC-AI-LLM-003` beziehungsweise ein
  eigener Vertrag.
- Kein SHA-512, kein Keccak. Wer das braucht, schreibt einen Vertrag, kein `match`.
