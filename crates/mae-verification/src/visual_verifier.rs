use std::fs;
use std::path::{Path, PathBuf};

use mae_domain::{
    ArtifactRevision, Contract, CoverageDisclosure, DomainError, EvidenceRecord, TolerancePolicy,
    VerificationCategory, VerificationStatus, VerifierPluginPort,
};

/// Visual layout, roundness, scale, and pixel comparison snapshot for a named UI state/route.
#[derive(Debug, Clone, PartialEq)]
pub struct VisualSnapshot {
    pub scenario_id: String,
    pub width: u32,
    pub height: u32,
    pub spacing_px: f64,
    pub padding_px: f64,
    pub border_radius_px: f64,
    pub font_size_px: f64,
    pub scale_factor: f64,
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
            border_radius_px: 8.0,
            font_size_px: 14.0,
            scale_factor: 1.0,
            primary_color: primary_color.into(),
            pixel_delta_ratio,
        }
    }

    pub fn with_roundness_and_scale(
        mut self,
        border_radius_px: f64,
        font_size_px: f64,
        scale_factor: f64,
    ) -> Self {
        self.border_radius_px = border_radius_px;
        self.font_size_px = font_size_px;
        self.scale_factor = scale_factor;
        self
    }
}

/// Independent visual parity verifier comparing geometry, spacing, roundness (`border-radius`),
/// typography/default scale (`font-size`, `scale-factor`), colors, and pixel delta ratio
/// against the contract's configured `TolerancePolicy`.
#[derive(Debug, Clone)]
pub struct VisualParityVerifier {
    pub baseline: VisualSnapshot,
    pub target: VisualSnapshot,
    pub target_dir: Option<PathBuf>,
    pub source_revision: ArtifactRevision,
}

impl VisualParityVerifier {
    pub fn new(baseline: VisualSnapshot, target: VisualSnapshot) -> Self {
        Self {
            baseline,
            target,
            target_dir: None,
            source_revision: ArtifactRevision::initial(),
        }
    }

    /// Creates a `VisualParityVerifier` that inspects emitted `.slint` files in `target_dir`
    /// (`ui/theme/tokens.slint` and `ui/app.slint`) at verification time to verify actual
    /// generated roundness (`border-radius`), `font-size`, `scale-factor`, spacing, padding, and geometry.
    pub fn for_target_dir(
        baseline: VisualSnapshot,
        target_dir: impl Into<PathBuf>,
        pixel_delta_ratio: f64,
    ) -> Self {
        let fallback_target = VisualSnapshot {
            pixel_delta_ratio,
            ..baseline.clone()
        };
        Self {
            baseline,
            target: fallback_target,
            target_dir: Some(target_dir.into()),
            source_revision: ArtifactRevision::initial(),
        }
    }

    fn resolve_target_snapshot(&self) -> Result<VisualSnapshot, DomainError> {
        let Some(ref dir) = self.target_dir else {
            return Ok(self.target.clone());
        };
        inspect_slint_target_visual_snapshot(
            dir,
            &self.baseline.scenario_id,
            self.target.pixel_delta_ratio,
        )
    }
}

fn inspect_slint_target_visual_snapshot(
    target_dir: &Path,
    scenario_id: &str,
    pixel_delta_ratio: f64,
) -> Result<VisualSnapshot, DomainError> {
    let tokens_path = target_dir.join("ui/theme/tokens.slint");
    let app_path = target_dir.join("ui/app.slint");

    let tokens_src = fs::read_to_string(&tokens_path).map_err(|e| {
        DomainError::PortError(format!("Failed reading {}: {e}", tokens_path.display()))
    })?;
    let app_src = fs::read_to_string(&app_path).map_err(|e| {
        DomainError::PortError(format!("Failed reading {}: {e}", app_path.display()))
    })?;

    let width = extract_slint_px(&app_src, "preferred-width:").unwrap_or(1280.0) as u32;
    let height = extract_slint_px(&app_src, "preferred-height:").unwrap_or(800.0) as u32;
    let spacing_px = extract_slint_px(&tokens_src, "spacing:").unwrap_or(0.0);
    let padding_px = extract_slint_px(&tokens_src, "padding:").unwrap_or(0.0);
    let border_radius_px = extract_slint_px(&tokens_src, "border-radius:").unwrap_or(0.0);
    let font_size_px = extract_slint_px(&tokens_src, "font-size:").unwrap_or(0.0);
    let scale_factor = extract_slint_float(&tokens_src, "scale-factor:").unwrap_or(0.0);
    let primary_color =
        extract_slint_string(&tokens_src, "--color-primary:").unwrap_or_else(|| "#2563eb".into());

    // Also verify `ui/app.slint` actually binds `ThemeTokens.border-radius` and `ThemeTokens.font-size`
    let effective_radius = if app_src.contains("ThemeTokens.border-radius") {
        border_radius_px
    } else {
        0.0
    };
    let effective_font_size = if app_src.contains("ThemeTokens.font-size") {
        font_size_px
    } else {
        0.0
    };

    Ok(VisualSnapshot::new(
        scenario_id,
        width,
        height,
        spacing_px,
        padding_px,
        primary_color,
        pixel_delta_ratio,
    )
    .with_roundness_and_scale(effective_radius, effective_font_size, scale_factor))
}

fn extract_slint_px(src: &str, key: &str) -> Option<f64> {
    for line in src.lines() {
        if let Some(idx) = line.find(key) {
            let after = line[idx + key.len()..].trim().trim_end_matches(';').trim();
            let num_str = after.strip_suffix("px").unwrap_or(after).trim();
            if let Ok(v) = num_str.parse::<f64>() {
                return Some(v);
            }
        }
    }
    None
}

fn extract_slint_float(src: &str, key: &str) -> Option<f64> {
    for line in src.lines() {
        if let Some(idx) = line.find(key) {
            let after = line[idx + key.len()..].trim().trim_end_matches(';').trim();
            if let Ok(v) = after.parse::<f64>() {
                return Some(v);
            }
        }
    }
    None
}

fn extract_slint_string(src: &str, key: &str) -> Option<String> {
    for line in src.lines() {
        if let Some(idx) = line.find(key) {
            let after = line[idx + key.len()..].trim().trim_end_matches(';').trim();
            let unquoted = after.trim_matches('"');
            if !unquoted.is_empty() {
                return Some(unquoted.to_string());
            }
        }
    }
    None
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
        let resolved_target = self.resolve_target_snapshot()?;
        let max_allowed_delta = match contract.tolerance {
            TolerancePolicy::Exact => 0.0,
            TolerancePolicy::MaxPixelDeltaRatio(ratio) => ratio,
            TolerancePolicy::MaxLatencyMs(_) => 0.02,
        };

        let geometry_matches = self.baseline.width == resolved_target.width
            && self.baseline.height == resolved_target.height
            && (self.baseline.spacing_px - resolved_target.spacing_px).abs() <= 2.0
            && (self.baseline.padding_px - resolved_target.padding_px).abs() <= 2.0
            && self
                .baseline
                .primary_color
                .eq_ignore_ascii_case(&resolved_target.primary_color);

        let roundness_matches =
            (self.baseline.border_radius_px - resolved_target.border_radius_px).abs() <= 1.0;
        let font_size_matches =
            (self.baseline.font_size_px - resolved_target.font_size_px).abs() <= 1.0;
        let scale_matches =
            (self.baseline.scale_factor - resolved_target.scale_factor).abs() <= 0.05;

        let delta_within_tolerance = resolved_target.pixel_delta_ratio <= max_allowed_delta;

        let status = if geometry_matches
            && roundness_matches
            && font_size_matches
            && scale_matches
            && delta_within_tolerance
        {
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
                "Visual parity passed for '{}': border_radius={}px, font_size={}px, scale={:.2}, pixel_delta_ratio={:.4} <= {:.4}",
                self.baseline.scenario_id,
                resolved_target.border_radius_px,
                resolved_target.font_size_px,
                resolved_target.scale_factor,
                resolved_target.pixel_delta_ratio,
                max_allowed_delta
            )
        } else {
            format!(
                "Visual parity failed for '{}': geometry_matches={}, roundness_matches={}, font_size_matches={}, scale_matches={}, pixel_delta_ratio={:.4} (max {:.4})",
                self.baseline.scenario_id,
                geometry_matches,
                roundness_matches,
                font_size_matches,
                scale_matches,
                resolved_target.pixel_delta_ratio,
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
        .with_measurement("border_radius_px", resolved_target.border_radius_px)
        .with_measurement("font_size_px", resolved_target.font_size_px)
        .with_measurement("scale_factor", resolved_target.scale_factor)
        .with_measurement("pixel_delta_ratio", resolved_target.pixel_delta_ratio)
        .with_measurement("max_allowed_delta", max_allowed_delta))
    }
}
