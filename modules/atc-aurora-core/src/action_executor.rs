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
/// The closure is never invoked for Deny or ApprovalRequired. Privileged
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

    fn request(capability: &str, action: &str, request_id: &str) -> ActionRequest {
        ActionRequest {
            request_id: request_id.into(),
            principal: "agent:assistant".into(),
            capability: capability.into(),
            action: action.into(),
            scope: "workspace:project-a".into(),
            approval: None,
        }
    }

    fn read_request() -> ActionRequest {
        request("filesystem.read", "read_file", "request:read-1")
    }

    fn write_request() -> ActionRequest {
        request("filesystem.write", "write_file", "request:write-1")
    }

    fn active_core() -> AuroraCore {
        let mut core = AuroraCore::new();
        core.start().unwrap();
        core.agent_registry.register(
            "agent:assistant",
            "Assistant",
            "assistant",
            vec![
                "filesystem.read".into(),
                "filesystem.write".into(),
                "wallet.export_key".into(),
            ],
        );
        core.capability_policy.register(CapabilityPolicy::new(
            "agent:assistant",
            "filesystem.read",
            "read_file",
            "workspace:project-a",
            false,
        ));
        core.capability_policy.register(CapabilityPolicy::new(
            "agent:assistant",
            "filesystem.write",
            "write_file",
            "workspace:project-a",
            true,
        ));
        // Even a mistakenly permissive policy must not override terminal deny.
        core.capability_policy.register(CapabilityPolicy::new(
            "agent:assistant",
            "wallet.export_key",
            "export_private_key",
            "workspace:project-a",
            false,
        ));
        core
    }

    #[test]
    fn filesystem_read_runs_once_when_allowed() {
        let core = active_core();
        let calls = Cell::new(0);
        let result = execute_authorized(&core, "agent:assistant", &read_request(), || {
            calls.set(calls.get() + 1);
            Ok::<_, ()>("read")
        });

        assert_eq!(result, Ok("read"));
        assert_eq!(calls.get(), 1);
    }

    #[test]
    fn filesystem_write_requires_approval_and_has_no_side_effect_without_it() {
        let core = active_core();
        let calls = Cell::new(0);
        let result = execute_authorized(&core, "agent:assistant", &write_request(), || {
            calls.set(calls.get() + 1);
            Ok::<_, ()>(())
        });

        assert_eq!(result, Err(ActionExecutionError::ApprovalRequired));
        assert_eq!(calls.get(), 0);
    }

    #[test]
    fn unknown_policy_is_denied_without_side_effect() {
        let mut core = active_core();
        core.agent_registry.register(
            "agent:untrusted",
            "Untrusted",
            "untrusted",
            vec!["filesystem.read".into()],
        );
        let mut request = read_request();
        request.principal = "agent:untrusted".into();
        let calls = Cell::new(0);
        let result = execute_authorized(&core, "agent:untrusted", &request, || {
            calls.set(calls.get() + 1);
            Ok::<_, ()>(())
        });

        assert_eq!(result, Err(ActionExecutionError::Denied));
        assert_eq!(calls.get(), 0);
    }

    #[test]
    fn identity_mismatch_has_no_side_effect() {
        let core = active_core();
        let calls = Cell::new(0);
        let result = execute_authorized(&core, "agent:other", &read_request(), || {
            calls.set(calls.get() + 1);
            Ok::<_, ()>(())
        });

        assert_eq!(result, Err(ActionExecutionError::Denied));
        assert_eq!(calls.get(), 0);
    }

    #[test]
    fn terminal_wallet_key_export_deny_has_no_side_effect() {
        let core = active_core();
        let request = request(
            "wallet.export_key",
            "export_private_key",
            "request:wallet-export",
        );
        let calls = Cell::new(0);
        let result = execute_authorized(&core, "agent:assistant", &request, || {
            calls.set(calls.get() + 1);
            Ok::<_, ()>(())
        });

        assert_eq!(result, Err(ActionExecutionError::Denied));
        assert_eq!(calls.get(), 0);
    }

    #[test]
    fn action_failure_is_reported_after_authorization() {
        let core = active_core();
        let result = execute_authorized(&core, "agent:assistant", &read_request(), || {
            Err::<(), _>("adapter failure")
        });

        assert_eq!(result, Err(ActionExecutionError::Action("adapter failure")));
    }
}
