// Copyright (c) 2026 Michael Wroblewski / ShivaCore / A-TownChain-Okosystems. All Rights Reserved.
// Central fail-closed dispatcher for privileged tool actions.
//
// This dispatcher only protects actions routed through it. Host/OS adapters must still
// enforce their own capabilities and must not expose a bypass around this boundary.

use std::collections::HashMap;

use crate::{ActionRequest, AuroraCore, PolicyDecision};

type ActionHandler = dyn Fn(&ActionRequest) -> Result<String, String> + Send + Sync;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecutorError {
    InvalidRegistration,
    DuplicateAction,
    UnknownAction,
    CapabilityMismatch,
    Denied,
    ApprovalRequired,
    HandlerFailed,
}

pub struct ActionExecutor {
    handlers: HashMap<String, RegisteredHandler>,
}

struct RegisteredHandler {
    capability: String,
    run: Box<ActionHandler>,
}

impl Default for ActionExecutor {
    fn default() -> Self {
        Self::new()
    }
}

impl ActionExecutor {
    pub fn new() -> Self {
        Self { handlers: HashMap::new() }
    }

    /// Register a trusted application-owned handler. Do not register handlers or
    /// capabilities from model output or untrusted requests.
    pub fn register<F>(
        &mut self,
        action: &str,
        capability: &str,
        handler: F,
    ) -> Result<(), ExecutorError>
    where
        F: Fn(&ActionRequest) -> Result<String, String> + Send + Sync + 'static,
    {
        if action.trim().is_empty() || capability.trim().is_empty() {
            return Err(ExecutorError::InvalidRegistration);
        }
        if self.handlers.contains_key(action) {
            return Err(ExecutorError::DuplicateAction);
        }
        self.handlers.insert(action.to_owned(), RegisteredHandler {
            capability: capability.to_owned(),
            run: Box::new(handler),
        });
        Ok(())
    }

    /// Resolve a handler by its registered action name, check its capability binding,
    /// and authorize the exact request before invoking any handler code. Every non-Allow
    /// decision and every validation error returns without running the handler.
    pub fn execute(
        &self,
        core: &AuroraCore,
        agent_id: &str,
        request: &ActionRequest,
    ) -> Result<String, ExecutorError> {
        let handler = self.handlers.get(&request.action).ok_or(ExecutorError::UnknownAction)?;
        if handler.capability != request.capability {
            return Err(ExecutorError::CapabilityMismatch);
        }

        match core.authorize_action(agent_id, request) {
            PolicyDecision::Allow => {}
            PolicyDecision::Deny => return Err(ExecutorError::Denied),
            PolicyDecision::ApprovalRequired => return Err(ExecutorError::ApprovalRequired),
        }

        (handler.run)(request).map_err(|_| ExecutorError::HandlerFailed)
    }

    pub fn registered_action_count(&self) -> usize {
        self.handlers.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::CapabilityPolicy;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    fn setup(approval_required: bool) -> (AuroraCore, ActionRequest) {
        let mut core = AuroraCore::new();
        core.start().unwrap();
        core.agent_registry.register(
            "agent:assistant",
            "Assistant",
            "assistant",
            vec!["filesystem.read".into()],
        );
        core.capability_policy.register(CapabilityPolicy::new(
            "agent:assistant",
            "filesystem.read",
            "read_file",
            "workspace:project-a",
            approval_required,
        ));
        let request = ActionRequest {
            request_id: "request:executor-1".into(),
            principal: "agent:assistant".into(),
            capability: "filesystem.read".into(),
            action: "read_file".into(),
            scope: "workspace:project-a".into(),
            approval: None,
        };
        (core, request)
    }

    #[test]
    fn deny_does_not_invoke_handler() {
        let (core, mut request) = setup(false);
        request.scope = "workspace:other".into();
        let calls = Arc::new(AtomicUsize::new(0));
        let observed = Arc::clone(&calls);
        let mut executor = ActionExecutor::new();
        executor.register("read_file", "filesystem.read", move |_| {
            observed.fetch_add(1, Ordering::SeqCst);
            Ok("should not run".into())
        }).unwrap();

        assert_eq!(executor.execute(&core, "agent:assistant", &request), Err(ExecutorError::Denied));
        assert_eq!(calls.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn approval_required_does_not_invoke_handler() {
        let (core, request) = setup(true);
        let calls = Arc::new(AtomicUsize::new(0));
        let observed = Arc::clone(&calls);
        let mut executor = ActionExecutor::new();
        executor.register("read_file", "filesystem.read", move |_| {
            observed.fetch_add(1, Ordering::SeqCst);
            Ok("should not run".into())
        }).unwrap();

        assert_eq!(executor.execute(&core, "agent:assistant", &request), Err(ExecutorError::ApprovalRequired));
        assert_eq!(calls.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn handler_capability_mismatch_does_not_invoke_handler() {
        let (core, request) = setup(false);
        let calls = Arc::new(AtomicUsize::new(0));
        let observed = Arc::clone(&calls);
        let mut executor = ActionExecutor::new();
        executor.register("read_file", "filesystem.write", move |_| {
            observed.fetch_add(1, Ordering::SeqCst);
            Ok("should not run".into())
        }).unwrap();

        assert_eq!(executor.execute(&core, "agent:assistant", &request), Err(ExecutorError::CapabilityMismatch));
        assert_eq!(calls.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn allow_invokes_only_the_registered_handler() {
        let (core, request) = setup(false);
        let calls = Arc::new(AtomicUsize::new(0));
        let observed = Arc::clone(&calls);
        let mut executor = ActionExecutor::new();
        executor.register("read_file", "filesystem.read", move |request| {
            observed.fetch_add(1, Ordering::SeqCst);
            Ok(request.scope.clone())
        }).unwrap();

        assert_eq!(executor.execute(&core, "agent:assistant", &request), Ok("workspace:project-a".into()));
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn duplicate_action_registration_is_rejected() {
        let mut executor = ActionExecutor::new();
        executor.register("read_file", "filesystem.read", |_| Ok("first".into())).unwrap();
        assert_eq!(
            executor.register("read_file", "filesystem.read", |_| Ok("second".into())),
            Err(ExecutorError::DuplicateAction)
        );
    }
}
