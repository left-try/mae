use std::fmt;

use crate::capability::MigrationStrategy;
use crate::ids::{ArtifactId, ContractId, TaskId};
use crate::ports::DomainError;

/// Lifecycle state machine for a migration task in the task DAG.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum TaskState {
    Pending,
    Ready,
    Running,
    Verifying,
    Accepted,
    NeedsRepair,
    Blocked,
    Failed,
    Cancelled,
}

impl TaskState {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Pending => "PENDING",
            Self::Ready => "READY",
            Self::Running => "RUNNING",
            Self::Verifying => "VERIFYING",
            Self::Accepted => "ACCEPTED",
            Self::NeedsRepair => "NEEDS_REPAIR",
            Self::Blocked => "BLOCKED",
            Self::Failed => "FAILED",
            Self::Cancelled => "CANCELLED",
        }
    }

    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Accepted | Self::Failed | Self::Cancelled)
    }

    pub fn is_accepted(&self) -> bool {
        matches!(self, Self::Accepted)
    }
}

impl fmt::Display for TaskState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Bounded, outcome-oriented unit of migration work in the DAG.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MigrationTask {
    pub id: TaskId,
    pub description: String,
    pub dependencies: Vec<TaskId>,
    pub required_contracts: Vec<ContractId>,
    pub input_artifacts: Vec<ArtifactId>,
    pub expected_outputs: Vec<String>,
    pub strategy: MigrationStrategy,
    pub state: TaskState,
    pub retry_count: u32,
    pub max_retries: u32,
}

impl MigrationTask {
    pub fn new(
        id: impl Into<TaskId>,
        description: impl Into<String>,
        strategy: MigrationStrategy,
    ) -> Self {
        Self {
            id: id.into(),
            description: description.into(),
            dependencies: Vec::new(),
            required_contracts: Vec::new(),
            input_artifacts: Vec::new(),
            expected_outputs: Vec::new(),
            strategy,
            state: TaskState::Pending,
            retry_count: 0,
            max_retries: 3,
        }
    }

    pub fn with_dependencies(mut self, deps: Vec<TaskId>) -> Self {
        self.dependencies = deps;
        self
    }

    pub fn with_dependency(mut self, dep: impl Into<TaskId>) -> Self {
        self.dependencies.push(dep.into());
        self
    }

    pub fn with_required_contracts(mut self, contracts: Vec<ContractId>) -> Self {
        self.required_contracts = contracts;
        self
    }

    pub fn with_required_contract(mut self, contract: impl Into<ContractId>) -> Self {
        self.required_contracts.push(contract.into());
        self
    }

    pub fn with_input_artifacts(mut self, inputs: Vec<ArtifactId>) -> Self {
        self.input_artifacts = inputs;
        self
    }

    pub fn with_expected_outputs(mut self, outputs: Vec<String>) -> Self {
        self.expected_outputs = outputs;
        self
    }

    pub fn with_max_retries(mut self, max: u32) -> Self {
        self.max_retries = max;
        self
    }

    /// Transitions Pending or NeedsRepair -> Ready once dependencies are met.
    pub fn mark_ready(&mut self) -> Result<(), DomainError> {
        match self.state {
            TaskState::Pending | TaskState::NeedsRepair => {
                self.state = TaskState::Ready;
                Ok(())
            }
            other => Err(DomainError::InvalidStateTransition {
                task_id: self.id.clone(),
                from: other,
                to: TaskState::Ready,
                reason: format!("Cannot mark ready from {}", other),
            }),
        }
    }

    /// Transitions Ready -> Running when code generation begins.
    pub fn start_running(&mut self) -> Result<(), DomainError> {
        match self.state {
            TaskState::Ready => {
                self.state = TaskState::Running;
                Ok(())
            }
            other => Err(DomainError::InvalidStateTransition {
                task_id: self.id.clone(),
                from: other,
                to: TaskState::Running,
                reason: format!("Cannot start running from {}", other),
            }),
        }
    }

    /// Transitions Running -> Verifying when target artifacts are emitted.
    pub fn start_verifying(&mut self) -> Result<(), DomainError> {
        match self.state {
            TaskState::Running => {
                self.state = TaskState::Verifying;
                Ok(())
            }
            other => Err(DomainError::InvalidStateTransition {
                task_id: self.id.clone(),
                from: other,
                to: TaskState::Verifying,
                reason: format!("Cannot start verifying from {}", other),
            }),
        }
    }

    /// Transitions Verifying or Running -> NeedsRepair upon failed verification.
    ///
    /// If retry count exceeds max_retries, transitions to Failed instead.
    pub fn mark_needs_repair(&mut self, reason: &str) -> Result<(), DomainError> {
        match self.state {
            TaskState::Verifying | TaskState::Running => {
                self.retry_count += 1;
                if self.retry_count > self.max_retries {
                    self.state = TaskState::Failed;
                    return Err(DomainError::MaxRetriesExceeded {
                        task_id: self.id.clone(),
                        retry_count: self.retry_count,
                        max_retries: self.max_retries,
                    });
                }
                self.state = TaskState::NeedsRepair;
                Ok(())
            }
            other => Err(DomainError::InvalidStateTransition {
                task_id: self.id.clone(),
                from: other,
                to: TaskState::NeedsRepair,
                reason: format!("Cannot mark needs repair from {}: {}", other, reason),
            }),
        }
    }

    /// Transitions any active state -> Blocked.
    pub fn mark_blocked(&mut self, reason: &str) -> Result<(), DomainError> {
        if self.state.is_terminal() {
            return Err(DomainError::InvalidStateTransition {
                task_id: self.id.clone(),
                from: self.state,
                to: TaskState::Blocked,
                reason: format!("Cannot block terminal task: {}", reason),
            });
        }
        self.state = TaskState::Blocked;
        Ok(())
    }

    /// Transitions any active state -> Failed.
    pub fn mark_failed(&mut self, reason: &str) -> Result<(), DomainError> {
        if self.state.is_terminal() {
            return Err(DomainError::InvalidStateTransition {
                task_id: self.id.clone(),
                from: self.state,
                to: TaskState::Failed,
                reason: format!("Cannot fail terminal task: {}", reason),
            });
        }
        self.state = TaskState::Failed;
        Ok(())
    }

    /// Transitions any active state -> Cancelled.
    pub fn cancel(&mut self, reason: &str) -> Result<(), DomainError> {
        if self.state.is_terminal() {
            return Err(DomainError::InvalidStateTransition {
                task_id: self.id.clone(),
                from: self.state,
                to: TaskState::Cancelled,
                reason: format!("Cannot cancel terminal task: {}", reason),
            });
        }
        self.state = TaskState::Cancelled;
        Ok(())
    }

    /// Crate-internal transition: can ONLY transition to Accepted via AcceptancePolicy / MigrationAggregate.
    pub(crate) fn accept_by_policy(&mut self) -> Result<(), DomainError> {
        match self.state {
            TaskState::Verifying => {
                self.state = TaskState::Accepted;
                Ok(())
            }
            other => Err(DomainError::InvalidStateTransition {
                task_id: self.id.clone(),
                from: other,
                to: TaskState::Accepted,
                reason: format!(
                    "Cannot accept task from {}; must be in VERIFYING state",
                    other
                ),
            }),
        }
    }
}
