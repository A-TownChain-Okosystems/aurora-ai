// Copyright (c) 2026 Michael Wroblewski / ShivaCore / A-TownChain-Okosystems. All Rights Reserved.
// Fail-closed execution boundary for privileged actions.

use crate::{ActionRequest, AuroraCore, PolicyDecision};

/// Failure returned when authorization does not permit an action to run.
#[derive(Debug, PartialEq, Eq)]
pub enum ActionExecutionError<E> {
    /// No matching, unambiguous policy authorized the request.
    Denied,
    /// The policy requires an approval that is not present and valid.
    ApprovalRequired,
    /// Authorization passed, but the action itself failed.
    Action(E),
}

/// Evaluate authorization immediately before invoking an action closure.
///
/// The closure is never invoked for `Deny` or `ApprovalRequired`. Privileged
/// adapters should place the complete side-effecting operation inside this closure
/// and must not perform part of the operation before calling this function.
///
/// This is an in-process enforcement boundary; OS/kernel adapters must still
/// independently enforce their own capabilities.
pub fn execute_authorized<T, E, F>(
    core: &AuroraCore,
    agent_id: &str,
    request: &ActionRequest,
    action: F,
) -> Result<T, ActionExecutionError<E>>
where
    F: FnOnce() -> Result<T, E>,
{
    match core.authorize_action(agent_id, request) {
        PolicyDecision::Allow => action().map_err(ActionExecutionError::Action),
        PolicyDecision::Deny => Err(ActionExecutionError::Denied),
        PolicyDecision::ApprovalRequired => Err(ActionExecutionError::ApprovalRequired),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::CapabilityPolicy;
    use std::cell::Cell;

    fn request() -> ActionRequest {
        ActionRequest {
            request_id: "request:secure-executor-1".into(),
            principal: "agent:assistant".into(),
            capability: "filesystem.write".into(),
            action: "write_file".into(),
            scope: "workspace:project-a".into(),
            approval: None,
        }
    }

    fn active_core(approval_required: bool) -> AuroraCore {
        let mut core = AuroraCore::new();
        core.start().unwrap();
        core.agent_registry.register(
            "agent:assistant",
            "Assistant",
            "assistant",
            vec!["filesystem.write".into()],
        );
        core.capability_policy.register(CapabilityPolicy::new(
            "agent:assistant",
            "filesystem.write",
            "write_file",
            "workspace:project-a",
            approval_required,
        ));
        core
    }

    #[test]
    fn allowed_action_runs_once() {
        let core = active_core(false);
        let calls = Cell::new(0);
        let result = execute_authorized(&core, "agent:assistant", &request(), || {
            calls.set(calls.get() + 1);
            Ok::<_, ()>("written")
        });

        assert_eq!(result, Ok("written"));
        assert_eq!(calls.get(), 1);
    }

    #[test]
    fn denied_action_has_no_side_effect() {
        let mut core = AuroraCore::new();
        core.start().unwrap();
        core.agent_registry.register(
            "agent:assistant",
            "Assistant",
            "assistant",
            vec!["filesystem.write".into()],
        );
        let calls = Cell::new(0);
        let result = execute_authorized(&core, "agent:assistant", &request(), || {
            calls.set(calls.get() + 1);
            Ok::<_, ()>(())
        });

        assert_eq!(result, Err(ActionExecutionError::Denied));
        assert_eq!(calls.get(), 0);
    }

    #[test]
    fn approval_required_action_has_no_side_effect() {
        let core = active_core(true);
        let calls = Cell::new(0);
        let result = execute_authorized(&core, "agent:assistant", &request(), || {
            calls.set(calls.get() + 1);
            Ok::<_, ()>(())
        });

        assert_eq!(result, Err(ActionExecutionError::ApprovalRequired));
        assert_eq!(calls.get(), 0);
    }

    #[test]
    fn identity_mismatch_has_no_side_effect() {
        let core = active_core(false);
        let calls = Cell::new(0);
        let result = execute_authorized(&core, "agent:other", &request(), || {
            calls.set(calls.get() + 1);
            Ok::<_, ()>(())
        });

        assert_eq!(result, Err(ActionExecutionError::Denied));
        assert_eq!(calls.get(), 0);
    }

    #[test]
    fn action_failure_is_reported_after_authorization() {
        let core = active_core(false);
        let result = execute_authorized(&core, "agent:assistant", &request(), || {
            Err::<(), _>("adapter failure")
        });

        assert_eq!(result, Err(ActionExecutionError::Action("adapter failure")));
    }
}
