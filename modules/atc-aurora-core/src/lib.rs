// atc-aurora-core — Central AI Engine, Model Hub, LLM Router
// Copyright (c) 2026 Michael Wroblewski / ShivaCore / A-TownChain-Okosystems. All Rights Reserved.

pub mod aurora_core;
pub mod model_hub;
pub mod llm_router;
pub mod agent_registry;
pub mod task_scheduler;
pub mod result_collector;
pub mod config_manager;
pub mod approval;
pub mod audit;
pub mod capability_policy;
pub mod event;
pub mod state;

pub use aurora_core::AuroraCore;
pub use model_hub::ModelHub;
pub use llm_router::LlmRouter;
pub use agent_registry::{AgentInfo, AgentRegistry, CapabilityMetadata, RegistryError};
pub use task_scheduler::{SchedulerError, Task, TaskScheduler, TaskState};
pub use result_collector::{AgentResult, CollectorError, ResultCollector};
pub use config_manager::ConfigManager;
pub use approval::{ApprovalEngine, ApprovalError, VerifiedApproval};
pub use audit::{AuditError, AuditLogger, AuditRecord};
pub use capability_policy::{
    ActionRequest, ApprovalGrant, CapabilityPolicy, CapabilityPolicyEngine, PolicyDecision,
};
pub use event::{AuroraEvent, EventBus, EventBusError, EventKind};
pub use state::{ExecutionState, StateMachine, StateTransitionError};
