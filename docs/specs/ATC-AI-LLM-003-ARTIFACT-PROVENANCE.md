---
document_id: ATC-AI-LLM-003
title: Herkunft und Authentizität von Aurora-Modellartefakten (K7) — Signaturvertrag
version: 1.0.0
status: proposed
owner: A-TownChain-Okosystems
created: 2026-11-11
updated: 2026-11-11
layer: L2 (aurora-ai)
security_class: S3
evidence_gate: AURORA-001
---

# K7 — Herkunft und Authentizität (Vertrag)

**Status: Vertrag vollständig, Freigabe durch den Owner ausstehend; keine
Implementierung.** Die Entscheidungen D1 bis D4 sind am 11.11.2026 entschieden
(Abschnitt 13). Dieses Dokument ist die verbindliche Grundlage für `atcs` und für
die Freigabeprüfung.

**Was K7 leistet, in einem Satz:** es beantwortet „von welchem Schlüssel, mit
welcher Rolle, unter welcher Freigabeschwelle" — und trennt diese Antwort strikt
von der Frage, ob die Bytes unverändert sind.

## 0. Ausgangslage

`ATC-AI-LLM-002` sichert **Integrität**: der SHA-256 im Header belegt, dass die
Bytes seit dem Schreiben unverändert sind. Er belegt **nicht** Authentizität: wer
schreiben kann, rechnet den Digest neu. Der Vertrag sagt das selbst (§9) und
verweigert stilles Vertrauen (`trustworthy = false`, `TrustAnchorMissing` im
strengen Modus).

Damit gibt es heute drei ehrliche Zustände: „unverändert, Herkunft unbekannt",
„stimmt mit einem erwarteten Digest überein" und „nichts bekannt". K7 fügt den
vierten hinzu: **„von Schlüsseln der Rolle `model-release` signiert, Signatur
gültig, Schwelle erreicht"**.

## 1. Grundentscheidung: externe Signatur, ATCM unverändert

Das ATCM-Format bleibt **unverändert** (1.2.0). Die Signatur liegt neben der
Datei. Gründe:

1. Das Format ist eingefroren. Ein Signaturfeld im Header erzwänge eine neue
   Formatversion und entwertete jedes bestehende Artefakt.
2. Ein Release trägt **mehrere** Signaturen (Schwelle), ohne dass das Format
   davon wissen muss.
3. Wer nur Integrität braucht, arbeitet ohne Signatur weiter.

**Verbindliche Trennung (Owner-Vorgabe):** ein gültiges ATCM ohne gültige
ATCS-Signatur bleibt ein **strukturell intaktes Artefakt**, ist aber **kein
authentifiziertes Release**. Die API liefert deshalb zwei getrennte Ergebnisse:
`Integrität` (aus `ATC-AI-LLM-002`) und `Herkunft` (aus diesem Dokument).

## 2. Signaturbehälter `ATCS` v1

Ein Behälter trägt **genau eine** Signatur. Ein Release mit Schwelle hat mehrere:
`<name>.atcm.sig`, `<name>.atcm.sig.2`, `<name>.atcm.sig.3`.

Detached, byte-genau, little endian, **152 Byte** fest:

| Offset | Länge | Feld | Regel |
|---|---|---|---|
| 0 | 4 | `magic` | `ATCS` |
| 4 | 2 | `version` | `1` |
| 6 | 2 | `alg` | `1` = secp256k1/RFC 6979/SHA-256. Andere Werte reserviert, nicht zulässig, bis eine neue Version sie einführt |
| 8 | 4 | `flags` | Bit 0 = über komprimierte Datei signiert (K8); alle anderen Bits 0 |
| 12 | 4 | `key_id` | Kennung des Signaturschlüssels, siehe Abschnitt 5 |
| 16 | 8 | `signed_len` | Länge der signierten Datei in Byte |
| 24 | 32 | `digest` | SHA-256 des Artefakts, wie im ATCM-Header |
| 56 | 2 | `sig_len` | 64 |
| 58 | 2 | `reserved` | 0 |
| 60 | 4 | `reserved2` | 0 |
| 64 | 8 | `issued_at` | Unix-Sekunden, **nicht signiert**, nur Anzeige |
| 72 | 2 | `signer_len` | 1..=16, Zeichen `[a-z0-9_.-]` |
| 74 | 14 | `signer` | Name des Unterzeichners, NUL-aufgefüllt, **nicht signiert** |
| 88 | 64 | `signature` | `r || s`, je 32 Byte, big endian |

Feldsumme: 4+2+2+4+4+8+32+2+2+4+8+2+14 = **88**, plus 64 Byte Signatur = **152**.
Reservierte Felder müssen 0 sein; alles andere ist `BadSignatureContainer`.

**`issued_at` und `signer` sind nicht signiert** (Befund F-03 der Selbstprüfung,
Abschnitt 14). Sie sind Etiketten für Menschen. Keine Entscheidung des Prüfers
darf auf ihnen beruhen — weder Gültigkeitsfenster noch Rollback noch
Widerruf. Das muss im Anwenderhandbuch stehen, sonst entsteht der Eindruck, der
Name sei bewiesen.

## 3. Signierte Nachricht (Domain-Trennung)

Signiert wird eine kanonische Nachricht fester Länge, **nicht** die Datei:

```text
"ATC-AI-ARTIFACT-SIGNATURE-v1\n"   (29 Byte ASCII)
alg:u16 || digest:32 || signed_len:u64
```

71 Byte, keine variablen Teile, keine Serialisierung. Signiert wird
`e = SHA-256(Nachricht)` (FIPS 180-4, 32 Byte), interpretiert als Ganzzahl.

Die Domain-Zeichenkette verhindert, dass eine Signatur aus einem anderen
Zusammenhang (Chain-Transaktion, DID-Rotation, Commit, Manifest) als
Artefaktsignatur durchgeht — und umgekehrt. `alg` liegt **innerhalb** der
Nachricht: sonst könnte derselbe Digest später unter einem anderen Verfahren
gültig erscheinen (Befund F-07).

## 4. Kryptografie (D1 entschieden: secp256k1)

| Punkt | Regel |
|---|---|
| Kurve | secp256k1, kanonische Parameter |
| Nonce | RFC 6979, HMAC-SHA256, deterministisch; keine Zufallsnonce, keine Wiederverwendung |
| Signaturkodierung | fest `r || s`, je 32 Byte, big endian; **kein DER** (eine Signatur hat genau eine Darstellung) |
| Low-S | verbindlich: `s ≤ n/2`. `s > n/2` wird abgelehnt, nicht normalisiert |
| Bereich | `r, s ∈ [1, n-1]`, sonst `SignatureInvalid` |
| Öffentlicher Schlüssel | komprimiert SEC1, 33 Byte (`0x02`/`0x03`); entpackt muss der Punkt auf der Kurve liegen, nicht der Punkt im Unendlichen sein; unkomprimiert (65 Byte) und Hybridformen werden abgelehnt |
| Prüfung | Verifikation aus `r`, `s`, `e` und dem gespeicherten Schlüssel; `recovery id` wird nicht benutzt und nicht gespeichert |
| Determinismus | dieselbe Nachricht mit demselben Schlüssel ergibt byte-identisch dieselbe Signatur (nachprüfbar in Tests) |

`e` ist 32 Byte und damit exakt die Gruppenlänge; die in ECDSA nötige Kürzung
entfällt, wird aber trotzdem als `e mod n` gerechnet, damit die Implementierung
nicht von der Länge abhängt.

## 5. Schlüsselrollen und `key_id`

**Owner-Vorgabe: gleicher Algorithmus heißt nicht gleicher Schlüssel.**
Modellartefakte werden von **dedizierten Release-Schlüsseln** signiert.
Operative DID-, Wallet- und Transaktionsschlüssel dürfen Modelle **nicht**
signieren, auch wenn sie dieselbe Kurve benutzen.

| Rolle | Zweck | Darf signieren | Darf Vertrauensspeicher signieren |
|---|---|---|---|
| `root` | verankert den Vertrauensspeicher | nein | ja (Schwelle) |
| `model-release` | Modelle freigeben | ja | nein |
| `test` | Prüfpfade, nie veröffentlichen | ja, nur `channel = test` | nein |
| `dev` | lokale Entwicklung | ja, nie veröffentlichen | nein |

`key_id` = **erste vier Byte** von `SHA-256(öffentlicher Schlüssel, komprimiert
SEC1, 33 Byte)`, big endian gelesen. Kollisionen sind bei 32 Bit möglich und
deshalb kein Identitätsnachweis: der Vertrauensspeicher führt **Schlüssel, nicht
Kennungen**, und eine mehrdeutige `key_id` wird mit `AmbiguousKey` abgelehnt.

## 6. Schwelle (D2 entschieden: 2 von 3)

Ein Release gilt als authentifiziert, wenn **zwei unabhängig gültige
Signaturen** von **zwei verschiedenen Schlüsseln** der Rolle `model-release`
vorliegen, die beide zum selben Artefakt-Digest gehören.

**Klarstellung, verbindlich:** das ist **keine kryptografische
Threshold-Signatur**. Es sind zwei vollständige Signaturen, die einzeln geprüft
werden. Es gibt keinen gemeinsamen Schlüsselanteil und keine Interaktion zwischen
den Unterzeichnern.

Regeln:

- Zwei Signaturen desselben `key_id` zählen **einmal** (Befund F-01).
- Zwei Behälter mit demselben `signer`-Namen und verschiedenen Schlüsseln zählen
  als zwei — der Name ist ein Etikett, die Kennung entscheidet.
- Ein einzelner kompromittierter Schlüssel darf keine Freigabe erzeugen: mit
  3 Schlüsseln und Schwelle 2 ist ein einzelner Schlüssel nicht ausreichend.
- Übergang mit **einem** Unterzeichner: `1 von 1` ist zulässig, muss aber im
  Manifest als `transitional` markiert und im Prüfergebnis so gemeldet werden.
- Nicht erreichte Schwelle: `ThresholdNotMet`, mit der Zahl der gültigen
  Signaturen im Ergebnis.

## 7. Vertrauensspeicher und Trust-Root (D3 entschieden)

Beide Quellen — `keys/trust-store.yaml` im Repository und die veröffentlichte
Kopie an der Verteilstelle — sind **versioniert und signiert**. Übereinstimmung
allein beweist nichts: wer beide Dateien ändern kann, stimmt sie überein.

### 7.1 Trust-Root

- `keys/trust-root.yaml` enthält die `root`-Schlüssel (Vorschlag: drei) mit
  `key_id`, komprimiertem SEC1-Schlüssel und `valid_from`.
- Der **SHA-256 dieser Datei ist im Prüfer verankert** (`TRUST_ROOT_SHA256`
  als Konstante im Quelltext) und zusätzlich im Standard dokumentiert.
  Abweichung → `NoTrustRoot`, Prüfung endet.
- Damit gilt die Kette: Prüfer-Binary ← verankerter Digest ← `trust-root.yaml`
  ← signiert den Vertrauensspeicher ← nennt die Release-Schlüssel.
- Der Trust-Root liegt **außerhalb** des zu prüfenden Vertrauensspeichers. Eine
  Datei, die sich selbst autorisiert, ist wertlos.

### 7.2 Vertrauensspeicher

| Feld | Regel |
|---|---|
| `store_version` | monoton steigende Ganzzahl; **Rollback wird abgelehnt** (Befund F-04) |
| `signatures` | mindestens zwei gültige `root`-Signaturen über den kanonischen Inhalt (gleiche Kodierung wie Abschnitt 3, eigene Domain `ATC-AI-TRUST-STORE-v1\n`) |
| `valid_from` / `valid_to` | Gültigkeitsfenster je Schlüssel, **die einzige Zeitquelle**, die zählt |
| `revoked` | Liste aus `key_id` und Widerrufszeitpunkt, nicht entfernbar durch ein späteres Update ohne `root`-Schwelle |
| Rollen | je Schlüssel genau eine Rolle aus Abschnitt 5 |
| Selbstautorisierung | verboten: ein Update darf `root`-Schlüssel nur mit `root`-Schwelle ändern (Befund F-05) |

Widerrufssemantik, verbindlich: geprüft wird gegen den **aktuellen**
Vertrauensspeicher. Ist ein Schlüssel dort widerrufen, sind seine Signaturen
ungültig (`RevokedKey`). Eine historische Prüfung ist nur mit ausdrücklich
angegebenem `store_version` zulässig und wird im Ergebnis als historisch
markiert. Ohne signierten Zeitstempel gibt es keine „Signatur vor dem Widerruf".

## 8. Metadatenmanifest (D4 entschieden: separat signiert)

Das Artefakt bleibt reine Numerik. Alles, was ein Release *beschreibt*, steht in
einem eigenen Manifest, das **eigene** Signaturen trägt:

`<name>.manifest.json` und `<name>.manifest.sig` (Schwelle wie Abschnitt 6,
Domain `ATC-AI-MANIFEST-SIGNATURE-v1\n`, signiert wird `SHA-256` der
Manifest-Bytes).

Kanonische Felder: `model` (Name), `model_version` (SemVer), `architecture`
(Profilkennung, z. B. `ref1`), `atcm_digest` (32 Byte hex), `atcm_bytes`,
`channel` (`test` | `release`), `released_at`, `previous` (Digest des vorigen
Manifests oder `null`).

Vertrauensstatus der Angaben, verbindlich:

| Information | Vertrauensstatus |
|---|---|
| ATCM-Digest | Integritätsnachweis |
| ATCS-Signatur | Herkunftsnachweis des Artefakts |
| `issued_at`, `signer` im ATCS | ohne eigene Signatur **nicht** vertrauenswürdig |
| `model_version`, `channel` | nur mit gültiger Manifestsignatur vertrauenswürdig |
| `previous` | verhindert das Zurückspielen eines älteren, korrekt signierten Releases |

Ohne Manifest ist ein Artefakt „authentisch, aber unbeschrieben"; es darf nicht
als Release mit Versionsangabe geführt werden.

## 9. Prüfreihenfolge (verbindlich)

1. ATCM-Struktur, Grenzen, Tabelle, Digest — unverändert nach `ATC-AI-LLM-002`.
2. Danach, falls Behälter vorhanden: `magic`, `version`, `alg` bekannt,
   reservierte Felder 0, `sig_len` = 64.
3. `signed_len` = Dateilänge, `digest` = berechneter Digest.
4. `key_id` im Vertrauensspeicher auflösen (mehrdeutig → `AmbiguousKey`),
   Rolle prüfen (`RoleNotAllowed`), Gültigkeitsfenster und Widerruf prüfen.
5. Öffentlichen Schlüssel strikt validieren (Abschnitt 4).
6. Signatur prüfen (`SignatureInvalid`), Low-S vorher.
7. Über alle Behälter: Schwelle (`ThresholdNotMet`), Duplikate (`DuplicateSigner`).
8. Manifest, falls vorhanden: eigene Signatur, `atcm_digest` gegen Schritt 1,
   `previous`-Kette, `channel` gegen Rolle.
9. Architekturprofil gegen Erwartung (wie `ATC-AI-LLM-002`), Endlichkeit optional.

**Strukturfehler werden immer vor Herkunftsfehlern gemeldet** (F-06 aus
`ATC-AI-LLM-002` gilt weiter). Eine ungültige Signatur heißt „Herkunft nicht
bestätigt", nicht „Datei beschädigt".

## 10. Fehler (verbindlich)

`BadSignatureContainer`, `UnsupportedAlgorithm`, `DigestMismatch`,
`LengthMismatch`, `UnknownKey`, `AmbiguousKey`, `RevokedKey`, `KeyExpired`,
`RoleNotAllowed`, `SignatureInvalid`, `DuplicateSigner`, `ThresholdNotMet`,
`StoreVersionRollback`, `StoreSignatureInvalid`, `NoTrustRoot`,
`ManifestMismatch`, `ManifestChainBroken`.

## 11. Ressourcengrenzen

| Größe | Grenze |
|---|---|
| Behälter | genau 152 Byte, keine Variable |
| Behälter je Artefakt | ≤ 8; mehr → `ThresholdNotMet` mit Hinweis auf die Grenze |
| Vertrauensspeicher | ≤ 1 MiB, ≤ 1024 Schlüssel, ≤ 4096 Widerrufe |
| Manifest | ≤ 64 KiB |
| Prüfaufwand | linear, ohne Rekursion, ohne Netzzugriff |

Es gibt **keinen** Netzwerkzugriff in der Prüfung: Trust-Root und
Vertrauensspeicher werden vom Aufrufer bereitgestellt. Woher er sie hat, ist
dessen Verantwortung und nicht Teil dieses Vertrags.

## 12. Was K7 ausdrücklich nicht leistet

- Keine PKI, keine Zertifizierungsstelle, keine Zertifikatskette.
- Keine Schlüsselverteilung und keine Geheimhaltung: Signaturen sind öffentlich.
- Keine Aussage über die **Qualität** eines Modells, nur über Herkunft und
  Freigabe.
- Keine Aussage über Trainingsdaten oder Lizenz (das wäre K9).
- Kein Schutz gegen einen kompromittierten Signaturschlüssel; dagegen helfen nur
  Widerruf und Schwelle.
- Keine Zeitbeweise: ohne signierten Zeitstempel gibt es keine belastbare
  Aussage „das war vor dem Widerruf".

## 13. Entscheidungen (Owner, 11.11.2026)

| ID | Entscheidung |
|---|---|
| D1 | **secp256k1 mit RFC 6979.** Gleicher Algorithmus bedeutet **nicht** gleicher Schlüssel: dedizierte Release-Schlüssel, keine Wiederverwendung operativer DID-/Wallet-/Transaktionsschlüssel. Zusätzlich: Low-S, kanonische Kodierung, strikte Schlüsselvalidierung, Domain-Trennung, keine Nonce-Wiederverwendung |
| D2 | **2 von 3**, drei unabhängige Schlüssel und getrennte Verantwortlichkeiten; zwei unabhängig gültige Signaturen, **keine** Threshold-Signatur. Widerruf, Rotation und dokumentierter Wiederherstellungsprozess; keine automatische Ersetzung eines widerrufenen Schlüssels |
| D3 | **Versionierter, signierter Vertrauensspeicher** plus extern verankerter Trust-Root; Übereinstimmung zweier Kopien ist kein Beweis; Rollback-Schutz, explizite Widerrufslisten, keine Selbstautorisierung |
| D4 | **Separat signiertes Manifest.** `issued_at` und `signer` sind ohne eigene Signatur nicht vertrauenswürdig; Modellversion und Release-Kanal nur mit gültiger Manifestsignatur |

## 14. Gates K7-G1 bis K7-G5 — Selbstprüfung

Die Prüfung wurde am 11.11.2026 am Text selbst durchgeführt (eine Person). Eine
unabhängige Prüfung steht aus; das ist bei einem Sicherheitsvertrag die
schwächere Variante und wird hier ausdrücklich so benannt.

| Gate | Gegenstand | Ergebnis |
|---|---|---|
| K7-G1 | Behälter byte-genau, Feldsumme = 88 + `sig_len`, keine Ausrichtungsannahme | **PASS** (Feldsumme nachgerechnet: 88; Gesamt 152) |
| K7-G2 | Kanonische Nachricht, Domain-getrennt, keine variablen Teile, `alg` innerhalb | **PASS** (71 Byte, `alg` in der Nachricht, F-07 behoben) |
| K7-G3 | Prüfreihenfolge Struktur vor Herkunft, Signaturfehler ≠ Strukturfehler | **PASS** |
| K7-G4 | Rollen, Widerruf, Schwelle, Trust-Root, keine Selbstautorisierung | **PASS mit Auflagen** (F-01, F-04, F-05 eingearbeitet) |
| K7-G5 | Ressourcen: fester Behälter, Obergrenzen, linear, kein Netz | **PASS** |

**Befunde der Selbstprüfung** (alle eingearbeitet):

| ID | Befund | Behebung |
|---|---|---|
| F-01 | Zwei Behälter mit demselben `key_id` hätten die Schwelle mit einem Schlüssel erfüllt | getrennte `key_id` verbindlich, `DuplicateSigner` |
| F-02 | ECDSA ohne Low-S erlaubt zwei gültige Darstellungen derselben Signatur | Low-S verbindlich, kein DER, feste Kodierung |
| F-03 | `issued_at` und `signer` sind unsigniert, könnten aber als Beweis gelesen werden | ausdrücklich keine Entscheidung darauf; Zeit kommt nur aus dem Vertrauensspeicher |
| F-04 | Ein älterer, korrekt signierter Speicher könnte einen widerrufenen Schlüssel wiederbeleben | `store_version` monoton, Rollback abgelehnt, Widerruf nicht entfernbar |
| F-05 | Ein Update könnte neue `root`-Schlüssel einführen | nur mit `root`-Schwelle, sonst `StoreSignatureInvalid` |
| F-06 | Ein gültiges ATCM ohne Signatur könnte als Release missverstanden werden | zwei getrennte Ergebnisse: Integrität und Herkunft |
| F-07 | `alg` außerhalb der Nachricht erlaubt Algorithmusverwechslung | `alg` in der Nachricht, Gleichheit mit dem Behälterfeld verbindlich |

## 15. Testfälle (KT-01 bis KT-20)

| ID | Fall | Erwartung |
|---|---|---|
| KT-01 | gültige Signatur, Rolle `model-release`, Schwelle erfüllt | authentifiziert |
| KT-02 | Signatur über anderen Digest | `SignatureInvalid` |
| KT-03 | `digest` im Behälter passt nicht zur Datei | `DigestMismatch` |
| KT-04 | `signed_len` falsch | `LengthMismatch` |
| KT-05 | unbekannte `key_id` | `UnknownKey` |
| KT-06 | mehrdeutige `key_id` im Speicher | `AmbiguousKey` |
| KT-07 | widerrufener Schlüssel | `RevokedKey` |
| KT-08 | Schlüssel außerhalb `valid_from`/`valid_to` | `KeyExpired` |
| KT-09 | `dev`-Schlüssel für `channel = release` | `RoleNotAllowed` |
| KT-10 | Behälter mit `alg = 2` | `UnsupportedAlgorithm` |
| KT-11 | Behälter mit falschem `magic` oder reserviertem Bit | `BadSignatureContainer` |
| KT-12 | Artefakt strukturell kaputt **und** Signatur falsch | Strukturfehler zuerst |
| KT-13 | `s > n/2` | `SignatureInvalid` (Low-S) |
| KT-14 | `r` oder `s` = 0 oder ≥ n | `SignatureInvalid` |
| KT-15 | Signatur in DER-Form | `BadSignatureContainer` |
| KT-16 | unkomprimierter Schlüssel (65 Byte) im Speicher | `BadSignatureContainer` |
| KT-17 | zwei Behälter, gleicher `key_id` | `DuplicateSigner` |
| KT-18 | drei Behälter, zwei verschiedene Schlüssel | authentifiziert (Schwelle 2 von 3) |
| KT-19 | Vertrauensspeicher mit `store_version` kleiner als bekannt | `StoreVersionRollback` |
| KT-20 | Vertrauensspeicher ohne `root`-Schwelle, aber neuen `root`-Schlüsseln | `StoreSignatureInvalid` |
| KT-21 | `trust-root.yaml` entspricht nicht dem verankerten Digest | `NoTrustRoot` |
| KT-22 | Manifest mit anderem `atcm_digest` | `ManifestMismatch` |
| KT-23 | `previous`-Kette zeigt auf unbekanntes Manifest | `ManifestChainBroken` |
| KT-24 | gültige Signatur, aber nur 1 von 2 Schlüsseln | `ThresholdNotMet` |
| KT-25 | zweimaliges Signieren derselben Nachricht | byte-identische Signatur (Determinismus) |

## 16. Reifeklasse und was als Nächstes gilt

RESEARCH. Es existiert keine Zeile `atcs`-Code. Reihenfolge nach der Freigabe
(Owner-Vorgabe): Vertrag freigeben → K7-G1 bis K7-G5 unabhängig prüfen →
Trust-Root und Vertrauensspeicher definieren → Rollen und Widerruf → `atcs`
Writer/Reader → Signaturprüfung und 2-von-3 → Manifest → KT-01 bis KT-25 →
Integration in den ATCM-Reader → frischer Klon, Tests, CI-Evidenz.

Kein Signaturcode vor der Freigabe dieses Dokuments.
