# Aurora AI — KI-System

> **Produkt-Repo des A-TownChain-Ökosystems** · [Monorepo](https://github.com/A-TownChain-Okosystems/a-townchain-os) · [Docs-Hub](https://github.com/A-TownChain-Okosystems/a-townchain-os-docs) · Mainnet: **15.09.2026**

Aurora ist das KI-System des Ökosystems: Agenten-Laufzeit (12 Agenten via Macro), KnowledgeBase & VectorStore, ModelHub & LlmRouter, ContextWindow, AI-Studio. Kernel-Integration ausschließlich über Event-Bridge + Capability-Tokens (AD-012), nie direkter Kernel-Speicher.

## Module (aus Monorepo `src/modules/` überführt)

| Modul | Dateien | Zeilen |
|---|---|---|
| `atc-aurora-core` | 21 | 775 |
| `atc-aurora-agents` | 26 | 1,105 |
| `atc-aurora-memory` | 21 | 730 |
| `atc-aurora-runtime` | 14 | 475 |
| `atc-aurora-ai` | 32 | 1,040 |
| `atc-aistudio` | 263 | 76,045 |
| **Total** | **377** | **80,170** |

## Richtlinien

- Architektur-Vorgaben: AD-012/AD-013 (ShivaCore Microkernel, Gate v1.1) — siehe Docs-Hub `docs/architecture/`
- Neue Produkt-Entwicklung läuft hier; Integration & Deployment über das Monorepo
- Standards: ATC-01…35 · ATS-1000…1007 · Lizenz: All Rights Reserved (Michael Wroblewski / ShivaCore / A-TownChain-Okosystems)

*Eingerichtet am 06.09.2026 durch Agent Aurora (Base44) im Auftrag des Owners.*
