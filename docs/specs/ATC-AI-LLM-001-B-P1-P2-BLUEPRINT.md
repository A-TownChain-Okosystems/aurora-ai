---
document_id: ATC-AI-LLM-001-B
title: P1/P2 Blueprint: sechs Module, sechsundzwanzig Anforderungen, 24 Testfälle
version: 1.1.2
status: proposed
owner: A-TownChain-Okosystems
created: 2026-11-11
updated: 2026-11-11
parent: ATC-AI-LLM-001 v1.3.0, Anhang ATC-AI-LLM-001-A, Messprotokoll ATC-AI-LLM-001-C
layer: L2 (aurora-ai)
security_class: S3
evidence_gate: AURORA-001
---

# P1/P2 Blueprint: Ref-1 in sechs Modulen

Umsetzungsplan für die Phasen P1 (Tokenizer) und P2 (Tensor-Engine und Autograd)
aus `ATC-AI-LLM-001`. Der Anhang `ATC-AI-LLM-001-A` legt die Grenzen fest; dieses
Dokument legt die Bausteine, ihre Zuordnung zu bestehenden Aurora-Komponenten und
die 24 verbindlichen Testfälle fest.

**Status: SPECIFIED. 0 Zeilen Produktivcode.** Grundlage ist der Beschluss
AD-052 vom 11.11.2026 (nativer, frameworkfreier Produktivpfad). Damit ist K1
geschlossen und P2 freigegeben.

---

## 1. Sechs Module

Die Zerlegung folgt der Datenrichtung, nicht der Ordnerstruktur. Jedes Modul hat
eine abgeschlossene Verantwortung und eine eigene Austrittsbedingung.

| Modul | Verantwortung | Crate | Phase |
|---|---|---|---|
| M-01 `tensor-core` | Datentyp, Form, Speicher, Budget, Elementzugriff | `atc-aurora-tensor` | P2 |
| M-02 `tensor-ops` | Matmul, Transponierung, Softmax, Norm, Aktivierung, Verlust | `atc-aurora-tensor` | P2 |
| M-03 `autograd` | Rechengraph, Rückwärtspass, Gradientenakkumulation | `atc-aurora-tensor` | P2 |
| M-04 `tokenizer` | Byte-Level-BPE, Vokabular, Kodierung, Digest | `atc-aurora-tokenizer` | P1 |
| M-05 `model` | Transformer, Konfiguration, Artefaktformat `.atcm` | `atc-aurora-model` | P3 |
| M-06 `train` | AdamW, Trainingsschleife, Checkpoint, Datenzufuhr | `atc-aurora-train` | P4 |

Abhängigkeiten, zyklusfrei: `M-01 → M-02 → M-03`, `M-03 → M-05`, `M-05 → M-06`,
`M-04` unabhängig. Kein Modul kennt ein Chain-Crate.

### 1.1 Reihenfolge innerhalb von P2

1. M-01 mit B-01 bis B-04.
2. M-02 mit B-05 bis B-08.
3. M-03 mit B-09 bis B-13.

Der Grund für diese Reihenfolge: Ein Autograd ohne geprüfte Vorwärtsoperationen
prüft den Fehler der Vorwärtsoperationen mit.

**Anmerkung zur Zuordnung von B-08 (11.11.2026, ohne Änderung des Testfalls):**
B-08 ist hier bei M-02 geführt, geprüft wird aber der Gradient der Kreuzentropie.
Der Testfall wurde deshalb zusammen mit M-03 abgeschlossen, also nach der
Gradientenseite. Die Erwartung ist unverändert: Gradient exakt `softmax − target`,
bei Zielwahrscheinlichkeit 0 kein `NaN` und kein `Inf`. Der Gradient ist die
Summe über die Zeilen, nicht ihr Mittelwert; nur so gilt die Bedingung für
beliebige Stapelgrößen, und die Mittelung bleibt Sache der Trainingsschleife.

---

## 2. Abgleich gegen die bestehenden Aurora-Komponenten

Der Abgleich beantwortet die Frage, die eine zweite Architektur verhindert: was
wird neu gebaut, was wird erweitert, was bleibt unberührt.

| Bestehende Komponente | Verhältnis zu den sechs Modulen | Begründung |
|---|---|---|
| `atc-aurora-core` | **unberührt**, kein Import eines neuen Moduls | Orchestrierung und Autorisierung bleiben getrennt von Modellmathematik (SSOT-004) |
| `atc-aurora-core::ModelHub` | **wird Konsument**, nicht Besitzer | Der Platzhalter wird nach Aufgabe #53 ein Provider hinter einem versionierten Vertrag; das native Modell ist ein Provider, kein zweites Register |
| `atc-aurora-core::LlmRouter` | **unberührt** | Routet weiterhin auf Modellnamen; die Namensauflösung auf ein Artefakt kommt mit M-05 |
| `atc-aurora-core::task_scheduler` | **unberührt**, wird nicht nachgebaut | Ein zweiter Scheduler wäre ein Verstoß gegen SSOT-004; M-06 nutzt den bestehenden |
| `atc-aurora-core::capability_policy`, `approval`, `audit` | **unberührt** | Training und Inferenz lösen keine privilegierte Aktion aus |
| `atc-aurora-memory::vector_store` | **unberührt**, keine Abhängigkeit | Speichert Einbettungen als `Vec<f32>`; Einbettungen erzeugt künftig M-05, der Speicher bleibt unverändert |
| `atc-aurora-memory::context_window` | **unberührt** | Kontextverwaltung der Anwendung, nicht des Modells |
| `atc-aurora-agents` | **unberührt** | Agenten rufen später über ModelHub, nie über Tensoren |
| `atc-aurora-localization` | **unberührt** | Kein Bezug zu Tokenisierung; eine Sprachumschaltung im Tokenizer wäre eine zweite Wahrheit |
| `sdk` | **unberührt**, später erweitert | Modell-ABI-Bindung erst mit P10 |
| `atc-aurora-runtime` | **bleibt Platzhalter**, Ziel von P8 | Inferenz, KV-Cache und Sampler gehören dorthin, nicht in `atc-aurora-model` |
| `modules/atc-aistudio` | **unberührt** | Frontend, enthält keine Modellmathematik; die dort liegende `federated_learning.atc` ist eine abweichende Alternativquelle und nicht Modellcode |

Ergebnis: **kein bestehendes Modul wird umgebaut.** Vier neue Crates entstehen
daneben, drei davon in P2 und P1. Es entsteht keine zweite Scheduler-, Policy-,
Register- oder Speicherarchitektur.

---

## 3. Vierundzwanzig verbindliche Testfälle

Jeder Testfall hat eine Austrittsbedingung, die ohne Ermessen entscheidbar ist.
Die Spalte Bezug nennt die Nummer aus der Testmatrix des Anhangs A.

### 3.1 M-01 `tensor-core`

| ID | Prüfung | Erwartung | Bezug |
|---|---|---|---|
| B-01 | Matmul mit unpassenden Formen, `(2,3) x (2,3)` | `ShapeMismatch`, kein Panik, kein stilles Auffüllen | T2.1 |
| B-02 | Addition nicht kompatibler Formen, `(3,1) + (1,4)` | `ShapeMismatch`; kein implizites Broadcasting | T2.1 |
| B-03 | Allokation über dem Budget | `BudgetExceeded` vor der Zuteilung, nicht danach | T2.5 |
| B-04 | Konstruktion aus Bytes, Little Endian, f32 | Länge exakt `4·n`; Länge 0 zulässig; falsche Länge `ShapeMismatch` | T2.1 |

### 3.2 M-02 `tensor-ops`

| ID | Prüfung | Erwartung | Bezug |
|---|---|---|---|
| B-05 | Matmul gegen ein von Hand gerechnetes 2x2-Beispiel | exakt in f64, in f32 innerhalb 1e-6 relativ | T2.2 |
| B-06 | doppelte Transponierung und Reihenfolgeidentität | `(Aᵀ)ᵀ == A`, `(AB)ᵀ == BᵀAᵀ` innerhalb Toleranz | T2.2 |
| B-07 | Softmax mit Logits ±1e5 | alle Werte endlich, Zeilensumme 1,0 ± 1e-6 | T2.3 |
| B-08 | Kreuzentropie | Gradient exakt `softmax − target`; bei Zielwahrscheinlichkeit 0 entsteht kein `NaN` und kein `Inf` | T2.3 |

### 3.3 M-03 `autograd`

| ID | Prüfung | Erwartung | Bezug |
|---|---|---|---|
| B-09 | Gradient der Matmul gegen zentrale Finitedifferenz in f64, `h = 1e-5` | relative Abweichung unter 1e-6 | T2.4 |
| B-10 | Kette Softmax und Kreuzentropie | relative Abweichung unter 1e-6 | T2.4 |
| B-11 | RMSNorm mit `ε = 1e-8` und Zeilen normnah 0 | endlicher Gradient, relative Abweichung unter 1e-5, kein Division-durch-Null | T2.4 |
| B-12 | zwei Rückwärtspässe auf dasselbe Blatt | Gradienten summieren sich; kein Überschreiben | T2.4 |
| B-13 | zusammengesetztes Netz, zwei Schichten, alle Parameter | relative Abweichung unter 1e-4 in f64, unter 1e-3 für den f32-Pfad | T2.4 |

### 3.4 M-04 `tokenizer`

| ID | Prüfung | Erwartung | Bezug |
|---|---|---|---|
| B-14 | Rundlauf über 10.000 Zufallsfolgen, inklusive ungültigem UTF-8 | `decode(encode(b)) == b` für jede Folge | T1.1 |
| B-15 | zwei Trainingsläufe auf identischer Quelle | identisches Vokabular und identischer Digest | T1.2 |
| B-16 | Grenzfälle: leer, ein Byte, Emoji, 4-Byte-Sequenz, 1 MB Zufall | kein Panik, keine stille Ersetzung durch ein Ersatzzeichen | T1.3 |
| B-17 | Laden mit manipulierter Merge-Tabelle | `DigestMismatch`, Laden verweigert | T1.2 |

### 3.5 M-05 `model`

| ID | Prüfung | Erwartung | Bezug |
|---|---|---|---|
| B-18 | Parameterzahl von Ref-1 | exakt 5.868.800, Abweichung 0 | T3.2 |
| B-19 | Kausalmaske, Änderung an Position `t` | Logits an Position `< t` unverändert, an Position `≥ t` verändert | T3.1 |
| B-20 | Vorwärtslauf über 512 Token | Form `(512, 4096)`, alle Werte endlich | T3.3 |
| B-21 | Artefakt schreiben, lesen, prüfen | Digest stimmt; manipulierter Tensor `DigestMismatch`; unbekannte `format_version` `InvalidArtifact` | T3.3 |

**Anmerkung zu B-18 (11.11.2026, Sollwert nachgezogen, Testfall unverändert):**
Der Sollwert war 5 771 776 und ist auf **5 868 800** nachgezogen (Owner-Entscheid A,
dokumentiert in `ATC-AI-LLM-001-A` Revision 1.4.0). 5 771 776 ist der Stand vor K5,
also `d_ff` = 683 ohne abschliessende Norm; K5 hat 704 entschieden, und die Formel
zählt nach F1 die abschliessende Norm mit `+ d` mit. Die Erwartung des Testfalls
bleibt „Abweichung 0", nur die Zahl ist die gültige. Zwischenstand nach K5 ohne
abschliessende Norm: 5 868 544. Kein anderer Testfall dieses Blueprints ändert sich.

### 3.6 M-06 `train`

| ID | Prüfung | Erwartung | Bezug |
|---|---|---|---|
| B-22 | 200 Schritte auf festem Mini-Korpus | Verlust fallend, höchstens zwei Ausreißer | T4.1 |
| B-23 | Checkpoint nach Schritt 100, Wiederaufnahme bis 200 | bitidentisch zu einem ununterbrochenen Lauf über 200 Schritte | T4.2 |
| B-24 | gleicher Seed zweimal, anderer Seed einmal | gleicher Seed bitidentisch, anderer Seed verschieden | T4.3 |

Summe: 4 + 4 + 5 + 4 + 4 + 3 = **24**.

### 3.7 Warum genau diese 24

Die Tests decken vier Fehlerklassen ab und keine darüber hinaus:

1. **Stille Fehlrechnung:** B-05, B-06, B-09 bis B-13. Ein Autograd-Fehler
   produziert plausible Zahlen. Nur der Vergleich mit der Finitedifferenz findet
   ihn.
2. **Numerischer Ausfall:** B-07, B-08, B-11. `NaN` breitet sich aus und wird
   sonst erst nach Stunden sichtbar.
3. **Reproduzierbarkeit:** B-14 bis B-17, B-23, B-24. Ohne diese Tests ist jeder
   spätere Vergleich zweier Läufe wertlos.
4. **Grenzverletzung:** B-01 bis B-04, B-19, B-21. Eine falsche Maske oder eine
   übersehene Form prüft sich sonst nie, weil das Modell trotzdem trainiert.

Nicht in P1/P2 enthalten und deshalb nicht in dieser Liste: Durchsatz (T2.6),
Überanpassung (T6.1), algorithmische Aufgabe (T6.2), Sprachkorpus (T6.3),
Sampler (T8.1), KV-Cache (T8.2), Backend-Toleranz (T9.1, T9.2), Isolation
(T10.1, T10.2).

---

## 4. Austrittsbedingungen

### P1 (Tokenizer)

B-14 bis B-17 grün, dazu messbarer Kompressionswert auf dem Zielkorpus, damit
die Vokabularwahl 4096 gegen 8192 entschieden werden kann (K4).

### P2 (Tensor und Autograd), technische Austrittsbedingung

B-01 bis B-13 grün **und** ein dokumentierter Durchsatzmesswert (T2.6) für
Ref-1 auf der Referenz-CPU (K3). Ohne den Messwert bleiben die Zeitangaben in
ATC-AI-LLM-001-A Annahmen.

### P2: Implementation Completion vs. Verification Completion

Implementierungsfortschritt und Verifikation sind zwei verschiedene Größen. Eine
fachlich abgeschlossene Implementierung darf nicht als unfertig gelten, weil eine
externe Referenzmessung blockiert ist. Sie darf umgekehrt ohne Messnachweis nicht
als verifiziert gelten. Dieser Abschnitt trennt beides und ändert keine einzige
Prüfung und keine Erwartung.

#### Statusdimensionen

| Dimension | Werte | Bedeutung |
|---|---|---|
| Implementierung | `NOT_STARTED`, `IN_PROGRESS`, `IMPLEMENTATION_COMPLETE` | alle vorgesehenen Module des Abschnitts sind implementiert und im Workspace baubar |
| Funktionale Prüfung | `NOT_TESTED`, `TESTED` | die zugehörigen Testfälle sind bestanden |
| Performance-Referenz | `PENDING`, `BLOCKED_EXTERNAL`, `VERIFIED` | T2.6 ist nach `ATC-AI-LLM-001-C` abgeschlossen |
| Gesamtabnahme | `PENDING_ACCEPTANCE`, `ACCEPTED` | alle verbindlichen Austrittsbedingungen sind erfüllt |

#### Austrittsbedingungen

Die bisherige Austrittsbedingung für P2 wird in vier einzeln prüfbare Bedingungen
zerlegt. Inhaltlich bleibt sie unverändert.

| Bedingung | Erfüllt, wenn |
|---|---|
| `P2_IMPLEMENTATION_COMPLETE` | M-01, M-02 und M-03 implementiert und im Workspace baubar |
| `P2_FUNCTIONALLY_TESTED` | B-01 bis B-13 grün |
| `P2_PERFORMANCE_VERIFIED` | T2.6 nach `ATC-AI-LLM-001-C` abgeschlossen, Streuung der drei Messläufe höchstens 5 Prozent |
| `P2_ACCEPTED` | alle drei vorgenannten Bedingungen erfüllt |

#### Reichweite, und warum P2 nicht sechs Module umfasst

P2 umfasst die Module M-01 bis M-03, also Tensor-Kern, Tensor-Operationen und
Autograd. Die sechs Module und die 24 Testfälle verteilen sich auf P1 bis P4:
M-04 und B-14 bis B-17 gehören zu P1, M-05 zu P3, M-06 zu P4. Die blueprintweiten
Zustände heißen deshalb getrennt `REF1_IMPLEMENTATION_COMPLETE` (alle sechs
Module) und `REF1_FUNCTIONALLY_TESTED` (alle 24 Testfälle). Sie sind keine
P2-Bedingungen und werden nicht mit ihnen vermischt.

Die 24 Testfälle und ihre Erwartungen bleiben unverändert.

#### Externe Blockierung

Die Performance-Referenz ist derzeit `BLOCKED_EXTERNAL`. Ursache ist nicht die
Implementierung, sondern die fehlende qualifizierte Maschine für T2.6 nach
`ATC-AI-LLM-001` Abschnitt 14.3 und Protokoll `ATC-AI-LLM-001-C`. Solange keine
Maschine das Qualifikationsprofil erfüllt, bleibt `P2_PERFORMANCE_VERIFIED`
unerreichbar, unabhängig vom Implementierungsstand.

#### Governance

1. `BLOCKED_EXTERNAL` wird niemals automatisch zu `VERIFIED`, `CI_VERIFIED` oder
   `E2E_VERIFIED` überführt. Ein Statuswechsel braucht einen Beleg nach Protokoll
   `ATC-AI-LLM-001-C`.
2. Eine fehlende externe Messumgebung ist kein Implementierungsfehler. Sie darf
   nicht rückwirkend als solcher gewertet werden und rechtfertigt keine Absenkung
   von Prüfungen.
3. Keine vorzeitige Gesamtfreigabe: `P2_ACCEPTED` ist erst zulässig, wenn alle drei
   Bedingungen erfüllt sind. `P2_IMPLEMENTATION_COMPLETE` und
   `P2_FUNCTIONALLY_TESTED` allein ergeben keine Abnahme.
4. Nachträgliche Fortsetzung: nach der Qualifikation einer Maschine wird die
   Performance-Verifikation ohne erneute Implementierungsarbeit nachgeholt. Der
   erreichte Implementierungs- und Teststand bleibt gültig und wird nicht
   wiederholt.
5. Die Stabilitätsgrenze von 5 Prozent aus Protokoll `ATC-AI-LLM-001-C` bleibt
   verbindlich. Eine Messung außerhalb der Grenze ist keine Verifikation, sondern
   eine Wiederholung.

#### Status am 11.11.2026

| Dimension | Status | Beleg |
|---|---|---|
| Implementierung | `IMPLEMENTATION_COMPLETE`, 3 von 3 P2-Modulen | `atc-aurora-tensor` enthält M-01, M-02 und M-03; `cargo build` sowie `cargo check --lib --target thumbv7em-none-eabihf` grün |
| Funktionale Prüfung | `TESTED` (lokal) | B-01 bis B-13 grün, 53 Tests im Crate. `CI_VERIFIED` bleibt offen, weil push-getriggerte Workflows am unbestätigten Konto scheitern |
| Performance-Referenz | `BLOCKED_EXTERNAL` | keine qualifizierte Maschine, Protokoll `ATC-AI-LLM-001-C` ohne Eintrag |
| Gesamtabnahme | `PENDING_ACCEPTANCE` | `P2_PERFORMANCE_VERIFIED` offen, also nur eine der drei Bedingungen |

Die Crates für M-04 bis M-06 (`atc-aurora-tokenizer`, `-model`, `-train`)
existieren weiterhin nicht; ihre Phase ist P1, P3 und P4 und von diesem Status
nicht berührt. `REF1_IMPLEMENTATION_COMPLETE` bleibt deshalb offen.

### Nicht Teil der Austrittsbedingung

CI-Evidenz. Sie ist derzeit nicht beschaffbar, weil push-getriggerte Workflows am
unbestätigten Konto scheitern. Die Tests laufen können lokal nachgewiesen werden,
CI_VERIFIED bleibt bis zur Kontobestätigung offen. Dieser Unterschied wird im
Status getrennt geführt und nicht vermischt.

---

## 5. Was in P1 und P2 nicht gebaut wird

GPU-Kernel, BF16-Pfad, SIMD-Intrinsics, Quantisierung, verteiltes Training,
Attention-Varianten jenseits von MHA, Positionsverfahren jenseits von RoPE,
Sampler, KV-Cache, Instruction Tuning, Zweitregister, zweiter Scheduler,
Ladepfad aus der Chain. Jeder Punkt gehört einer späteren Phase und wird nicht
vorbereitet.

---

## 6. Offene Punkte

| ID | Punkt | Wirkung |
|---|---|---|
| K3 | Referenz-CPU für T2.6 benennen | macht aus Annahmen Messwerte |
| K4 | Vokabular 4096 oder 8192 für Ref-1 | Kompression gegen Trainingszeit |
| K5 | `d_ff = 683` oder 704 | 1,7 Prozent Parameter gegen SIMD-Ausrichtung |
| K6 | ~~`rope_base` normativ festlegen~~ — **ENTSCHIEDEN 11.11.2026: 10 000 ist Normwert** (Option A) | ändert die Bedeutung trainierter Gewichte; das Artefakt deklariert den Wert (`ATC-AI-LLM-002` §3), der Leser lehnt Abweichungen ab; andere Basen nur per neuer Architekturversion |

---

*Dieses Dokument enthält 0 Zeilen Produktivcode. Es ordnet sechs Module den
bestehenden Aurora-Komponenten zu und legt 24 Testfälle fest. Kein Testfall ist
ausgeführt.*
