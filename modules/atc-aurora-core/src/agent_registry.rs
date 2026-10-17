// Copyright (c) 2026 Michael Wroblewski / ShivaCore / A-TownChain-Okosystems. All Rights Reserved.
// Agent Registry — validates identities and capability metadata before registration.

use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapabilityMetadata {
    pub name: String,
    pub description: String,
    /// Informational classification only; authorization is enforced by CapabilityPolicyEngine.
    pub risk_level: String,
}

impl CapabilityMetadata {
    pub fn new(name: &str, description: &str, risk_level: &str) -> Self {
        Self {
            name: name.trim().into(),
            description: description.trim().into(),
            risk_level: risk_level.trim().into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentInfo {
    pub id: String,
    pub name: String,
    pub role: String,
    pub capabilities: Vec<String>,
    pub capability_metadata: Vec<CapabilityMetadata>,
    pub is_active: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RegistryError {
    EmptyId,
    EmptyName,
    EmptyRole,
    DuplicateId(String),
    EmptyCapability,
    DuplicateCapability(String),
    InvalidCapabilityMetadata(String),
    AgentNotFound(String),
}

#[derive(Debug, Default)]
pub struct AgentRegistry {
    agents: HashMap<String, AgentInfo>,
}

impl AgentRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Register an agent with explicit, stable identity and capability names.
    /// IDs are caller-assigned and never silently overwritten.
    pub fn register(
        &mut self,
        id: &str,
        name: &str,
        role: &str,
        caps: Vec<String>,
    ) -> Result<(), RegistryError> {
        let metadata = caps
            .iter()
            .map(|cap| CapabilityMetadata::new(cap, "", "unspecified"))
            .collect();
        self.register_with_metadata(id, name, role, metadata)
    }

    pub fn register_with_metadata(
        &mut self,
        id: &str,
        name: &str,
        role: &str,
        metadata: Vec<CapabilityMetadata>,
    ) -> Result<(), RegistryError> {
        let id = id.trim();
        let name = name.trim();
        let role = role.trim();
        if id.is_empty() {
            return Err(RegistryError::EmptyId);
        }
        if name.is_empty() {
            return Err(RegistryError::EmptyName);
        }
        if role.is_empty() {
            return Err(RegistryError::EmptyRole);
        }
        if self.agents.get(id).is_some() {
            return Err(RegistryError::DuplicateId(id.into()));
        }

        let mut seen = HashSet::new();
        let mut capabilities = Vec::with_capacity(metadata.len());
        for item in &metadata {
            if item.name.trim().is_empty() {
                return Err(RegistryError::EmptyCapability);
            }
            if !seen.insert(item.name.trim().to_owned()) {
                return Err(RegistryError::DuplicateCapability(item.name.clone()));
            }
            if item.name != item.name.trim() || item.description != item.description.trim()
                || item.risk_level != item.risk_level.trim()
            {
                return Err(RegistryError::InvalidCapabilityMetadata(item.name.clone()));
            }
            capabilities.push(item.name.clone());
        }

        self.agents.insert(
            id.into(),
            AgentInfo {
                id: id.into(),
                name: name.into(),
                role: role.into(),
                capabilities,
                capability_metadata: metadata,
                is_active: true,
            },
        );
        Ok(())
    }

    pub fn unregister(&mut self, id: &str) -> Result<AgentInfo, RegistryError> {
        self.agents
            .remove(id)
            .ok_or_else(|| RegistryError::AgentNotFound(id.into()))
    }

    pub fn get_agent(&self, id: &str) -> Option<&AgentInfo> {
        self.agents.get(id)
    }

    pub fn list_active(&self) -> Vec<&AgentInfo> {
        let mut agents: Vec<_> = self.agents.values().filter(|a| a.is_active).collect();
        agents.sort_by(|left, right| left.id.cmp(&right.id));
        agents
    }

    pub fn count(&self) -> usize {
        self.agents.len()
    }

    pub fn set_active(&mut self, id: &str, active: bool) -> Result<(), RegistryError> {
        let agent = self
            .agents
            .get_mut(id)
            .ok_or_else(|| RegistryError::AgentNotFound(id.into()))?;
        agent.is_active = active;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registers_capabilities_and_metadata() {
        let mut registry = AgentRegistry::new();
        registry
            .register_with_metadata(
                "agent:research-1",
                "Research Agent",
                "research",
                vec![CapabilityMetadata::new(
                    "web.search",
                    "Search approved public sources",
                    "low",
                )],
            )
            .unwrap();
        let agent = registry.get_agent("agent:research-1").unwrap();
        assert_eq!(agent.capabilities, vec!["web.search"]);
        assert_eq!(agent.capability_metadata[0].risk_level, "low");
    }

    #[test]
    fn duplicate_id_is_rejected_without_overwrite() {
        let mut registry = AgentRegistry::new();
        registry.register("agent:one", "First", "research", vec![]).unwrap();
        assert_eq!(
            registry.register("agent:one", "Second", "coding", vec![]),
            Err(RegistryError::DuplicateId("agent:one".into()))
        );
        assert_eq!(registry.get_agent("agent:one").unwrap().name, "First");
        assert_eq!(registry.count(), 1);
    }

    #[test]
    fn invalid_identity_and_duplicate_capabilities_are_rejected() {
        let mut registry = AgentRegistry::new();
        assert_eq!(
            registry.register(" ", "Agent", "role", vec![]),
            Err(RegistryError::EmptyId)
        );
        assert_eq!(
            registry.register(
                "agent:dup",
                "Agent",
                "role",
                vec!["read".into(), "read".into()]
            ),
            Err(RegistryError::DuplicateCapability("read".into()))
        );
        assert_eq!(registry.count(), 0);
    }

    #[test]
    fn active_listing_is_deterministic_and_unknown_agent_errors() {
        let mut registry = AgentRegistry::new();
        registry.register("agent:z", "Z", "role", vec![]).unwrap();
        registry.register("agent:a", "A", "role", vec![]).unwrap();
        registry.set_active("agent:z", false).unwrap();
        assert_eq!(registry.list_active()[0].id, "agent:a");
        assert_eq!(
            registry.set_active("missing", true),
            Err(RegistryError::AgentNotFound("missing".into()))
        );
    }
}
