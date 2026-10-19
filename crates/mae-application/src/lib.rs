//! Application and Integration Layer for MAE (Migrate Absolutely Everything).
//!
//! Implements Clean Architecture use cases and adapter integrations:
//! - Open Prompt Format (OPF) 0.2 prompt loading, validation, template rendering, and cryptographic provenance.
//! - Open Knowledge Format (OKF) repository brain bundle validation and search.
//! - Versioned artifact storage with cryptographic content-hash integrity verification.
//! - AAMF runtime wave-barrier topological scheduling and deterministic checkpoint persistence.
//! - AI agent backend CLI argument mappings (Codex, Claude Code, Copilot, DeterministicMock).
//! - Complete end-to-end MigrationOrchestrator coordinating all port adapters.

pub mod aamf_runtime;
pub mod artifact_store;
pub mod okf;
pub mod opf;
pub mod orchestrator;

pub use aamf_runtime::{
    CheckpointManager, CheckpointSnapshot, ClaudeCodeAgentBackend, CodexAgentBackend,
    CopilotAgentBackend, DeterministicMockAgentBackend, ExecutionMode, WaveSchedule,
};
pub use artifact_store::{FileSystemArtifactRepository, InMemoryArtifactRepository};
pub use okf::OkfKnowledgeRepository;
pub use opf::{LegacyPromptProvider, OpfPrompt, OpfPromptProvider};
pub use orchestrator::{MigrationOrchestrator, MigrationRunReport};
