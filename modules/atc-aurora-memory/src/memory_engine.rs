// Copyright (c) 2026 A-TownChain-Okosystems. All Rights Reserved.
// Aurora Memory Engine — in-process MVP contracts.
// Persistent storage, encryption-at-rest and OS capability integration are intentionally
// adapter responsibilities and are NOT implemented by this module.

use std::collections::{HashMap, VecDeque};

const ALLOWED_ROLES: &[&str] = &["user", "assistant", "tool", "system"];

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AccessPrincipal {
    pub tenant_id: String,
    pub user_id: String,
    pub agent_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct SessionKey {
    tenant_id: String,
    user_id: String,
    agent_id: String,
    session_id: String,
}

impl SessionKey {
    fn new(principal: &AccessPrincipal, session_id: &str) -> Self {
        Self {
            tenant_id: principal.tenant_id.clone(),
            user_id: principal.user_id.clone(),
            agent_id: principal.agent_id.clone(),
            session_id: session_id.to_owned(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShortTermMessage {
    pub role: String,
    pub content: String,
    pub created_at: u64,
    pub expires_at: Option<u64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryKind {
    Preference,
    ProjectFact,
    Episode,
    SemanticFact,
    Procedure,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrustLevel {
    Untrusted,
    UserProvided,
    VerifiedSource,
    SystemApproved,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Provenance {
    /// Stable source identifier (document ID, conversation turn ID, or tool result ID).
    pub source_id: String,
    pub source_kind: String,
    pub observed_at: u64,
    /// Optional version, revision, or source commit.
    pub source_version: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemoryDraft {
    pub kind: MemoryKind,
    pub content: String,
    pub provenance: Provenance,
    pub trust: TrustLevel,
    /// An approval decision from the trusted application layer, never model output.
    pub approved_by_user: bool,
    /// Optional absolute Unix timestamp.
    pub expires_at: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemoryEntry {
    pub id: u64,
    pub tenant_id: String,
    pub owner_user_id: String,
    /// When present, only this agent within the owner scope can read the entry.
    pub owner_agent_id: Option<String>,
    pub kind: MemoryKind,
    pub content: String,
    pub provenance: Provenance,
    pub trust: TrustLevel,
    pub approved_by_user: bool,
    pub created_at: u64,
    pub expires_at: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MemoryError {
    InvalidInput(&'static str),
    ApprovalRequired,
    UntrustedContent,
    AccessDenied,
    NotFound,
    DerivedArtifactCleanupFailed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContextItem {
    pub memory_id: u64,
    pub content: String,
    pub source_id: String,
    pub source_version: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuiltContext {
    pub session_messages: Vec<ShortTermMessage>,
    pub long_term_items: Vec<ContextItem>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExplanationSource {
    pub source_id: String,
    pub memory_id: Option<u64>,
    pub source_version: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExplanationRecord {
    pub task_understood: String,
    /// References only sources explicitly supplied by the caller as actually used.
    pub sources_used: Vec<ExplanationSource>,
    pub decision_summary: String,
    pub rationale_summary: String,
    pub uncertainty: Vec<String>,
    pub next_action: String,
    pub generated_at: u64,
}

/// Adapter contract: implementations must delete every derived artifact for the memory
/// (embeddings, indexes, summaries and caches) or return an error. The canonical record
/// is retained if cleanup fails.
pub trait DerivedArtifactCleaner {
    fn delete_by_memory_id(&mut self, memory_id: u64) -> Result<(), MemoryError>;
}

pub struct MemoryManager {
    short_term: HashMap<SessionKey, VecDeque<ShortTermMessage>>,
    long_term: HashMap<u64, MemoryEntry>,
    next_id: u64,
    max_short_term_messages: usize,
    max_entry_chars: usize,
}

impl MemoryManager {
    pub fn new(max_short_term_messages: usize, max_entry_chars: usize) -> Self {
        Self {
            short_term: HashMap::new(),
            long_term: HashMap::new(),
            next_id: 1,
            max_short_term_messages: max_short_term_messages.max(1),
            max_entry_chars: max_entry_chars.max(1),
        }
    }

    /// Adds a session-scoped message. Short-term state is isolated by tenant, user,
    /// agent and session and is bounded by message count.
    pub fn append_message(
        &mut self,
        principal: &AccessPrincipal,
        session_id: &str,
        role: &str,
        content: &str,
        now: u64,
        ttl_seconds: Option<u64>,
    ) -> Result<(), MemoryError> {
        self.validate_principal(principal)?;
        if session_id.trim().is_empty() || !ALLOWED_ROLES.contains(&role) {
            return Err(MemoryError::InvalidInput("invalid session or role"));
        }
        self.validate_content(content)?;
        let expires_at = ttl_seconds.map(|ttl| now.saturating_add(ttl));
        let key = SessionKey::new(principal, session_id);
        let messages = self.short_term.entry(key).or_default();
        messages.retain(|message| message.expires_at.map_or(true, |expiry| expiry > now));
        while messages.len() >= self.max_short_term_messages {
            messages.pop_front();
        }
        messages.push_back(ShortTermMessage {
            role: role.to_owned(),
            content: content.to_owned(),
            created_at: now,
            expires_at,
        });
        Ok(())
    }

    pub fn session_messages(
        &mut self,
        principal: &AccessPrincipal,
        session_id: &str,
        now: u64,
    ) -> Result<Vec<ShortTermMessage>, MemoryError> {
        self.validate_principal(principal)?;
        let key = SessionKey::new(principal, session_id);
        let Some(messages) = self.short_term.get_mut(&key) else {
            return Ok(Vec::new());
        };
        messages.retain(|message| message.expires_at.map_or(true, |expiry| expiry > now));
        Ok(messages.iter().cloned().collect())
    }

    /// Stores a long-term memory only after a trusted approval decision and basic
    /// provenance/trust validation. This in-process implementation is not durable.
    pub fn remember(
        &mut self,
        principal: &AccessPrincipal,
        draft: MemoryDraft,
        now: u64,
    ) -> Result<u64, MemoryError> {
        self.validate_principal(principal)?;
        self.validate_content(&draft.content)?;
        if !draft.approved_by_user {
            return Err(MemoryError::ApprovalRequired);
        }
        if draft.trust == TrustLevel::Untrusted {
            return Err(MemoryError::UntrustedContent);
        }
        if draft.provenance.source_id.trim().is_empty()
            || draft.provenance.source_kind.trim().is_empty()
        {
            return Err(MemoryError::InvalidInput("provenance is required"));
        }
        if draft.expires_at.map_or(false, |expiry| expiry <= now) {
            return Err(MemoryError::InvalidInput("memory is already expired"));
        }

        let id = self.next_id;
        self.next_id = self.next_id.checked_add(1)
            .ok_or(MemoryError::InvalidInput("memory ID space exhausted"))?;
        self.long_term.insert(id, MemoryEntry {
            id,
            tenant_id: principal.tenant_id.clone(),
            owner_user_id: principal.user_id.clone(),
            owner_agent_id: Some(principal.agent_id.clone()),
            kind: draft.kind,
            content: draft.content,
            provenance: draft.provenance,
            trust: draft.trust,
            approved_by_user: draft.approved_by_user,
            created_at: now,
            expires_at: draft.expires_at,
        });
        Ok(id)
    }

    pub fn retrieve(
        &self,
        principal: &AccessPrincipal,
        now: u64,
        limit: usize,
    ) -> Result<Vec<MemoryEntry>, MemoryError> {
        self.validate_principal(principal)?;
        let mut entries: Vec<_> = self.long_term.values()
            .filter(|entry| self.can_read(entry, principal))
            .filter(|entry| entry.approved_by_user && entry.trust != TrustLevel::Untrusted)
            .filter(|entry| entry.expires_at.map_or(true, |expiry| expiry > now))
            .cloned()
            .collect();
        entries.sort_by_key(|entry| entry.id);
        entries.truncate(limit);
        Ok(entries)
    }

    pub fn build_context(
        &mut self,
        principal: &AccessPrincipal,
        session_id: &str,
        now: u64,
        memory_limit: usize,
    ) -> Result<BuiltContext, MemoryError> {
        let session_messages = self.session_messages(principal, session_id, now)?;
        let long_term_items = self.retrieve(principal, now, memory_limit)?
            .into_iter()
            .map(|entry| ContextItem {
                memory_id: entry.id,
                content: entry.content,
                source_id: entry.provenance.source_id,
                source_version: entry.provenance.source_version,
            })
            .collect();
        Ok(BuiltContext { session_messages, long_term_items })
    }

    pub fn delete<C: DerivedArtifactCleaner>(
        &mut self,
        principal: &AccessPrincipal,
        memory_id: u64,
        cleaner: &mut C,
    ) -> Result<(), MemoryError> {
        self.validate_principal(principal)?;
        let entry = self.long_term.get(&memory_id).ok_or(MemoryError::NotFound)?;
        if !self.can_read(entry, principal) {
            return Err(MemoryError::AccessDenied);
        }
        // Fail closed: do not remove the source record until derived cleanup succeeds.
        cleaner.delete_by_memory_id(memory_id)
            .map_err(|_| MemoryError::DerivedArtifactCleanupFailed)?;
        self.long_term.remove(&memory_id);
        Ok(())
    }

    pub fn create_explanation(
        &self,
        principal: &AccessPrincipal,
        task_understood: &str,
        sources_used: Vec<ExplanationSource>,
        decision_summary: &str,
        rationale_summary: &str,
        uncertainty: Vec<String>,
        next_action: &str,
        now: u64,
    ) -> Result<ExplanationRecord, MemoryError> {
        self.validate_principal(principal)?;
        for value in [task_understood, decision_summary, rationale_summary, next_action] {
            if value.trim().is_empty() {
                return Err(MemoryError::InvalidInput("explanation fields must not be empty"));
            }
            self.validate_content(value)?;
        }
        // If a source references a memory record, confirm the caller is authorized to
        // read that record. This records a concise explanation, not hidden reasoning.
        for source in &sources_used {
            if let Some(id) = source.memory_id {
                let entry = self.long_term.get(&id).ok_or(MemoryError::NotFound)?;
                if !self.can_read(entry, principal) {
                    return Err(MemoryError::AccessDenied);
                }
                if entry.provenance.source_id != source.source_id {
                    return Err(MemoryError::InvalidInput("source reference mismatch"));
                }
            }
        }
        Ok(ExplanationRecord {
            task_understood: task_understood.to_owned(),
            sources_used,
            decision_summary: decision_summary.to_owned(),
            rationale_summary: rationale_summary.to_owned(),
            uncertainty,
            next_action: next_action.to_owned(),
            generated_at: now,
        })
    }

    pub fn count(&self) -> usize {
        self.long_term.len()
    }

    fn can_read(&self, entry: &MemoryEntry, principal: &AccessPrincipal) -> bool {
        entry.tenant_id == principal.tenant_id
            && entry.owner_user_id == principal.user_id
            && entry.owner_agent_id.as_ref().map_or(true, |agent| agent == &principal.agent_id)
    }

    fn validate_principal(&self, principal: &AccessPrincipal) -> Result<(), MemoryError> {
        if principal.tenant_id.trim().is_empty()
            || principal.user_id.trim().is_empty()
            || principal.agent_id.trim().is_empty()
        {
            return Err(MemoryError::InvalidInput("principal fields must not be empty"));
        }
        Ok(())
    }

    fn validate_content(&self, content: &str) -> Result<(), MemoryError> {
        if content.trim().is_empty() {
            return Err(MemoryError::InvalidInput("content must not be empty"));
        }
        if content.chars().count() > self.max_entry_chars {
            return Err(MemoryError::InvalidInput("content exceeds configured limit"));
        }
        if looks_like_secret(content) {
            return Err(MemoryError::InvalidInput("possible secret material rejected"));
        }
        Ok(())
    }
}

/// Conservative tripwire only; not a substitute for a dedicated secret scanner or
/// encrypted secret-management service. Private keys and credentials must never be
/// passed to this API in the first place.
fn looks_like_secret(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    [
        "seed phrase",
        "mnemonic phrase",
        "private_key=",
        "private key:",
        "api_key=",
        "api key:",
        "password=",
        "password:",
    ].iter().any(|needle| lower.contains(needle))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn principal(tenant: &str, user: &str, agent: &str) -> AccessPrincipal {
        AccessPrincipal {
            tenant_id: tenant.into(),
            user_id: user.into(),
            agent_id: agent.into(),
        }
    }

    fn draft(content: &str, approved: bool, trust: TrustLevel, expiry: Option<u64>) -> MemoryDraft {
        MemoryDraft {
            kind: MemoryKind::ProjectFact,
            content: content.into(),
            provenance: Provenance {
                source_id: "conversation:turn-7".into(),
                source_kind: "conversation".into(),
                observed_at: 100,
                source_version: Some("v1".into()),
            },
            trust,
            approved_by_user: approved,
            expires_at: expiry,
        }
    }

    #[derive(Default)]
    struct Cleaner {
        deleted: Vec<u64>,
        fail: bool,
    }

    impl DerivedArtifactCleaner for Cleaner {
        fn delete_by_memory_id(&mut self, memory_id: u64) -> Result<(), MemoryError> {
            if self.fail {
                return Err(MemoryError::DerivedArtifactCleanupFailed);
            }
            self.deleted.push(memory_id);
            Ok(())
        }
    }

    #[test]
    fn short_term_is_bounded_and_session_isolated() {
        let mut manager = MemoryManager::new(2, 200);
        let p = principal("t1", "u1", "a1");
        manager.append_message(&p, "s1", "user", "first", 1, None).unwrap();
        manager.append_message(&p, "s1", "assistant", "second", 2, None).unwrap();
        manager.append_message(&p, "s1", "user", "third", 3, None).unwrap();
        assert_eq!(manager.session_messages(&p, "s1", 3).unwrap().len(), 2);
        assert_eq!(manager.session_messages(&p, "s2", 3).unwrap().len(), 0);
    }

    #[test]
    fn long_term_requires_approval_and_trusted_content() {
        let mut manager = MemoryManager::new(8, 200);
        let p = principal("t1", "u1", "a1");
        assert_eq!(
            manager.remember(&p, draft("not approved", false, TrustLevel::UserProvided, None), 100),
            Err(MemoryError::ApprovalRequired)
        );
        assert_eq!(
            manager.remember(&p, draft("poisoned", true, TrustLevel::Untrusted, None), 100),
            Err(MemoryError::UntrustedContent)
        );
        assert_eq!(manager.count(), 0);
    }

    #[test]
    fn retrieval_enforces_tenant_user_agent_and_expiry() {
        let mut manager = MemoryManager::new(8, 200);
        let p = principal("t1", "u1", "a1");
        manager.remember(&p, draft("approved fact", true, TrustLevel::VerifiedSource, Some(200)), 100).unwrap();
        assert_eq!(manager.retrieve(&principal("t2", "u1", "a1"), 150, 10).unwrap().len(), 0);
        assert_eq!(manager.retrieve(&principal("t1", "u2", "a1"), 150, 10).unwrap().len(), 0);
        assert_eq!(manager.retrieve(&principal("t1", "u1", "a2"), 150, 10).unwrap().len(), 0);
        assert_eq!(manager.retrieve(&p, 200, 10).unwrap().len(), 0);
    }

    #[test]
    fn rejects_obvious_secret_markers() {
        let mut manager = MemoryManager::new(8, 200);
        let p = principal("t1", "u1", "a1");
        assert!(matches!(
            manager.append_message(&p, "s1", "user", "api_key=do-not-store", 1, None),
            Err(MemoryError::InvalidInput(_))
        ));
    }

    #[test]
    fn delete_cleans_derived_artifacts_before_source_record() {
        let mut manager = MemoryManager::new(8, 200);
        let p = principal("t1", "u1", "a1");
        let id = manager.remember(&p, draft("approved fact", true, TrustLevel::UserProvided, None), 100).unwrap();
        let mut cleaner = Cleaner { fail: true, ..Cleaner::default() };
        assert_eq!(manager.delete(&p, id, &mut cleaner), Err(MemoryError::DerivedArtifactCleanupFailed));
        assert_eq!(manager.count(), 1);
        cleaner.fail = false;
        manager.delete(&p, id, &mut cleaner).unwrap();
        assert_eq!(manager.count(), 0);
        assert_eq!(cleaner.deleted, vec![id]);
    }

    #[test]
    fn explanation_requires_authorized_and_matching_source() {
        let mut manager = MemoryManager::new(8, 200);
        let p = principal("t1", "u1", "a1");
        let id = manager.remember(&p, draft("approved fact", true, TrustLevel::VerifiedSource, None), 100).unwrap();
        let explanation = manager.create_explanation(
            &p,
            "Find the project identifier",
            vec![ExplanationSource {
                source_id: "conversation:turn-7".into(),
                memory_id: Some(id),
                source_version: Some("v1".into()),
            }],
            "Return the stored identifier",
            "The approved project fact is the relevant source",
            vec!["The source has not been independently revalidated".into()],
            "Ask for confirmation before changing project configuration",
            120,
        ).unwrap();
        assert_eq!(explanation.sources_used.len(), 1);
        assert_eq!(explanation.uncertainty.len(), 1);
    }
}
