---
document_id: ATC-AI-AUD-002-GR
title: Gate-Prüfung G1–G5 des Artefaktvertrags ATC-AI-LLM-002 (.atcm v1) — Matrix, Befunde, Entscheidungen
version: 1.0.0
status: accepted
owner: A-TownChain-Okosystems
created: 2026-11-11
updated: 2026-11-11
subject: ATC-AI-LLM-002 v1.0.0 (geprüft), v1.1.0 (nachgezogen)
peer: aurora-ai HEAD 55bc2bdbd306b5483725223f9e1987ed4fb09e0b
layer: L2 (aurora-ai)
security_class: S3
evidence_gate: AURORA-001
---

# Gate-Prüfung G1–G5 des Artefaktvertrags

**Gegenstand:** `docs/specs/ATC-AI-LLM-002-ATCM-ARTIFACT-CONTRACT.md`, Fassung 1.0.0
auf `aurora-ai` HEAD `55bc2bdbd`. **Geprüft wurde der Vertragstext**, nicht Code:
es gibt keinen. Zeilenangaben beziehen sich auf Fassung 1.0.0 an diesem Commit.

**Auftrag des Owners (11.11.2026):** Gate-Matrix mit Fundstellen, dokumentierte
Vertragslücken und Sicherheitsrisiken, Ergänzung des
Ressourcenerschöpfungsschutzes, Entscheidung je Gate, **erst danach** Freigabe für
`write` und `read`.

## 1. Gate-Matrix

| Gate | Gegenstand | Fassung 1.0.0 | Nach Nachführung (1.1.0) | Befunde |
|---|---|---|---|---|
| **G1** | Byte-Layout und Struktur (Header, Tabelle, Offsets, Reihenfolge, Ausrichtung) | **BLOCKED** | **PASS** | F-01, F-02, F-04 |
| **G2** | Determinismus und Kanonizität (Digest, Reihenfolge, Wiederholbarkeit) | PASS | **PASS** | F-06 |
| **G3** | Parser-Sicherheit (fail-closed, keine ungeprüften Offsets, Overflow) | PASS | **PASS** | F-03, F-08 |
| **G4** | Vertrauensmodell (Integrität ≠ Authentizität) | **RESIDUAL** | **RESIDUAL** | F-07 |
| **G5** | Ressourcenerschöpfung (Grenzen, Zuteilung, Aufwand) | **BLOCKED** | **PASS** | F-05a–e |

**Gesamtergebnis: PASS mit einem RESIDUAL in G4.** Die Freigabe für `write` und
`read` ist damit erteilt, mit der Auflage aus Abschnitt 5 (Ankerpflicht auf
Netzwerkpfaden).

## 2. Befunde mit Fundstellen

### F-01 — Header ist 104 Byte, nicht 96 (G1, blockierend)

Der Vertrag nennt an vier Stellen 96 Byte: Zeile 71 („Der Header ist genau
**96 Byte**"), Zeile 113 (Datenbereich ab `96 + 64 * tensor_count`), Zeile 116
(Dateilänge), Zeile 172 (Mindestlänge beim Lesen) und Zeile 177 (Längenprüfung).
Die Feldtabelle selbst endet bei `digest` mit „Offset 72, Länge 32", also bei
Byte **104**; die Feldbreiten summieren sich auf 4+2+2+44+4+4+4+8+32 = **104**.

Ein Leser, der der Textangabe folgt, beginnt acht Byte zu früh mit der
Tabelleninterpretation und liest den Digest als ersten Tabelleneintrag. Das ist
kein Schönheitsfehler, sondern ein Formatbruch: ein anderer Leser, der der
Tabelle folgt, wäre inkompatibel. **Behoben** in 1.1.0: 104 an allen Stellen,
Rechenweg dokumentiert.

### F-02 — Nicht gebundene Einbettung ist nicht darstellbar (G1, blockierend)

`tensor_count == 2 + 8 * n_layers` (Zeile 86) und die kanonische Folge (Zeilen
119–137) kennen nur `tok_emb` und `final_norm` neben den Blocktensoren. Der
Parameterhaushalt des Crates führt `tied_embedding = false` als **zulässige**
Konfiguration (`modules/atc-aurora-model/src/params.rs`, `output_projection`
ungleich null; Test `b_18_nicht_geteilte_einbettung_addiert_genau_eine_einbettung`
erwartet +1 048 576). Ein solches Modell bräuchte einen eigenen Ausgabekopf
`lm_head (d_model, vocab)`; der Vertrag hätte ihn entweder verboten oder
verschluckt. **Behoben** in 1.1.0: `lm_head` in der Folge, `tensor_count`
abhängig von `flags` Bit 0, drei Testfälle AT-29 bis AT-31.

### F-03 — Keine Grenze für Elemente je Tensor (G3, nicht blockierend)

Begrenzt waren die einzelne Dimension (≤ 2^20) und die Gesamtdatenmenge, nicht
das Produkt. AT-20 prüft den Überlauf, aber eine explizite Grenze fehlt.
**Ergänzt**: Elemente je Tensor ≤ 2^28.

### F-04 — Keine Regel zur Ausrichtung (G1/G3, nicht blockierend)

Die `u64`-Felder liegen bei Offset 48 und 56 im Eintrag, `tensor_data_bytes` bei
64 im Header. Ausrichtung ist damit nicht garantiert, und in `unsafe`-freiem
Rust ist ein Ausrichtungs-Cast ohnehin unzulässig. Der Vertrag sagte dazu
nichts. **Ergänzt**: Felder sind byteweise zu lesen, Alignment-Annahme verboten.

### F-05 — Ressourcenerschöpfung nicht ausreichend behandelt (G5, blockierend)

Fünf Lücken, die zusammen einen Erschöpfungsangriff erlauben:

- **F-05a** Keine Aufrufergrenze für die Dateilänge selbst. Die harte Grenze
  begrenzt den Datenbereich, aber eine 1-GiB-Datei darf gehasht werden; Hashing
  kostet CPU proportional zur Länge. Ohne Grenze vor dem Hashen ist wiederholtes
  Senden großer Dateien ein Aufwandsvektor. **Behoben**: `max_bytes`, geprüft
  **vor** dem Hashen, Testfall AT-34.
- **F-05b** Keine Aussage über Kopien. `read` hätte das Modell duplizieren
  können; dann wäre der Zusatzspeicher O(Modellgröße). **Behoben**: Regel 2 in
  Abschnitt 8.1, Sichten statt Kopien.
- **F-05c** Keine Aussage über den Arbeitsaufwand. **Behoben**: Regel 3
  (ein Durchlauf, keine Rekursion, keine Allokation je Tabelleneintrag, fester
  32-Byte-Namensvergleich).
- **F-05d** Keine Aussage, dass Ratenbegrenzung Aufrufersache ist — ein Leser
  hätte sie zugesagt bekommen oder stillschweigend angenommen. **Behoben**:
  Regel 5, ausdrückliche Nicht-Zusage.
- **F-05e** **Innere Inkonsistenz der Grenzen:** `n_layers` ≤ 1024 erlaubt nach
  der Formel 2 + 8·1024 = 8194 Tensoren, `tensor_count` war aber auf 8192
  begrenzt. Ein zulässiges Modell wäre am Artefakt gescheitert. **Behoben**:
  `tensor_count` ≤ 8194, mit Begründung im Text.

### F-06 — Präzedenz Struktur vor Anker fehlte (G2, nicht blockierend)

Abschnitt 7 prüft den Digest (Schritt 7) vor dem Anker (Schritt 8), sagte aber
nicht, was bei doppeltem Fehler gilt. **Ergänzt**: Strukturfehler immer zuerst.

### F-07 — Authentizität bleibt offen (G4, RESIDUAL)

Der Vertrag leistet, was er leisten kann: er deklariert ausdrücklich, dass der
Digest **Integrität, nicht Authentizität** belegt, und verweigert stilles
Vertrauen (`trustworthy = false`, `TrustAnchorMissing` im strengen Modus). Was
fehlt, ist ein Signaturverfahren (offener Punkt K7). Das ist ein **RESIDUAL**,
kein BLOCKED: kein Pfad behauptet Vertrauen, das er nicht hat.

### F-08 — Testabdeckung lückenhaft (G3, nicht blockierend)

Nicht abgedeckt waren: nicht gebundene Einbettung (F-02), doppelte Namen,
`offset` nahe 2^63 (Überlauf beim Addieren), und die Aufrufergrenze für die
Dateilänge. **Ergänzt**: AT-29 bis AT-34.

## 3. Was die Prüfung bestätigt hat

Damit die Matrix nicht nur Mängel nennt: diese Punkte sind tragfähig und bleiben
unverändert.

- Der Digest bindet **Header, Tabelle und Daten** in einer Prüfsumme (Digest-Feld
  genullt). Ein geänderter Architekturparameter fällt auf, nicht nur ein
  Zahlenwert.
- Die kanonische Tensor-Reihenfolge ist als Vertrag formuliert, nicht als
  Konvention, mit `TensorOrderInvalid` als erzwungenem Fehler. Das ist die
  Voraussetzung für byte-identisches Neuschreiben (Abnahmepunkt 6).
- `rope_base` und `norm_eps` stehen im Artefakt statt nur im Code. Ohne sie wäre
  ein Modell nicht reproduzierbar, auch bei identischen Gewichten.
- Die Größengrenzen greifen **vor** jeder Zuteilung, und AT-28 (1000 zufällige
  Bitmutationen) prüft genau die Eigenschaft, die ein Parser braucht: an keiner
  Byteposition ungeprüft lesen.

## 4. Entscheidungen je Gate

| Gate | Entscheidung | Begründung |
|---|---|---|
| G1 | **PASS** (nach Nachführung) | F-01 und F-02 waren blockierend und sind im Vertrag geschlossen; F-04 ergänzt |
| G2 | **PASS** | Determinismus und Kanonizität sind vollständig festgelegt; F-06 nachgezogen |
| G3 | **PASS** | fail-closed Reihenfolge, geprüfte Arithmetik, keine ungeprüften Offsets; F-03/F-08 nachgezogen |
| G4 | **RESIDUAL** | Signatur fehlt (K7). Kein stilles Vertrauen, Ankerpflicht im strengen Modus. Auflage aus Abschnitt 5 |
| G5 | **PASS** (nach Nachführung) | F-05a–e geschlossen, Grenzen konsistent |

## 5. Auflagen für die Implementierung

1. **Ankerpflicht auf Netzwerkpfaden.** Ein Artefakt, das über Netzwerk,
  Nachricht oder Dateitausch empfangen wurde, darf nur mit `expected_digest`
  geladen werden. Ohne Anker ist `trustworthy = false` zu behandeln wie „nicht
  geladen".
2. **`max_bytes` ist Pflichtparameter**, nicht optional. Ein Aufruf ohne Grenze
  ist ein Vertragsverstoß des Aufrufers.
3. **AT-01 bis AT-34** sind vor der Freigabe von `write` und `read` grün
   nachzuweisen, AT-28 inklusive.
4. **Kein Trainingslauf** (M-06) und keine Verteilung von Artefakten, bevor K7
  entschieden ist. Trainieren darf auf dem eigenen Knoten auch ohne Signatur.
5. Die Reifeklasse bleibt bei `SPECIFIED`. `IMPLEMENTED` erst mit Code, `TESTED`
   erst mit grüner adversarialer Suite, `CI_VERIFIED` erst mit CI-Lauf am
   Commit.

## 6. nächster Schritt

Der Vertrag ist mit Fassung 1.1.0 freigegeben. Als nächstes: `write` und `read`
gegen genau diese Fassung implementieren, dann AT-01 bis AT-34. Der
Parameterhaushalt bleibt unberührt: `Weights` liefert die kanonische Reihenfolge
bereits in der Form, die der Vertrag verlangt.
