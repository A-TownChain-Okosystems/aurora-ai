# atc-aurora-model

M-05 `model` der Aurora-Referenzimplementierung (Blueprint ATC-AI-LLM-001-B,
Abschnitt 3.5).

## Stand

Dieser Schnitt enthaelt Konfiguration und Parameterhaushalt: welche Tensoren das
Referenzmodell besitzt, wie viele Elemente sie zusammen haben und welche
Konfigurationen unzulaessig sind. Es gibt **keinen** Vorwaertslauf, keine
Attention, keine Gewichte und keine Zuteilung. Wer hier einen Zustand erwartet,
findet keinen.

Die Zahl wird abgeleitet, nicht gesetzt:

```
Embedding        4096 x 256            1 048 576
Attention        6 x 1024 x 256        1 572 864
SwiGLU           6 x 2112 x 256        3 244 032
Block-Normen     6 x 2 x 256               3 072
Finale Norm            256                   256
                                      ----------
gesamt                                 5 868 800
```

## Entscheidung zu B-18

Der Owner hat am 11.11.2026 entschieden: die aus K4/K5 abgeleitete Architektur ist
massgeblich, der Sollwert **5 771 776** wird nicht als Implementierungsvorgabe
uebernommen. Dokument: `docs/decisions/M-05-B18-PARAMETERHAUSHALT.md`.

Die Ableitung ergibt **5 868 800** einschliesslich der abschliessenden Normierung
und **5 868 544** ohne sie. Differenz zum Sollwert: **97 024** beziehungsweise
**96 768** Elemente. Die Normierung erklaert 256 Parameter, die restlichen 96 768
nicht.

B-18 gilt damit als `INCONSISTENT / REQUIRES RECONCILIATION`, ausdruecklich nicht
als veraltet: die urspruengliche Herleitung (Formel, Architekturversion,
Referenz-Commit) ist nicht nachgewiesen. Der Test
`b_18_blueprint_sollwert_weicht_von_der_ableitung_ab` haelt die Abweichung
reproduzierbar fest. Die Implementierung folgt der Architektur, nicht dem
Sollwert; ein Test auf 5 771 776 waere entweder falsch oder wuerde eine unbekannte
Architekturaenderung verdecken.

## Grenze der Aussage

Getestet ist der Parameterhaushalt, nicht das Modell. Nicht behauptet und nicht
vorhanden: Modellinferenz, Attention-Berechnung, Gewichtsspeicherung, Training,
Modellartefakt-Erzeugung. Die naechste Verifikationsstufe folgt erst nach
Aufloesung der Referenzabweichung.
