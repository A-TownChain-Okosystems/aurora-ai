---
document_id: ATC-DOC-AI-003
title: Aurora AI Architecture
version: 2.0.0
status: active
owner: A-TownChain-Okosystems
created: 2026-09-09
updated: 2026-09-28
standard: ATC-STD-MD-001
---

# Aurora AI Architecture

> Canonical architecture of Aurora AI as the ecosystem intelligence platform.

**Status:** ARCHITECTURE_ONLY  
**Normative authority:** atc-standards

# Canonical System Role — GlobusOS AI Control Plane

Aurora AI is the **AI Control / Intelligence Plane of GlobusOS**.

The canonical system boundary is:

```text
AURORA AI
    │
    │ Intelligence / Planning / Agent Execution
    ▼
GLOBUS OS
    │
    │ OS Services / IPC / Resource Management
    ▼
SHIVACORE
    │
    │ Kernel / TCB / Capabilities / Isolation
    ▼
HARDWARE
```

Aurora is **not** the operating-system kernel and does not own blockchain consensus authority. GlobusOS remains the operating-system integration and service layer; ShivaCore remains the trusted kernel / TCB / capability boundary.

## Aurora Responsibilities

The Aurora platform provides the AI-side capabilities for:

- ModelHub and model/provider integration
- Agent runtime and lifecycle
- Planning and context processing
- Skills and tools
- Memory, RAG and knowledge services
- Multimodal processing
- Dialogue AI, Quest AI, World AI, Character AI, Creature AI, Faction AI and Security AI
- AI-based system analysis
- Authorized automation
- Evaluation, telemetry, provenance and evidence
- Authorized integration with Genesis, A-TownChain and external services

These are architectural responsibilities. Their implementation status is determined exclusively by **AURORA-001 evidence**.

## Canonical Layering

```text
AURORA AI
    │
    ▼
Aurora Authority Plane
    │
    ▼
Aurora Runtime
    │
    ▼
ATC AI Runtime
    │
    +---- CPU Backend
    +---- GPU Backend
    +---- NPU Backend
    │
    ▼
GlobusOS AI Services
    │
    ▼
ShivaCore IPC / Capability Boundary
```

## Authority Boundary

Aurora follows this authority chain:

```text
MODEL
  │ proposes
  ▼
AGENT
  │ requests
  ▼
CAPABILITY
  │ checked by
  ▼
POLICY
  │ may require
  ▼
APPROVAL
  │ authorizes
  ▼
TOOL
  │ executes
  ▼
GLOBUS OS
  │ capability + IPC
  ▼
SHIVACORE
```

The following invariants are mandatory:

- AI models have no implicit execution authority.
- Agents cannot bypass capability checks.
- Capabilities are subject to policy evaluation.
- Approval may be required for sensitive operations.
- Tools execute only through authorized interfaces.
- Aurora has no direct kernel authority.
- Aurora has no direct blockchain consensus authority.
- All privileged execution must be auditable.

## Hardware Target & Backends

The canonical hardware target is ATC AI Compute Platform (ATC-AICP):

```text
ATC-AICP
├── CPU
├── GPU
├── NPU
├── MEMORY
├── STORAGE
├── SECURITY
│   ├── TPM
│   ├── SECURE_PROCESSOR
│   └── TEE
├── BOOT
│   ├── SECURE_BOOT
│   └── MEASURED_BOOT
└── FIRMWARE
```

```text
Aurora Runtime
      │
      +---- CPU Backend
      +---- GPU Backend
      +---- NPU Backend
```

Backends expose capability contracts rather than vendor-specific semantics to Aurora.

## Security Boundary

Aurora communicates with the kernel only through authorized GlobusOS interfaces:

```text
Aurora
   │
   ▼
Authorized GlobusOS Interface
   │
   ▼
IPC / Capability Boundary
   │
   ▼
ShivaCore
```

Aurora does not directly access kernel memory, physical memory, device registers, unrestricted DMA, TPM private key material, or firmware replacement interfaces.

## Completeness and Authority Gate

Every Aurora capability is subject to **AURORA-001 — Platform Completeness & Authority Gate**.

Evidence progression:

```text
ARCHITECTURE_ONLY
        ↓
SPECIFIED
        ↓
IMPLEMENTED
        ↓
TESTED
        ↓
CI_VERIFIED
        ↓
INTEGRATED
        ↓
E2E_VERIFIED
```

Exceptional states are:

```text
MISSING / BLOCKED / DUPLICATE / DISCONNECTED
```

A file, documentation entry or module directory alone is not implementation evidence. CI evidence is valid only when tied to the exact source commit being assessed.
