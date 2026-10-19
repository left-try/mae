use std::fmt;

use crate::contract::ContractSeverity;
use crate::ids::ContractId;

/// Contextual capability level of target platform relative to source requirement.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CapabilityLevel {
    Supported,
    Partial,
    Unsupported,
    Unknown,
}

impl CapabilityLevel {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Supported => "SUPPORTED",
            Self::Partial => "PARTIAL",
            Self::Unsupported => "UNSUPPORTED",
            Self::Unknown => "UNKNOWN",
        }
    }
}

impl fmt::Display for CapabilityLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Approved architectural strategy for migrating a capability or component.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum MigrationStrategy {
    DirectNativeMapping,
    SemanticReconstruction,
    ReplacementComponent,
    CustomImplementation,
    CompatibilityWrapper,
    HybridBridge,
    ManualAdaptation,
    Unsupported,
}

impl MigrationStrategy {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::DirectNativeMapping => "DIRECT_NATIVE_MAPPING",
            Self::SemanticReconstruction => "SEMANTIC_RECONSTRUCTION",
            Self::ReplacementComponent => "REPLACEMENT_COMPONENT",
            Self::CustomImplementation => "CUSTOM_IMPLEMENTATION",
            Self::CompatibilityWrapper => "COMPATIBILITY_WRAPPER",
            Self::HybridBridge => "HYBRID_BRIDGE",
            Self::ManualAdaptation => "MANUAL_ADAPTATION",
            Self::Unsupported => "UNSUPPORTED",
        }
    }
}

impl fmt::Display for MigrationStrategy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Sanka-inspired stable reason code classifying why a source construct cannot be trivially mapped.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum GapReasonCode {
    UiBrowserOnlyApi,
    UiUnsupportedCss,
    UiDynamicStyle,
    UiThirdPartyWidget,
    UiCanvas,
    UiComplexAnimation,
    UiUnresolvedStateLibrary,
    UiUnresolvedRouting,
    UiUnresolvedEffect,
    UiManualAsset,
    UiAccessibilityGap,
}

impl GapReasonCode {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::UiBrowserOnlyApi => "UI_BROWSER_ONLY_API",
            Self::UiUnsupportedCss => "UI_UNSUPPORTED_CSS",
            Self::UiDynamicStyle => "UI_DYNAMIC_STYLE",
            Self::UiThirdPartyWidget => "UI_THIRD_PARTY_WIDGET",
            Self::UiCanvas => "UI_CANVAS",
            Self::UiComplexAnimation => "UI_COMPLEX_ANIMATION",
            Self::UiUnresolvedStateLibrary => "UI_UNRESOLVED_STATE_LIBRARY",
            Self::UiUnresolvedRouting => "UI_UNRESOLVED_ROUTING",
            Self::UiUnresolvedEffect => "UI_UNRESOLVED_EFFECT",
            Self::UiManualAsset => "UI_MANUAL_ASSET",
            Self::UiAccessibilityGap => "UI_ACCESSIBILITY_GAP",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "UI_BROWSER_ONLY_API" => Some(Self::UiBrowserOnlyApi),
            "UI_UNSUPPORTED_CSS" => Some(Self::UiUnsupportedCss),
            "UI_DYNAMIC_STYLE" => Some(Self::UiDynamicStyle),
            "UI_THIRD_PARTY_WIDGET" => Some(Self::UiThirdPartyWidget),
            "UI_CANVAS" => Some(Self::UiCanvas),
            "UI_COMPLEX_ANIMATION" => Some(Self::UiComplexAnimation),
            "UI_UNRESOLVED_STATE_LIBRARY" => Some(Self::UiUnresolvedStateLibrary),
            "UI_UNRESOLVED_ROUTING" => Some(Self::UiUnresolvedRouting),
            "UI_UNRESOLVED_EFFECT" => Some(Self::UiUnresolvedEffect),
            "UI_MANUAL_ASSET" => Some(Self::UiManualAsset),
            "UI_ACCESSIBILITY_GAP" => Some(Self::UiAccessibilityGap),
            _ => None,
        }
    }
}

impl fmt::Display for GapReasonCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// A specific capability gap identified in the source codebase.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MigrationGap {
    pub source_location: String,
    pub affected_contracts: Vec<ContractId>,
    pub severity: ContractSeverity,
    pub strategy: MigrationStrategy,
    pub reason_code: GapReasonCode,
    pub human_decision_required: bool,
    pub description: String,
}

impl MigrationGap {
    pub fn new(
        source_location: impl Into<String>,
        reason_code: GapReasonCode,
        severity: ContractSeverity,
        strategy: MigrationStrategy,
        description: impl Into<String>,
    ) -> Self {
        Self {
            source_location: source_location.into(),
            affected_contracts: Vec::new(),
            severity,
            strategy,
            reason_code,
            human_decision_required: false,
            description: description.into(),
        }
    }

    pub fn with_affected_contracts(mut self, contracts: Vec<ContractId>) -> Self {
        self.affected_contracts = contracts;
        self
    }

    pub fn with_human_decision_required(mut self, required: bool) -> Self {
        self.human_decision_required = required;
        self
    }
}

/// Contextual capability assessment for a given feature or module.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapabilityAssessment {
    pub feature_name: String,
    pub level: CapabilityLevel,
    pub strategy: MigrationStrategy,
    pub gaps: Vec<MigrationGap>,
    pub notes: String,
}

impl CapabilityAssessment {
    pub fn new(
        feature_name: impl Into<String>,
        level: CapabilityLevel,
        strategy: MigrationStrategy,
    ) -> Self {
        Self {
            feature_name: feature_name.into(),
            level,
            strategy,
            gaps: Vec::new(),
            notes: String::new(),
        }
    }

    pub fn with_gaps(mut self, gaps: Vec<MigrationGap>) -> Self {
        self.gaps = gaps;
        self
    }

    pub fn with_notes(mut self, notes: impl Into<String>) -> Self {
        self.notes = notes.into();
        self
    }
}

/// Complete report of capabilities, strategies, and gaps across the codebase.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CapabilityReport {
    pub assessments: Vec<CapabilityAssessment>,
    pub gaps: Vec<MigrationGap>,
}

impl CapabilityReport {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_assessment(&mut self, assessment: CapabilityAssessment) {
        self.assessments.push(assessment);
    }

    pub fn add_gap(&mut self, gap: MigrationGap) {
        self.gaps.push(gap);
    }

    /// Returns true if any critical gap remains unresolved (e.g. requires human decision or is unsupported).
    pub fn has_critical_unresolved_gaps(&self) -> bool {
        self.gaps.iter().any(|g| {
            g.severity == ContractSeverity::Critical
                && (g.human_decision_required || g.strategy == MigrationStrategy::Unsupported)
        }) || self.assessments.iter().any(|a| {
            a.level == CapabilityLevel::Unsupported
                && a.gaps
                    .iter()
                    .any(|g| g.severity == ContractSeverity::Critical)
        })
    }
}
