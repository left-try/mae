use std::error::Error;
use std::fmt;

use crate::artifact::VersionedArtifact;
use crate::capability::CapabilityReport;
use crate::contract::{Contract, VerificationCategory};
use crate::ids::{ArtifactId, ArtifactRevision, ContractId, TaskId};
use crate::semantic_ui::SemanticUiModel;
use crate::task::{MigrationTask, TaskState};
use crate::verification::EvidenceRecord;

/// Standard domain error covering state machine, entity lookup, and port failures.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DomainError {
    InvalidStateTransition {
        task_id: TaskId,
        from: TaskState,
        to: TaskState,
        reason: String,
    },
    MaxRetriesExceeded {
        task_id: TaskId,
        retry_count: u32,
        max_retries: u32,
    },
    TaskNotFound(TaskId),
    ContractNotFound(ContractId),
    ArtifactNotFound(ArtifactId),
    InvalidArtifact(String),
    PlanHashMismatch {
        expected: String,
        actual: String,
    },
    PortError(String),
}

impl fmt::Display for DomainError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidStateTransition {
                task_id,
                from,
                to,
                reason,
            } => write!(
                f,
                "Invalid task state transition for {}: {} -> {} ({})",
                task_id, from, to, reason
            ),
            Self::MaxRetriesExceeded {
                task_id,
                retry_count,
                max_retries,
            } => write!(
                f,
                "Max retries exceeded for task {}: {}/{}",
                task_id, retry_count, max_retries
            ),
            Self::TaskNotFound(id) => write!(f, "Task not found: {}", id),
            Self::ContractNotFound(id) => write!(f, "Contract not found: {}", id),
            Self::ArtifactNotFound(id) => write!(f, "Artifact not found: {}", id),
            Self::InvalidArtifact(msg) => write!(f, "Invalid artifact: {}", msg),
            Self::PlanHashMismatch { expected, actual } => {
                write!(
                    f,
                    "Plan hash mismatch: expected {}, got {}",
                    expected, actual
                )
            }
            Self::PortError(msg) => write!(f, "Port adapter error: {}", msg),
        }
    }
}

impl Error for DomainError {}

/// Raw prompt source loaded from OPF catalog.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PromptSource {
    pub id: String,
    pub name: String,
    pub template: String,
    pub variables: Vec<String>,
}

/// Cryptographic provenance tracking which prompt version generated an artifact.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PromptProvenance {
    pub prompt_id: String,
    pub content_digest: String,
    pub rendered_vars: Vec<(String, String)>,
}

/// Scenario stimulus invocation for behavioral differential testing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScenarioInvocation {
    pub scenario_id: String,
    pub contract_id: ContractId,
    pub input_actions: Vec<String>,
    pub timeout_ms: u64,
}

/// Output returned by an AI agent execution backend.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentResult {
    pub output_artifacts: Vec<VersionedArtifact>,
    pub log_summary: String,
    pub success: bool,
}

/// Summary report after scaffolding a native target project.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TargetScaffoldSummary {
    pub files_generated: Vec<String>,
    pub target_crate_name: String,
    pub has_windows_subsystem: bool,
}

/// Deterministic build gate report.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuildGateReport {
    pub success: bool,
    pub compiler_messages: Vec<String>,
    pub clippy_clean: bool,
    pub fmt_clean: bool,
}

/// Port trait for source analysis adapters (e.g. React / TypeScript analyzer).
pub trait SourceAdapterPort {
    fn discover(&self, source_path: &str) -> Result<VersionedArtifact, DomainError>;
    fn extract_semantics(&self, source_path: &str) -> Result<SemanticUiModel, DomainError>;
    fn assess_capabilities(&self, model: &SemanticUiModel)
        -> Result<CapabilityReport, DomainError>;
}

/// Port trait for target code generation adapters (e.g. Windows Rust + Slint).
pub trait TargetAdapterPort {
    fn scaffold_target(
        &self,
        model: &SemanticUiModel,
        target_dir: &str,
    ) -> Result<TargetScaffoldSummary, DomainError>;

    fn emit_target_code(
        &self,
        task: &MigrationTask,
        model: &SemanticUiModel,
        target_dir: &str,
    ) -> Result<ArtifactRevision, DomainError>;
}

/// Port trait for independent verification plugins (Build, NativeCompliance, Behavioral, Visual).
pub trait VerifierPluginPort {
    fn category(&self) -> VerificationCategory;
    fn verify(
        &self,
        contract: &Contract,
        target_rev: ArtifactRevision,
    ) -> Result<EvidenceRecord, DomainError>;
}

/// Port trait for prompt catalog providers (OPF).
pub trait PromptProviderPort {
    fn get_prompt(&self, prompt_name: &str) -> Result<PromptSource, DomainError>;
    fn render_prompt(
        &self,
        prompt_name: &str,
        vars: &[(&str, &str)],
    ) -> Result<(String, PromptProvenance), DomainError>;
}

/// Port trait for AI coding agent backends (Codex, Claude, Copilot, Mock).
pub trait AgentBackendPort {
    fn execute_prompt(
        &self,
        rendered_prompt: &str,
        provenance: &PromptProvenance,
    ) -> Result<AgentResult, DomainError>;
}

/// Port trait for versioned artifact repositories.
pub trait ArtifactRepositoryPort {
    fn store(&mut self, artifact: VersionedArtifact) -> Result<(), DomainError>;
    fn get(
        &self,
        id: &ArtifactId,
        revision: ArtifactRevision,
    ) -> Result<Option<VersionedArtifact>, DomainError>;
    fn list_revisions(&self, id: &ArtifactId) -> Result<Vec<ArtifactRevision>, DomainError>;
}

/// Port trait for knowledge repositories (OKF).
pub trait KnowledgeRepositoryPort {
    fn search(&self, query: &str) -> Result<Vec<String>, DomainError>;
    fn validate_bundle(&self) -> Result<bool, DomainError>;
}

/// Port trait for CLI source adapters (e.g. Python argparse / click analyzer).
pub trait CliSourceAdapterPort {
    fn discover(&self, source_path: &str) -> Result<VersionedArtifact, DomainError>;
    fn extract_semantics(
        &self,
        source_path: &str,
    ) -> Result<crate::semantic_cli::CliSemanticModel, DomainError>;
    fn assess_capabilities(
        &self,
        model: &crate::semantic_cli::CliSemanticModel,
    ) -> Result<CapabilityReport, DomainError>;
}

/// Port trait for CLI target code generation adapters (e.g. Rust CLI generator).
pub trait CliTargetAdapterPort {
    fn scaffold_target(
        &self,
        model: &crate::semantic_cli::CliSemanticModel,
        target_dir: &str,
    ) -> Result<TargetScaffoldSummary, DomainError>;

    fn emit_target_code(
        &self,
        task: &MigrationTask,
        model: &crate::semantic_cli::CliSemanticModel,
        target_dir: &str,
    ) -> Result<ArtifactRevision, DomainError>;
}

/// Port trait for differential CLI parity verification.
pub trait CliVerifierPort {
    fn verify_scenario(
        &self,
        python_script: &str,
        target_dir: &str,
        scenario: &ScenarioInvocation,
        target_rev: ArtifactRevision,
    ) -> Result<EvidenceRecord, DomainError>;
}
