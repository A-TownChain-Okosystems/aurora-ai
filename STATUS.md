---
document_id: ATC-DOC-AI-001
title: Aurora AI Repository Status
version: 1.0.0
status: active
owner: A-TownChain-Okosystems
created: 2026-09-09
updated: 2026-10-18
standard: ATC-STD-MD-001
---

# Status — Aurora AI

> **Evidence freshness notice (2026-10-18):** This change adds Rust orchestration components on a feature branch. Build/test verification is **PENDING** until GitHub Actions completes for the exact pull-request head SHA. The historical status declarations below are not current verification evidence.

| Property | Value |
|---|---|
| Repository | aurora-ai |
| Version | 0.1.0 |
| Status | development |
| Build | NOT VERIFIED for this change until exact-SHA checks complete |
| Tests | NOT VERIFIED for this change until exact-SHA checks complete |
| Security | S2 (Sicherheitsklasse S2) |
| Documentation | ATC-STD-README-001 / ATC-STD-MD-001 |
| Last audit | 2026-09-09 (historical; not a current audit) |

## Current implementation evidence

- Rust AgentRegistry: input validation, duplicate-ID rejection, capability metadata and typed errors.
- Rust TaskScheduler: lifecycle states, dependency gating, bounded dispatch, timeout and cancellation.
- Rust ResultCollector: expected-agent attribution, payload validation, duplicate rejection and deterministic ordering.
- The .atc files under modules/atc-aurora-core/src/agent_registry.atc and modules/atc-aurora-runtime/src/ remain separate legacy/planned files. Their presence does not establish equivalent ATCLang functionality.
- The PR must link the exact head SHA and completed workflow runs before the verification status can be upgraded.

## Modellkette (P4, 13.11.2026)

Die Kette `Tokenizer -> Autograd -> Backpropagation -> Optimizer -> Training ->
Evaluation -> Registry -> Inferenz` ist lokal geschlossen und getestet, **nicht**
in CI bestaetigt.

| Glied | Crate | Beleg |
|---|---|---|
| Tensor-Kern, Autograd | `atc-aurora-tensor` | 61 Tests; neu: `gather`, `slice_cols`, `rope` mit Finitedifferenz |
| Vorwaertslauf, differenzierbarer Graph, Greedy-Dekodierung | `atc-aurora-model` | 69 Tests (1 ignoriert); Graph-Logits gleich `forward` (1e-4 relativ) |
| Optimierer, Training, Bewertung, Pruefpunktregister | `atc-aurora-train` | 15 Tests; Verlust sinkt, Pruefpunkt bytegleich, Fortsetzung identisch |
| Tokenizer | `atc-aurora-tokenizer` | 34 Tests |
| Digest-Autoritaet | `atc-aurora-digest` | 5 Tests |

Produktionsbasis ist **Stable Rust**, geprueft mit 1.83.0; ein Nightly ist fuer
diese Kette nicht erforderlich (AD-P4, D6). Zwei Stellen, die bis 1.87 stabil
nicht verfuegbar waren (`is_multiple_of`, `as_chunks`), sind durch `%` und
`chunks_exact` ersetzt, ohne Semantikaenderung.

Zwei Befunde sind dabei geschlossen worden: `Graph::cross_entropy` gab sein
Budget nicht zurueck (jetzt gehalten durch `p4_graph_gibt_budget_zurueck`), und
der Autograd-Contract fuehrte Einbettungs-Lookup, RoPE und Kopfzerlegung als
offen (jetzt implementiert, `AUTOGRAD-CONTRACT.md` Abschnitt 2.1.1).

Bewusste Grenzen: kein Lauf auf Ref-1, keine Sprachqualitaet, kein KV-Cache,
keine Laufzeitbindung (`atc-aurora-runtime` ist weiterhin `.atc`-Entwurf), keine
Lernratenplanung. Der Verlust sinkt auf einem winzigen Korpus mit winziger
Konfiguration; das ist ein Rechennachweis, keine Modellguete.

`CI_VERIFIED` und `E2E_VERIFIED` bleiben **offen**: alle Workflow-Laeufe enden
mit `startup_failure` ohne Job, solange die Konto-E-Mail unverifiziert ist (F-T2).
