use mae_domain::{
    ArtifactRevision, Contract, CoverageDisclosure, DomainError, EvidenceRecord, TolerancePolicy,
    VerificationCategory, VerificationStatus, VerifierPluginPort,
};

/// Visual layout and pixel comparison snapshot for a named UI state/route.
#[derive(Debug, Clone, PartialEq)]
pub struct VisualSnapshot {
    pub scenario_id: String,
    pub width: u32,
    pub height: u32,
    pub spacing_px: f64,
    pub padding_px: f64,
    pub primary_color: String,
    pub pixel_delta_ratio: f64,
}

impl VisualSnapshot {
    pub fn new(
        scenario_id: impl Into<String>,
        width: u32,
        height: u32,
        spacing_px: f64,
        padding_px: f64,
        primary_color: impl Into<String>,
        pixel_delta_ratio: f64,
    ) -> Self {
        Self {
            scenario_id: scenario_id.into(),
            width,
            height,
            spacing_px,
            padding_px,
            primary_color: primary_color.into(),
            pixel_delta_ratio,
        }
    }
}

/// Independent visual parity verifier comparing geometry, spacing, colors, and pixel delta ratio
/// against the contract's configured `TolerancePolicy`.
#[derive(Debug, Clone)]
pub struct VisualParityVerifier {
    pub baseline: VisualSnapshot,
    pub target: VisualSnapshot,
    pub source_revision: ArtifactRevision,
}

impl VisualParityVerifier {
    pub fn new(baseline: VisualSnapshot, target: VisualSnapshot) -> Self {
        Self {
            baseline,
            target,
            source_revision: ArtifactRevision::initial(),
        }
    }
}

impl VerifierPluginPort for VisualParityVerifier {
    fn category(&self) -> VerificationCategory {
        VerificationCategory::Visual
    }

    fn verify(
        &self,
        contract: &Contract,
        target_rev: ArtifactRevision,
    ) -> Result<EvidenceRecord, DomainError> {
        let max_allowed_delta = match contract.tolerance {
            TolerancePolicy::Exact => 0.0,
            TolerancePolicy::MaxPixelDeltaRatio(ratio) => ratio,
            TolerancePolicy::MaxLatencyMs(_) => 0.02,
        };

        let geometry_matches = self.baseline.width == self.target.width
            && self.baseline.height == self.target.height
            && (self.baseline.spacing_px - self.target.spacing_px).abs() <= 2.0
            && (self.baseline.padding_px - self.target.padding_px).abs() <= 2.0
            && self
                .baseline
                .primary_color
                .eq_ignore_ascii_case(&self.target.primary_color);

        let delta_within_tolerance = self.target.pixel_delta_ratio <= max_allowed_delta;

        let status = if geometry_matches && delta_within_tolerance {
            VerificationStatus::Pass
        } else {
            VerificationStatus::Fail
        };

        let scenarios = vec![self.baseline.scenario_id.clone()];
        let coverage = if status.is_pass() {
            CoverageDisclosure::full(scenarios)
        } else {
            CoverageDisclosure::with_unexecuted(
                scenarios.clone(),
                scenarios,
                Vec::new(),
                Vec::new(),
            )
        };

        let summary = if status.is_pass() {
            format!(
                "Visual parity passed for '{}': pixel_delta_ratio={:.4} <= {:.4}",
                self.baseline.scenario_id, self.target.pixel_delta_ratio, max_allowed_delta
            )
        } else {
            format!(
                "Visual parity failed for '{}': geometry_matches={}, pixel_delta_ratio={:.4} (max {:.4})",
                self.baseline.scenario_id,
                geometry_matches,
                self.target.pixel_delta_ratio,
                max_allowed_delta
            )
        };

        Ok(EvidenceRecord::new(
            contract.id.clone(),
            "mae.verifier.visual-parity/0.1",
            VerificationCategory::Visual,
            self.source_revision,
            target_rev,
            status,
            coverage,
            summary,
        )
        .with_measurement("pixel_delta_ratio", self.target.pixel_delta_ratio)
        .with_measurement("max_allowed_delta", max_allowed_delta))
    }
}
