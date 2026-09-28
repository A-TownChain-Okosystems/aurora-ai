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

## System Role

Aurora is the intelligent system of the ecosystem. It is not the ShivaCore TCB, not blockchain consensus authority, and not canonical blockchain state authority.

## Canonical Layering

```
AURORA AI
    |
    v
Aurora Authority Plane
    |
    v
Aurora Runtime
    |
    v
ATC AI Runtime
    |
    +---- CPU Backend
    +---- GPU Backend
    +---- NPU Backend
    |
    v
GlobusOS AI Services
    |
    v
ShivaCore IPC / Capability Boundary
```

## Aurora Authority Plane

Responsibilities:

- capability evaluation
- policy evaluation
- approval requirements
- identity binding
- tool authorization
- audit and trust records
- explicit denial of unauthorized operations

Aurora has no implicit hardware or kernel authority.

## Aurora Runtime

Responsibilities:

- agent execution
- scheduling
- context management
- model lifecycle
- inference orchestration
- execution routing
- multimodal orchestration

The runtime executes only requests admitted by the applicable authority and policy contracts.

## ATC AI Runtime

Responsibilities:

- ATC Model ABI
- model loading and verification
- graph representation
- tensor execution
- compilation/planning
- quantization
- backend selection
- CPU/GPU/NPU execution

The ATC AI Runtime is owned by the ATC/Aurora architecture and is not delegated to an external platform AI runtime.

## Hardware Backends

```
Aurora Runtime
      |
      +---- CPU Backend
      +---- GPU Backend
      +---- NPU Backend
```

Backends expose capability contracts rather than vendor-specific semantics to Aurora.

## Canonical Execution Path

```
Aurora Agent
  -> Capability Request
  -> Aurora Authority Plane
  -> ATC Model ABI
  -> Model Verification
  -> ATC AI Runtime
  -> Execution Planner
  -> CPU/GPU/NPU
  -> Verified Execution
  -> Result Validation
  -> Aurora
```

## Security Boundary

```
Hardware Root of Trust
  -> Secure Boot
  -> Measured Boot
  -> ShivaCore
  -> GlobusOS
  -> Aurora Runtime
  -> Model Verification
  -> Capability / Policy
  -> AI Execution
```

Aurora does not directly access:

- kernel memory
- physical memory
- device registers
- unrestricted DMA
- TPM private key material
- firmware replacement interfaces

All privileged operations cross explicit IPC/capability boundaries.

## Hardware Target

The canonical hardware target is ATC AI Compute Platform:

```
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

Hardware capability discovery is not implementation or security evidence.

## Existing Aurora Components

1. Rust Core Engine (modules/atc-aurora-core) — scheduling, IPC and security-sensitive runtime primitives.
2. Agent Framework (modules/atc-aurora-agents) — agent lifecycle and role management.
3. Memory Layer (modules/atc-aurora-memory) — memory/context services.
4. Runtime Layer (modules/atc-aurora-runtime) — runtime and integration services.
5. AI Services (modules/atc-aurora-ai) — model/AI services.
6. AI Studio (modules/atc-aistudio) — development tooling.

These components do not individually become the system authority merely by existing.

## Status Semantics

Architecture, implementation, testing, CI verification, integration and E2E verification remain independent states:

```
ARCHITECTURE_ONLY
  -> SPECIFIED
  -> IMPLEMENTED
  -> TESTED
  -> CI_VERIFIED
  -> INTEGRATED
  -> E2E_VERIFIED
```

**No Evidence, No Trust.**
