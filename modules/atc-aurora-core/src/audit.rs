// Copyright (c) 2026 Michael Wroblewski / ShivaCore / A-TownChain-Okosystems. All Rights Reserved.
// Append-only in-memory audit reference implementation. Persistence/export is intentionally
// outside this core module and must be provided by an authorized runtime adapter.

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuditRecord {
    pub event_id: String,
    pub request_id: String,
    pub actor: String,
    pub action: String,
    pub outcome: String,
    pub detail: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuditError {
    EmptyEventId,
    EmptyRequestId,
    DuplicateEventId,
}

#[derive(Debug, Default)]
pub struct AuditLogger {
    records: Vec<AuditRecord>,
}

impl AuditLogger {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn append(&mut self, record: AuditRecord) -> Result<(), AuditError> {
        if record.event_id.is_empty() {
            return Err(AuditError::EmptyEventId);
        }
        if record.request_id.is_empty() {
            return Err(AuditError::EmptyRequestId);
        }
        if self
            .records
            .iter()
            .any(|existing| existing.event_id == record.event_id)
        {
            return Err(AuditError::DuplicateEventId);
        }
        self.records.push(record);
        Ok(())
    }

    pub fn records(&self) -> &[AuditRecord] {
        &self.records
    }

    pub fn records_for_request(&self, request_id: &str) -> Vec<&AuditRecord> {
        self.records
            .iter()
            .filter(|record| record.request_id == request_id)
            .collect()
    }

    pub fn len(&self) -> usize {
        self.records.len()
    }

    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record(event_id: &str) -> AuditRecord {
        AuditRecord {
            event_id: event_id.into(),
            request_id: "request:1".into(),
            actor: "agent:assistant".into(),
            action: "filesystem.read".into(),
            outcome: "Allow".into(),
            detail: "unit test".into(),
        }
    }

    #[test]
    fn audit_log_is_append_only_and_queryable() {
        let mut audit = AuditLogger::new();
        audit.append(record("event:1")).unwrap();
        audit.append(record("event:2")).unwrap();
        assert_eq!(audit.len(), 2);
        assert_eq!(audit.records_for_request("request:1").len(), 2);
    }

    #[test]
    fn duplicate_audit_event_is_rejected() {
        let mut audit = AuditLogger::new();
        audit.append(record("event:1")).unwrap();
        assert_eq!(
            audit.append(record("event:1")),
            Err(AuditError::DuplicateEventId)
        );
        assert_eq!(audit.len(), 1);
    }
}
