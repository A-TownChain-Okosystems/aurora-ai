# Autograd-Contract-Abgleich (M-03)

**Rang:** abgeleitetes Arbeitspapier, keine neue Autorität. Es legt nichts fest,
was nicht schon in den Contracts steht, und es ersetzt keinen Testfall. Wo die
Contracts schweigen, steht hier eine **Festlegung** mit Begründung, die der Owner
widersprechen kann.

**Quellen:** `ATC-AI-LLM-001` v1.3.0 (P2-Abschnitt, Abschnitte 7.1 bis 7.3, 8),
`ATC-AI-LLM-001-A` (Abschnitt 5.1 Konfiguration, 7.1 bis 7.3 Toleranzen),
`ATC-AI-LLM-001-B` v1.1.0 (Testfälle B-08 bis B-13, Kapitel 4).

**Stand:** 11.11.2026, gegen `atc-aurora-tensor` nach M-01 und M-02.

---

## 1. Die drei Fragen

Der Abgleich beantwortet vor jeder Zeile Code drei Fragen:

1. Welche Tensoroperationen brauchen **tatsächlich** einen Gradienten?
2. Welche Ableitungen sind **mathematisch verbindlich**, und gegen welche
   Referenz werden sie geprüft?
3. Welche **Speicherlebensdauer** verlangt der Graph, und wer gibt was zurück?

---

## 2. Frage 1: Welche Operationen bekommen eine Ableitung

Der Contract nennt die Gradientenpflicht in einem Absatz (P2, „Vorwärts und
rückwärts für: …") und die API-Fläche in Abschnitt 8. Beide Mengen sind **nicht
deckungsgleich**. Diese Tabelle ist die Auflösung, Operation für Operation.

| Operation | Rang | Backward | Begründung | Prüfung |
|---|---|---|---|---|
| `add` | beliebig | ja | Grundoperation jeder Restverbindung | B-13 (implizit), Raute-Test |
| `mul` | beliebig | ja | Kettenregel, Skalierung, Masken | B-13 |
| `scale` | beliebig | ja | Ableitung ist der Faktor selbst | B-12 |
| `matmul` | 2 | ja | Kern der Schichten | **B-09** |
| `transpose` | 2 | ja | Ableitung ist die Transponierung | Determinismus-Test |
| `softmax` | 1, 2 | ja | Jacobi-Vektor-Produkt `s ∘ (g − ⟨g,s⟩)` | **B-10** |
| `sum` | beliebig | ja | skalarer Verlust, damit „Skalarprodukt" abbildbar ist | B-09, B-12 |
| `rms_norm` | 1, 2 | ja | kanonische Norm der Ref-1-Konfiguration | **B-11**, B-13 |
| `swiglu` | 2 | ja | kanonische Aktivierung der Ref-1-Konfiguration | B-13 |
| `cross_entropy` | 2 | ja | Verlust der Referenz, Gradient gefaltet | **B-08**, B-10 |
| `relu`, `sigmoid` | beliebig | **nein** | nicht in der Gradientenliste des P2-Absatzes; die Konfiguration nutzt SwiGLU. Forward bleibt, weil M-02 sie führt | – |
| `dot`, `mean` | beliebig | **nein** | keine Graphenknoten: `dot(a,b)` ist `sum(mul(a,b))`, `mean` ist `scale(sum, 1/n)`. Eine eigene Ableitung wäre eine zweite Wahrheit | – |

**„Skalarprodukt" ist damit abgedeckt, ohne neue Operation:** der P2-Absatz
verlangt Vorwärts und Rückwärts für das Skalarprodukt. Es gibt keinen eigenen
Knoten dafür, sondern `sum(mul(a,b))`; die Ableitung entsteht aus den beiden
vorhandenen Operatoren.

### 2.1 Befund: vier Operationen stehen im Umfang, aber nirgends sonst

Der P2-Absatz nennt außerdem **LayerNorm, GELU, Dropout und Einbettungs-Lookup**.
Keine dieser vier steht in der API-Fläche (Abschnitt 8), keine wird von einem
Testfall B-01 bis B-24 geprüft, keine kommt in der Ref-1-Konfiguration vor
(Abschnitt 5.1: RMSNorm, SwiGLU, Dropout 0,0, gebundenes Embedding). Sie sind
**nicht implementiert**, und zwar ausdrücklich, nicht stillschweigend.

| Operation | Status | Begründung | offen als |
|---|---|---|---|
| LayerNorm | nicht implementiert | Konfiguration nutzt RMSNorm; zwei Normen wären zwei Semantiken | **K6** |
| GELU | nicht implementiert | Konfiguration nutzt SwiGLU; keine Ableitung ohne Testfall | **K6** |
| Dropout | nicht implementiert | Referenzwert 0,0, also Identität; ein Gradient für die Identität wäre eine leere Zusage | **K6** |
| Embedding-Lookup | nicht implementiert | braucht die Modellschicht (M-05, P3); ohne sie kein sinnvoller Test | **K6** |

Das ist eine **offene Entscheidung für den Owner**, keine erledigte Lücke: entweder
werden die vier in P2 nachgezogen (dann brauchen sie Testfälle, denn ohne
Finitedifferenz ist eine Ableitung eine Behauptung), oder der P2-Absatz wird auf
die API-Fläche eingeengt.

### 2.2 Regel gegen Wildwuchs

Nur die zehn Operationen aus Abschnitt 2 haben eine Ableitung. Es gibt **keinen
generischen Rückfall** und keine Registry: eine Operation ohne `Backward`-Zweig
lässt sich nicht in den Graphen aufnehmen, weil jeder Knoten eine `Op`-Variante
braucht. Eine unbekannte Operation ist damit ein Übersetzungsfehler und kein
Laufzeitverhalten.

---

## 3. Frage 2: Welche Ableitungen sind verbindlich

Notation: `g` ist der ankommende Gradient, `⊤` die Transponierung, `n` die
Zeilenzahl, `d` die Zeilenbreite, `s = softmax(z)`.

| Operation | Vorwärts | Gradient | Quelle |
|---|---|---|---|
| `add` | `c = a + b` | `ga = g`, `gb = g` | Definition |
| `mul` | `c = a ∘ b` | `ga = g ∘ b`, `gb = g ∘ a` | Produktregel |
| `scale` | `c = α·a` | `ga = α·g` | linear |
| `matmul` | `C = A·B`, `(m,k)·(k,n)` | `gA = g·Bᵀ`, `gB = Aᵀ·g` | B-09, T2.2 |
| `transpose` | `C = Aᵀ` | `gA = gᵀ` | B-06, T2.2 |
| `softmax` | `s = softmax(z)` zeilenweise | `gz = s ∘ (g − ⟨g,s⟩)` | B-10, T2.3 |
| `sum` | `c = Σ a` | `ga[i] = g` | linear |
| `rms_norm` | `y_i = x_i · w_i · r⁻¹`, `r = √(mean(x²)+ε)` | `gx_i = g_i w_i r⁻¹ − x_i (d r³)⁻¹ Σ_k g_k w_k x_k`, `gw_i = Σ_zeilen g_i x_i r⁻¹` | B-11, T2.4 |
| `swiglu` | `y = silu(a) ∘ b`, `a = x[:h]`, `b = x[h:]` | `ga = g ∘ b ∘ silu'(a)`, `gb = g ∘ silu(a)`, `silu'(t) = σ(t)(1 + t(1−σ(t)))` | B-13 |
| `cross_entropy` | `L = Σ_zeilen (logsumexp(z) − z_t)` | `gz = s − onehot(t)` **ohne** `1/n` | B-08, B-10 |

**Zwei Punkte, die der Contract entscheidet und die man leicht falsch macht:**

1. **Softmax braucht den Ausgang, nicht den Eingang.** `∂s/∂z` ist über `s`
   ausgedrückt. Wer den Eingang einsetzt, rechnet plausibel und falsch. Genau
   diese Verwechslung ist als Mutation geprüft und wird von
   `b_10_softmax_knoten_hat_eigene_ableitung` erkannt.
2. **Kreuzentropie mit Summe, nicht mit Mittelwert.** B-08 verlangt wörtlich
   „Gradient exakt `softmax − target`". Bei einem Mittelwert käme `1/n` hinzu und
   die Bedingung wäre nur für eine Zeile erfüllt. Die Mittelung über einen Stapel
   gehört in die Trainingsschleife (M-06). Auch diese Abweichung ist als Mutation
   geprüft (`b_08_…` schlägt bei `1/n` fehl).

### 3.1 Referenz und Toleranzen

Referenz ist immer die **zentrale Finitedifferenz in f64** mit `h = 1e-5`
(Abschnitt 7.1 des Anhangs), gebildet über eine eigene f64-Rechnung im Test, nicht
über den Vorwärtspfad des Crates. Eine Referenz aus dem eigenen Code wäre eine
Selbstbestätigung.

| Prüfung | Toleranz laut Contract | gemessen (11.11.2026) |
|---|---|---|
| B-09 Matmul, je Element | 1e-6 relativ | 4,6e-8 (dA), 4,1e-8 (dB) |
| B-10 Kette Softmax/CE | 1e-6 relativ | 1,3e-7 |
| B-10 Softmax-JVP | 1e-6 relativ | 1,8e-7 |
| B-11 RMSNorm, `ε = 1e-8`, normnahe Zeilen | 1e-5 relativ | 1,6e-6 (dx), 2,4e-7 (dw) |
| B-13 Netz, alle Parameter | 1e-4 in f64, 1e-3 im f32-Pfad | 4,6e-8 (w0), 5,0e-6 (W1), 4,3e-7 (W2) |

Anmerkung zu B-13: der Test prüft den **f32-Pfad gegen die f64-Referenz** und
fordert deshalb 1e-3. Der gemessene Worst Case liegt bei 5,0e-6 und damit auch
unter der strengeren 1e-4-Grenze, die der Contract für einen f64-Pfad nennt. Ein
f64-Pfad im Crate existiert nicht (f32 ist der verbindliche Datentyp), deshalb
wird die strengere Zahl nicht als Bedingung geführt, sondern als Messwert.

---

## 4. Frage 3: Speicherlebensdauer des Graphen

### 4.1 Besitzverhältnisse

Der Graph **besitzt** alle Vorwärtswerte, die er selbst berechnet hat. Ein Knoten
verweist auf seine Eingaben über einen **Index**, nie über eine Referenz. Damit
gibt es keine Lebensdauerkopplung an den Aufrufer und keinen Zeiger, der nach
einem `clear` ins Leere zeigt.

| Objekt | Eigentümer | Lebensdauer | im Budget verbucht |
|---|---|---|---|
| Blattwert (Parameter, Konstante) | Aufrufer, an den Graphen übergeben | bis `clear` (der Graph hält ihn) | nein, kam vom Aufrufer |
| Wert eines berechneten Knotens | Graph | bis `clear` | ja |
| Seed des Rückwärtspasses | Aufrufer, wird kopiert | Dauer des Aufrufs | ja (die Kopie) |
| Gradient eines Zwischenknotens | Rückwärtspass | bis er verteilt ist, dann frei | ja |
| Gradient eines Parameters | Graph | bis `zero_grads` oder `clear` | ja |
| Gradient der Eingabe (Blatt ohne Gradientenpflicht) | niemand | wird verworfen | ja, dann freigegeben |

**Kein Checkpointing in P2.** Der Graph hält alle Werte, die der Rückwärtspass
liest. Für Ref-1 sind das nach Abschnitt 4.2 des Anhangs 0,30 GB Aktivierungen;
Wiederberechnung (`c ≈ 8 Byte`) ist eine P4-Optimierung und wird hier nicht
vorbereitet.

### 4.2 Zyklusfreiheit als Konstruktionsregel

Jede Operation darf nur Knoten mit **kleinerem Index** als Eingabe nehmen. Daraus
folgt: der Graph ist immer gerichtet und zyklenfrei, die Indexreihenfolge ist eine
gültige topologische Ordnung, und der Rückwärtspass ist eine Schleife über
absteigende Indizes ohne Sortierschritt. Es gibt keinen Weg, einen Zyklus zu
bauen, auch nicht über zwei Aufrufe.

### 4.3 Budgetvertrag

1. **Prüfung vor Zuteilung** gilt im Vorwärts- wie im Rückwärtspfad. Jeder Puffer
   entsteht über `Tensor::zeros(shape, budget)`, also über `Budget::reserve`.
2. **Der Rückwärtspass ist transaktional.** Schlägt er in der Mitte fehl, wird
   jede in diesem Aufruf reservierte Byte zurückgegeben und kein Gradient
   übernommen. Geprüft durch `rueckwaertspass_ist_transaktional`: bei Budget 0
   kommt `BudgetExceeded { requested: 4, limit: 0 }`, `used() == 0` und
   `grad(...) == None`.
3. **Nichts bleibt unbemerkt belegt.** `clear` gibt Werte und Gradienten zurück,
   `zero_grads` nur die Gradienten. Geprüft durch `graph_gibt_alles_zurueck`
   (Buchung vorher/nachher gleich) und `held_bytes()`.
4. **Beitrag, der in einen bestehenden Gradienten addiert wird, wird sofort
   freigegeben.** Wer das vergisst, hat ein Budgetleck ohne Funktionsfehler; genau
   dieser Fehler ist beim Bau aufgetreten und wird seitdem durch
   `graph_gibt_alles_zurueck` gefangen.

### 4.4 Akkumulation (B-12)

Gradienten werden **addiert**, nie überschrieben. Zwei Fälle sind zu
unterscheiden:

- **Mehrere Pfade im selben Pass** (Raute): Beiträge laufen in denselben
  Knotenpuffer und summieren sich. Geprüft durch
  `b_12_raute_zwei_pfade_auf_dasselbe_blatt` gegen die Finitedifferenz.
- **Mehrere Rückwärtspässe auf dasselbe Blatt:** Parameter behalten ihren
  Gradienten, jeder weitere Pass addiert darauf. Geprüft durch
  `b_12_zwei_rueckwaertspaesse_summe_statt_ueberschreiben` (zweiter Pass ergibt
  exakt das Doppelte, `zero_grads` setzt zurück).

### 4.5 Determinismus

- Die Verteilung läuft in absteigender Indexreihenfolge, die Addition in der
  Reihenfolge der Elterneinträge. Beides ist durch die Konstruktion festgelegt,
  nicht durch eine Hashmap.
- Zwei Läufe desselben Graphen liefern **bitgleiche** Gradienten; geprüft durch
  `rueckwaertspass_ist_deterministisch`.
- Der Seed ist der Gradient am Ausgang: ohne Angabe `1,0` und der Ausgang muss ein
  Skalar sein, sonst `ShapeMismatch`. Ein nicht skalares Ziel braucht einen Seed
  mit passender Form.

---

## 5. Festlegungen, wo der Contract schweigt

| Nr. | Frage | Festlegung | Warum | Widerspruch möglich |
|---|---|---|---|---|
| F1 | Aufteilung bei `swiglu` | letzte Achse gerade, `h = Breite/2`, `y = silu(x[:h]) · x[h:]` | die Konfiguration nennt „drei Matrizen, gated", aber keine Halbierung; diese Variante ist die einzige, die mit einer Matrix ohne Zusatzop auskommt | ja, dann ändert sich Vorwärts und Rückwärts gemeinsam |
| F2 | Reduktion der Kreuzentropie | Summe über Zeilen | B-08 verlangt `softmax − target` ohne Faktor | nein, das ist die Contract-Lesart |
| F3 | `eps <= 0` bei RMSNorm | `InvalidArgument`, keine Rechnung | `1/√0` wäre `inf` und würde als Zahlenwert durchlaufen (B-11 verbietet genau das) | nein |
| F4 | Wer Gradienten behält | nur Blätter mit Gradientenpflicht (Parameter) | Zwischengradienten nach dem Pass zu halten wäre Speicher ohne Nutzen; B-12 braucht nur Parameter | ja |
| F5 | `relu`, `sigmoid` ohne Backward | Forward bleibt, Backward fehlt | nicht in der P2-Gradientenliste, in der Konfiguration ungenutzt | ja, dann mit Testfall |

---

## 6. Nicht gebaut (und warum)

| Nicht gebaut | Grund |
|---|---|
| Dynamischer Dispatch, Hooks, Callbacks | keine zweite Steuerungsebene; der Graph ist eine `enum`-Liste |
| In-place-Mutation von Werten | würde gespeicherte Aktivierungen zerstören, auf denen der Rückwärtspass rechnet |
| Gradient-Checkpointing / Wiederberechnung | P4-Optimierung (Anhang Abschnitt 4.2) |
| Zweiter Scheduler, zweites Register | SSOT-004, und M-06 nutzt den bestehenden |
| Autograd im Kernel | M-03 gehört zu L2; der Kernel enthält seit dem 11.11.2026 keinen KI-Code |

---

## 7. Offene Punkte

| ID | Punkt | Wirkung |
|---|---|---|
| **K6** | LayerNorm, GELU, Dropout, Embedding-Lookup: im P2-Absatz genannt, in API-Fläche und Testfällen nicht vorhanden | Entscheidung des Owners: nachziehen mit Testfällen oder P2-Absatz auf die API-Fläche einengen |
| – | f64-Pfad im Crate | B-13 nennt 1e-4 „in f64"; das Crate führt f32 als einzigen Puffer. Die Zahl ist als Messwert erreicht (5,0e-6), nicht als Bedingung geprüft |
| – | T2.6 Durchsatz | `BLOCKED_EXTERNAL` bis zur qualifizierten Maschine (K3), unabhängig von M-03 |

---

*Dieses Dokument enthält keinen Produktivcode. Es ordnet die Gradientenpflicht,
die Ableitungen und die Speicherlebensdauer, bevor M-03 entstand, und hält fest,
wo der Contract schweigt und wer dort entschieden hat.*
