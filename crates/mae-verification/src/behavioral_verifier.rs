use std::collections::BTreeSet;

use mae_domain::{
    ArtifactRevision, Contract, CoverageDisclosure, DomainError, EvidenceRecord,
    VerificationCategory, VerificationStatus, VerifierPluginPort,
};

/// Observed state trace for a single scenario execution on either source (React) or target (Rust + Slint).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScenarioTrace {
    pub scenario_id: String,
    pub actions: Vec<String>,
    pub observed_states: Vec<(String, String)>,
}

impl ScenarioTrace {
    pub fn new(scenario_id: impl Into<String>) -> Self {
        Self {
            scenario_id: scenario_id.into(),
            actions: Vec::new(),
            observed_states: Vec::new(),
        }
    }

    pub fn with_action(mut self, action: impl Into<String>) -> Self {
        self.actions.push(action.into());
        self
    }

    pub fn with_state(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.observed_states.push((key.into(), value.into()));
        self
    }
}

/// Differential behavioral verifier comparing source (React) and target (Slint) scenario traces.
///
/// Enforces two strict Sanka-inspired rules:
/// 1. Only explicitly declared `allowed_normalizations` keys may be normalized away.
/// 2. Every declared scenario must be executed; unexecuted scenarios are disclosed in `CoverageDisclosure`.
#[derive(Debug, Clone)]
pub struct BehavioralDifferentialVerifier {
    pub declared_scenarios: Vec<String>,
    pub source_traces: Vec<ScenarioTrace>,
    pub target_traces: Vec<ScenarioTrace>,
    pub allowed_normalizations: BTreeSet<String>,
    pub source_revision: ArtifactRevision,
}

impl BehavioralDifferentialVerifier {
    pub fn new(
        declared_scenarios: Vec<String>,
        source_traces: Vec<ScenarioTrace>,
        target_traces: Vec<ScenarioTrace>,
    ) -> Self {
        Self {
            declared_scenarios,
            source_traces,
            target_traces,
            allowed_normalizations: BTreeSet::new(),
            source_revision: ArtifactRevision::initial(),
        }
    }

    pub fn with_allowed_normalization(mut self, key: impl Into<String>) -> Self {
        self.allowed_normalizations.insert(key.into());
        self
    }

    fn normalize_states(&self, states: &[(String, String)]) -> Vec<(String, String)> {
        states
            .iter()
            .filter(|(k, _)| !self.allowed_normalizations.contains(k))
            .cloned()
            .collect()
    }
}

impl VerifierPluginPort for BehavioralDifferentialVerifier {
    fn category(&self) -> VerificationCategory {
        VerificationCategory::Behavioral
    }

    fn verify(
        &self,
        contract: &Contract,
        target_rev: ArtifactRevision,
    ) -> Result<EvidenceRecord, DomainError> {
        let mut executed = Vec::new();
        let mut passed = Vec::new();
        let mut unexecuted = Vec::new();
        let mut mismatch_notes = Vec::new();

        for scenario_id in &self.declared_scenarios {
            let src_opt = self
                .source_traces
                .iter()
                .find(|t| &t.scenario_id == scenario_id);
            let tgt_opt = self
                .target_traces
                .iter()
                .find(|t| &t.scenario_id == scenario_id);

            match (src_opt, tgt_opt) {
                (Some(src), Some(tgt)) => {
                    executed.push(scenario_id.clone());
                    let norm_src = self.normalize_states(&src.observed_states);
                    let norm_tgt = self.normalize_states(&tgt.observed_states);

                    if src.actions == tgt.actions && norm_src == norm_tgt {
                        passed.push(scenario_id.clone());
                    } else {
                        mismatch_notes.push(format!(
                            "Scenario '{}' mismatch: source={:?} vs target={:?}",
                            scenario_id, norm_src, norm_tgt
                        ));
                    }
                }
                _ => {
                    unexecuted.push(scenario_id.clone());
                }
            }
        }

        let coverage = CoverageDisclosure::with_unexecuted(
            self.declared_scenarios.clone(),
            executed,
            passed,
            unexecuted,
        );

        let status = if !mismatch_notes.is_empty() {
            VerificationStatus::Fail
        } else if !coverage.is_complete() {
            VerificationStatus::Inconclusive
        } else {
            VerificationStatus::Pass
        };

        let summary = if mismatch_notes.is_empty() && coverage.is_complete() {
            format!(
                "All {} declared behavioral scenarios matched between React source and Slint target",
                self.declared_scenarios.len()
            )
        } else if !mismatch_notes.is_empty() {
            mismatch_notes.join("; ")
        } else {
            format!(
                "Incomplete scenario execution: unexecuted={:?}",
                coverage.unexecuted
            )
        };

        Ok(EvidenceRecord::new(
            contract.id.clone(),
            "mae.verifier.behavioral-differential/0.1",
            VerificationCategory::Behavioral,
            self.source_revision,
            target_rev,
            status,
            coverage,
            summary,
        ))
    }
}
