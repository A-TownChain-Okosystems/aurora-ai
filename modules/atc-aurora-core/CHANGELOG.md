# 📝 Changelog — atc-aurora-core

## [Unreleased] — 2026-10-18

### Hinzugefügt
- UUIDv4 generation for agent IDs and execution run IDs.
- JSON registry snapshots that preserve agent identity and revocation state across process restarts.
- Agent-bound task assignment with distinct swarm_id, task_id, agent_id, agent_role, capabilities and run_id.
- Fail-closed dispatch for assigned tasks when the agent is missing or revoked.
- Result attribution now requires task_id, agent_id and run_id.
- Structured agent/swarm/task/run attribution and agent-based lookup for audit records and execution/security events.
- Tests for duplicate identities, concurrent registration, persistence/reload, revocation and run-aware result aggregation.

### Verification status
- Changes are on a feature branch and are not yet verified.
- Cargo build/test passed on the earlier head, before the latest audit/event attribution edits; checks must pass again on the final PR head SHA.
- Formatting checks and remaining required CI must pass on the exact PR head SHA before this is marked verified.
- Registry persistence is opt-in through save_json/load_json; deployment code must call these methods and protect the storage path.
- Metrics-backend integration is not included.

## [2026-08-08] — Initial
- Initial-Repository erstellt
- ARCHITECTURE.md, COMPONENT_PLAN.md, STATUS.md hinzugefügt

---
*Auto-generiert 2026-10-18 · Aurora (MasterBrain · Base44)*
