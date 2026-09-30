---
document_id: ATC-AI-AURORA-MATRIX-001
title: Aurora AI — Canonical Vision → Components → Functions → Evidence Matrix
version: 1.0.0
status: active
owner: A-TownChain-Okosystems
updated: 2026-09-30
---

# Aurora AI — Canonical Function Matrix

This document is the implementation contract for the Aurora AI platform. It does not convert architectural presence into implementation or verification evidence.

## Canonical architecture

```
AI Governance
    ↓
Identity / Access
    ↓
ModelHub
    ↓
AI Runtime
    ↓
Agent System
    ↓
Capability / Policy / Approval
    ↓
Tool & Action Layer
    ↓
Memory + RAG + Data
    ↓
Evaluation + Safety + Context + Multimodal
    ↓
GlobusOS / A-TownChain interfaces
    ↓
ShivaCore enforcement boundary
    ↓
Hardware
```

## Authority invariant

```
Model output
  → Agent proposal
  → Capability check
  → Policy evaluation
  → Approval gate where required
  → Tool invocation
  → Domain validation
  → Authoritative state transition
```

Model output MUST NOT directly grant capability, authorization, OS privilege, signing authority, blockchain authority, governance authority or canonical state mutation.

## Domain matrix

| ID | Domain | Canonical functions | Primary implementation boundary |
|---|---|---|---|
| AI-01 | Vision & Governance | policy, risk classification, oversight, lifecycle, audit | docs / atc-standards |
| AI-02 | Identity & Access | user/service/model/agent identity, AuthN/AuthZ, sessions, secrets | GlobusOS + Aurora |
| AI-03 | Model Layer | model metadata, versions, local/remote providers, lifecycle | ModelHub |
| AI-04 | ModelHub | registry, discovery, routing, selection, health, compatibility | atc-aurora-core |
| AI-05 | AI Runtime | inference, context, scheduling, CPU/GPU/NPU, quotas, isolation | atc-aurora-runtime |
| AI-06 | Agent System | lifecycle, planning, decomposition, tool selection, state, termination | atc-aurora-agents |
| AI-07 | Capability / Policy | capabilities, scopes, policy evaluation, approval, revocation | capability_policy + GlobusOS |
| AI-08 | Tool & Action | registry, schemas, validation, OS/network/filesystem/chain adapters | runtime / integration |
| AI-09 | Memory | working, conversational, semantic, episodic, long-term, retention | atc-aurora-memory |
| AI-10 | RAG / Knowledge | ingestion, parsing, chunking, embeddings, retrieval, ranking, attribution | atc-aurora-memory |
| AI-11 | Data | contracts, lineage, classification, quality, versioning | data/integration boundary |
| AI-12 | AI Safety & Security | injection defense, exfiltration prevention, sandboxing, fail-closed, emergency stop | policy/security boundary |
| AI-13 | Evaluation | accuracy, groundedness, robustness, safety, regression, E2E | evaluation/CI |
| AI-14 | Prompt / Context | system/developer/user context, templates, versioning, injection detection | runtime |
| AI-15 | Multimodal | text/image/audio/video, STT/TTS, vision, documents | provider/runtime boundary |
| AI-16 | OS Integration | IPC, processes, filesystem, network, devices, GPU/NPU, quotas | GlobusOS |
| AI-17 | Blockchain Integration | wallet, RPC, tx construction, signing request, state queries, contract calls | A-TownChain interfaces |
| AI-18 | Developer Platform | SDKs, APIs, CLI, tool/model/agent extension contracts | Aurora developer surface |
| AI-19 | Observability | traces, tool logs, policy decisions, resource/error telemetry, audit | runtime / governance |
| AI-20 | Operations | deployment, health, scaling, failover, rollback, updates | operations |
| AI-21 | Supply Chain | provenance, signing, SBOM, version pinning, reproducibility | release/security |
| AI-22 | Testing & Verification | unit, integration, security, policy, RAG, regression, E2E, exact-SHA | CI / evidence |
| AI-23 | Evidence & Governance | evidence provenance, status separation, verification records, residuals | atc-standards + ecosystem |

## Evidence contract

Every implemented function is classified independently:

```
Architecture
  → Source Discovery
  → Implementation
  → Contract
  → Test
  → CI
  → Exact-SHA Evidence
  → Verification
  → Residual
```

The following are never interchangeable:

```
Error Evidence ≠ Finding Evidence ≠ Verification Evidence
Source present ≠ Implemented
Implemented ≠ Tested
Tested ≠ CI Verified
CI Verified ≠ E2E Verified
```

## Current concrete implementation in this change

- `CapabilityPolicyEngine`: fail-closed capability/scope/action authorization.
- Approval is enforced when a policy requires it.
- Unknown capability/principal/action/scope returns DENY.
- ModelHub inference preview is UTF-8 safe.
- The complete 23-domain matrix is now the canonical implementation decomposition.

## Explicit residuals

The matrix is broader than the currently implemented runtime. Provider-specific inference, production-grade RAG, multimodal backends, full agent planning, blockchain signing execution, deployment automation and complete exact-SHA verification remain separate implementation/evidence gates until corresponding source and CI evidence exists.

No item in this document is a production-readiness claim.
