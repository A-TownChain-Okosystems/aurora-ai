// Copyright (c) 2026 Michael Wroblewski / ShivaCore / A-TownChain-Okosystems. All Rights Reserved.
// In-memory approval reference implementation. The caller of approve_verified must be a
// trusted adapter that has authenticated the human decision; this module does not perform I/O.

use std::collections::HashMap;

use crate::capability_policy::{ActionRequest, ApprovalGrant};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApprovalError {
    EmptyRequestId,
    EmptyApprover,
    RequestAlreadyPending,
    RequestNotPending,
    RequestAlreadyCarriesApproval,
}

#[derive(Debug, Default)]
pub struct ApprovalEngine {
    pending: HashMap<String, ActionRequest>,
}

impl ApprovalEngine {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn request_approval(&mut self, request: ActionRequest) -> Result<(), ApprovalError> {
        if request.request_id.is_empty() {
            return Err(ApprovalError::EmptyRequestId);
        }
        if request.approval.is_some() {
            return Err(ApprovalError::RequestAlreadyCarriesApproval);
        }
        if self.pending.contains_key(&request.request_id) {
            return Err(ApprovalError::RequestAlreadyPending);
        }

        self.pending.insert(request.request_id.clone(), request);
        Ok(())
    }

    /// Records an approval already authenticated by the trusted UI/identity boundary.
    /// This method is crate-private so untrusted external callers cannot mint grants.
    pub(crate) fn approve_verified(
        &mut self,
        request_id: &str,
        approver_id: &str,
    ) -> Result<ApprovalGrant, ApprovalError> {
        if approver_id.is_empty() {
            return Err(ApprovalError::EmptyApprover);
        }

        let request = self
            .pending
            .remove(request_id)
            .ok_or(ApprovalError::RequestNotPending)?;
        Ok(ApprovalGrant::verified_for(&request, approver_id))
    }

    pub fn reject(&mut self, request_id: &str) -> bool {
        self.pending.remove(request_id).is_some()
    }

    pub fn pending_count(&self) -> usize {
        self.pending.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request() -> ActionRequest {
        ActionRequest {
            request_id: "request:approval-1".into(),
            principal: "agent:assistant".into(),
            capability: "filesystem.write".into(),
            action: "write_file".into(),
            scope: "workspace:project-a".into(),
            approval: None,
        }
    }

    #[test]
    fn verified_approval_is_one_shot_and_bound_to_request() {
        let mut engine = ApprovalEngine::new();
        engine.request_approval(request()).unwrap();

        let grant = engine
            .approve_verified("request:approval-1", "human:owner")
            .unwrap();

        assert_eq!(grant.approver_id(), "human:owner");
        assert_eq!(engine.pending_count(), 0);
        assert_eq!(
            engine.approve_verified("request:approval-1", "human:owner"),
            Err(ApprovalError::RequestNotPending)
        );
    }

    #[test]
    fn rejects_duplicate_pending_request() {
        let mut engine = ApprovalEngine::new();
        engine.request_approval(request()).unwrap();
        assert_eq!(
            engine.request_approval(request()),
            Err(ApprovalError::RequestAlreadyPending)
        );
    }

    #[test]
    fn reject_removes_pending_request() {
        let mut engine = ApprovalEngine::new();
        engine.request_approval(request()).unwrap();
        assert!(engine.reject("request:approval-1"));
        assert_eq!(engine.pending_count(), 0);
        assert!(!engine.reject("request:approval-1"));
    }
}
