// Copyright (c) 2026 Michael Wroblewski / ShivaCore / A-TownChain-Okosystems. All Rights Reserved.
// atc-aurora-memory — Knowledge Base, Vector Store, Learning
pub mod knowledge_base;
pub mod vector_store;
pub mod memory_index;
pub mod learning_pipeline;
pub mod context_window;
pub mod memory_engine;

pub use knowledge_base::KnowledgeBase;
pub use vector_store::VectorStore;
pub use memory_index::MemoryIndex;
pub use learning_pipeline::LearningPipeline;
pub use context_window::ContextWindow;
pub use memory_engine::{
    AccessPrincipal, BuiltContext, ContextItem, DerivedArtifactCleaner, ExplanationRecord,
    ExplanationSource, MemoryDraft, MemoryEntry, MemoryError, MemoryKind, MemoryManager,
    Provenance, ShortTermMessage, TrustLevel,
};
