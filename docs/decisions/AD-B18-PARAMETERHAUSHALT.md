---
document_id: ATC-DOC-AI-AD-001
title: B-18 Parameterzahl Ref-1 — Sollwert und Ableitung
version: 0.1.0
status: proposed
owner: A-TownChain-Okosystems
created: 2026-11-11
standard: atc-standards
---

# AD-B18-PARAMETERHAUSHALT — Parameterzahl von Ref-1: Sollwert gegen Ableitung

> **Status:** OFFEN — Owner-Entscheidung ausstehend. Diese Vorlage entscheidet nichts; sie legt den Widerspruch, den Rechenweg und die Optionen vor.
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
| **A** | Sollwert korrigieren: B-18 auf 5.868.800 (mit finaler Norm), Nebenzeile 5.868.544 | Blueprint-Fassung +1, Test wird auf exakte Erwartung umgestellt, B-18 wird erfüllbar. Keine Architekturänderung, Crate unverändert | klein |
| **B** | Architektur ändern: K4/K5 neu einfrieren mit `d_ff` 683 oder Vokabular 3.717 | Abhängige Größen (Speicherbudget, Tokenizer-Kopplung, Messprotokoll T3.x), Mirror, alle Folgerechnungen neu | groß |
| **C** | Sollwert stehen lassen, Abweichung dauerhaft als `RESIDUAL` führen | B-18 bleibt dauerhaft nicht erfüllbar, T3.2 bleibt offen, der Widerspruch bleibt in jeder späteren Lesung sichtbar | klein, aber dauerhaft |

**Empfehlung: A**, solange keine Originalrechnung zum Sollwert beigebracht wird. Begründung:
die Ableitung ist aus den eingefrorenen K-Werten reproduzierbar und im Test festgehalten; für
5.771.776 gibt es im Repository keine Herleitung. A ändert keine Architektur, sondern nur den
Sollwert — und macht B-18 erst dadurch zu einer prüfbaren Anforderung.

## Was bis zur Entscheidung gilt

- Die Implementierung folgt der Architektur, nicht dem Sollwert: **5.868.800**.
- Kein Test erzwingt 5.771.776. `b_18_blueprint_sollwert_weicht_von_der_ableitung_ab` hält
  beide Werte und die Differenz fest, damit sie nicht stillschweigend verschwindet.
- Ändert sich K4 oder K5, ändert sich der Test mit Begründung — nicht durch Anpassen der Zahl.
- Reifegrad unverändert: M-05 `IMPLEMENTED` und `TESTED`, `CI_VERIFIED` offen.

## Prüfbarkeit

- `cargo test` in `modules/atc-aurora-model` (8 Tests, darunter die vier B-18-Zählprüfungen).
- Rechenweg unabhängig nachrechenbar: Tabelle oben, oder `param_count` mit
  `Config { d_ff: 683, ..Config::ref1() }` abzüglich `Breakdown::final_norm`.

## Nicht Teil dieser Entscheidung

Attention, Kausalmaske, RoPE, Gewichte, Initialisierung, Dateiformat und Training. Diese Teile
gehören zu M-05 (Rest), M-06 und zur Artefaktschicht; sie sind hier weder angefangen noch
vorgetäuscht.
