use std::fmt;

use crate::contract::VerificationCategory;
use crate::ids::{ArtifactRevision, ContractId};

/// Deterministic outcome status of a contract verification attempt.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum VerificationStatus {
    Pass,
    Fail,
    Inconclusive,
    NotRun,
}

impl VerificationStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Pass => "PASS",
            Self::Fail => "FAIL",
            Self::Inconclusive => "INCONCLUSIVE",
            Self::NotRun => "NOT_RUN",
        }
    }

    pub fn is_pass(&self) -> bool {
        matches!(self, Self::Pass)
    }

    pub fn is_fail(&self) -> bool {
        matches!(self, Self::Fail)
    }
}

impl fmt::Display for VerificationStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Sanka-inspired scenario accounting preventing partial test runs from masquerading as full parity.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CoverageDisclosure {
    pub declared_scenarios: Vec<String>,
    pub executed: Vec<String>,
    pub passed: Vec<String>,
    pub unexecuted: Vec<String>,
}

impl CoverageDisclosure {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn full(scenarios: Vec<String>) -> Self {
        Self {
            declared_scenarios: scenarios.clone(),
            executed: scenarios.clone(),
            passed: scenarios,
            unexecuted: Vec::new(),
        }
    }

    pub fn with_unexecuted(
        declared: Vec<String>,
        executed: Vec<String>,
        passed: Vec<String>,
        unexecuted: Vec<String>,
    ) -> Self {
        Self {
            declared_scenarios: declared,
            executed,
            passed,
            unexecuted,
        }
    }

    /// Returns true if all declared scenarios were executed, all executed passed, and none were unexecuted.
    pub fn is_complete(&self) -> bool {
        self.unexecuted.is_empty()
            && self.executed.len() == self.declared_scenarios.len()
            && self.passed.len() == self.executed.len()
    }
}

/// Independent deterministic verification evidence for a migration contract.
#[derive(Debug, Clone, PartialEq)]
pub struct EvidenceRecord {
    pub contract_id: ContractId,
    pub verifier_id: String,
    pub category: VerificationCategory,
    pub source_revision: ArtifactRevision,
    pub target_revision: ArtifactRevision,
    pub status: VerificationStatus,
    pub coverage: CoverageDisclosure,
    pub summary: String,
    pub measurements: Vec<(String, f64)>,
    pub produced_files: Vec<String>,
}

impl EvidenceRecord {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        contract_id: impl Into<ContractId>,
        verifier_id: impl Into<String>,
        category: VerificationCategory,
        source_revision: ArtifactRevision,
        target_revision: ArtifactRevision,
        status: VerificationStatus,
        coverage: CoverageDisclosure,
        summary: impl Into<String>,
    ) -> Self {
        Self {
            contract_id: contract_id.into(),
            verifier_id: verifier_id.into(),
            category,
            source_revision,
            target_revision,
            status,
            coverage,
            summary: summary.into(),
            measurements: Vec::new(),
            produced_files: Vec::new(),
        }
    }

    pub fn with_measurement(mut self, name: impl Into<String>, value: f64) -> Self {
        self.measurements.push((name.into(), value));
        self
    }

    pub fn with_produced_file(mut self, path: impl Into<String>) -> Self {
        self.produced_files.push(path.into());
        self
    }

    /// Returns true if this evidence was generated against the current target revision.
    pub fn is_fresh(&self, current_target_revision: ArtifactRevision) -> bool {
        self.target_revision == current_target_revision
    }

    /// Returns true if the status is Pass.
    pub fn is_pass(&self) -> bool {
        self.status.is_pass()
    }

    /// Returns true if this evidence is fresh, passes, and has complete scenario coverage.
    pub fn is_fresh_complete_pass(&self, current_target_revision: ArtifactRevision) -> bool {
        self.is_fresh(current_target_revision) && self.is_pass() && self.coverage.is_complete()
    }
}
