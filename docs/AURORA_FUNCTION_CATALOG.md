---
document_id: ATC-DOC-AI-009
title: Aurora AI Function Inventory and Gap Register
version: 0.1.0
status: draft
owner: A-TownChain-Okosystems
standard: ATC-STD-MD-001
source_branch: fix/aurora-core-p0-input-safety
source_head_at_creation: 3a8318d5cf2043b9e0bcc1996205d225fc035f5e
---

# Aurora AI Function Inventory and Gap Register

This is a source-backed inventory snapshot, not a claim of platform completeness. File presence and unit tests do not prove security or production readiness. The source SHAs below are Git blob SHAs observed during inventory; the repository commit SHA for this branch must be recorded separately by CI/review.

## Status vocabulary

- `SPECIFIED`: contract documented; no implementation claim.
- `IMPLEMENTED`: source implementation exists.
- `TESTED`: relevant test executed successfully.
- `CI_VERIFIED`: passing CI evidence is tied to the exact commit SHA.
- `INTEGRATED`: dependency integration is demonstrated.
- `E2E_VERIFIED`: end-to-end evidence is recorded.
- `GAP`: required behavior not found in inspected source.
- `UNMAPPED_RESIDUAL`: inventory is incomplete or no authoritative source/test mapping is available.

## Inspected source inventory

| Function ID | Module / source | Observed implementation | Source blob SHA | Status from inspection |
|---|---|---|---|---|
| AUR-CORE-001 | `modules/atc-aurora-core/src/aurora_core.rs` | `AuroraCore::new/start/stop/is_active/process_request`; lifecycle coordinator | `edf81f530e1a759f3b8e2af3d5f7993b59c319b4` before this branch's change | IMPLEMENTED; CI pending |
| AUR-CORE-002 | `modules/atc-aurora-core/src/model_hub.rs` | Model registration, default model, listing, placeholder inference response | `a4e38b68dcba4bd9c71c527ad089e0b2bd922e28` before this branch's change | IMPLEMENTED; real model inference GAP |
| AUR-CORE-003 | `modules/atc-aurora-core/src/llm_router.rs` | Keyword-based route rules and fallback model | `c0615d7f4106af3ef2188e69da849bcc29ce54e4` | IMPLEMENTED; provider/capability negotiation GAP |
| AUR-CORE-004 | `modules/atc-aurora-core/src/agent_registry.rs` | In-memory agent registry, role/capability labels, active flag | `ad2d7826e66ef3c20924ae10e16b6bb5abd1c32a` | IMPLEMENTED; capability enforcement GAP |
| AUR-CORE-005 | `modules/atc-aurora-core/src/config_manager.rs` | In-memory string key/value defaults and accessors | `b1adf540f1b4dc60259e3e65759b2f9bd23796a0` | IMPLEMENTED; typed schema/validation/secrets GAP |
| AUR-AGENT-001 | `modules/atc-aurora-agents/src/lib.rs` | Exposes `agents`, `agent_base`, `agent_pool` modules | `6bb82069e246b9ba474a72c0a3780c0806122151` | IMPLEMENTED; orchestration/limits require deeper audit |
| AUR-MEM-001 | `modules/atc-aurora-memory/src/lib.rs` | Exposes knowledge base, vector store, index, learning pipeline, context window | `43165c771dd83e2bfd6294ffb8554fd272ccc77d` | IMPLEMENTED; ACL/retention/poisoning controls not yet verified |
| AUR-RUNTIME-001 | `modules/atc-aurora-runtime/README.md` | Describes runtime/tool system; expected Cargo manifest and source entrypoint not found at the initially probed paths | `20e47459be937269e33022d768f1a1b0ed63c6bd` | UNMAPPED_RESIDUAL |
| AUR-PY-001 | `modules/atc-aurora-ai/README.md`, `requirements.txt` | Python AI service description and dependencies; pyproject manifest not found at the probed path | README blob SHA `07f0456835f8a4ca419c81ba09159e6e623788bd` | UNMAPPED_RESIDUAL |

## P0 security and correctness gaps

| Gap ID | Requirement | Evidence found | Required next action | Release status |
|---|---|---|---|---|
| AUR-GAP-001 | Policy engine, deny-by-default decisions and decision records | No policy engine found in inspected core sources | Locate canonical policy implementation or add an explicit policy module and tests | BLOCKING |
| AUR-GAP-002 | Capability enforcement before every tool/service invocation | Registry stores capability labels, but inspected `process_request` did not enforce them | Add authorization interface; negative tests for missing, expired, wrong-scope and revoked capabilities | BLOCKING |
| AUR-GAP-003 | Explicit approval gate for privileged operations | No approval contract found in inspected core sources | Define approval states; ensure reject/timeout never means approval | BLOCKING |
| AUR-GAP-004 | Tool contracts, side-effect declarations and service boundary | No authoritative tool execution contract found in inspected core sources | Map runtime source and enforce input/output schemas, timeouts, cancellation and resource limits | BLOCKING |
| AUR-GAP-005 | Typed, validated configuration | Config values are arbitrary strings | Define versioned schema, reject invalid values, prohibit secrets in model context/logs | BLOCKING |
| AUR-GAP-006 | Structured audit/provenance | No end-to-end decision/tool provenance found in inspected core sources | Add correlation IDs and append-only records linking intent, policy, approval, tool and result | BLOCKING |
| AUR-GAP-007 | Real model provider ABI and inference | `ModelHub::inference` returns a formatted placeholder string | Implement provider contract or mark the API explicitly as a mock; test provider failures and fallback | BLOCKING for model claims |
| AUR-GAP-008 | Agent state enforcement | `process_request` previously accepted registered inactive agents | This branch adds inactive-agent rejection and a regression test | CI pending |
| AUR-GAP-009 | UTF-8-safe prompt preview | `ModelHub::inference` sliced a string by byte index and could panic on a multibyte boundary | This branch uses character iteration and adds a regression test | CI pending |
| AUR-GAP-010 | Current exact-SHA verification | `STATUS.md` reports current build/test evidence as UNKNOWN / NOT VERIFIED until linked | Run CI on the proposed PR head and capture run/job/step/exit/log evidence | BLOCKING |

## Required evidence mapping

For every function, complete the following fields before promoting its status beyond `IMPLEMENTED`:

- stable function ID and priority;
- canonical module and contract/API signature;
- input/output schemas and error behavior;
- security, privacy and resource constraints;
- dependencies and failure behavior;
- source path, commit SHA, blob SHA and line range;
- test path and test name;
- exact GitHub Actions Run → Job → Step → exit code/log URL;
- separate implementation, test, CI and integration statuses.

## Architecture invariants

1. Aurora is a user-space AI service; it is not ShivaCore TCB, kernel authority, blockchain consensus, or canonical ATC-VM.
2. Required action path: Intent/Proposal → Policy → Capability Check → Approval when required → Tool/Service Boundary → authoritative runtime.
3. Missing policy, capability, required approval or trusted evidence means deny/fail closed.
4. Model output and retrieved content are untrusted data; they cannot grant authority or override policy.
5. No production-ready status until P0 gaps are closed and exact-SHA CI/security evidence is linked.

## Scope note

This inventory covers only files actually inspected during the current pass. It is intentionally conservative. It is not a complete recursive source-tree census, and absence in this table must not be interpreted as proof that a capability is absent elsewhere in the repository.
