// Copyright (c) 2026 Michael Wroblewski / ShivaCore / A-TownChain-Okosystems. All Rights Reserved.
// Memory index — tag and time lookups are scoped to an authenticated principal.
use std::collections::HashMap;
use crate::{AccessPrincipal, MemoryError};

#[derive(Debug, Clone)]
pub struct Memory {
    pub id: u64,
    pub tenant_id: String,
    pub owner_user_id: String,
    pub owner_agent_id: String,
    pub content: String,
    pub tags: Vec<String>,
    pub timestamp: u64,
}

pub struct MemoryIndex {
    memories: Vec<Memory>,
    tag_index: HashMap<String, Vec<usize>>,
}

impl MemoryIndex {
    pub fn new() -> Self { Self { memories: Vec::new(), tag_index: HashMap::new() } }

    pub fn add(&mut self, principal: &AccessPrincipal, content: &str, tags: Vec<String>, ts: u64) -> Result<u64, MemoryError> {
        validate_principal(principal)?;
        if content.trim().is_empty() { return Err(MemoryError::InvalidInput("memory content must not be empty")); }
        let id = self.memories.len() as u64;
        for tag in &tags {
            self.tag_index.entry(tag.clone()).or_default().push(self.memories.len());
        }
        self.memories.push(Memory {
            id, tenant_id: principal.tenant_id.clone(), owner_user_id: principal.user_id.clone(),
            owner_agent_id: principal.agent_id.clone(), content: content.into(), tags, timestamp: ts,
        });
        Ok(id)
    }

    pub fn by_tag(&self, principal: &AccessPrincipal, tag: &str) -> Result<Vec<&Memory>, MemoryError> {
        validate_principal(principal)?;
        Ok(self.tag_index.get(tag)
            .map(|indices| indices.iter().filter_map(|&i| self.memories.get(i))
                .filter(|memory| can_read(memory, principal)).collect())
            .unwrap_or_default())
    }

    pub fn count(&self, principal: &AccessPrincipal) -> Result<usize, MemoryError> {
        validate_principal(principal)?;
        Ok(self.memories.iter().filter(|memory| can_read(memory, principal)).count())
    }
}

fn can_read(memory: &Memory, principal: &AccessPrincipal) -> bool {
    memory.tenant_id == principal.tenant_id
        && memory.owner_user_id == principal.user_id
        && memory.owner_agent_id == principal.agent_id
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
    fn tag_lookup_is_principal_scoped() {
        let mut mi = MemoryIndex::new();
        let alice = principal("tenant-a", "alice", "agent-a");
        let bob = principal("tenant-b", "bob", "agent-b");
        mi.add(&alice, "private kernel note", vec!["kernel".into()], 100).unwrap();
        mi.add(&bob, "other tenant note", vec!["kernel".into()], 200).unwrap();
        assert_eq!(mi.by_tag(&alice, "kernel").unwrap().len(), 1);
        assert_eq!(mi.by_tag(&alice, "kernel").unwrap()[0].content, "private kernel note");
        assert_eq!(mi.count(&alice).unwrap(), 1);
    }
}
