# Status atc-aurora-model (M-05)

| Bereich | Status | Beleg |
|---|---|---|
| Parameterhaushalt | `IMPLEMENTED` | `Config`, `Breakdown`, `param_count`, gepruefte Arithmetik |
| Lokale Pruefung | `TESTED` | 61 Tests (inkl. Messfall, `#[ignore]`), `cargo test` gruen, clippy `-D warnings` sauber, fmt sauber |
| Vorwaertslauf (B-19, B-20) | `IMPLEMENTED` | `forward`, `Weights`, RoPE, kausale Multi-Head-Attention, gebundener Kopf |
| Artefakt (B-21) | `IMPLEMENTED` | `write`/`read` gegen `ATC-AI-LLM-002` v1.2.0, AT-01…AT-35 grün |
| `CI_VERIFIED` | **offen** | Actions starten nicht, solange die Commit-Herkunft unverifiziert ist |
| `E2E_VERIFIED` | **offen** | kein Lauf im Gesamtsystem |

## Offene Entscheidung

B-18 nennt 5 771 776, die Ableitung ergibt 5 868 800 (mit abschliessender Norm)
beziehungsweise 5 868 544 (ohne). Differenz 97 024 beziehungsweise 96 768. Der
Widerspruch ist dokumentiert und getestet, nicht geglaettet. Entweder ist der
Sollwert veraltet, oder die Referenz hat eine andere Architektur als K4/K5
beschreiben. Beides ist eine Owner-Entscheidung, keine Implementierungsfrage.

Zerlegung: 97 024 = 379 x 256, ohne abschliessende Norm 96 768 = 6 x 63 Zeilen.
Die Differenz ist damit genau die abschliessende Norm plus 63 Zeilen je Block,
also `d_ff` 683 statt 704; mit `d_ff` 683 und ohne abschliessende Norm ergibt die
Ableitung exakt 5 771 776 (mit dem Crate nachgerechnet).

Belegt: 5 771 776 ist der Stand **vor** K5. Der Blueprint nennt `d_ff` 683 oder 704
in §6 als offenen Punkt (1,7 Prozent Parameter), die Spezifikation nennt nach K5 in
§7.1.1 5 868 544 aus der Formel `P = L(4d^2 + 3 d d_ff + 2d) + V d`. Diese Formel
zaehlt die abschliessende Norm nicht mit (5 868 800 mit ihr).

Zwei offene Owner-Entscheidungen: **A** B-18 auf den K5-Wert nachziehen,
**F1/F2** ob die abschliessende Norm mitzaehlt. Vorlage mit Belegstellen, Rechenweg
und Folgen: [`docs/decisions/AD-B18-PARAMETERHAUSHALT.md`](../../docs/decisions/AD-B18-PARAMETERHAUSHALT.md).

## Vorwaertslauf

Der Pfad ist vollstaendig verdrahtet: Einbettung ziehen, je Block RMSNorm, Q/K/V,
RoPE, kausale Attention (`j <= i`, alles darueber `-inf`), Ausgabeprojektion,
Restpfad, RMSNorm, SwiGLU-Gate und Down-Projektion, Restpfad; danach
abschliessende Norm und gebundener Ausgabekopf. **Keine Operation ist
zweitimplementiert** — gerechnet wird ausschliesslich mit `atc-aurora-tensor`
(M-01 bis M-03).

Tests: B-19 (Aenderung an Position `t` bewegt nur Positionen `>= t`; zweite
Richtung ueber die letzte Position), B-20 (512 Token, Form `(512, 4096)`, alle
Werte endlich), eine Strukturprobe der Maske (bei Gleichstand mittelt die
Attention genau ueber `0..=i`), Reproduzierbarkeit bei gleichem Startwert,
Gewichtssumme gleich Parameterhaushalt, und vier Fehlerpfade (leer, zu lang,
Token ausserhalb, Budget zu klein, unzulaessige Konfiguration).

Bewusste Grenzen: die Gewichte sind **deterministisch initialisiert, nicht
trainiert** — B-19 und B-20 pruefen Rechnung und Grenzen, nicht Sprachqualitaet.
Kein KV-Cache, kein Batching, kein Sampler (gehoert zu P8/`atc-aurora-runtime`).
`ROPE_BASE` ist 10 000 und **noch nicht normativ festgelegt**; die Spezifikation
nennt nur "RoPE". Er ist damit ein expliziter Implementierungsparameter, kein
Normwert, und steht im Artefaktformat als deklariertes Feld (`ATC-AI-LLM-002` §3,
offener Punkt K6). B-20 kostet im Debug-Build etwa 22 Sekunden.

## Artefakt: Stand der Umsetzung

`write` und `read` sind implementiert (`src/artifact/`): Header mit zentralen
Offsets (104 Byte, Befund F-01 damit dauerhaft geschlossen), kanonische
Tensorfolge einschließlich `lm_head` bei nicht gebundener Einbettung, SHA-256
über die ganze Datei mit genulltem Digest-Feld (eigene, geprüfte Umsetzung),
fail-closed Lesereihenfolge, `max_bytes` **vor** dem Hashen, geprüfte
Integer-Arithmetik, Sichten statt Modellkopie.

Geprüft sind 35 Fälle: AT-01 bis AT-35, dazu Roundtrip (geteilt und getrennt),
Determinismus, byte-identisches Neuschreiben eines gelesenen Artefakts und vier
bekannte SHA-256-Werte. Gesamt 61 Tests im Crate.

**Kosten, gemessen** (`docs/measurements/2026-11-11-artifact-decode-cost.md`):
eine Ref-1-Datei mit 23,5 MB wird in 65 ms gelesen, davon sind 65 ms SHA-256. Die
Strukturpruefung kostet unter einer Millisekunde, das Dekodieren aller 50 Tensoren
5,6 ms, die Vollkopie 13,4 ms. Der Digest ist der Preis, nicht das Format.

**K7 (Signatur)** ist als vollständiger Vertrag spezifiziert, nicht implementiert:
`ATC-AI-LLM-003` **v1.0.0**. Externer Behälter `ATCS` (152 Byte, genau 88 +
`sig_len`) neben dem unveraenderten Artefakt, kanonische 71-Byte-Nachricht mit
Domain-Trennung, secp256k1 mit RFC 6979, Low-S, fester Kodierung `r||s` (kein DER)
und strikter Schluesselvalidierung, dedizierte `model-release`-Schluessel getrennt
von DID-/Wallet-/Transaktionsschluesseln, Schwelle **2 von 3** als zwei unabhaengig
gueltige Signaturen (ausdruecklich **keine** Threshold-Signatur), versionierter
und signierter Vertrauensspeicher mit monotoner `store_version` und extern
verankertem Trust-Root, separat signiertes Metadatenmanifest. Entscheidungen D1
bis D4 sind am 11.11.2026 entschieden und im Vertrag festgehalten.

Die Selbstpruefung der Gates K7-G1 bis K7-G5 ergab fuenf PASS und ein PASS mit
Auflagen; sieben Befunde wurden dabei gefunden und eingearbeitet (F-01 doppelte
`key_id` erfuellte die Schwelle mit einem Schluessel, F-02 ECDSA-Malleabilitaet
ohne Low-S, F-03 unsignierte Etiketten als Beweis gelesen, F-04 Rollback eines
Speichers belebt widerrufene Schluessel, F-05 Selbstautorisierung neuer
root-Schluessel, F-06 gueltiges ATCM ohne Signatur als Release missverstanden,
F-07 `alg` ausserhalb der Nachricht erlaubt Algorithmusverwechslung). Eine
**unabhaengige** Pruefung des Vertrags steht aus; die Implementierung beginnt
erst danach.

**K6 ist ENTSCHIEDEN** (11.11.2026, Option A): `rope_base = 10 000` ist Normwert
fuer das bestehende Modellprofil (`ATC-AI-LLM-001` 1.5.0). Das Artefakt deklariert
den Wert weiterhin, der Leser lehnt Abweichungen ab; andere Basen nur ueber eine
neue Architekturversion. Kein Gewicht, kein Byte und kein Test hat sich dadurch
geaendert.

`CI_VERIFIED` bleibt offen: Actions laufen auf diesem Commit wegen der
unverifizierten Konto-E-Mail nicht an.

## Artefaktvertrag, Prüfstand

Der Vertrag ist am 11.11.2026 gegen sich selbst geprüft worden (Protokoll
[`docs/audits/2026-11-11-ATC-AI-LLM-002-GATE-REVIEW.md`](../../docs/audits/2026-11-11-ATC-AI-LLM-002-GATE-REVIEW.md)):
G1, G2, G3 und G5 **PASS**, G4 **RESIDUAL** (Signatur fehlt, offener Punkt K7).
Zwei blockierende Befunde sind dabei aufgefallen und im Vertrag geschlossen: der
Header ist 104 Byte statt der behaupteten 96, und ein Modell mit nicht gebundener
Einbettung braucht einen eigenen Ausgabekopf `lm_head`. Dazu kommen der
Ressourcenerschöpfungsschutz (Abschnitt 8.1) und sechs zusätzliche Testfälle
(AT-29 bis AT-34, zusammen 34).

## Nicht enthalten

Training (M-06), KV-Cache, Sampler, die Implementierung des Dateiformats `.atcm`
(der Vertrag steht in `ATC-AI-LLM-002`, der Code fehlt noch). Diese Teile sind hier weder angefangen noch vorgetaeuscht.
