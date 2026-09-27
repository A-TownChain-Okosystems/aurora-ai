---
document_id: ATC-DOC-AI-003
title: Aurora AI Architecture
version: 1.0.0
status: active
owner: A-TownChain-Okosystems
created: 2026-09-09
updated: 2026-09-09
standard: ATC-STD-MD-001
---

# Aurora AI Architecture

> Technische Architektur der KI-Services und Agenten-Infrastruktur.

## Systemübersicht

Aurora AI ist als hybrid strukturierte Dual-Engine aufgebaut:

1. **Rust Core Engine (`modules/atc-aurora-core`):** Performantes Scheduling, IPC, HAL und Sicherheitsprüfungen.
2. **Agent Framework (`modules/atc-aurora-agents`):** Agent-Lifecycle-Steuerung und Rollenverteilung.
3. **Memory Layer (`modules/atc-aurora-memory`):** Vektorspeicher und temporärer/persistenter Kontext.
4. **Runtime Layer (`modules/atc-aurora-runtime`):** Laufzeit-Umgebung mit Event-Bridge zum Kernel.
5. **Python AI Services (`modules/atc-aurora-ai`):** DefenderGPT, MinerWatcherGPT und KAI-Anbindung.
6. **AI Studio (`modules/atc-aistudio`):** Entwickler-Suite und Asset-Management.

## Datenfluss

```text
Kernel Event Bridge -> Rust Core IPC -> Agent Runtime -> Python AI Stack (DefenderGPT/MinerWatcherGPT) -> Memory
```


---

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

## A-TownChain Boundary

Aurora may request blockchain operations only through the canonical interface:

```text
AURORA
   │ request
   ▼
A-TownChain Interface
   ▼
Node
   ▼
Consensus
   ▼
ATC-VM
   ▼
State
```

Aurora may propose, plan and request operations. Deterministic chain components retain authority over consensus, VM execution and authoritative state transitions.

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

No Aurora model, agent or tool may directly bypass this boundary.

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
