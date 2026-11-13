---
document_id: ATC-AI-DIGEST-001
title: Digest-API-Vertrag (SHA-256) — Streaming als normative Autoritaet
version: 1.0.0
status: proposed
owner: A-TownChain-Okosystems
created: 2026-11-13
updated: 2026-11-13
layer: L2 (aurora-ai)
security_class: S3
evidence_gate: AURORA-001
---

# Digest-API-Vertrag (SHA-256)

Vertrag für das Crate `atc-aurora-digest` (AD-FT1). Er legt fest, was die eine
Prüfsummen-Autorität der Aurora-Implementierung leistet, was sie garantiert und was
sie ausdrücklich nicht tut.

## 1. Geltung

Für jedes Aurora-Artefakt, dessen Integrität über SHA-256 gebunden wird:
Tokenizer-Artefakte (`ATC-AI-LLM-004`), Modellartefakte (`ATC-AI-LLM-002`),
Provenienzbelege (`ATC-AI-LLM-003`). Andere Crates berechnen keine eigene Fassung,
sondern benutzen diese.

## 2. API

| Element | Signatur | Bedeutung |
|---|---|---|
| `Sha256::new()` | `const fn() -> Sha256` | frischer Zustand nach FIPS 180-4 |
| `Sha256::update` | `fn(&mut self, data: &[u8])` | schiebt Daten hinein, Zustand bleibt konstant |
| `Sha256::finish` | `fn(self) -> [u8; 32]` | schließt ab, verbraucht die Instanz |
| `digest` | `fn(&[u8]) -> [u8; 32]` | ein vollständiger Durchlauf, keine Eingabekopie |
| `sha256` | `fn(&[u8]) -> [u8; 32]` | Alias für `digest`, erhält die Tokenizer-Fassade |
| `hex` | `fn(&[u8; 32]) -> String` | 64 Zeichen kleingeschriebenes Hex |
| `sha256_hex` | `fn(&[u8]) -> String` | `hex(digest(data))` |
| `DIGEST_LEN` | `const usize = 32` | Länge in Bytes |
| `API_VERSION` | `const &str` | `ATC-AI-DIGEST-001 v1.0.0` |

## 3. Klauseln

**D-01** Streaming ist die normative Form. Jede Einmal-API ist eine Bequemlichkeit über
derselben Zustandsmaschine, kein zweiter Rechenweg. Es gibt genau eine
Kompressionsfunktion im Crate.

**D-02** Kein Puffer der Eingabe ist Voraussetzung: `update` verarbeitet Blöcke im
Durchlauf, der Zustand ist konstant (256 Bit Zustand plus 512 Bit Blockpuffer). Eine
Implementierung, die die gesamte Eingabe kopiert, um sie zu padden, verstößt gegen
diesen Vertrag.

**D-03** SHA-256 nach FIPS 180-4, Byte-Ordnung big-endian, Padding mit `0x80`, Nullen
und 64-Bit-Längenfeld. Die Ausgabe ist big-endian.

**D-04** Gleichheit ist bytegenau gegen einen **unabhängigen** Orakel-Vektor zu
belegen, nicht gegen eine zweite eigene Umsetzung. Der Vektor in
`modules/atc-aurora-digest/tests/vectors.txt` wurde mit python hashlib über ein festes
Byte-Muster in 301 Längen erzeugt und liegt im Repo.

**D-05** Zusätzlich sind die FIPS-Beispiele (leere Eingabe, `abc`, der 56-Byte-Satz)
zu prüfen.

**D-06** Chunking darf das Ergebnis nicht ändern: beliebige Portionen, auch leere und
größer als ein Block, ergeben denselben Digest.

**D-07** Das Crate hat keine Abhängigkeit und kein `unsafe`
(`#![forbid(unsafe_code)]`). Die Prüfsumme eines Artefakts hängt an einem Algorithmus,
den niemand von außen austauschen darf; eine Fremdbibliothek wäre eine austauschbare
Abhängigkeit genau an dieser Stelle.

**D-08** Das Crate baut ohne `std`. `alloc` wird ausschließlich für die Hex-Ausgabe
gebraucht.

**D-09** Hex-Ausgabe ist 64 Zeichen `[0-9a-f]` (Format nach `ATC-AI-LLM-004` C-22).
Eine Normalisierung von Groß-/Kleinschreibung findet bei der Prüfung nicht statt; das
ist Sache des aufrufenden Vertrags.

**D-10** Versionierung ist eigenständig: Änderung an API-Oberfläche oder Klauseln
erfordert eine neue Minor-Version, Änderung der Byte-Ausgabe eine neue Major-Version
und einen neuen Vertrag. Ein Digest-Wechsel ist niemals rückwärtskompatibel.

**D-11** Cross-Crate-Prüfung: `atc-aurora-model` und `atc-aurora-tokenizer` prüfen
jeweils ihre eigene Fassade gegen denselben Orakel-Vektor
(`tests/digest_vectors.rs`). Damit ist belegt, dass beide Pfade bytegenau dasselbe
liefern, ohne dass eines vom anderen abhängt.

**D-12** Abhängigkeitsrichtung ist strikt: Model → Digest, Tokenizer → Digest. Niemals
Model ↔ Tokenizer. Der Digest kennt weder Modell noch Tokenizer.

## 4. Abgrenzung

Nicht Gegenstand: Signatur und Authentizität (`ATC-AI-LLM-003`), Artefakt-Layout
(`ATC-AI-LLM-002`, `ATC-AI-LLM-004`), Auswahl des Algorithmus. HMAC, HKDF und andere
Konstruktionen sind nicht Teil dieses Vertrags; sie brauchen einen eigenen.

## 5. Nachweisstand

| Punkt | Stand |
|---|---|
| Vektoren 0..300 gegen python hashlib | PASS |
| FIPS-Beispiele | PASS |
| Chunking-Invarianz (11 Schrittweiten, 14 Längen) | PASS |
| Cross-Crate-Test Model | PASS |
| Cross-Crate-Test Tokenizer | PASS |
| `cargo fmt --check`, `cargo clippy -D warnings` | PASS (lokal) |
| `CI_VERIFIED` | offen, Actions startet nicht (F-T2) |
