# Changelog

## 2026-11-11 — M-05 Vorwaertslauf (B-19, B-20)

- **Neu `src/weights.rs`**: `Weights`, `BlockWeights` und `SplitMix64`. Gewichte stammen
  ausschliesslich aus der Konfiguration (Formen aus `params.rs`, Normgewichte starten bei 1),
  `Weights::param_count()` muss `param_count(&cfg)` entsprechen — als Test festgehalten,
  für die kleine Konfiguration und für Ref-1. Initialisierung ohne `rand` und ohne
  Betriebssystemzufall, damit sie auf jedem Rechner identisch ist.
- **Neu `src/forward.rs`**: Einbettung, je Block RMSNorm → Q/K/V → RoPE → kausale
  Multi-Head-Attention → Ausgabeprojektion → Restpfad, dann RMSNorm → SwiGLU → Down →
  Restpfad, abschliessende Norm, gebundener Ausgabekopf. **Keine Operation ist
  zweitimplementiert**: gerechnet wird ausschliesslich mit `atc-aurora-tensor` (M-01…M-03).
  Die Maske setzt `j > i` auf `-inf`, damit ein Maskenfehler nicht als kleiner Zahlenwert
  durchgeht. Fehlerpfade typisiert (`ForwardError`), kein `String`, keine Panik.
- **Neu `src/tests_forward.rs`, 8 Tests**: B-19 in beiden Richtungen (Änderung an Position
  `t` bewegt nur Positionen `>= t`; Änderung der letzten Position lässt alles davor
  unberührt), B-20 (512 Token, Form `(512, 4096)`, alle Werte endlich), Strukturprobe der
  Maske (bei Gleichstand mittelt die Attention genau über `0..=i`), Reproduzierbarkeit
  (gleicher Seed identisch, anderer Seed verschieden), Gewichtssumme gleich
  Parameterhaushalt, Fehlerpfade (leer, zu lang, Token ausserhalb, Budget zu klein,
  unzulässige Konfiguration).
- **Grenzen, die im Dokument stehen und nicht versteckt sind**: initialisierte, nicht
  trainierte Gewichte — die Tests prüfen Rechnung und Grenzen, nicht Sprachqualität;
  kein KV-Cache, kein Batching, kein Sampler (P8); `ROPE_BASE` ist 10 000 und noch nicht
  normativ festgelegt, die Spezifikation nennt nur „RoPE". B-20 kostet im Debug-Build
  rund 22 Sekunden.
- **16 Tests im Crate** (8 alt, 8 neu), clippy `-D warnings` sauber, fmt sauber.
  Register auf 504 Dateien nachgezogen (`.rs` 48). Reifegrad: B-19 und B-20 `IMPLEMENTED`
  und `TESTED`; `CI_VERIFIED` und `E2E_VERIFIED` bleiben offen, ebenso B-21 (Artefakt).

## 2026-11-11 — B-18: Herkunft des Sollwerts belegt (Fassung 0.2.0)

- **Der Sollwert 5.771.776 ist die Zahl vor K5.** Belegstelle im Blueprint selbst:
  `ATC-AI-LLM-001-B` §6 führt `d_ff` 683 oder 704 als offenen Punkt K5 mit „1,7 Prozent
  Parameter gegen SIMD-Ausrichtung". 683 gegen 704 sind exakt 96.768 Parameter, also
  1,68 Prozent. B-18 ist damit die einzige Stelle, die K5 nicht mitgezogen hat;
  `ATC-AI-LLM-001-A` §7.1.1 nennt nach K5 **5.868.544**.
- **Zweiter Befund: die Formel zählt die abschliessende Norm nicht mit.**
  `P = L·(4d² + 3·d·d_ff + 2d) + V·d` ergibt 5.868.544; mit der abschliessenden RMSNorm
  vor dem Ausgabekopf sind es **5.868.800**, und so rechnet das Crate. Die Block-Normen
  (`2d`) sind in derselben Formel mitgezählt, es fehlt genau ein `+ d` ausserhalb der Blöcke.
- **Entscheidungsdokument auf 0.2.0**: Belegstellen, zweiter Befund, Optionen A/B/C,
  Auflösungen F1 (Formel korrigieren, empfohlen) und F2 (keine abschliessende Norm).
  Empfehlung jetzt **A zusammen mit F1** — A ist die Nachführung einer getroffenen
  Entscheidung, keine neue Architekturfrage. Das Dokument entscheidet weiterhin nichts.
- **README und STATUS** nennen die Belegstellen und beide offenen Entscheidungen.
  Keine Zahl, kein Status und keine Reifeklasse geändert: M-05 bleibt `IMPLEMENTED`
  und `TESTED`, `CI_VERIFIED` offen. Register unverändert 501 Dateien, 59 Verzeichnisse.

## 2026-11-11 — Entscheidungsvorlage B-18: Sollwert gegen Ableitung

- **Neues Dokument `docs/decisions/AD-B18-PARAMETERHAUSHALT.md`** (ATC-DOC-AI-AD-001,
  Status offen, neues Verzeichnis `docs/decisions`). Inhalt: der Widerspruch zwischen
  B-18 und der Ableitung, der Rechenweg aus K4/K5, die Zerlegung der Differenz, zwei
  arithmetisch exakte Auflösungen, der durchsuchte Raum, drei Optionen mit Folgen und
  Aufwand, eine Empfehlung, was bis zur Entscheidung gilt, und was ausdrücklich nicht
  Teil der Frage ist. Das Dokument entscheidet nichts.
- **Differenz zerlegt.** 97 024 = 379 × 256, ohne die abschliessende Norm
  96 768 = 6 × 63 Zeilen. Die Abweichung ist damit genau die abschliessende Norm plus
  63 Zeilen je Block, also `d_ff` 683 statt 704 — und 683 ist aufgerundetes 8/3 × 256.
  Zweite exakte Auflösung: Vokabular 3.717 mit finaler Norm (3² × 7 × 59, unplausibel).
  **Gegenprobe mit dem Crate selbst:** `param_count` auf `Config { d_ff: 683, ..ref1() }`
  abzüglich `Breakdown::final_norm` ergibt 5 771 776, `d_ff` 683 mit finaler Norm
  5 772 032. Für den Sollwert selbst existiert im Repository keine Herleitung, nur die
  Tabellenzeile B-18.
- **Empfehlung A** (Sollwert auf 5 868 800 korrigieren), solange keine Originalrechnung
  beigebracht wird. B (Architektur neu einfrieren) und C (dauerhaftes `RESIDUAL`) sind
  mit Folgen und Aufwand dokumentiert. Die Entscheidung liegt beim Owner.
- **README und STATUS** verweisen auf die Vorlage. Keine Zahl, kein Status und keine
  Reifeklasse geändert: M-05 bleibt `IMPLEMENTED` und `TESTED`, `CI_VERIFIED` offen.
- **Register** auf 501 Dateien und 59 Verzeichnisse nachgezogen (`.md` 86).

## 2026-11-11 — M-05 begonnen: Parameterhaushalt, B-18 mit Befund

- **Neues Crate `modules/atc-aurora-model`** (Blueprint Abschnitt 3.5, M-05).
  Inhalt dieses Schnitts: `Config` mit den Architekturbedingungen (Breite,
  Kopfteilung, KV-Kopfe, Zwischenraum, Vokabular, geteilte Einbettung),
  `Breakdown` und `param_count` mit gepruefter Arithmetik. Kein Vorwaertslauf,
  keine Attention, keine Gewichte, keine Initialisierung, kein Dateiformat.
- **B-18 abgeleitet statt gesetzt.** Die Zahl kommt aus der Konfiguration:
  Embedding 1 048 576, Attention 1 572 864, SwiGLU 3 244 032, Block-Normen
  3 072, finale Norm 256, zusammen **5 868 800** (ohne finale Norm 5 868 544).
- **BEFUND: der Blueprint-Sollwert passt nicht dazu.** B-18 nennt **5 771 776**.
  Die Differenz ist **97 024** Elemente (96 768 ohne die abschliessende Norm) —
  keine Rundung, sondern ein Widerspruch zwischen Sollwert und eingefrorener
  Architektur (K4 Vokabular 4096, K5 `d_ff` 704, sechs Bloecke, geteilte
  Einbettung, keine Bias-Terme). Der Test
  `b_18_blueprint_sollwert_weicht_von_der_ableitung_ab` haelt die Abweichung
  fest; die Implementierung folgt der Architektur, nicht dem Sollwert. Die
  Aufloesung ist eine Owner-Entscheidung, keine Implementierungsfrage.
- **Pruefungen:** 8 Tests gruen, clippy `-D warnings` sauber, rustfmt sauber.
  `CI_VERIFIED` bleibt offen (Commit-Herkunft unverifiziert, Actions starten
  nicht); `M-05` ist damit `IMPLEMENTED` und `TESTED`, mehr nicht.
- **Register** auf 500 Dateien und 58 Verzeichnisse nachgezogen.

## 2026-11-11 — CI-Blockade eingegrenzt

- **Abrechnung als Ursache ausgeschlossen.** Alle betroffenen Repositories sind
  öffentlich (Actions-Minuten frei), und im Mirror-Repo sind am 11.11. um 07:18
  Läufe bis auf Job-Ebene gelaufen, mit `success` und `failure`. Ein
  Kontingent- oder Spending-Limit-Problem kann das nicht erzeugen.
- **Muster gefunden.** Jeder Lauf mit `startup_failure` sitzt auf einem Commit mit
  Autor `michael.worob@gmail.com` und `verification.verified = false`. Läufe, die
  starten, sitzen auf einem Commit des zweiten Kontos
  (`alexander.gregwood@gmail.com`) oder auf einem signierten Commit
  (`verified = true`), zum Beispiel einem Web-UI-Commit.
- **Gegenprobe im Verbund:** `a-townchain-os` hat einen `push`-Erfolg auf
  `91905a86` (`verified=true`), während `aurora-ai` und `a-townchain` auf
  unsignierten Commits desselben Kontos bei null Jobs stehen bleiben.
- **Konsequenz:** Es ist kein Repository-, Workflow- oder Triggerproblem, sondern
  eine Frage der Commit-Herkunft. Zwei Wege: E-Mail des Kontos bestätigen, oder
  als sofortige Gegenprobe eine Datei über die Web-Oberfläche ändern (GitHub
  signiert solche Commits selbst). `CI_VERIFIED` bleibt offen.

## 2026-11-11 — Integrations- und Evidenzphase M-03

- **Mirror integriert.** `scripts/sync_modules.py --check` zeigte vorher
  `diff=2 fehlt=15 orphan=0`; `--sync` hat 17 Dateien kopiert, danach ist
  `--check` aus frischem Klon `IN_SYNC` (1/1 Komponenten, Exit 0, 437 von 437
  Scope-Dateien byte-identisch). Der Mirror-HEAD wird bewusst nicht gepinnt:
  jeder Sync zieht ihn weiter, der Nachweis ist der Blob-Vergleich. `atc-aurora-tensor` liegt mit 14 Dateien
  im Mirror, alle 15 Blobs byte-identisch zum keine Orphans, keine zweite Aurora-Implementierung. Die Übertragung ins
  Mirror-Repo lief über die Contents-API, weil `git push` am unbestätigten
  Konto scheitert; kopiert wurde ausschließlich über den vorgesehenen
  Mechanismus, nichts von Hand.
- **Tests auf dem gespiegelten Stand:** `cargo test` auf den Mirror-Bytes,
  53 grün. Damit ist nicht nur die Dateiidentität belegt, sondern dass der
  gespiegelte Code läuft.
- **Testlücke bei der Akkumulation geschlossen.** `b_12` prüft jetzt beide
  Akkumulationsstellen in einem Fall: Raute plus zwei Rückwärtspässe, mit von
  Hand gerechneten, dyadisch exakten Werten (`[3,10]` nach einem Pass, `[6,20]`
  nach zwei, `[9,30]` nach drei). Beide Akkumulations-Mutationen lassen genau
  diesen Test fehlschlagen; die Verteilungs-Mutation zusätzlich die Raute.
- **CI-Lage reproduziert und dokumentiert.** `workflow_dispatch` auf
  `code-quality.yml` und `test-suite.yml` gestartet: beide `startup_failure`,
  null Jobs, keine Logs, keine Check-Runs, keine Artefakte. Actions ist
  `enabled`, beide Workflows `active`. Die Ursache liegt damit nicht am Trigger
  und nicht am Workflow-Inhalt. `CI_VERIFIED` bleibt offen.
- **Statusführung** in `modules/atc-aurora-tensor/STATUS.md` getrennt nach
  Implementierung, lokaler Prüfung, `CI_VERIFIED`, `E2E_VERIFIED`,
  Mirror-Integration, K6 (`RESIDUAL`), Git-Identität (`BLOCKED`) und
  Performance-Referenz (`BLOCKED_EXTERNAL`).
- **K6 als eigene Entscheidung geführt.** `AUTOGRAD-CONTRACT.md` Abschnitt 7.1
  legt je Operator (LayerNorm, GELU, Dropout, Embedding-Lookup) die sieben
  Prüfpunkte fest, die vor einer Implementierung beantwortet sein müssen.
  Dropout braucht danach eine eigene Randomness- und
  Reproduzierbarkeitsstrategie. M-03 wird dafür nicht aufgebrochen.

## 2026-11-11 — M-03 Autograd und B-08 bis B-13

- `atc-aurora-tensor`: M-03 implementiert. Neuer Rechengraph (`Autograd`-Modul)
  mit zehn ableitungsfähigen Operationen, Rückwärtspass, Gradientenakkumulation,
  Budgetprüfung vor jeder Zuteilung und transaktionalem Fehlerverhalten: schlägt
  der Pass fehl, ist keine Byte verbucht und kein Gradient übernommen.
  53 Tests grün aus frischem Klon, clippy und fmt sauber, Bare-Metal-Build für
  `thumbv7em-none-eabihf` grün.
- B-08 bis B-13 aus den Contracts abgeleitet und umgesetzt, Referenz ist die
  zentrale Finitedifferenz in f64 mit `h = 1e-5` im Test. Messwerte: B-09 4,6e-8,
  B-10 1,3e-7, B-11 1,6e-6, B-13 5,0e-6 (jeweils relative Abweichung, alle
  innerhalb der Contract-Grenzen).
- `modules/atc-aurora-tensor/AUTOGRAD-CONTRACT.md`: Abgleich vor der
  Implementierung. Zehn Operationen mit Ableitung, begründete Ausschlüsse,
  verbindliche Ableitungen, Speicherlebensdauer, fünf Festlegungen bei
  Contract-Lücken, offener Punkt K6.
- Zwei Contract-Befunde, beide korrigiert und als Mutation geprüft: der
  Softmax-Gradient braucht den **Ausgang** der Operation, nicht den Eingang, und
  die Kreuzentropie ist eine **Summe** über die Zeilen, weil B-08 den Gradienten
  wörtlich als `softmax − target` festlegt.
- Fünf Mutationen am Rückwärtspfad geprüft, jede von den erwarteten Tests
  erkannt. Einschränkung dokumentiert: die Akkumulations-Mutation wird nur von
  der Raute erkannt, nicht vom Test der zwei Rückwärtspässe.
- Blueprint `ATC-AI-LLM-001-B` auf 1.1.1: P2-Status auf
  `IMPLEMENTATION_COMPLETE` und `TESTED` (lokal) gezogen,
  `P2_PERFORMANCE_VERIFIED` bleibt `BLOCKED_EXTERNAL`, `P2_ACCEPTED` offen;
  Anmerkung zur Zuordnung von B-08 ergänzt, Testfall unverändert.
- Offen und nicht stillschweigend übergangen: **K6** — LayerNorm, GELU, Dropout
  und Einbettungs-Lookup stehen im P2-Absatz, aber nicht in der API-Fläche und in
  keinem Testfall. Sie sind nicht gebaut; Entscheidung des Owners.

## 2026-11-11 — B-05 bis B-07 und Register-Invariante

- `atc-aurora-tensor`: Vertragstests B-05, B-06, B-07 aus dem Blueprint
  abgeleitet und umgesetzt (sechs neue Tests), dazu ein Test, der für alle
  zuteilenden Operationen prüft, dass das Budget vor der Zuteilung greift.
  36 Tests grün, clippy und fmt sauber, `no_std`-Profil baut.
- Wirksamkeitsnachweis durch drei Mutationen am Operationssatz; jede wurde von
  den Contract-Tests erkannt, die doppelte Transponierung allein erkennt die
  Transponierungs-Mutation nicht.
- `tools/check_file_register.py`: Invarianten-Test für das Dateiregister
  (Mengengleichheit gegen `git ls-files`, Sortierung, Verzeichniszahl,
  Standvermerk, Vollständigkeit von `docs/specs/`, Zahlen je Endung).

Alle wesentlichen Änderungen an diesem Projekt werden in dieser Datei dokumentiert.

## [Unreleased] - 2026-11-10

### Added
- `docs/specs/ATC-AI-LLM-001-AURORA-FOUNDATION-MODEL.md`: P0 specification for the native Aurora Foundation Model (Aurora LLM). Covers verified baseline inventory, numeric and determinism contract, crate layout, artifact format `.atcm`, parameter and compute sizing, phase plan P0-P10, test matrix, and the three-level learning acceptance criterion.

### Changed (Blueprint ATC-AI-LLM-001-B v1.1.0, Abschlusszustand getrennt)
- Neuer Abschnitt "P2: Implementation Completion vs. Verification Completion" im Blueprint: vier Statusdimensionen (Implementierung, Funktionale Prüfung, Performance-Referenz, Gesamtabnahme) statt eines pauschalen P2-Status.
- P2-Austrittsbedingung in vier einzeln prüfbare Bedingungen zerlegt: P2_IMPLEMENTATION_COMPLETE, P2_FUNCTIONALLY_TESTED, P2_PERFORMANCE_VERIFIED, P2_ACCEPTED. Inhaltlich unverändert, die 24 Testfälle bleiben unangetastet.
- Governance: BLOCKED_EXTERNAL wird nie automatisch zu VERIFIED, CI_VERIFIED oder E2E_VERIFIED. Eine fehlende Messumgebung ist kein Implementierungsfehler und rechtfertigt keine Absenkung von Prüfungen. Keine Gesamtfreigabe ohne alle drei Bedingungen. Nach der Hardwarequalifikation wird die Performance-Verifikation ohne erneute Implementierungsarbeit nachgeholt.

### Added (Revision 1.3.0, K3 entschieden)
- Referenzregel First Qualified Hardware in Abschnitt 14.3: die erste Maschine, die das vollständige Qualifikationsprofil erfüllt und exklusiv messen darf, wird die verbindliche CPU-Referenz. Keine CPU wird auf Verdacht benannt.
- Neues Dokument `ATC-AI-LLM-001-C`, T2.6-Messprotokoll: Qualifikationsprofil, Pflichtfelder der Umgebung, Messablauf mit fünf Aufwärm- und drei Messläufen, Median, Streuungsgrenze 5 Prozent, Rohdatenpflicht.
- Governance für den Wechsel der Referenz: neuer Fingerprint, neues Protokoll, Vergleich gegen die bisherige Referenz, dokumentierte Freigabe. Keine stillschweigende Ersetzung.

### Changed (Revision 1.2.0, K4 und K5 entschieden)
- `d_ff` von 683 auf 704, Parameterzahl 5.868.544 statt 5.771.776, also 1,68 Prozent mehr. Grund: 2.816 Byte je Zeile sind exakt 44 Cache-Lines zu 64 Byte und exakt 88 AVX2-Vektoren, Rest 0. Mit 683 überquert jede Zeile eine Cache-Line.
- Vokabular bleibt 4096 und ist jetzt verbindlich, mit Falsifikator: unter 2,2 Byte je Token auf dem P1-Zielkorpus wird neu bewertet.
- Neue verbindliche Modellkonfiguration in Abschnitt 7.1.1 als einzige Quelle. Kein Crate darf einen eigenen Default für diese Werte definieren.
- Anhang nachgezogen: Parameterrechnung, 4,46e7 FLOPs je Token, 1,83e11 FLOPs je Schritt, 93,9 MB Trainingszustand.
- K1 und K2 als geschlossen geführt, K3 offen mit entschiedenem Messverfahren.

### Added (Revision 1.1.0 und Anhang)
- `docs/specs/ATC-AI-LLM-001-A-REF1-IMPLEMENTATION-ENVELOPE.md`: Implementierungsgrenzen fuer Ref-1. Korrigiertes Rechenmodell (Attention-Term, 16 bis 27 Prozent Aufschlag gegen die 6ND-Naeherung), Speichermodell als eigentliche Grenze, Schrittbudget fuer L1 bis L3, Determinismusverfahren, hergeleitete Toleranzen, festgelegte API-Flaeche, Abgrenzung zu AD-021 und zu den bestehenden aurora-ai-Kopien.
- `ATC-AI-LLM-001` auf Version 1.1.0: Entscheidungen D1 bis D5 angenommen, vier neue Entscheidungen K1 bis K4 offen.

### Fixed
- `modules/atc-aurora-runtime/STATUS.md`: behauptete "0 .atc Dateien" bei fuenf vorhandenen und verschwieg das fehlende `Cargo.toml` und die fehlenden Rust-Quellen. Fassung vom 2026-08-08 ersetzt.
- `modules/atc-aurora-runtime/COMPONENT_PLAN.md`: Status der fuenf Dateien von "GEPLANT" auf "Notiz vorhanden, nicht ausfuehrbar" korrigiert.
- `FILE_REGISTER.md`: von 425 auf 476 Dateien neu erzeugt. Die alte Fassung nannte 16 `.rs` statt 31 und 61 `.md` statt 78.

### Changed
- Nothing in code. Specification, two documentation corrections and a regenerated file register. 0 lines of production code.

### Verification
- Baseline inventory is bound to commit `b4858f350fcc8db4d48c1cb20b2f116be53c41fe` (file list via Git Trees API, keyword scan over all `*.rs`).
- Parameter counts are computed from a stated formula, not measured, and are labelled as computed.
- Throughput figures are assumptions with a measurement planned as P2 exit criterion T2.6.
- No build, test or runtime claim is made by this change.

## [Unreleased] — 2026-10-18

### Added
- Validated Rust agent registry with unique caller-assigned IDs, capability metadata and typed errors.
- Bounded task scheduler with explicit lifecycle states, dependency gating, timeout handling and cancellation.
- Result collector with expected-agent attribution, duplicate/empty-result rejection and deterministic ordering.
- Unit tests for registry, scheduling, dependency, timeout/cancellation and result aggregation invariants.

### Verification
- Implementation commit evidence is recorded in the pull request.
- Build/test and GitHub Actions results remain pending until the exact PR head SHA has completed its checks.
- The existing .atc files are legacy/planned stubs; this implementation is in the canonical Rust atc-aurora-core crate and does not claim the ATCLang stubs are implemented.

## [0.1.0] - 2026-09-09

### Added
- Standards-Konformität hergestellt gemäß ATC-STD-README-001 und ATC-STD-MD-001.
- Erstellung von STATUS.md, ROADMAP.md, ARCHITECTURE.md, CONTRIBUTING.md, GOVERNANCE.md, CODE_OF_CONDUCT.md.
- Aktualisierung der AI Agent Instructions in AGENTS.md.

## [0.0.1] - 2026-09-07

### Added
- Governance-Überarbeitung nach ATC-STD-201/202/203.
- Rust Core und Python AI Services Grundstruktur.
