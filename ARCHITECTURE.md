---
document_id: ATC-DOC-AI-003
title: Aurora AI Architecture
version: 2.0.0
status: active
owner: A-TownChain-Okosystems
created: 2026-09-09
updated: 2026-09-30
standard: ATC-STD-MD-001
---

# Aurora AI Architecture

Aurora AI is the intelligence/control-plane userspace of the A-TownChain ecosystem. It is not part of the ShivaCore TCB and has no implicit kernel, governance or blockchain authority.

## Canonical boundary

```
Hardware / Firmware
        ↓
ShivaCore — TCB / capability enforcement
        ↓
GlobusOS — OS services / IPC / resources
        ↓
Aurora AI — models / agents / memory / tools / intelligence
        ↓
Genesis / Applications / A-TownChain interfaces
```

Aurora can propose actions. Authoritative state changes remain owned by the target deterministic domain.

## Canonical AI architecture

1. **Vision & Governance** — lifecycle, risk, oversight, policy and audit.
2. **Identity & Access** — user, service, model and agent identity; AuthN/AuthZ; sessions and secrets.
3. **Model Layer** — model metadata, versions, providers and lifecycle.
4. **ModelHub** — registry, discovery, routing, selection, health and compatibility.
5. **AI Runtime** — inference, context, scheduling, resources and isolation.
6. **Agent System** — lifecycle, planning, decomposition, tools, state and termination.
7. **Capability / Policy** — explicit capabilities, scopes, policy evaluation, approval and revocation.
8. **Tool & Action Layer** — typed tool registry, input/output validation and domain adapters.
9. **Memory** — working, conversation, semantic, episodic and long-term memory.
10. **RAG / Knowledge** — ingestion, parsing, chunking, embeddings, retrieval, ranking and source attribution.
11. **Data Layer** — contracts, lineage, classification, quality and versioning.
12. **AI Safety & Security** — injection defense, exfiltration prevention, sandboxing, fail-closed enforcement and emergency stop.
13. **Evaluation** — correctness, groundedness, robustness, safety and regression evaluation.
14. **Prompt / Context** — system/developer/user context, templates, versioning and injection controls.
15. **Multimodal AI** — text, image, audio, video, STT/TTS, vision and document understanding.
16. **OS Integration** — IPC, process, filesystem, network, devices, GPU/NPU and quotas.
17. **Blockchain Integration** — RPC, state queries, transaction construction and signing requests behind authorization.
18. **Developer Platform** — SDKs, APIs, CLI and extension contracts.
19. **Observability** — inference, agent, tool, resource, policy and security telemetry.
20. **Operations** — deployment, health, scaling, failover, rollback and updates.
21. **Supply Chain** — provenance, signatures, SBOM, pinning and reproducibility.
22. **Testing & Verification** — unit, integration, security, policy, RAG, regression, E2E and exact-SHA evidence.
23. **Evidence & Governance** — provenance, evidence separation, verification and residual tracking.

The implementation decomposition and evidence state are maintained in `docs/AURORA_CANONICAL_FUNCTION_MATRIX.md`.

## Authority chain

```
Model
  ↓
Agent
  ↓
Capability
  ↓
Policy
  ↓
Approval
  ↓
Tool
  ↓
GlobusOS
  ↓
ShivaCore
```

For blockchain operations the target domain additionally validates the transaction and protocol rules before any canonical state transition.

**Invariant:** model output MUST NOT itself authorize an action.

## Current runtime enforcement

The Rust core exposes `CapabilityPolicyEngine` with fail-closed semantics:

- exact principal/capability/action/scope matching;
- explicit approval when required;
- unknown authorization context → DENY;
- unit tests for approval, unknown capability and scope mismatch.

ModelHub prompt previews are UTF-8 safe.

## Evidence semantics

```
Architecture ≠ Implementation ≠ Test ≠ CI Evidence ≠ Verification
Error Evidence ≠ Finding Evidence ≠ Verification Evidence
```

Exact-SHA CI evidence remains mandatory for VERIFIED status.

## Residual boundary

The canonical architecture is broader than the current runtime. Provider-specific production inference, full production RAG, multimodal providers, complete agent planning/execution, blockchain signing execution, deployment automation and end-to-end exact-SHA verification remain separate implementation gates until source and CI evidence exists.
