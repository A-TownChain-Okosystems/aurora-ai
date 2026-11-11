---
document_id: ATC-AI-LLM-003
title: Herkunft und Authentizität von Aurora-Modellartefakten (K7) — Signaturvertrag
version: 0.1.0
status: proposed
owner: A-TownChain-Okosystems
created: 2026-11-11
updated: 2026-11-11
layer: L2 (aurora-ai)
security_class: S3
evidence_gate: AURORA-001
---

# K7 — Herkunft und Authentizität

**Status: Entwurf zur Owner-Freigabe.** Kein Code, keine Formatänderung. Dieses
Dokument beantwortet die eine Frage, die `ATC-AI-LLM-002` ausdrücklich offen
lässt: woher ein Artefakt kommt und wer es verantwortet.

## 0. Ausgangslage

`ATC-AI-LLM-002` sichert **Integrität**: der SHA-256 im Header belegt, dass die
Bytes seit dem Schreiben unverändert sind. Er belegt **nicht** Authentizität: wer
schreiben kann, rechnet den Digest neu. Der Vertrag sagt das selbst
(Abschnitt 9) und verweigert stilles Vertrauen (`trustworthy = false`,
`TrustAnchorMissing` im strengen Modus).

Damit gibt es heute drei ehrliche Zustände: „unverändert, Herkunft unbekannt",
„stimmt mit einem erwarteten Digest überein" und „nichts bekannt". Was fehlt, ist
der vierte: **„von diesem Schlüssel signiert, Signatur gültig"**.

## 1. Grundentscheidung: externe Signatur, eingefrorenes Format

Das ATCM-Format bleibt **unverändert**. Die Signatur liegt neben der Datei, nicht
in ihr. Gründe:

1. Das Format ist eingefroren (1.2.0). Ein Signaturfeld im Header würde eine
   neue Formatversion erzwingen und jedes bestehende Artefakt entwerten.
2. Ein Artefakt kann mehrere Signaturen tragen (Mehrparteienfreigabe), ohne dass
   das Format davon wissen muss.
3. Wer nur Integrität braucht, arbeitet ohne Signatur weiter.

Konsequenz: **ein Artefakt ohne Signatur ist nicht unsigniert im Sinne von
„kaputt"**, sondern hat eine unbekannte Herkunft. Der Leser unterscheidet das
weiterhin über `trustworthy`.

## 2. Signaturbehälter `ATCS` v1

Detached, byte-genau, little endian, 88 Byte + Signaturlänge:

| Offset | Länge | Feld | Regel |
|---|---|---|---|
| 0 | 4 | `magic` | `ATCS` |
| 4 | 2 | `version` | `1` |
| 6 | 2 | `alg` | 1 = secp256k1/RFC 6979/SHA-256, 2 = Ed25519/RFC 8032 (offene Entscheidung D1) |
| 8 | 4 | `flags` | Bit 0 = über komprimierte Datei signiert (K8); sonst 0 |
| 12 | 4 | `key_id` | 32-Bit-Kennung des Signaturschlüssels |
| 16 | 8 | `signed_len` | Länge der signierten Datei in Byte |
| 24 | 32 | `digest` | SHA-256 des Artefakts, wie im ATCM-Header |
| 56 | 2 | `sig_len` | 64 (secp256k1) oder 64 (Ed25519) |
| 58 | 2 | `reserved` | 0 |
| 60 | 4 | `reserved2` | 0 |
| 64 | 8 | `issued_at` | Unix-Sekunden, nur Anzeige, **nicht** Teil der Signatur |
| 72 | 2 | `signer_len` | 1..=16, `[a-z0-9_.-]` |
| 74 | 14 | `signer` | Name des Unterzeichners, NUL-aufgefüllt |
| 88 | `sig_len` | `signature` | Signatur über die Nachricht aus Abschnitt 3 |

`issued_at` und `signer` sind **nicht** signiert: sie sind Etiketten für Menschen.
Wer sie fälscht, ändert die Gültigkeit nicht. Das ist Absicht und muss im
Anwenderhandbuch stehen, sonst entsteht der Eindruck, der Name sei bewiesen.

## 3. Signierte Nachricht (Domain-Trennung)

Signiert wird **nicht** die Datei, sondern eine kanonische Nachricht fester Länge:

```
"ATC-AI-ARTIFACT-SIGNATURE-v1\n"   (29 Byte, ASCII)
alg:u16 || digest:32 || signed_len:u64
```

71 Byte (29 + 2 + 32 + 8), keine variablen Teile, keine Serialisierung. Die Domain-Zeichenkette
verhindert, dass eine Signatur aus einem anderen Zusammenhang (Chain-Transaktion,
DID-Rotation, Commit) als Artefaktsignatur durchgeht — und umgekehrt.

## 4. Ablauf beim Schreiben

1. `write` erzeugt das Artefakt und liefert den Digest (heute schon so).
2. Der Aufrufer signiert die Nachricht aus Abschnitt 3 mit seinem Schlüssel.
3. Der Behälter wird neben die Datei gelegt: `<name>.atcm` und `<name>.atcm.sig`.

Die Bibliothek **signiert nicht selbst**, solange die Algorithmusentscheidung
offen ist. Signieren ist ein getrennter Schritt mit einem getrennten Werkzeug.

## 5. Ablauf beim Lesen (Prüfreihenfolge)

1. Struktur, Grenzen, Tabelle, Digest — **unverändert** wie in `ATC-AI-LLM-002`.
2. Danach, falls ein Behälter vorliegt: `magic`, `version`, `alg` bekannt,
   reservierte Felder 0, `digest` gleich dem berechneten Digest, `signed_len`
   gleich der Dateilänge, `signature` gültig gegen den Schlüssel aus dem
   Vertrauensspeicher.
3. Strukturfehler werden **immer** vor Signaturfehlern gemeldet (F-06 gilt auch
   hier).

Ein Fehlschlag bei 2 führt zu `UntrustedSignature`/`UnknownKey`/`RevokedKey` und
**nicht** zu „Datei kaputt". Diese Unterscheidung ist operativ wichtig: eine
ungültige Signatur heißt „Herkunft nicht bestätigt", nicht „Bytes beschädigt".

## 6. Vertrauensspeicher

Ohne Verwaltung ist eine Signatur eine Behauptung. Deshalb:

- **Schlüsselregister**: `key_id` → öffentlicher Schlüssel, Rolle
  (`release`, `test`, `dev`), Gültigkeitsfenster.
- **Rollen**: nur `release`-Schlüssel dürfen Artefakte für die Verteilung
  signieren. Ein `dev`-Schlüssel darf lokal prüfen, nicht veröffentlichen.
- **Widerruf**: eine Widerrufsliste mit `key_id` und Zeitpunkt. Ein widerrufener
  Schlüssel macht Signaturen **nach** dem Widerrufszeitpunkt ungültig und
  Signaturen davor zu einer dokumentierten Entscheidung des Prüfers.
- **Mehrparteienfreigabe**: für Veröffentlichungen gilt eine Schwelle (Vorschlag
  **2 von 3** freigegebenen release-Schlüsseln). Ein einzelner Schlüssel darf
  nicht genügen, sobald mehr als eine Person Modelle veröffentlicht.
- **Kein Selbstvertrauen**: ein Artefakt, das seinen eigenen Schlüssel mitbringt,
  ist nicht vertrauenswürdig. Der Vertrauensspeicher liegt außerhalb der Datei.

## 7. Was K7 ausdrücklich nicht leistet

- Keine Public-Key-Infrastruktur, keine Zertifikatskette, keine Zertifizierungsstelle.
- Keine Schlüsselverteilung und keine Geheimhaltung: Signaturen sind öffentlich.
- Keine Aussage über die **Qualität** eines Modells, nur über seine Herkunft.
- Keine Aussage über Trainingsdaten oder Lizenz (das wäre K9, nicht K7).
- Kein Schutz gegen einen kompromittierten Signaturschlüssel; dagegen hilft nur
  Widerruf und Schwelle.

## 8. Offene Entscheidungen (vor Freigabe zu beantworten)

| ID | Frage | Empfehlung |
|---|---|---|
| D1 | Algorithmus: secp256k1/RFC 6979 oder Ed25519 | **secp256k1**, weil das Ökosystem ihn bereits für DID und Transaktionen führt; Ed25519 wäre einfacher, brächte aber einen zweiten Schlüsselbestand |
| D2 | Schwelle für Veröffentlichungen | **2 von 3**; bei einem Unterzeichner 1 von 1, ausdrücklich als Übergang markiert |
| D3 | Wo liegt der Vertrauensspeicher | im Repository (`keys/release.yaml`) und zusätzlich in der Verteilstelle; zwei Quellen, die übereinstimmen müssen |
| D4 | Signiert K7 auch die Metadaten (Modellname, Version)? | nein, nur Artefakt + Digest; Metadaten sind eine zweite Datei, die ihrerseits signiert werden kann |

## 9. Prüfkriterien für die Freigabe (K7-Gates)

| Gate | Gegenstand |
|---|---|
| K7-G1 | Behälterformat byte-genau, Feldsumme = 88 + `sig_len`, keine Ausrichtungsannahme |
| K7-G2 | Signierte Nachricht kanonisch, Domain-getrennt, keine variablen Teile |
| K7-G3 | Prüfreihenfolge: Struktur vor Signatur; Signaturfehler ≠ Strukturfehler |
| K7-G4 | Vertrauensspeicher: Rollen, Widerruf, Schwelle, kein Selbstvertrauen |
| K7-G5 | Ressourcen: Behälter ≤ 152 Byte, Prüfung ohne Rekursion, ein Schlüssel je Versuch |

## 10. Testfälle (Vorschlag, KT-01 bis KT-12)

| ID | Fall | Erwartung |
|---|---|---|
| KT-01 | gültige Signatur, Schlüssel im Speicher | `trustworthy = true` |
| KT-02 | Signatur über anderen Digest | `SignatureInvalid` |
| KT-03 | `digest` im Behälter passt nicht zur Datei | `DigestMismatch` |
| KT-04 | `signed_len` falsch | `LengthMismatch` |
| KT-05 | unbekannte `key_id` | `UnknownKey` |
| KT-06 | widerrufener Schlüssel, Signatur danach | `RevokedKey` |
| KT-07 | widerrufener Schlüssel, Signatur davor | akzeptiert, im Ergebnis markiert |
| KT-08 | `release`-Schwelle nicht erreicht (1 von 2) | `ThresholdNotMet` |
| KT-09 | `dev`-Schlüssel für Veröffentlichung | `RoleNotAllowed` |
| KT-10 | Behälter mit unbekanntem `alg` | `UnsupportedAlgorithm` |
| KT-11 | Behälter mit falschem `magic` | `BadSignatureContainer` |
| KT-12 | Artefakt strukturell kaputt **und** Signatur falsch | Strukturfehler zuerst |

## 11. Reifeklasse

RESEARCH. Nichts davon ist implementiert. Die Freigabe von D1 bis D4 ist die
Voraussetzung; erst danach wird der Behälter als `signature.rs` gebaut und die
Gates K7-G1 bis K7-G5 werden geprüft, bevor eine Zeile Verifikation entsteht.
