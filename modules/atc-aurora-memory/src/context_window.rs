// Copyright (c) 2026 Michael Wroblewski / ShivaCore / A-TownChain-Okosystems. All Rights Reserved.
// Context window — conversation context is partitioned by tenant, user, agent and session.
use std::collections::HashMap;
use crate::{AccessPrincipal, MemoryError};

type SessionKey = (String, String, String, String);

pub struct ContextWindow {
    sessions: HashMap<SessionKey, Vec<(String, String)>>,
    max_size: usize,
}

impl ContextWindow {
    pub fn new(max_size: usize) -> Self {
        Self { sessions: HashMap::new(), max_size: max_size.max(1) }
    }

    pub fn add(&mut self, principal: &AccessPrincipal, session_id: &str, role: &str, content: &str) -> Result<(), MemoryError> {
        validate_principal(principal)?;
        if session_id.trim().is_empty() || role.trim().is_empty() || content.trim().is_empty() {
            return Err(MemoryError::InvalidInput("session, role and content must not be empty"));
        }
        let key = session_key(principal, session_id);
        let messages = self.sessions.entry(key).or_default();
        if messages.len() >= self.max_size { messages.remove(0); }
        messages.push((role.into(), content.into()));
        Ok(())
    }

    pub fn get_context(&self, principal: &AccessPrincipal, session_id: &str) -> Result<String, MemoryError> {
        validate_principal(principal)?;
        if session_id.trim().is_empty() { return Err(MemoryError::InvalidInput("session ID must not be empty")); }
        Ok(self.sessions.get(&session_key(principal, session_id))
            .map(|messages| messages.iter().map(|(r, c)| format!("{}: {}", r, c)).collect::<Vec<_>>().join("\n"))
            .unwrap_or_default())
    }

    pub fn clear(&mut self, principal: &AccessPrincipal, session_id: &str) -> Result<(), MemoryError> {
        validate_principal(principal)?;
        if session_id.trim().is_empty() { return Err(MemoryError::InvalidInput("session ID must not be empty")); }
        self.sessions.remove(&session_key(principal, session_id));
        Ok(())
    }

    pub fn len(&self, principal: &AccessPrincipal, session_id: &str) -> Result<usize, MemoryError> {
        validate_principal(principal)?;
        Ok(self.sessions.get(&session_key(principal, session_id)).map_or(0, Vec::len))
    }

    pub fn is_empty(&self, principal: &AccessPrincipal, session_id: &str) -> Result<bool, MemoryError> {
        Ok(self.len(principal, session_id)? == 0)
    }

    pub fn is_full(&self, principal: &AccessPrincipal, session_id: &str) -> Result<bool, MemoryError> {
        Ok(self.len(principal, session_id)? >= self.max_size)
    }
}

fn session_key(principal: &AccessPrincipal, session_id: &str) -> SessionKey {
    (principal.tenant_id.clone(), principal.user_id.clone(), principal.agent_id.clone(), session_id.to_owned())
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
    fn sessions_are_isolated_by_principal_and_session() {
        let mut cw = ContextWindow::new(3);
        let alice = principal("tenant-a", "alice", "agent-a");
        let bob = principal("tenant-b", "bob", "agent-b");
        cw.add(&alice, "session-1", "user", "private hello").unwrap();
        cw.add(&bob, "session-1", "user", "other hello").unwrap();
        assert!(cw.get_context(&alice, "session-1").unwrap().contains("private hello"));
        assert!(!cw.get_context(&alice, "session-1").unwrap().contains("other hello"));
        cw.clear(&alice, "session-1").unwrap();
        assert_eq!(cw.len(&alice, "session-1").unwrap(), 0);
        assert_eq!(cw.len(&bob, "session-1").unwrap(), 1);
    }

    #[test]
    fn invalid_principal_is_rejected() {
        let mut cw = ContextWindow::new(3);
        assert_eq!(cw.add(&principal("", "user", "agent"), "session", "user", "hello"), Err(MemoryError::InvalidInput("principal fields must not be empty")));
    }
}
