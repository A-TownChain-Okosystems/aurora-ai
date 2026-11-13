# Status atc-aurora-digest (M-06)

| Bereich | Status | Beleg |
|---|---|---|
| Streaming-API (`Sha256`) | `IMPLEMENTED` | `Sha256::new/update/finish`, konstanter Zustand, kein Eingabepuffer |
| Einmal-Fassaden (`digest`, `sha256`, `hex`, `sha256_hex`) | `IMPLEMENTED` | über derselben Zustandsmaschine, kein zweiter Rechenweg |
| Lokale Prüfung | `TESTED` | 1 Unit-Test, 4 Integrationstests, clippy `-D warnings`, fmt sauber |
| Cross-Crate | `TESTED` | `tests/digest_vectors.rs` in `atc-aurora-model` und `atc-aurora-tokenizer` |
| `CI_VERIFIED` | **offen** | Actions startet nicht (F-T2); der Rust-Job ist eingebaut, aber nie gelaufen |

## Prüfumfang

| Prüfung | Umfang |
|---|---|
| Orakel-Vektor | 301 Längen (0..300) eines festen Musters, Sollwerte aus python hashlib |
| FIPS 180-4 | leere Eingabe, `abc`, 56-Byte-Beispielsatz |
| Chunking-Invarianz | 14 Längen × 11 Schrittweiten, inklusive leerer und blockgroßer Portionen |
| Hex-Form | 64 Zeichen, ausschließlich `[0-9a-f]` |
| API-Version | `ATC-AI-DIGEST-001 v1.0.0` gepinnt |

## Bewusste Grenzen

- Nur SHA-256. Kein generischer `Digest`-Trait, keine Algorithmus-Auswahl.
- Keine Signatur, keine Schlüsselableitung.
- Kein Zeitstempel, keine Datei-Ein-/Ausgabe: das Crate rechnet über Bytes, mehr nicht.
