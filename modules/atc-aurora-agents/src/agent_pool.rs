// Copyright (c) 2026 Michael Wroblewski / ShivaCore / A-TownChain-Okosystems. All Rights Reserved.
// Agent Pool — manages all 12 agents.

use std::collections::HashMap;

use atc_aurora_core::{execute_authorized, ActionExecutionError, ActionRequest, AuroraCore};
use crate::agent_base::{Agent, AgentContext, AgentResponse};
use crate::agents::*;

#[derive(Debug, PartialEq, Eq)]
pub enum AuthorizedAgentExecutionError {
    AgentNotFound(String),
    Authorization(ActionExecutionError<std::convert::Infallible>),
}

pub struct AgentPool {
    agents: HashMap<String, Box<dyn Agent>>,
}

impl AgentPool {
    pub fn new() -> Self {
        let mut pool = Self {
            agents: HashMap::new(),
        };
        pool.register(Box::new(GovernanceAgent));
        pool.register(Box::new(ProductAgent));
        pool.register(Box::new(RoadmapAgent));
        pool.register(Box::new(SecurityAgent));
        pool.register(Box::new(AnalyticsAgent));
        pool.register(Box::new(DevOpsAgent));
        pool.register(Box::new(CommunityAgent));
        pool.register(Box::new(TreasuryAgent));
        pool.register(Box::new(AuditAgent));
        pool.register(Box::new(ResearchAgent));
        pool.register(Box::new(SupportAgent));
        pool.register(Box::new(BridgeAgent));
        pool
    }

    pub fn register(&mut self, agent: Box<dyn Agent>) {
        let id = agent.id().to_string();
        self.agents.insert(id, agent);
    }

    /// Execute an agent's pure planning/response step only.
    ///
    /// This method is not an authorization boundary and must not be used by adapters
    /// to perform privileged tool, filesystem, network, wallet, or IPC side effects.
    /// Privileged dispatch must use execute_authorized with the canonical AuroraCore.
    pub fn execute(&self, agent_id: &str, ctx: AgentContext) -> Result<AgentResponse, String> {
        let agent = self
            .agents
            .get(agent_id)
            .ok_or_else(|| format!("Agent '{}' not found", agent_id))?;
        Ok(agent.execute(ctx))
    }

    /// Dispatch through Aurora's canonical fail-closed authorization gateway.
    ///
    /// principal_id is the security identity registered in AuroraCore; agent_id selects
    /// the implementation in this pool. The caller must supply an ActionRequest whose
    /// principal, capability, action and scope are registered in the canonical policy
    /// engine. The closure containing agent execution is never invoked on Deny or
    /// ApprovalRequired. Kernel/OS enforcement remains independently required.
    pub fn execute_authorized(
        &self,
        core: &AuroraCore,
        principal_id: &str,
        agent_id: &str,
        request: &ActionRequest,
        ctx: AgentContext,
    ) -> Result<AgentResponse, AuthorizedAgentExecutionError> {
        let agent = self
            .agents
            .get(agent_id)
            .ok_or_else(|| AuthorizedAgentExecutionError::AgentNotFound(agent_id.to_string()))?;

        if request.principal != principal_id {
            return Err(AuthorizedAgentExecutionError::Authorization(
                ActionExecutionError::Denied,
            ));
        }

        execute_authorized(core, principal_id, request, || {
            Ok::<AgentResponse, std::convert::Infallible>(agent.execute(ctx))
        })
        .map_err(AuthorizedAgentExecutionError::Authorization)
    }

    pub fn list(&self) -> Vec<(&str, &str, &str)> {
        self.agents
            .values()
            .map(|a| (a.id(), a.name(), a.role()))
            .collect()
    }

    pub fn count(&self) -> usize {
        self.agents.len()
    }
}

impl Default for AgentPool {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use atc_aurora_core::{CapabilityPolicy, PolicyDecision};

    fn authorized_core(approval_required: bool) -> AuroraCore {
        let mut core = AuroraCore::new();
        core.start().unwrap();
        core.agent_registry.register(
            "agent:governance",
            "Governance",
            "governance",
            vec!["agent.execute".into()],
        );
        core.capability_policy.register(CapabilityPolicy::new(
            "agent:governance",
            "agent.execute",
            "execute_agent",
            "agent:governance",
            approval_required,
        ));
        core
    }

    fn request() -> ActionRequest {
        ActionRequest {
            request_id: "request:agent-execute-1".into(),
            principal: "agent:governance".into(),
            capability: "agent.execute".into(),
            action: "execute_agent".into(),
            scope: "agent:governance".into(),
            approval: None,
        }
    }

    #[test]
    fn test_pool() {
        let pool = AgentPool::new();
        assert_eq!(pool.count(), 12);
        let ctx = AgentContext::new("task-1", "test");
        assert!(pool.execute("governance", ctx).is_ok());
        assert!(pool
            .execute("nonexistent", AgentContext::new("x", "y"))
            .is_err());
    }

    #[test]
    fn authorized_dispatch_runs_when_policy_allows() {
        let pool = AgentPool::new();
        let core = authorized_core(false);
        let result = pool.execute_authorized(
            &core,
            "agent:governance",
            "governance",
            &request(),
            AgentContext::new("task-allowed", "test"),
        );
        assert!(result.is_ok());
        assert_eq!(core.authorize_action("agent:governance", &request()), PolicyDecision::Allow);
    }

    #[test]
    fn approval_required_does_not_dispatch_agent() {
        let pool = AgentPool::new();
        let core = authorized_core(true);
        let result = pool.execute_authorized(
            &core,
            "agent:governance",
            "governance",
            &request(),
            AgentContext::new("task-needs-approval", "test"),
        );
        assert!(matches!(
            result,
            Err(AuthorizedAgentExecutionError::Authorization(
                ActionExecutionError::ApprovalRequired
            ))
        ));
    }

    #[test]
    fn mismatched_principal_is_denied_before_dispatch() {
        let pool = AgentPool::new();
        let core = authorized_core(false);
        let mut mismatched = request();
        mismatched.principal = "agent:attacker".into();
        let result = pool.execute_authorized(
            &core,
            "agent:governance",
            "governance",
            &mismatched,
            AgentContext::new("task-mismatch", "test"),
        );
        assert!(matches!(
            result,
            Err(AuthorizedAgentExecutionError::Authorization(
                ActionExecutionError::Denied
            ))
        ));
    }
}
