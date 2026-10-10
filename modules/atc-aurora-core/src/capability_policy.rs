// Copyright (c) 2026 Michael Wroblewski / ShivaCore / A-TownChain-Okosystems. All Rights Reserved.
// Aurora capability/policy boundary.
//
// Security invariant:
// model output is never an authorization source.
// Authorization is evaluated before a tool/action is executed.
// Unknown identities, capabilities, scopes or policies fail closed.

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActionRequest {
    pub principal: String,
    pub capability: String,
    pub action: String,
    pub scope: String,
    pub approval: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PolicyDecision {
    Allow,
    Deny,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapabilityPolicy {
    principal: String,
    capability: String,
    action: String,
    scope: String,
    approval_required: bool,
}

impl CapabilityPolicy {
    pub fn new(
        principal: &str,
        capability: &str,
        action: &str,
        scope: &str,
        approval_required: bool,
    ) -> Self {
        Self {
            principal: principal.into(),
            capability: capability.into(),
            action: action.into(),
            scope: scope.into(),
            approval_required,
        }
    }

    pub fn evaluate(&self, request: &ActionRequest) -> PolicyDecision {
        if request.principal != self.principal
            || request.capability != self.capability
            || request.action != self.action
            || request.scope != self.scope
        {
            return PolicyDecision::Deny;
        }

        if self.approval_required && !request.approval {
            return PolicyDecision::Deny;
        }

        PolicyDecision::Allow
    }
}

#[derive(Debug, Default)]
pub struct CapabilityPolicyEngine {
    policies: Vec<CapabilityPolicy>,
}

impl CapabilityPolicyEngine {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&mut self, policy: CapabilityPolicy) {
        self.policies.push(policy);
    }

    pub fn evaluate(&self, request: &ActionRequest) -> PolicyDecision {
        self.policies
            .iter()
            .find(|policy| {
                policy.principal == request.principal
                    && policy.capability == request.capability
                    && policy.action == request.action
                    && policy.scope == request.scope
            })
            .map(|policy| policy.evaluate(request))
            .unwrap_or(PolicyDecision::Deny)
    }

    pub fn policy_count(&self) -> usize {
        self.policies.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn policy() -> CapabilityPolicy {
        CapabilityPolicy::new(
            "agent:test",
            "chain.tx.submit",
            "submit_transaction",
            "chain:mainnet",
            true,
        )
    }

    #[test]
    fn exact_match_requires_approval() {
        let mut engine = CapabilityPolicyEngine::new();
        engine.register(policy());

        let denied = ActionRequest {
            principal: "agent:test".into(),
            capability: "chain.tx.submit".into(),
            action: "submit_transaction".into(),
            scope: "chain:mainnet".into(),
            approval: false,
        };
        assert_eq!(engine.evaluate(&denied), PolicyDecision::Deny);

        let allowed = ActionRequest { approval: true, ..denied };
        assert_eq!(engine.evaluate(&allowed), PolicyDecision::Allow);
    }

    #[test]
    fn unknown_capability_fails_closed() {
        let mut engine = CapabilityPolicyEngine::new();
        engine.register(policy());

        let request = ActionRequest {
            principal: "agent:test".into(),
            capability: "chain.admin".into(),
            action: "submit_transaction".into(),
            scope: "chain:mainnet".into(),
            approval: true,
        };
        assert_eq!(engine.evaluate(&request), PolicyDecision::Deny);
    }

    #[test]
    fn scope_mismatch_fails_closed() {
        let mut engine = CapabilityPolicyEngine::new();
        engine.register(policy());

        let request = ActionRequest {
            principal: "agent:test".into(),
            capability: "chain.tx.submit".into(),
            action: "submit_transaction".into(),
            scope: "chain:testnet".into(),
            approval: true,
        };
        assert_eq!(engine.evaluate(&request), PolicyDecision::Deny);
    }
}
