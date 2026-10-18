// Copyright (c) 2026 Michael Wroblewski / ShivaCore / A-TownChain-Okosystems. All Rights Reserved.
// Learning pipeline — feedback state is partitioned by authenticated principal.
use std::collections::HashMap;
use crate::{AccessPrincipal, MemoryError};

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct LearningState {
    pub iterations: u64,
    pub accuracy: f64,
}

pub struct LearningPipeline {
    states: HashMap<(String, String, String), LearningState>,
}

impl LearningPipeline {
    pub fn new() -> Self { Self { states: HashMap::new() } }

    pub fn train(&mut self, principal: &AccessPrincipal, feedback_score: f64) -> Result<LearningState, MemoryError> {
        validate_principal(principal)?;
        if !feedback_score.is_finite() || !(0.0..=1.0).contains(&feedback_score) {
            return Err(MemoryError::InvalidInput("feedback score must be finite and between zero and one"));
        }
        let key = principal_key(principal);
        let state = self.states.entry(key).or_default();
        state.iterations = state.iterations.checked_add(1)
            .ok_or(MemoryError::InvalidInput("learning iteration counter exhausted"))?;
        let alpha = 0.01;
        state.accuracy += alpha * (feedback_score - state.accuracy);
        Ok(*state)
    }

    pub fn state(&self, principal: &AccessPrincipal) -> Result<LearningState, MemoryError> {
        validate_principal(principal)?;
        Ok(self.states.get(&principal_key(principal)).copied().unwrap_or_default())
    }

    pub fn is_converged(&self, principal: &AccessPrincipal, threshold: f64) -> Result<bool, MemoryError> {
        if !threshold.is_finite() || !(0.0..=1.0).contains(&threshold) {
            return Err(MemoryError::InvalidInput("threshold must be finite and between zero and one"));
        }
        Ok(self.state(principal)?.accuracy >= threshold)
    }
}

fn principal_key(principal: &AccessPrincipal) -> (String, String, String) {
    (principal.tenant_id.clone(), principal.user_id.clone(), principal.agent_id.clone())
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
    fn training_state_is_principal_scoped() {
        let mut lp = LearningPipeline::new();
        let alice = principal("tenant-a", "alice", "agent-a");
        let bob = principal("tenant-b", "bob", "agent-b");
        for _ in 0..100 { lp.train(&alice, 0.95).unwrap(); }
        assert!(lp.state(&alice).unwrap().accuracy > 0.5);
        assert!(lp.is_converged(&alice, 0.5).unwrap());
        assert_eq!(lp.state(&bob).unwrap(), LearningState::default());
        assert!(!lp.is_converged(&bob, 0.5).unwrap());
    }

    #[test]
    fn invalid_feedback_is_rejected_without_mutation() {
        let mut lp = LearningPipeline::new();
        let alice = principal("tenant-a", "alice", "agent-a");
        assert!(lp.train(&alice, f64::NAN).is_err());
        assert_eq!(lp.state(&alice).unwrap(), LearningState::default());
    }
}
