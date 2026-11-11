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

## Befund zu B-18

Das Blueprint nennt fuer B-18 den Sollwert **5 771 776**. Die Ableitung aus der
eingefrorenen Konfiguration (K4 Vokabular 4096, K5 `d_ff` 704, sechs Bloecke,
geteilte Einbettung, keine Bias-Terme) ergibt **5 868 800**, ohne die
abschliessende Norm **5 868 544**.

Differenz: **97 024** beziehungsweise **96 768** Elemente. Das ist keine
Rundung und keine Frage der Zaehlweise, sondern eine offene Entscheidung: der
Sollwert und die Architektur passen nicht zueinander. Der Test
`b_18_blueprint_sollwert_weicht_von_der_ableitung_ab` haelt die Abweichung fest,
damit sie nicht stillschweigend verschwindet.

Bis der Owner entscheidet, gilt: die Implementierung folgt der Architektur, nicht
dem Sollwert. Ein Test, der 5 771 776 erzwingen wuerde, waere entweder falsch
oder wuerde eine unbekannte Architekturaenderung verstecken.
