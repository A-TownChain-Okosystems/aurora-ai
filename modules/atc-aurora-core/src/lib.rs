// atc-aurora-core — Central AI Engine, Model Hub, LLM Router
// Copyright (c) 2026 Michael Wroblewski / ShivaCore / A-TownChain-Okosystems. All Rights Reserved.

pub mod action_executor;
pub mod agent_registry;
pub mod approval;
pub mod audit;
pub mod aurora_core;
pub mod capability_policy;
pub mod config_manager;
pub mod event;
pub mod llm_router;
pub mod model_hub;
pub mod state;

pub use action_executor::{ActionExecutor, ExecutorError};
pub use agent_registry::AgentRegistry;
pub use approval::{ApprovalEngine, ApprovalError, VerifiedApproval};
pub use audit::{AuditError, AuditLogger, AuditRecord};
pub use aurora_core::AuroraCore;
pub use capability_policy::{
    ActionRequest, ApprovalGrant, CapabilityPolicy, CapabilityPolicyEngine, PolicyDecision,
};
pub use config_manager::ConfigManager;
pub use event::{AuroraEvent, EventBus, EventBusError, EventKind};
pub use llm_router::LlmRouter;
pub use model_hub::ModelHub;
pub use state::{ExecutionState, StateMachine, StateTransitionError};
