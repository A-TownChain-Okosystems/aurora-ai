# ATC Aurora AI

> KI-Dienste und Agenten-Infrastruktur des A-TownChain-Ökosystems (Rust Core, Python AI-Layer, DefenderGPT, MinerWatcherGPT).

**Project:** aurora-ai
**Organization:** A-TownChain-Okosystems
**Status:** `development`
**Version:** `0.1.0`
**License:** `Apache-2.0` (SPDX, Org-weite Einheitslizenz per AD-F-046)

| ATC COMPLIANCE | ![Status](https://img.shields.io/badge/ATC--201%2F202%2F203-R3_COMPLIANT-00c853) R3 · Repository Governance (ATC-STD-201/202/203) |
|---|---|

## Overview

Aurora AI stellt die zentrale KI-Infrastruktur und KI-Dienste (Layer L2, Domain ai) des A-TownChain-Ökosystems bereit. Das System verbindet eine hochperformante Rust-Core-Engine (Scheduler, Model Manager, IPC, Memory Management) mit einer flexiblen Python-AI-Schicht (Agenten, DefenderGPT, MinerWatcherGPT, KAI-Integration).

Aurora AI ermöglicht autonome Agentensteuerung, Echtzeit-Bedrohungserkennung und dezentrale KI-Inferenz innerhalb der A-TownChain-Architektur.

## Purpose

ATC Aurora AI provides the canonical implementation of AI services and agent infrastructure within the A-TownChain ecosystem. It is responsible for:

- Bereitstellung autonomer AI-Services (DefenderGPT für Security-Monitoring, MinerWatcherGPT für Telemetrie/Anomalie-Erkennung).
- Ausführung von Agenten-Workflows über das Rust-Core-Framework (`atc-aurora-core`, `atc-aurora-agents`, `atc-aurora-runtime`).
- Verwaltung von persistentem Agenten-Speicher (`atc-aurora-memory`) und Vektor-Kontexten.
- Anbindung der AI Studio Entwicklungsumgebung (`atc-aistudio`) zur Agenten-Erstellung und -Simulation.

Davon hängen ab: `a-townchain-os`, `atc-security-shield`, Kernel-Event-Bridge und Ökosystem-Management-Tools.

## Status

**Status:** `development` — Rebuild-Status v0.1.0 (AD-020/AD-021/AD-027). Meilenstein M3 (KI läuft: Agent-Task Ende-zu-Ende via Kernel-Event-Bridge mit Capability-Token) ist aktiv.

## Architecture

### Components

- `modules/atc-aurora-core`: Rust Core Engine (Model Manager, Scheduler, HAL, IPC, Security).
- `modules/atc-aurora-agents`: Agent-Framework und Rollenverwaltung.
- `modules/atc-aurora-memory`: Vektorspeicher, Kontextverwaltung und Speicherbereinigung.
- `modules/atc-aurora-runtime`: Laufzeitumgebung und Kernel-Event-Bridge.
- `modules/atc-aurora-ai`: Python-AI-Services (DefenderGPT, MinerWatcherGPT, KAI-Orchestrierung).
- `modules/atc-aistudio`: AI Studio UI und Visualisierungs-Tools.

### Data Flow

```text
Kernel Event Bridge / IPC -> Rust Core Scheduler -> Agent Runtime -> Python AI Layer (DefenderGPT/MinerWatcherGPT) -> Memory Layer
```

### Dependencies

| Component | Purpose | Required |
|---|---|---|
| Rust Engine (`atc-aurora-core`) | Performantes Scheduling & Inter-Process Communication | Yes |
| Python AI Stack (`atc-aurora-ai`) | DefenderGPT, MinerWatcherGPT & KAI Services | Yes |
| Agent Memory (`atc-aurora-memory`) | Kontext- und Vektorspeicher | Yes |
| Kernel Event Bridge | Synchronisation mit a-townchain-os | Yes |

## Features

- **DefenderGPT:** Autonome Bedrohungserkennung und Security-Audit-Überwachung.
- **MinerWatcherGPT:** Telemetrie-Analyse und Leistungsoptimierung für Miner/Knoten.
- **Dual-Engine-Architektur:** Rust-Performance-Core kombiniert mit Python-AI-Flexibilität.
- **Kernel-Event-Bridge:** Ende-zu-Ende-Eventverarbeitung mit Capability-Tokens (M3).
- **AI Studio:** Integrierte Suite zur Modellierung und Entwicklung von AI-Assets.

## Repository Structure

```text
.
├── .atc/
├── .github/
├── docs/
└── modules/
    ├── atc-aistudio/
    ├── atc-aurora-agents/
    ├── atc-aurora-ai/
    ├── atc-aurora-core/
    ├── atc-aurora-memory/
    └── atc-aurora-runtime/
```

## Requirements

- **Rust:** 1.75+ (für `atc-aurora-core`, `atc-aurora-agents`, `atc-aurora-memory`, `atc-aurora-runtime`)
- **Python:** 3.10+ (für `atc-aurora-ai`)
- **Node.js:** 18+ (für `atc-aistudio`)
- **Cargo & Pip:** Aktuelle Versionen

## Installation

```bash
# Repository klonen
git clone https://github.com/A-TownChain-Okosystems/aurora-ai.git
cd aurora-ai

# Rust-Komponenten bauen
cd modules/atc-aurora-core && cargo build --release && cd ../..

# Python-AI-Services installieren
cd modules/atc-aurora-ai && pip install -r requirements.txt && cd ../..
```

## Configuration

Die Konfiguration erfolgt über Umgebungsvariablen und `.atc/repository.yaml`:

```bash
export AURORA_LOG_LEVEL=info
export AURORA_CORE_IPC_PATH=/tmp/aurora-ipc.sock
```

## Usage

```bash
# Starten der AI-Services
python3 modules/atc-aurora-ai/src/main.py

# Ausführen von DefenderGPT Monitoring
python3 modules/atc-aurora-ai/agents/defender_gpt.py
```

## Development

Entwicklungs-Guidelines und Workflows folgen den A-TownChain-Standards (ATC-STD-000, ATC-STD-201):

```bash
# Entwicklungs-Setup für Rust-Module
cd modules/atc-aurora-core && cargo check
```

## Testing

Testergebnisse und Suites werden je Modul ausgeführt:

```bash
# Rust-Tests ausführen
cd modules/atc-aurora-core && cargo test

# Python-Tests ausführen
cd modules/atc-aurora-ai && pytest

# Erwartetes Ergebnis: PASS (0 Errors)
```

## Security

Sicherheitsrelevante Schwachstellen dürfen **NICHT** öffentlich über GitHub Issues gemeldet werden. Bitte beachten Sie den offiziellen ATC-Security-Reporting-Prozess in `SECURITY.md` sowie ATC-STD-203.

- **Sicherheitsklasse:** S2
- **Emergency-Prozess:** ATC-STD-000 §32

## Documentation

Weiterführende Dokumentation befindet sich im `docs/`-Verzeichnis sowie in den einzelnen Modulen:

- `docs/REPOSITORY_STANDARD.md`
- `ARCHITECTURE.md`
- `AGENTS.md`

## Governance

Änderungen an Architektur, Schnittstellen und Governance unterliegen dem A-TownChain Enterprise Governance Framework (ATC-STD-201/202). Review- und Approval-Pflicht für sicherheitskritische Änderungen.

## Standards & Compliance

| Standard | Version | Compliance |
|---|---:|---|
| ATC-STD-000 | 1.2.0 | ✅ |
| ATC-STD-201 | 1.0.0 | ✅ |
| ATC-STD-202 | 1.1.0 | ✅ |
| ATC-STD-203 | 1.0.0 | ✅ |
| ATC-STD-README-001 | 1.0.0 | ✅ |
| ATC-STD-MD-001 | 1.0.0 | ✅ |

## Roadmap

Kanonische Roadmap-Quellen:

- [ROADMAP.md](ROADMAP.md) (Repository-Wurzel)
- A-TownChain Master Roadmap (`a-townchain-os-docs`)
- GitHub Issues & Projects

## Contributing

Beiträge sind willkommen. Bitte lesen Sie vorab [CONTRIBUTING.md](CONTRIBUTING.md) und beachten Sie [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md).

## License

Apache-2.0 — Copyright Michael Wroblewski / A-TownChain-Okosystems (siehe [LICENSE](LICENSE)). Lizenz-Divergenz README-vs.-LICENSE behoben gem. Owner-Entscheidung F-046 (SCR-0036).

## Maintainers

**Organization:** A-TownChain-Okosystems  
**Owner:** Michael Wroblewski  
**Maintainer:** ATC-AI-ARCH-001 (Aurora #1)

## Repository Metadata

<!-- atc metadata block (ATC-STD-README-001 §14) -->
<!--
atc:
  standard: ATC-STD-README-001
  version: 1.0.0
repository:
  id: ATC-REPO-AI-001
  name: aurora-ai
  type: software
  status: development
ownership:
  organization: A-TownChain-Okosystems
technology:
  primary_language: Rust/Python
governance:
  security_class: S2
  criticality: HIGH
-->
