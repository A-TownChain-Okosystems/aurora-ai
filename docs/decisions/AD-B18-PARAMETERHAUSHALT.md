---
document_id: ATC-DOC-AI-AD-001
title: B-18 Parameterzahl Ref-1 — Sollwert und Ableitung
version: 0.3.0
status: proposed
owner: A-TownChain-Okosystems
created: 2026-11-11
standard: atc-standards
---

# AD-B18-PARAMETERHAUSHALT — Parameterzahl von Ref-1: Sollwert gegen Ableitung

> **Status:** ENTSCHIEDEN am 11.11.2026 — **A + F1** durch den Owner bestätigt. Sollwert B-18 = **5 868 800**; Formel `P = L·(4d² + 3·d·d_ff + 2d) + V·d + d`. K5 (`d_ff` 704) bleibt bestehen, die abschliessende Norm bleibt bestehen. Die historische Rechnung bleibt dokumentiert.
> **Fassung 0.3.0 (11.11.2026):** Entscheidung eingetragen. A: B-18 auf 5 868 800 nachgezogen.
> F1: Formel um `+ d` korrigiert. Umgesetzt in `ATC-AI-LLM-001-A` Revision 1.4.0, in
> `ATC-AI-LLM-001-B` (Anmerkung zu B-18) und in `modules/atc-aurora-model` (Test auf Abweichung 0).
> **Fassung 0.2.0 (11.11.2026):** Belegstellen zu K5 ergänzt (Blueprint §6, Modellspezifikation §7.1.1). Damit ist die Herkunft von 5.771.776 erklärt. Neuer zweiter Befund: die Formel in §7.1.1 zählt die abschliessende Norm nicht mit.
> **Geltung:** `modules/atc-aurora-model`, Blueprint `ATC-AI-LLM-001-B` §3.5, Prüfung B-18 (Bezug T3.2)
> **Betroffene Artefakte:** Crate `atc-aurora-model`, Blueprint-Fassung, Mirror in `a-townchain-ecosystem`

## Worum es geht

B-18 fordert „exakt 5.771.776, Abweichung 0". Die Implementierung leitet die Zahl aus der
eingefrorenen Architektur ab (K4 Vokabular 4096, K5 `d_ff` 704, sechs Blöcke, geteilte
Einbettung, keine Bias-Terme) und kommt auf **5.868.800**. Beide Zahlen sind exakt
reproduzierbar, sie beschreiben aber nicht dasselbe Modell. Das ist keine Rundung und keine
Frage der Zählweise.

## Rechenweg

Alle Werte in Elementen, Breite `d_model` 256.

| Position | Rechnung | Elemente |
|---|---|---|
| Einbettung (geteilt) | 4096 × 256 | 1.048.576 |
| Attention je Block | (8×32) + 2×(8×32) + 256 = 1.024 → × 256 | 262.144 |
| Attention, 6 Blöcke | 6 × 262.144 | 1.572.864 |
| SwiGLU je Block | 2×704 + 704 = 2.112 → × 256 | 540.672 |
| SwiGLU, 6 Blöcke | 6 × 540.672 | 3.244.032 |
| Block-Normen | 6 × 2 × 256 | 3.072 |
| Finale Norm | 256 | 256 |
| **Summe** | | **5.868.800** |
| ohne finale Norm | Summe − 256 | 5.868.544 |

Ableitung im Code: `modules/atc-aurora-model/src/params.rs` (`breakdown`, `param_count`),
mit geprüfter Arithmetik. Festgehalten durch `b_18_parameterzahl_der_referenz_ist_abgeleitet`.

## Die Differenz, zerlegt

    Sollwert        5.771.776
    Ableitung       5.868.800
    Differenz          97.024   = 379 × 256
    ohne finale Norm   96.768   = 378 × 256 = 6 × 63 Zeilen

Die Differenz ist damit **genau die finale Norm (eine Zeile) plus 63 Zeilen je Block**.
63 Zeilen je Block sind im SwiGLU-Block `3 × 21` Zeilen, also `d_ff` 704 − 21 = **683**.

## Zwei exakte Auflösungen

1. **`d_ff` 683 und keine finale Norm** ergibt exakt 5.771.776. Und 683 ist genau
   `⌈8/3 × 256⌉` = 682,67 aufgerundet. Das sieht nach einer Rechnung aus, die vor der
   Einfrierung von K5 (`d_ff` 704) und vor der Aufnahme der finalen Norm entstanden ist.
2. **Vokabular 3.717 mit finaler Norm** ergibt ebenfalls exakt 5.771.776. 3717 = 3² × 7 × 59,
   also eine ungewöhnliche Vokabulargröße ohne Bezug zu einer Zweierpotenz.

Suchraum der Gegenprobe: `n_layers` 1–16, `d_ff` 256–4096 in Schritten von 8, Vokabular
1.000–8.000, übrige Werte eingefroren, zusätzlich systematisch ohne finale Norm. Das ist eine
Suche über einen benannten Raum, kein Beweis, dass keine andere Kombination existiert.

Für den Sollwert existiert im Repository **keine Herleitung**, nur die Tabellenzeile B-18.
Das ist ein Befund über die Herkunft, keine Aussage darüber, welcher Wert normativ gilt —
das entscheidet der Owner.

## Optionen

| Option | Inhalt | Konsequenz | Aufwand |
|---|---|---|---|
| **A** | Sollwert korrigieren: B-18 auf 5.868.544 (Formel nach §7.1.1) beziehungsweise 5.868.800 (mit abschliessender Norm, siehe F1) | Blueprint-Fassung +1, Test wird auf exakte Erwartung umgestellt, B-18 wird erfüllbar. Keine Architekturänderung, Crate unverändert | klein |
| **B** | Architektur ändern: K4/K5 neu einfrieren mit `d_ff` 683 oder Vokabular 3.717 | Abhängige Größen (Speicherbudget, Tokenizer-Kopplung, Messprotokoll T3.x), Mirror, alle Folgerechnungen neu | groß |
| **C** | Sollwert stehen lassen, Abweichung dauerhaft als `RESIDUAL` führen | B-18 bleibt dauerhaft nicht erfüllbar, T3.2 bleibt offen, der Widerspruch bleibt in jeder späteren Lesung sichtbar | klein, aber dauerhaft |

## Belegstellen: der Sollwert ist die Zahl vor K5

Die Herkunft von 5.771.776 ist im Dokumentenverbund belegt, es braucht keine Vermutung mehr:

- Blueprint `ATC-AI-LLM-001-B` §6 (Offene Punkte), Zeile K5: „`d_ff = 683` oder 704 |
  1,7 Prozent Parameter gegen SIMD-Ausrichtung". Der Unterschied zwischen 683 und 704 ist
  genau 96.768 Parameter, also 1,68 Prozent. **5.771.776 ist die Ableitung mit 683**, also
  der Stand vor der K5-Entscheidung.
- Modellspezifikation `ATC-AI-LLM-001-A` §7.1.1 („verbindliche Modellkonfiguration, einzige
  Quelle") nennt nach K5 **5.868.544** „aus der Formel, nicht gemessen", mit
  `P = L·(4d² + 3·d·d_ff + 2d) + V·d`.

Damit ist B-18 die einzige Stelle, die K5 nicht mitgezogen hat. Option A ist deshalb nicht
mehr eine Empfehlung unter drei, sondern die Nachführung einer bereits getroffenen
Entscheidung. B wäre die Rücknahme von K5 und bräuchte eine neue Entscheidung mit derselben
Begründung — 704 f32 sind exakt 44 Cache-Lines und 88 AVX2-Vektoren, 683 überquert jede
Cache-Line.

## Zweiter Befund: die Formel zählt die abschliessende Norm nicht mit

`P = L·(4d² + 3·d·d_ff + 2d) + V·d` ergibt 5.868.544. Ein Pre-Norm-Transformer braucht vor
dem Ausgabekopf eine abschliessende RMSNorm; ihre `d` Gewichte sind echte Parameter. Mit ihr
sind es **5.868.800**, und genau diesen Wert leitet das Crate ab. §7.1.1 nennt 5.868.544 als
„exakt", die Formel lässt also ein `+ d` vermissen.

Zwei Auflösungen, beide eine Owner-Entscheidung:

- **F1** Formel korrigieren auf `P = L·(4d² + 3·d·d_ff + 2d) + d + V·d` = 5.868.800.
  Die Architektur bleibt, wie das Crate sie rechnet; nur die Zahl in der Spezifikation
  wird genauer.
- **F2** Es gibt keine abschliessende Norm. Dann ist 5.868.544 richtig und das Crate muss
  die finale Norm streichen. Das ist eine Architekturentscheidung mit messbarem Effekt
  auf die Logitskalierung, keine Zählfrage.

**Empfehlung: A zusammen mit F1.** A ist die Nachführung von K5, F1 die Korrektur einer
vergessenen Zeile in der Formel. Grund für F1: eine Pre-Norm-Architektur ohne abschliessende
Norm vor dem Ausgabekopf ist unüblich, und die Block-Normen (`2d`) sind in derselben Formel
korrekt mitgezählt — es fehlt genau die eine Zeile, die ausserhalb der Blöcke liegt.

## Was bis zur Entscheidung gilt

- Die Implementierung folgt der Architektur, nicht dem Sollwert: **5.868.800**.
- Kein Test erzwingt 5.771.776. `b_18_blueprint_sollwert_weicht_von_der_ableitung_ab` hält
  beide Werte und die Differenz fest, damit sie nicht stillschweigend verschwindet.
- Ändert sich K4 oder K5, ändert sich der Test mit Begründung — nicht durch Anpassen der Zahl.
- Reifegrad unverändert: M-05 `IMPLEMENTED` und `TESTED`, `CI_VERIFIED` offen.
- Zwei Entscheidungen sind offen und werden hier nicht vorweggenommen: **A** (B-18 auf
  den K5-Wert nachziehen) und **F1/F2** (zählt die abschliessende Norm mit).

## Prüfbarkeit

- `cargo test` in `modules/atc-aurora-model` (8 Tests, darunter die vier B-18-Zählprüfungen).
- Rechenweg unabhängig nachrechenbar: Tabelle oben, oder `param_count` mit
  `Config { d_ff: 683, ..Config::ref1() }` abzüglich `Breakdown::final_norm`.

## Nicht Teil dieser Entscheidung

Attention, Kausalmaske, RoPE, Gewichte, Initialisierung, Dateiformat und Training. Diese Teile
gehören zu M-05 (Rest), M-06 und zur Artefaktschicht; sie sind hier weder angefangen noch
vorgetäuscht.

## Entscheidung vom 11.11.2026

Der Owner hat **A + F1** bestätigt. Damit ist der Widerspruch geschlossen, und zwar
ohne Architekturänderung:

| Festlegung | Wert |
|---|---|
| Sollwert B-18 | **5 868 800** |
| K5 `d_ff` | 704, unverändert |
| Abschliessende RMSNorm | bleibt Bestandteil der Architektur |
| Parameterformel | `P = L·(4d² + 3·d·d_ff + 2d) + V·d + d` |
| 5 771 776 | historischer Stand vor K5, keine gültige Anforderung |
| 5 868 544 | Zwischenrechnung nach K5 ohne abschliessende Norm |

Umgesetzt an vier Stellen, damit Crate und Spezifikation dieselbe Rechengrundlage
haben: `ATC-AI-LLM-001-A` Revision 1.4.0 (Formel, Sollwert, Herkunftstabelle),
`ATC-AI-LLM-001-B` (B-18-Sollwert und Anmerkung), dieses Dokument (Status), und
`modules/atc-aurora-model/src/tests.rs` (B-18 prüft Abweichung 0; ein zweiter Test
hält 97 024 = 96 768 + 256 als Herkunftsnachweis fest).

Nicht entschieden und nicht Teil dieser Entscheidung: `ROPE_BASE`, das Verhalten
bei späteren Architekturänderungen (Upgrade-Regeln des Storage- und
Artefaktformats) und die Frage der Authentizität von Artefakten. Das
Artefaktformat selbst wird in `ATC-AI-LLM-002` spezifiziert.
