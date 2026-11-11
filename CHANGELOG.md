# Changelog

## 2026-11-11 — Integrations- und Evidenzphase M-03

- **Mirror integriert.** `scripts/sync_modules.py --check` zeigte vorher
  `diff=2 fehlt=15 orphan=0`; `--sync` hat 17 Dateien kopiert, danach ist
  `--check` aus frischem Klon `IN_SYNC` (1/1 Komponenten, Exit 0, Mirror
  `a-townchain-ecosystem` `5daf7036`). `atc-aurora-tensor` liegt mit 14 Dateien
  im Mirror, alle 15 Blobs byte-identisch zum Kanon `aurora-ai` `8f6ccb54`,
  keine Orphans, keine zweite Aurora-Implementierung. Die Übertragung ins
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
