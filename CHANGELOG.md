# Changelog — aurora-ai

Alle wesentlichen Änderungen an diesem Projekt werden in dieser Datei dokumentiert.

## [Unreleased] - 2026-11-10

### Added
- `docs/specs/ATC-AI-LLM-001-AURORA-FOUNDATION-MODEL.md`: P0 specification for the native Aurora Foundation Model (Aurora LLM). Covers verified baseline inventory, numeric and determinism contract, crate layout, artifact format `.atcm`, parameter and compute sizing, phase plan P0-P10, test matrix, and the three-level learning acceptance criterion.

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
