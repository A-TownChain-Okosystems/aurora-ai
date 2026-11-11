---
document_id: ATC-AI-AD-K6
title: Entscheidungsvorlage K6 — `rope_base` normativ festlegen
version: 0.1.0
status: proposed
owner: A-TownChain-Okosystems
created: 2026-11-11
updated: 2026-11-11
layer: L2 (aurora-ai)
evidence_gate: AURORA-001
---

# K6 — `rope_base` normativ festlegen

**Status: zur Entscheidung.** Drei Optionen, eine Empfehlung, keine Vorwegnahme.

## 1. Stand

`ATC-AI-LLM-001` nennt für den Vorwärtspass „RoPE" und keinen Zahlenwert. Das
Crate führt `ROPE_BASE = 10000.0` als **Implementierungsparameter**
(`src/forward.rs`), nicht als Normwert. Seit B-21 steht der Wert im Artefakt
(`ATC-AI-LLM-002` Abschnitt 3), und der Leser lehnt ein Artefakt mit anderem
Wert ab (`ArchMismatch { field: "rope_base" }`), weil der Rechenkern nur seine
eigene Konstante kennt.

Damit ist die Lage heute: **ein Artefakt deklariert seinen Wert, und das Crate
akzeptiert genau einen.** Das ist sicher, aber nicht normativ.

## 2. Warum das eine Entscheidung braucht

`rope_base` verändert die Bedeutung jedes Gewichts. Zwei Modelle mit gleichen
Gewichten und verschiedener Basis sind verschiedene Modelle, auch wenn jede
Zahl gleich aussieht. Solange der Wert nicht normativ ist:

- kann ein zweites Werkzeug mit anderer Basis dieselben Gewichte anders rechnen,
  ohne dass es jemandem auffällt;
- ist „Modell reproduzierbar" nur für die eigene Implementierung wahr;
- ist die Reifeklasse von B-19/B-20 an eine Zahl gebunden, die niemand
  festgeschrieben hat.

## 3. Optionen

### Option A — 10 000 normativ festschreiben (Empfehlung)

`ROPE_BASE = 10000.0` wird in `ATC-AI-LLM-001` als Normwert aufgenommen, das
Artefakt deklariert ihn weiterhin, und der Leser lehnt Abweichungen ab.

- Vorteil: deckt sich mit dem, was die Referenz bereits tut; keine Gewichte
  ändern sich; die Deklaration im Artefakt bleibt als Sicherheitsnetz.
- Nachteil: schreibt eine Zahl fest, die in der Literatur nicht einheitlich ist
  (andere Modelle nutzen größere Basen für lange Kontexte).
- Folge für `max_seq` 512: 10 000 ist für diese Kontextlänge die übliche Wahl.

### Option B — andere Basis normativ (z. B. 500 000 für lange Kontexte)

- Vorteil: bessere Eigenschaften bei deutlich längeren Sequenzen.
- Nachteil: alle Referenzgewichte müssten neu erzeugt werden; `max_seq` ist 512,
  der Nutzen wäre heute null. Das wäre eine Architekturänderung mit
  Neuerzeugung aller Evidenz für einen Vorteil, der erst bei langen Kontexten
  entsteht.

### Option C — Implementierungsparameter bleiben (Status quo)

- Vorteil: keine Entscheidung, maximale Freiheit.
- Nachteil: „normativ" bleibt offen, und Dritte können nicht behaupten, dasselbe
  Modell zu rechnen. Die Artefaktdeklaration fängt nur den Fall ab, dass jemand
  das Crate benutzt — nicht den Fall einer zweiten Implementierung.

## 4. Empfehlung

**Option A.** Sie ändert keine Gewichte, keine Bytes und keine Testzahl; sie
schreibt fest, was ohnehin gilt, und macht die Aussage „dieses Artefakt rechnet
mit Basis 10 000" überprüfbar statt nur deklariert. Option B ist eine
Architekturentscheidung für lange Kontexte, wenn `max_seq` wächst, nicht jetzt.
Option C lässt genau die Lücke offen, die K6 geschlossen werden soll.

## 5. Umsetzung bei Option A

1. `ATC-AI-LLM-001` erhält in der RoPE-Beschreibung den Normwert 10 000 mit
   Begründung und Verweis auf K6 (Revisionseintrag, Version 1.5.0).
2. Blueprint: K6 wird von „offen" auf „entschieden" gesetzt, mit Verweis.
3. `ROPE_BASE` im Crate bekommt einen Kommentar „Normwert, K6 entschieden am …".
4. Der Leser bleibt streng: `ArchMismatch` bei Abweichung. Eine Toleranz wäre
   falsch, weil zwei Basen zwei Modelle sind.
5. Keine Teständerung: 61 Tests bleiben unverändert, weil sich kein Byte ändert.

## 6. Wenn Option B gewählt wird

Dann gehören in denselben Schritt: neue Referenzgewichte, neue B-18-Zahl nur falls
sich die Parameterzahl ändert (sie ändert sich **nicht**, RoPE hat keine
Parameter), neue Artefakt-Digests, Neuerzeugung der Messprotokolle und ein
Revisionseintrag in `ATC-AI-LLM-001`, `ATC-AI-LLM-002` und im Blueprint. Das ist
ein eigener Bauabschnitt, kein Nebenprodukt.

## 7. Entscheidung

Offen. Erwartet: `A`, `B` (mit Wert) oder `C`.
