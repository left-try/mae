use std::collections::HashSet;
use std::fmt;

use crate::capability::CapabilityReport;
use crate::contract::{Contract, VerificationCategory};
use crate::ids::{ArtifactRevision, ContractId, TaskId};
use crate::task::{MigrationTask, TaskState};
use crate::verification::{EvidenceRecord, VerificationStatus};

/// Rejection reasons explaining why a task cannot transition to Accepted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AcceptanceRejectionReason {
    DependencyNotAccepted(TaskId),
    MissingRequiredContractEvidence(ContractId),
    ContractVerificationFailed(ContractId),
    ContractVerificationInconclusiveOrNotRun(ContractId, VerificationStatus),
    StaleEvidenceForTargetRevision {
        contract_id: ContractId,
        evidence_rev: ArtifactRevision,
        current_rev: ArtifactRevision,
    },
    PartialScenarioCoverage(ContractId),
    NativeComplianceFailed,
    CriticalCapabilityGapExists,
    TaskNotInVerifyingState(TaskState),
}

impl fmt::Display for AcceptanceRejectionReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DependencyNotAccepted(id) => write!(f, "Dependency {} is not yet Accepted", id),
            Self::MissingRequiredContractEvidence(id) => {
                write!(f, "Required contract {} has no verification evidence", id)
            }
            Self::ContractVerificationFailed(id) => {
                write!(f, "Verification for contract {} failed", id)
            }
            Self::ContractVerificationInconclusiveOrNotRun(id, status) => {
                write!(
                    f,
                    "Contract {} verification is inconclusive or not run: {}",
                    id, status
                )
            }
            Self::StaleEvidenceForTargetRevision {
                contract_id,
                evidence_rev,
                current_rev,
            } => write!(
                f,
                "Evidence for contract {} is stale (generated for {} but current target is {})",
                contract_id, evidence_rev, current_rev
            ),
            Self::PartialScenarioCoverage(id) => {
                write!(f, "Scenario coverage for contract {} is incomplete", id)
            }
            Self::NativeComplianceFailed => {
                write!(
                    f,
                    "Native compliance verification failed (target violates native UI invariants)"
                )
            }
            Self::CriticalCapabilityGapExists => {
                write!(
                    f,
                    "Critical unresolved capability gap exists in capability report"
                )
            }
            Self::TaskNotInVerifyingState(state) => {
                write!(
                    f,
                    "Task must be in VERIFYING state to be accepted (current: {})",
                    state
                )
            }
        }
    }
}

/// Result of evaluating an acceptance policy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AcceptanceVerdict {
    Accepted,
    Rejected {
        reasons: Vec<AcceptanceRejectionReason>,
    },
}

impl AcceptanceVerdict {
    pub fn is_accepted(&self) -> bool {
        matches!(self, Self::Accepted)
    }

    pub fn reasons(&self) -> &[AcceptanceRejectionReason] {
        match self {
            Self::Accepted => &[],
            Self::Rejected { reasons } => reasons,
        }
    }
}

/// Pure domain acceptance policy enforcing all 6 migration invariants from AGENTS.md.
pub struct AcceptancePolicy;

impl AcceptancePolicy {
    /// Evaluates whether a MigrationTask can transition to Accepted.
    pub fn evaluate(
        task: &MigrationTask,
        accepted_tasks: &HashSet<TaskId>,
        current_target_rev: ArtifactRevision,
        _contracts: &[Contract],
        capability_report: &CapabilityReport,
        evidence_records: &[EvidenceRecord],
    ) -> AcceptanceVerdict {
        let mut reasons = Vec::new();

        // Invariant: Task must be currently in Verifying state
        if task.state != TaskState::Verifying {
            reasons.push(AcceptanceRejectionReason::TaskNotInVerifyingState(
                task.state,
            ));
        }

        // Invariant 1: Any required dependency must be Accepted
        for dep in &task.dependencies {
            if !accepted_tasks.contains(dep) {
                reasons.push(AcceptanceRejectionReason::DependencyNotAccepted(
                    dep.clone(),
                ));
            }
        }

        // Invariant 6: Capability report must not have critical unresolved gaps
        if capability_report.has_critical_unresolved_gaps() {
            reasons.push(AcceptanceRejectionReason::CriticalCapabilityGapExists);
        }

        // Invariant 2, 3, 5: Each required contract must have fresh, complete Pass evidence
        for contract_id in &task.required_contracts {
            // Find latest evidence matching this contract
            let latest_evidence = evidence_records
                .iter()
                .rfind(|e| e.contract_id == *contract_id);

            match latest_evidence {
                None => {
                    reasons.push(AcceptanceRejectionReason::MissingRequiredContractEvidence(
                        contract_id.clone(),
                    ));
                }
                Some(evidence) => {
                    // Check freshness
                    if evidence.target_revision != current_target_rev {
                        reasons.push(AcceptanceRejectionReason::StaleEvidenceForTargetRevision {
                            contract_id: contract_id.clone(),
                            evidence_rev: evidence.target_revision,
                            current_rev: current_target_rev,
                        });
                    }

                    // Check status
                    match evidence.status {
                        VerificationStatus::Pass => {}
                        VerificationStatus::Fail => {
                            reasons.push(AcceptanceRejectionReason::ContractVerificationFailed(
                                contract_id.clone(),
                            ));
                        }
                        VerificationStatus::Inconclusive | VerificationStatus::NotRun => {
                            reasons.push(
                                AcceptanceRejectionReason::ContractVerificationInconclusiveOrNotRun(
                                    contract_id.clone(),
                                    evidence.status,
                                ),
                            );
                        }
                    }

                    // Check scenario coverage
                    if !evidence.coverage.is_complete() {
                        reasons.push(AcceptanceRejectionReason::PartialScenarioCoverage(
                            contract_id.clone(),
                        ));
                    }
                }
            }
        }

        // Invariant 4: Native compliance check
        let native_compliance_failed = evidence_records.iter().any(|e| {
            e.category == VerificationCategory::NativeCompliance
                && e.target_revision == current_target_rev
                && e.status == VerificationStatus::Fail
        });
        if native_compliance_failed {
            reasons.push(AcceptanceRejectionReason::NativeComplianceFailed);
        }

        if reasons.is_empty() {
            AcceptanceVerdict::Accepted
        } else {
            AcceptanceVerdict::Rejected { reasons }
        }
    }
}

/// Crane-inspired health score: health = progress * correctness_gate.
#[derive(Debug, Clone, PartialEq)]
pub struct HealthScore {
    pub progress: f64,
    pub correctness_gate: u32,
    pub score: f64,
}

impl HealthScore {
    pub fn new(progress: f64, correctness_gate: u32) -> Self {
        let score = progress * (correctness_gate as f64);
        Self {
            progress,
            correctness_gate,
            score,
        }
    }
}

/// Evaluates migration health score.
pub struct HealthScorePolicy;

impl HealthScorePolicy {
    pub fn compute(
        tasks: &[MigrationTask],
        contracts: &[Contract],
        evidence: &[EvidenceRecord],
        current_target_rev: ArtifactRevision,
    ) -> HealthScore {
        if tasks.is_empty() {
            return HealthScore::new(0.0, 1);
        }

        let accepted_count = tasks.iter().filter(|t| t.state.is_accepted()).count();
        let progress = (accepted_count as f64) / (tasks.len() as f64);

        // Correctness gate is 0 if:
        // 1. Any critical contract has current Fail evidence
        // 2. Native compliance failed on current revision
        let mut correctness_gate = 1u32;

        for contract in contracts.iter().filter(|c| c.is_critical()) {
            let has_current_fail = evidence.iter().any(|e| {
                e.contract_id == contract.id
                    && e.target_revision == current_target_rev
                    && e.status == VerificationStatus::Fail
            });
            if has_current_fail {
                correctness_gate = 0;
                break;
            }
        }

        let native_failed = evidence.iter().any(|e| {
            e.category == VerificationCategory::NativeCompliance
                && e.target_revision == current_target_rev
                && e.status == VerificationStatus::Fail
        });
        if native_failed {
            correctness_gate = 0;
        }

        HealthScore::new(progress, correctness_gate)
    }
}

/// Final completion gate for the entire migration run.
pub struct CompletionPolicy;

impl CompletionPolicy {
    pub fn evaluate(
        tasks: &[MigrationTask],
        contracts: &[Contract],
        evidence: &[EvidenceRecord],
        current_target_rev: ArtifactRevision,
        capability_report: &CapabilityReport,
    ) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();

        if tasks.is_empty() {
            errors.push("Migration has no tasks".to_string());
            return Err(errors);
        }

        // 1. All tasks must be accepted
        let unaccepted_tasks: Vec<_> = tasks
            .iter()
            .filter(|t| !t.state.is_accepted())
            .map(|t| format!("{}({})", t.id, t.state))
            .collect();
        if !unaccepted_tasks.is_empty() {
            errors.push(format!(
                "Unaccepted tasks remain: {}",
                unaccepted_tasks.join(", ")
            ));
        }

        // 2. No critical capability gap
        if capability_report.has_critical_unresolved_gaps() {
            errors
                .push("Critical unresolved capability gap exists in capability report".to_string());
        }

        // 3. All critical contracts must have Pass on current target revision
        for critical_contract in contracts.iter().filter(|c| c.is_critical()) {
            let has_fresh_pass = evidence.iter().any(|e| {
                e.contract_id == critical_contract.id
                    && e.target_revision == current_target_rev
                    && e.status == VerificationStatus::Pass
                    && e.coverage.is_complete()
            });
            if !has_fresh_pass {
                errors.push(format!(
                    "Critical contract {} does not have fresh complete Pass evidence on {}",
                    critical_contract.id, current_target_rev
                ));
            }
        }

        // 4. Build gate Pass on current revision
        let has_fresh_build_pass = evidence.iter().any(|e| {
            e.category == VerificationCategory::Build
                && e.target_revision == current_target_rev
                && e.status == VerificationStatus::Pass
        });
        if !has_fresh_build_pass {
            errors.push(format!(
                "No fresh Build verification Pass found for revision {}",
                current_target_rev
            ));
        }

        // 5. Native compliance Pass on current revision
        let has_fresh_native_pass = evidence.iter().any(|e| {
            e.category == VerificationCategory::NativeCompliance
                && e.target_revision == current_target_rev
                && e.status == VerificationStatus::Pass
        });
        let has_native_fail = evidence.iter().any(|e| {
            e.category == VerificationCategory::NativeCompliance
                && e.target_revision == current_target_rev
                && e.status == VerificationStatus::Fail
        });
        if !has_fresh_native_pass || has_native_fail {
            errors.push(format!(
                "Native compliance verification not satisfied on revision {}",
                current_target_rev
            ));
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

/// Categorized kind of failure observed during migration task execution or verification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum FailureKind {
    ImplementationDefect,
    VerifierDefect,
    UnsupportedCapability,
    StaleEvidence,
    InfrastructureFailure,
    AmbiguousSemantics,
}

/// Deterministic recovery decision output.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RecoveryDecision {
    Repair,
    Replan,
    Escalate,
    Block,
}

/// Deterministic policy for failure recovery routing.
pub struct RecoveryPolicy;

impl RecoveryPolicy {
    pub fn decide(
        failure_kind: FailureKind,
        retry_count: u32,
        max_retries: u32,
    ) -> RecoveryDecision {
        if retry_count >= max_retries {
            return match failure_kind {
                FailureKind::InfrastructureFailure => RecoveryDecision::Block,
                _ => RecoveryDecision::Escalate,
            };
        }

        match failure_kind {
            FailureKind::ImplementationDefect => RecoveryDecision::Repair,
            FailureKind::StaleEvidence => RecoveryDecision::Repair,
            FailureKind::UnsupportedCapability => RecoveryDecision::Replan,
            FailureKind::AmbiguousSemantics => RecoveryDecision::Escalate,
            FailureKind::VerifierDefect => RecoveryDecision::Escalate,
            FailureKind::InfrastructureFailure => RecoveryDecision::Block,
        }
    }
}
