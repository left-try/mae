//! Pure Domain Core for MAE (Migrate Absolutely Everything).
//!
//! Enforces Domain-Driven Design (DDD), Object-Oriented Design (OOP),
//! SOLID principles, and Clean Architecture with ZERO outward dependencies (std only).

pub mod aggregate;
pub mod artifact;
pub mod capability;
pub mod contract;
pub mod eval;
pub mod ids;
pub mod policies;
pub mod ports;
pub mod semantic_cli;
pub mod semantic_ui;
pub mod task;
pub mod verification;

pub use aggregate::{DomainEvent, MigrationAggregate};
pub use artifact::{ArtifactKind, ArtifactProvenance, ProvenanceOrigin, VersionedArtifact};
pub use capability::{
    CapabilityAssessment, CapabilityLevel, CapabilityReport, GapReasonCode, MigrationGap,
    MigrationStrategy,
};
pub use contract::{
    Contract, ContractSeverity, ContractStimulus, ObservableExpectation, TolerancePolicy,
    VerificationCategory,
};
pub use eval::{estimate_token_count, MigrationEvalMetrics};
pub use ids::{
    ArtifactId, ArtifactRevision, ContentHash, ContractId, MigrationId, PlanHash, TaskId,
};
pub use policies::{
    AcceptancePolicy, AcceptanceRejectionReason, AcceptanceVerdict, CompletionPolicy, FailureKind,
    HealthScore, HealthScorePolicy, RecoveryDecision, RecoveryPolicy,
};
pub use ports::{
    AgentBackendPort, AgentResult, ArtifactRepositoryPort, BuildGateReport, CliSourceAdapterPort,
    CliTargetAdapterPort, CliVerifierPort, DomainError, KnowledgeRepositoryPort, PromptProvenance,
    PromptProviderPort, PromptSource, ScenarioInvocation, SourceAdapterPort, TargetAdapterPort,
    TargetScaffoldSummary, VerifierPluginPort,
};
pub use semantic_cli::{
    CliArgument, CliArgumentKind, CliCommandNode, CliExitCodeMapping, CliGapReasonCode,
    CliIoContract, CliSemanticModel, CliValueType,
};
pub use semantic_ui::{
    ApiEndpointBinding, AssetEntry, ComponentNode, EffectBinding, InteractionBinding,
    LayoutConstraint, PropDefinition, RouteDefinition, SemanticUiModel, StateField, StyleToken,
    TargetPlatform, UiElementKind, UiElementNode,
};
pub use task::{MigrationTask, TaskState};
pub use verification::{CoverageDisclosure, EvidenceRecord, VerificationStatus};
