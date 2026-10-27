// Copyright (c) 2026 Michael Wroblewski / ShivaCore / A-TownChain-Okosystems. All Rights Reserved.
// Vector store — embeddings are keyed and searched within an authenticated principal scope.
use std::collections::HashMap;

use crate::{AccessPrincipal, MemoryError};

type ScopedKey = (String, String, String, String);

pub struct VectorStore {
    vectors: HashMap<ScopedKey, Vec<f32>>,
}

impl VectorStore {
    pub fn new() -> Self {
        Self {
            vectors: HashMap::new(),
        }
    }

    pub fn store(
        &mut self,
        principal: &AccessPrincipal,
        key: &str,
        vector: Vec<f32>,
    ) -> Result<(), MemoryError> {
        validate_principal(principal)?;
        if key.trim().is_empty() {
            return Err(MemoryError::InvalidInput("vector key must not be empty"));
        }
        self.vectors.insert(scoped_key(principal, key), vector);
        Ok(())
    }

    pub fn get(
        &self,
        principal: &AccessPrincipal,
        key: &str,
    ) -> Result<Option<&Vec<f32>>, MemoryError> {
        validate_principal(principal)?;
        if key.trim().is_empty() {
            return Err(MemoryError::InvalidInput("vector key must not be empty"));
        }
        Ok(self.vectors.get(&scoped_key(principal, key)))
    }

    pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
        if a.len() != b.len() || a.is_empty() {
            return 0.0;
        }
        let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
        let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
        let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
        if norm_a == 0.0 || norm_b == 0.0 {
            return 0.0;
        }
        dot / (norm_a * norm_b)
    }

    pub fn search(
        &self,
        principal: &AccessPrincipal,
        query: &[f32],
        limit: usize,
    ) -> Result<Vec<(String, f32)>, MemoryError> {
        validate_principal(principal)?;
        let mut results: Vec<(String, f32)> = self
            .vectors
            .iter()
            .filter(|(k, _)| {
                k.0 == principal.tenant_id && k.1 == principal.user_id && k.2 == principal.agent_id
            })
            .map(|(k, v)| (k.3.clone(), Self::cosine_similarity(query, v)))
            .filter(|(_, score)| *score > 0.0)
            .collect();
        results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        results.truncate(limit);
        Ok(results)
    }

    pub fn count(&self, principal: &AccessPrincipal) -> Result<usize, MemoryError> {
        validate_principal(principal)?;
        Ok(self
            .vectors
            .keys()
            .filter(|k| {
                k.0 == principal.tenant_id && k.1 == principal.user_id && k.2 == principal.agent_id
            })
            .count())
    }
}

impl Default for VectorStore {
    fn default() -> Self {
        Self::new()
    }
}

fn scoped_key(principal: &AccessPrincipal, key: &str) -> ScopedKey {
    (
        principal.tenant_id.clone(),
        principal.user_id.clone(),
        principal.agent_id.clone(),
        key.to_owned(),
    )
}

fn validate_principal(principal: &AccessPrincipal) -> Result<(), MemoryError> {
    if principal.tenant_id.trim().is_empty()
        || principal.user_id.trim().is_empty()
        || principal.agent_id.trim().is_empty()
    {
        return Err(MemoryError::InvalidInput(
            "principal fields must not be empty",
        ));
    }
    Ok(())
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

    #[test]
    fn search_and_get_are_principal_scoped() {
        let mut vs = VectorStore::new();
        let alice = principal("tenant-a", "alice", "agent-a");
        let bob = principal("tenant-b", "bob", "agent-b");
        vs.store(&alice, "same-key", vec![1.0, 0.0, 0.0]).unwrap();
        vs.store(&bob, "secret", vec![1.0, 0.0, 0.0]).unwrap();
        assert_eq!(vs.get(&alice, "secret").unwrap(), None);
        assert_eq!(
            vs.search(&alice, &[1.0, 0.0, 0.0], 10).unwrap(),
            vec![("same-key".into(), 1.0)]
        );
        assert_eq!(vs.count(&alice).unwrap(), 1);
    }

    #[test]
    fn empty_principal_is_rejected() {
        let mut vs = VectorStore::new();
        assert_eq!(
            vs.store(&principal("", "user", "agent"), "key", vec![1.0]),
            Err(MemoryError::InvalidInput(
                "principal fields must not be empty"
            ))
        );
    }
}
