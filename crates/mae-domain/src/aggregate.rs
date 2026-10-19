use std::collections::{BTreeMap, HashSet};

use crate::capability::CapabilityReport;
use crate::contract::Contract;
use crate::ids::{ArtifactRevision, ContractId, MigrationId, PlanHash, TaskId};
use crate::policies::{
    AcceptancePolicy, AcceptanceRejectionReason, AcceptanceVerdict, CompletionPolicy, HealthScore,
    HealthScorePolicy,
};
use crate::ports::DomainError;
use crate::task::{MigrationTask, TaskState};
use crate::verification::{EvidenceRecord, VerificationStatus};

/// Domain events emitted by the MigrationAggregate root.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DomainEvent {
    TaskStateChanged {
        task_id: TaskId,
        from: TaskState,
        to: TaskState,
    },
    TargetRevisionAdvanced {
        from: ArtifactRevision,
        to: ArtifactRevision,
    },
    EvidenceRecorded {
        contract_id: ContractId,
        status: VerificationStatus,
        target_rev: ArtifactRevision,
    },
    TaskAccepted {
        task_id: TaskId,
        revision: ArtifactRevision,
    },
    TaskRejected {
        task_id: TaskId,
        reasons: Vec<AcceptanceRejectionReason>,
    },
    PlanHashLocked {
        plan_hash: PlanHash,
    },
    MigrationCompleted {
        target_revision: ArtifactRevision,
    },
}

/// Aggregate root governing a single source-to-target software migration run.
///
/// Owns the required Contract set, CapabilityReport, reviewed PlanHash,
/// MigrationTask DAG, target revision progression, and EvidenceRecord ledger.
#[derive(Debug, Clone)]
pub struct MigrationAggregate {
    pub id: MigrationId,
    pub source_fingerprint: String,
    pub contracts: BTreeMap<ContractId, Contract>,
    pub tasks: BTreeMap<TaskId, MigrationTask>,
    pub capability_report: CapabilityReport,
    pub plan_hash: Option<PlanHash>,
    pub current_target_revision: ArtifactRevision,
    pub evidence_ledger: Vec<EvidenceRecord>,
    pub uncommitted_events: Vec<DomainEvent>,
}

impl MigrationAggregate {
    pub fn new(id: impl Into<MigrationId>, source_fingerprint: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            source_fingerprint: source_fingerprint.into(),
            contracts: BTreeMap::new(),
            tasks: BTreeMap::new(),
            capability_report: CapabilityReport::new(),
            plan_hash: None,
            current_target_revision: ArtifactRevision::initial(),
            evidence_ledger: Vec::new(),
            uncommitted_events: Vec::new(),
        }
    }

    pub fn add_contract(&mut self, contract: Contract) {
        self.contracts.insert(contract.id.clone(), contract);
    }

    pub fn add_task(&mut self, task: MigrationTask) {
        self.tasks.insert(task.id.clone(), task);
    }

    pub fn get_task(&self, task_id: &TaskId) -> Option<&MigrationTask> {
        self.tasks.get(task_id)
    }

    pub fn get_task_mut(&mut self, task_id: &TaskId) -> Option<&mut MigrationTask> {
        self.tasks.get_mut(task_id)
    }

    pub fn set_capability_report(&mut self, report: CapabilityReport) {
        self.capability_report = report;
    }

    pub fn lock_plan_hash(&mut self, plan_hash: PlanHash) {
        self.plan_hash = Some(plan_hash.clone());
        self.uncommitted_events
            .push(DomainEvent::PlanHashLocked { plan_hash });
    }

    /// Advances the current target revision by 1.
    ///
    /// Every time new target code is emitted, advancing revision automatically
    /// renders prior target-revision evidence stale!
    pub fn advance_target_revision(&mut self) -> ArtifactRevision {
        let from = self.current_target_revision;
        let to = from.next();
        self.current_target_revision = to;
        self.uncommitted_events
            .push(DomainEvent::TargetRevisionAdvanced { from, to });
        to
    }

    /// Appends verification evidence to the ledger.
    pub fn record_evidence(&mut self, evidence: EvidenceRecord) {
        self.uncommitted_events.push(DomainEvent::EvidenceRecorded {
            contract_id: evidence.contract_id.clone(),
            status: evidence.status,
            target_rev: evidence.target_revision,
        });
        self.evidence_ledger.push(evidence);
    }

    /// Attempts to transition a task to Accepted via AcceptancePolicy.
    pub fn accept_task(&mut self, task_id: &TaskId) -> Result<(), Vec<AcceptanceRejectionReason>> {
        let task = self.tasks.get(task_id).ok_or_else(|| {
            vec![AcceptanceRejectionReason::DependencyNotAccepted(
                task_id.clone(),
            )]
        })?;

        let accepted_tasks: HashSet<TaskId> = self
            .tasks
            .values()
            .filter(|t| t.state.is_accepted())
            .map(|t| t.id.clone())
            .collect();

        let contract_slice: Vec<Contract> = self.contracts.values().cloned().collect();

        let verdict = AcceptancePolicy::evaluate(
            task,
            &accepted_tasks,
            self.current_target_revision,
            &contract_slice,
            &self.capability_report,
            &self.evidence_ledger,
        );

        match verdict {
            AcceptanceVerdict::Accepted => {
                let task_mut = self.tasks.get_mut(task_id).expect("task must exist");
                let _ = task_mut.accept_by_policy();
                self.uncommitted_events.push(DomainEvent::TaskAccepted {
                    task_id: task_id.clone(),
                    revision: self.current_target_revision,
                });
                Ok(())
            }
            AcceptanceVerdict::Rejected { reasons } => {
                self.uncommitted_events.push(DomainEvent::TaskRejected {
                    task_id: task_id.clone(),
                    reasons: reasons.clone(),
                });
                Err(reasons)
            }
        }
    }

    /// Transitions task to Ready state.
    pub fn mark_task_ready(&mut self, task_id: &TaskId) -> Result<(), DomainError> {
        let task = self
            .tasks
            .get_mut(task_id)
            .ok_or_else(|| DomainError::TaskNotFound(task_id.clone()))?;
        let from = task.state;
        task.mark_ready()?;
        self.uncommitted_events.push(DomainEvent::TaskStateChanged {
            task_id: task_id.clone(),
            from,
            to: TaskState::Ready,
        });
        Ok(())
    }

    /// Transitions task to Running state.
    pub fn start_task_running(&mut self, task_id: &TaskId) -> Result<(), DomainError> {
        let task = self
            .tasks
            .get_mut(task_id)
            .ok_or_else(|| DomainError::TaskNotFound(task_id.clone()))?;
        let from = task.state;
        task.start_running()?;
        self.uncommitted_events.push(DomainEvent::TaskStateChanged {
            task_id: task_id.clone(),
            from,
            to: TaskState::Running,
        });
        Ok(())
    }

    /// Transitions task to Verifying state.
    pub fn start_task_verifying(&mut self, task_id: &TaskId) -> Result<(), DomainError> {
        let task = self
            .tasks
            .get_mut(task_id)
            .ok_or_else(|| DomainError::TaskNotFound(task_id.clone()))?;
        let from = task.state;
        task.start_verifying()?;
        self.uncommitted_events.push(DomainEvent::TaskStateChanged {
            task_id: task_id.clone(),
            from,
            to: TaskState::Verifying,
        });
        Ok(())
    }

    /// Transitions task to NeedsRepair state.
    pub fn mark_task_needs_repair(
        &mut self,
        task_id: &TaskId,
        reason: &str,
    ) -> Result<(), DomainError> {
        let task = self
            .tasks
            .get_mut(task_id)
            .ok_or_else(|| DomainError::TaskNotFound(task_id.clone()))?;
        let from = task.state;
        task.mark_needs_repair(reason)?;
        self.uncommitted_events.push(DomainEvent::TaskStateChanged {
            task_id: task_id.clone(),
            from,
            to: task.state,
        });
        Ok(())
    }

    /// Computes the current Crane-inspired health score.
    pub fn health_score(&self) -> HealthScore {
        let task_slice: Vec<MigrationTask> = self.tasks.values().cloned().collect();
        let contract_slice: Vec<Contract> = self.contracts.values().cloned().collect();

        HealthScorePolicy::compute(
            &task_slice,
            &contract_slice,
            &self.evidence_ledger,
            self.current_target_revision,
        )
    }

    /// Checks if migration meets all completion requirements.
    pub fn check_completion(&mut self) -> Result<(), Vec<String>> {
        let task_slice: Vec<MigrationTask> = self.tasks.values().cloned().collect();
        let contract_slice: Vec<Contract> = self.contracts.values().cloned().collect();

        let result = CompletionPolicy::evaluate(
            &task_slice,
            &contract_slice,
            &self.evidence_ledger,
            self.current_target_revision,
            &self.capability_report,
        );

        if result.is_ok() {
            self.uncommitted_events
                .push(DomainEvent::MigrationCompleted {
                    target_revision: self.current_target_revision,
                });
        }

        result
    }

    /// Takes all uncommitted domain events, clearing the internal buffer.
    pub fn take_events(&mut self) -> Vec<DomainEvent> {
        std::mem::take(&mut self.uncommitted_events)
    }
}
