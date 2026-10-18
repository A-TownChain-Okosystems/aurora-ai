// Copyright (c) 2026 Michael Wroblewski / ShivaCore / A-TownChain-Okosystems. All Rights Reserved.
// Knowledge base — facts are scoped to an authenticated tenant/user/agent principal.
use std::collections::HashMap;
use crate::{AccessPrincipal, MemoryError};

#[derive(Debug, Clone)]
pub struct KnowledgeEntry {
    pub id: u64,
    pub tenant_id: String,
    pub owner_user_id: String,
    pub owner_agent_id: String,
    pub subject: String,
    pub predicate: String,
    pub object: String,
    pub confidence: f64,
    pub timestamp: u64,
}

pub struct KnowledgeBase {
    entries: Vec<KnowledgeEntry>,
    index: HashMap<String, Vec<usize>>,
}

impl KnowledgeBase {
    pub fn new() -> Self { Self { entries: Vec::new(), index: HashMap::new() } }

    pub fn add(
        &mut self,
        principal: &AccessPrincipal,
        subject: &str,
        predicate: &str,
        object: &str,
        confidence: f64,
    ) -> Result<u64, MemoryError> {
        validate_principal(principal)?;
        if subject.trim().is_empty() || predicate.trim().is_empty() || object.trim().is_empty() {
            return Err(MemoryError::InvalidInput("knowledge fields must not be empty"));
        }
        let id = self.entries.len() as u64;
        let entry = KnowledgeEntry {
            id,
            tenant_id: principal.tenant_id.clone(),
            owner_user_id: principal.user_id.clone(),
            owner_agent_id: principal.agent_id.clone(),
            subject: subject.into(), predicate: predicate.into(),
            object: object.into(), confidence, timestamp: 0,
        };
        self.index.entry(subject.into()).or_default().push(self.entries.len());
        self.entries.push(entry);
        Ok(id)
    }

    pub fn query(&self, principal: &AccessPrincipal, subject: &str) -> Result<Vec<&KnowledgeEntry>, MemoryError> {
        validate_principal(principal)?;
        Ok(self.index.get(subject)
            .map(|indices| indices.iter().filter_map(|&i| self.entries.get(i))
                .filter(|entry| can_read(entry, principal)).collect())
            .unwrap_or_default())
    }

    /// Count only records visible to this principal; global counts can leak tenant activity.
    pub fn count(&self, principal: &AccessPrincipal) -> Result<usize, MemoryError> {
        validate_principal(principal)?;
        Ok(self.entries.iter().filter(|entry| can_read(entry, principal)).count())
    }
}

fn can_read(entry: &KnowledgeEntry, principal: &AccessPrincipal) -> bool {
    entry.tenant_id == principal.tenant_id
        && entry.owner_user_id == principal.user_id
        && entry.owner_agent_id == principal.agent_id
}

fn validate_principal(principal: &AccessPrincipal) -> Result<(), MemoryError> {
    if principal.tenant_id.trim().is_empty() || principal.user_id.trim().is_empty()
        || principal.agent_id.trim().is_empty() {
        return Err(MemoryError::InvalidInput("principal fields must not be empty"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn principal(tenant: &str, user: &str, agent: &str) -> AccessPrincipal {
        AccessPrincipal { tenant_id: tenant.into(), user_id: user.into(), agent_id: agent.into() }
    }

    #[test]
    fn queries_and_counts_are_principal_scoped() {
        let mut kb = KnowledgeBase::new();
        let alice = principal("tenant-a", "alice", "agent-a");
        let bob = principal("tenant-b", "bob", "agent-b");
        kb.add(&alice, "atc", "has_chain_id", "658467", 1.0).unwrap();
        kb.add(&bob, "atc", "has_chain_id", "secret-chain", 1.0).unwrap();
        assert_eq!(kb.query(&alice, "atc").unwrap().len(), 1);
        assert_eq!(kb.query(&alice, "atc").unwrap()[0].object, "658467");
        assert_eq!(kb.count(&alice).unwrap(), 1);
        assert_eq!(kb.query(&bob, "atc").unwrap()[0].object, "secret-chain");
    }

    #[test]
    fn empty_principal_is_rejected() {
        let mut kb = KnowledgeBase::new();
        assert_eq!(kb.add(&principal("", "user", "agent"), "s", "p", "o", 1.0), Err(MemoryError::InvalidInput("principal fields must not be empty")));
    }
}
