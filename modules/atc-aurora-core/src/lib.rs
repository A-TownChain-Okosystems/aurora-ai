// atc-aurora-core — Central AI Engine, Model Hub, LLM Router
// Copyright (c) 2026 Michael Wroblewski / ShivaCore / A-TownChain-Okosystems. All Rights Reserved.

pub mod aurora_core;
pub mod model_hub;
pub mod llm_router;
pub mod agent_registry;
pub mod config_manager;
pub mod capability_policy;
pub mod state;

pub use aurora_core::AuroraCore;
pub use model_hub::ModelHub;
pub use llm_router::LlmRouter;
pub use agent_registry::AgentRegistry;
pub use config_manager::ConfigManager;
pub use capability_policy::{
    ActionRequest, ApprovalGrant, CapabilityPolicy, CapabilityPolicyEngine, PolicyDecision,
};
pub use state::{ExecutionState, StateMachine, StateTransitionError};
