// Copyright (c) 2026 A-TownChain-Okosystems. All Rights Reserved.
// Aurora Memory Engine — in-process MVP contracts.
// Persistent storage, encryption-at-rest and OS capability integration are intentionally
// adapter responsibilities and are NOT implemented by this module.

use std::collections::{HashMap, VecDeque};
use std::path::Path;
use crate::persistence::{DurableMemoryStore, DurableRecord, StoreError, StorePrincipal};

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
    PersistenceFailure,
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
    persistent_store: Option<DurableMemoryStore>,
}

impl MemoryManager {
    pub fn new(max_short_term_messages: usize, max_entry_chars: usize) -> Self {
        Self {
            short_term: HashMap::new(),
            long_term: HashMap::new(),
            next_id: 1,
            max_short_term_messages: max_short_term_messages.max(1),
            max_entry_chars: max_entry_chars.max(1),
            persistent_store: None,
        }
    }

    /// Opens the canonical manager backed by the durable journal and restores all
    /// persisted long-term entries. Short-term session messages remain ephemeral.
    pub fn open_persistent(
        path: impl AsRef<Path>,
        max_short_term_messages: usize,
        max_entry_chars: usize,
    ) -> Result<Self, MemoryError> {
        let store = DurableMemoryStore::open(path).map_err(|_| MemoryError::PersistenceFailure)?;
        let mut manager = Self::new(max_short_term_messages, max_entry_chars);
        let mut max_id = 0u64;
        for record in store.records_for_recovery() {
            let entry = decode_entry(record).ok_or(MemoryError::PersistenceFailure)?;
            max_id = max_id.max(entry.id);
            manager.long_term.insert(entry.id, entry);
        }
        manager.next_id = max_id.checked_add(1).ok_or(MemoryError::PersistenceFailure)?;
        manager.persistent_store = Some(store);
        Ok(manager)
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
        let next_id = self.next_id.checked_add(1)
            .ok_or(MemoryError::InvalidInput("memory ID space exhausted"))?;
        let entry = MemoryEntry {
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
        };
        if let Some(store) = self.persistent_store.as_mut() {
            store.put(encode_entry(&entry)?).map_err(|_| MemoryError::PersistenceFailure)?;
        }
        self.long_term.insert(id, entry);
        self.next_id = next_id;
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
        if let Some(store) = self.persistent_store.as_mut() {
            let scope = StorePrincipal {
                tenant_id: entry.tenant_id.clone(),
                user_id: entry.owner_user_id.clone(),
                agent_id: entry.owner_agent_id.clone().unwrap_or_default(),
            };
            store.delete(&scope, memory_id).map_err(|_| MemoryError::PersistenceFailure)?;
        }
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


fn encode_entry(entry: &MemoryEntry) -> Result<DurableRecord, MemoryError> {
    let payload = [
        kind_tag(entry.kind).to_string(),
        hex_encode(entry.content.as_bytes()),
        hex_encode(entry.provenance.source_id.as_bytes()),
        hex_encode(entry.provenance.source_kind.as_bytes()),
        entry.provenance.observed_at.to_string(),
        entry.provenance.source_version.as_ref().map(|v| hex_encode(v.as_bytes())).unwrap_or_else(|| "-".into()),
        trust_tag(entry.trust).to_string(),
    ].join("\n").into_bytes();
    Ok(DurableRecord {
        id: entry.id,
        tenant_id: entry.tenant_id.clone(),
        user_id: entry.owner_user_id.clone(),
        agent_id: entry.owner_agent_id.clone().unwrap_or_default(),
        created_at: entry.created_at,
        expires_at: entry.expires_at,
        approved: entry.approved_by_user,
        payload,
    })
}

fn decode_entry(record: &DurableRecord) -> Option<MemoryEntry> {
    if !record.approved || record.agent_id.is_empty() { return None; }
    let payload = std::str::from_utf8(&record.payload).ok()?;
    let fields: Vec<&str> = payload.split('\n').collect();
    if fields.len() != 7 { return None; }
    let kind = parse_kind(fields[0])?;
    let content = String::from_utf8(hex_decode(fields[1])?).ok()?;
    let source_id = String::from_utf8(hex_decode(fields[2])?).ok()?;
    let source_kind = String::from_utf8(hex_decode(fields[3])?).ok()?;
    let observed_at = fields[4].parse().ok()?;
    let source_version = if fields[5] == "-" { None } else {
        Some(String::from_utf8(hex_decode(fields[5])?).ok()?)
    };
    let trust = parse_trust(fields[6])?;
    if content.trim().is_empty() || source_id.trim().is_empty() || source_kind.trim().is_empty()
        || trust == TrustLevel::Untrusted { return None; }
    Some(MemoryEntry {
        id: record.id, tenant_id: record.tenant_id.clone(), owner_user_id: record.user_id.clone(),
        owner_agent_id: Some(record.agent_id.clone()), kind, content,
        provenance: Provenance { source_id, source_kind, observed_at, source_version },
        trust, approved_by_user: record.approved, created_at: record.created_at,
        expires_at: record.expires_at,
    })
}

fn kind_tag(kind: MemoryKind) -> u8 {
    match kind { MemoryKind::Preference => 1, MemoryKind::ProjectFact => 2, MemoryKind::Episode => 3,
        MemoryKind::SemanticFact => 4, MemoryKind::Procedure => 5 }
}
fn parse_kind(tag: &str) -> Option<MemoryKind> {
    match tag { "1" => Some(MemoryKind::Preference), "2" => Some(MemoryKind::ProjectFact),
        "3" => Some(MemoryKind::Episode), "4" => Some(MemoryKind::SemanticFact),
        "5" => Some(MemoryKind::Procedure), _ => None }
}
fn trust_tag(trust: TrustLevel) -> u8 {
    match trust { TrustLevel::Untrusted => 0, TrustLevel::UserProvided => 1,
        TrustLevel::VerifiedSource => 2, TrustLevel::SystemApproved => 3 }
}
fn parse_trust(tag: &str) -> Option<TrustLevel> {
    match tag { "0" => Some(TrustLevel::Untrusted), "1" => Some(TrustLevel::UserProvided),
        "2" => Some(TrustLevel::VerifiedSource), "3" => Some(TrustLevel::SystemApproved), _ => None }
}
fn hex_encode(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes { out.push(HEX[(byte >> 4) as usize] as char); out.push(HEX[(byte & 0x0f) as usize] as char); }
    out
}
fn hex_decode(value: &str) -> Option<Vec<u8>> {
    if value.len() % 2 != 0 { return None; }
    fn nibble(byte: u8) -> Option<u8> { match byte {
        b'0'..=b'9' => Some(byte - b'0'), b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10), _ => None } }
    let bytes = value.as_bytes();
    let mut out = Vec::with_capacity(bytes.len() / 2);
    for pair in bytes.chunks_exact(2) { out.push((nibble(pair[0])? << 4) | nibble(pair[1])?); }
    Some(out)
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
    fn persistent_manager_restores_entries_and_next_id_after_restart() {
        use std::fs;
        use std::time::{SystemTime, UNIX_EPOCH};
        let n = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
        let path = std::env::temp_dir().join(format!("aurora-manager-restart-{}-{n}.db", std::process::id()));
        let p = principal("t1", "u1", "a1");
        let id;
        {
            let mut manager = MemoryManager::open_persistent(&path, 8, 200).unwrap();
            id = manager.remember(&p, draft("durable fact", true, TrustLevel::VerifiedSource, None), 100).unwrap();
        }
        {
            let mut manager = MemoryManager::open_persistent(&path, 8, 200).unwrap();
            let restored = manager.retrieve(&p, 101, 10).unwrap();
            assert_eq!(restored.len(), 1);
            assert_eq!(restored[0].id, id);
            assert_eq!(restored[0].content, "durable fact");
            let next = manager.remember(&p, draft("second durable fact", true, TrustLevel::UserProvided, None), 102).unwrap();
            assert!(next > id);
        }
        let _ = fs::remove_file(path);
    }

    #[test]
    fn persistent_manager_deletion_survives_restart() {
        use std::fs;
        use std::time::{SystemTime, UNIX_EPOCH};
        let n = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
        let path = std::env::temp_dir().join(format!("aurora-manager-delete-{}-{n}.db", std::process::id()));
        let p = principal("t1", "u1", "a1");
        let mut manager = MemoryManager::open_persistent(&path, 8, 200).unwrap();
        let id = manager.remember(&p, draft("delete me", true, TrustLevel::UserProvided, None), 100).unwrap();
        manager.delete(&p, id, &mut Cleaner::default()).unwrap();
        drop(manager);
        let manager = MemoryManager::open_persistent(&path, 8, 200).unwrap();
        assert_eq!(manager.retrieve(&p, 101, 10).unwrap().len(), 0);
        let _ = fs::remove_file(path);
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
