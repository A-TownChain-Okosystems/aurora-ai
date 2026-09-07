# aurora-ai [L2]

Aurora AI — Rust Core + Python AI-Layer (AD-021): Agents, Memory, Runtime, AI Studio.

**Vault-Restauration (07.09.2026, AD-020/026/027):** Inhalt aus dem Wiki-Vault
(docs/archive/monorepo-full/) restauriert — vor der Repo-Leerung byte-identisch gesichert. Keine — Vault-Stand konsistent (Chain-ID 658467).

**Module:** atc-aurora-core, atc-aurora-agents, atc-aurora-memory, atc-aurora-runtime, atc-aurora-ai, atc-aistudio

**Meile (AD-027):** M3 — KI laeuft: Agent-Task Ende-zu-Ende via Kernel-Event-Bridge mit Capability-Token

**Hinweis:** Basis fuer den Rebuild; Gate-Kriterien laut LAUFFAEHIGKEITS_ROADMAP
(a-townchain-os-docs/docs/roadmap/).

---

## ATC Compliance & Governance (ATC-STD-201 / 202 / 203)

**ATC COMPLIANCE: R3** — auditiert am 2026-09-07 (atc-repo-audit; R-Level aus `.atc/repository.yaml`).
Architekturentscheidungen: zentral im [DECISIONS_REGISTER](https://github.com/A-TownChain-Okosystems/a-townchain-os-docs/blob/main/docs/DECISIONS_REGISTER.md) (AD-Nummern verbindlich; lokale Entscheidungen in `docs/decisions/`).

- **Purpose:** Aurora AI — der AI-Stack des Oekosystems (L2).
- **Scope:** Layer L2, Domain ai — aurora-ai als AI in der 23-Repo-Landschaft (AD-024/026).
- **Architecture:** Rust Core (Model Manager, Scheduler, HAL, Security, IPC) + Python AI-Layer (AD-021); Event-Bridge zum Kernel (M3).
- **Features:** 6 Module (aurora-core/agents/memory/runtime/ai + aistudio); KAI-Integration.
- **Installation:** Modul-Build je Sprache (rust); Integration via Monorepo-Workspace (a-townchain-os, sync_modules.py).
- **Development:** Conventional Commits; Governance-Regeln aus atc-standards; Naming gemaess ATC-STD-000 §7.
- **Testing:** cargo/pytest je Modul; Integration via Monorepo-Workspace.
- **Security:** SECURITY.md; S-Klasse S2; ATC-STD-203 Release-Gates; Emergency-Prozess ATC-STD-000 §32.
- **Roadmap:** Einordnung in die Lauffaehigkeits-Roadmap M1-M8 (AD-027) und Bauhierarchie L0-L7 (AD-026).
- **Version:** CHANGELOG.md; SemVer; Releases als ATC-REL-X.Y.Z.
- **License:** Proprietaer — All Rights Reserved, Michael Wroblewski / ShivaCore / A-TownChain-Okosystems (ATC-LIC/ATS-LIC).
