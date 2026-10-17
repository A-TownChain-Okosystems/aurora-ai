# 📝 Changelog — atc-aurora-core

## [Unreleased] — 2026-10-18

### Hinzugefügt
- UUIDv4 generation for agent IDs and execution run IDs.
- JSON registry snapshots that preserve agent identity and revocation state across process restarts.
- Agent-bound task assignment with distinct swarm_id, task_id, agent_id, agent_role, capabilities and run_id.
- Fail-closed dispatch for assigned tasks when the agent is missing or revoked.
- Result attribution now requires task_id, agent_id and run_id.
- Tests for duplicate identities, concurrent registration, persistence/reload, revocation and run-aware result aggregation.

### Verification status
- Changes are on a feature branch and are not yet verified.
- Required Cargo tests, formatting checks and CI must pass on the exact PR head SHA before this is marked verified.
- Registry persistence is opt-in through save_json/load_json; deployment code must call these methods and protect the storage path.

## [2026-08-08] — Initial
- Initial-Repository erstellt
- ARCHITECTURE.md, COMPONENT_PLAN.md, ROADMAP.md, STATUS.md hinzugefügt

---
*Auto-generiert 2026-10-18 · Aurora (MasterBrain · Base44)*
