# AD-FT1 — Eine Digest-Autorität statt zwei SHA-256-Implementierungen

**Datum:** 13.11.2026
**Status:** entschieden (Owner-Freigabe)
**Betrifft:** `atc-aurora-model`, `atc-aurora-tokenizer`, neu `atc-aurora-digest`
**Vertrag:** `ATC-AI-DIGEST-001` v1.0.0

## Befund

Bis zum 13.11.2026 gab es zwei SHA-256-Umsetzungen im Repo:

| Stelle | Form | Zustand |
|---|---|---|
| `modules/atc-aurora-model/src/artifact/sha256.rs` | Streaming, 140 Zeilen | konstanter Zustand, zusätzlich `digest()` |
| `modules/atc-aurora-tokenizer/src/sha256.rs` | Einmal-API, 121 Zeilen | baut eine gepufferte Kopie der Eingabe |

Der Vergleich über echten Code (nicht über den Quelltext) ergab **bytegenaue
Gleichheit**: 301 Längen von 0 bis 300 mit Zufallsdaten, die Padding-Grenzlängen
0/1/54/55/56/57/63/64/65/119/120/127/128/129/191/192, und die Modell-Fassung
zusätzlich in unregelmäßigen Portionen gefüttert: 0 Abweichungen. Der Unterschied lag
im Vertrag, nicht im Ergebnis.

Das Risiko war also nicht ein falscher Digest, sondern ein Auseinanderlaufen an zwei
Stellen: eine zweite Krypto-Umsetzung, die irgendwann nur an einer Stelle gepflegt
wird.

## Entscheidung

Ein gemeinsames Crate **`atc-aurora-digest`** wird die Autorität.

1. Streaming-API (`Sha256::new/update/finish`) ist die normative Implementierungsform.
2. `atc-aurora-model` und `atc-aurora-tokenizer` benutzen denselben Code.
3. Keine parallelen Hash-Implementierungen im Repo.
4. Keine gepufferte Eingabekopie ist Voraussetzung (D-02).
5. Bytegenaue Kompatibilität wird durch Cross-Crate-Tests gegen einen **unabhängigen**
   Orakel-Vektor abgesichert, nicht durch Vergleich zweier eigener Fassungen.
6. Eigenständige Versionierung und eigener API-Vertrag (`ATC-AI-DIGEST-001`).

Die Zusammenführung erfolgt **über** das neue Crate, nicht durch Unterordnung des
Tokenizers unter das Model.

## Verworfene Alternativen

| Alternative | Warum nicht |
|---|---|
| Tokenizer hängt vom Model ab | falsche Richtung: der Tokenizer ist eine Stufe früher im Datenfluss, er darf kein Modell brauchen |
| Model hängt vom Tokenizer ab | dieselbe falsche Richtung, nur umgekehrt |
| Zwei Fassungen behalten, Gleichheit per Test pinnen | kostet eine Dev-Abhängigkeit zwischen den Crates und lässt zwei Stellen im Repo, die man ändern kann |
| Fremdbibliothek (`sha2`) | austauschbare Abhängigkeit genau an der Stelle, an der ein Artefaktformat Stabilität braucht |

## Umsetzung

| Datei | Änderung |
|---|---|
| `modules/atc-aurora-digest/src/lib.rs` | neue Autorität: Streaming plus Einmal-Fassaden, keine Abhängigkeit, kein `unsafe`, `no_std` |
| `modules/atc-aurora-model/src/artifact/sha256.rs` | eigene Umsetzung entfernt, Re-Export |
| `modules/atc-aurora-tokenizer/src/sha256.rs` | eigene Umsetzung entfernt, Re-Export |
| beide `Cargo.toml` | Abhängigkeit auf `atc-aurora-digest` |
| `tests/digest_vectors.rs` in Model und Tokenizer | Cross-Crate-Nachweis gegen denselben Orakel-Vektor |

## Konsequenzen

- Ein Digest-Fehler kann nur noch an einer Stelle entstehen und nur dort behoben werden.
- Die Zahlen vorher und nachher sind unverändert: 61 Tests im Model (1 ignoriert), 33 im
  Tokenizer, dazu 1 im Digest-Crate und je 1 Cross-Crate-Test.
- Wer den Digest ändert, ändert einen Vertrag, nicht eine Hilfsfunktion.
- Offener Rest: `CI_VERIFIED` fehlt weiterhin, weil Actions mit `startup_failure`
  startet (F-T2, unverifizierte Konto-E-Mail).
