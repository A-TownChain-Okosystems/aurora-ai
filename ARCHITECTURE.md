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
