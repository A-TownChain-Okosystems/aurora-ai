# Status atc-aurora-model (M-05)

| Bereich | Status | Beleg |
|---|---|---|
| Parameterhaushalt | `IMPLEMENTED` | `Config`, `Breakdown`, `param_count`, gepruefte Arithmetik |
| Lokale Pruefung | `TESTED` | 8 Tests, `cargo test` gruen, clippy `-D warnings` sauber, fmt sauber |
| Vorwaertslauf (B-19, B-20) | **offen** | braucht Attention, RoPE, Einbettung, SwiGLU-Verdrahtung |
| Artefakt (B-21) | **offen** | braucht Format, Digest und Zustandsschreiben |
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
Ableitung exakt 5 771 776 (mit dem Crate nachgerechnet). Entscheidungsvorlage mit
Optionen A/B/C: [`docs/decisions/AD-B18-PARAMETERHAUSHALT.md`](../../docs/decisions/AD-B18-PARAMETERHAUSHALT.md).

## Nicht enthalten

Attention, Kausalmaske, RoPE, Gewichte, Initialisierung, Dateiformat, Training.
Diese Teile gehoeren zu M-05 (Rest), M-06 und zur Artefaktschicht; sie sind hier
weder angefangen noch vorgetaeuscht.
