use std::fmt;

use crate::ids::ContractId;

/// Category of verification governed by a contract.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum VerificationCategory {
    Build,
    Behavioral,
    Structural,
    Visual,
    NativeCompliance,
    Accessibility,
    Performance,
}

impl VerificationCategory {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Build => "BUILD",
            Self::Behavioral => "BEHAVIORAL",
            Self::Structural => "STRUCTURAL",
            Self::Visual => "VISUAL",
            Self::NativeCompliance => "NATIVE_COMPLIANCE",
            Self::Accessibility => "ACCESSIBILITY",
            Self::Performance => "PERFORMANCE",
        }
    }
}

impl fmt::Display for VerificationCategory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Severity classification of a contract failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ContractSeverity {
    Critical,
    High,
    Medium,
    Low,
}

impl ContractSeverity {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Critical => "CRITICAL",
            Self::High => "HIGH",
            Self::Medium => "MEDIUM",
            Self::Low => "LOW",
        }
    }

    pub fn is_critical(&self) -> bool {
        matches!(self, Self::Critical)
    }
}

impl fmt::Display for ContractSeverity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Quantitative tolerance policy for contract satisfaction.
#[derive(Debug, Clone, PartialEq)]
pub enum TolerancePolicy {
    /// Exact match required (e.g. state keys, exact strings, return codes).
    Exact,
    /// Maximum allowed visual pixel difference ratio (e.g. 0.02 for 2%).
    MaxPixelDeltaRatio(f64),
    /// Maximum allowed latency in milliseconds.
    MaxLatencyMs(u64),
}

impl fmt::Display for TolerancePolicy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Exact => write!(f, "exact"),
            Self::MaxPixelDeltaRatio(ratio) => write!(f, "max_pixel_delta_ratio: {:.4}", ratio),
            Self::MaxLatencyMs(ms) => write!(f, "max_latency_ms: {}", ms),
        }
    }
}

/// Stimulus or action applied during behavioral verification.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ContractStimulus {
    pub action_type: String,
    pub payload: String,
}

impl ContractStimulus {
    pub fn new(action_type: impl Into<String>, payload: impl Into<String>) -> Self {
        Self {
            action_type: action_type.into(),
            payload: payload.into(),
        }
    }
}

/// Observable expectation verifying an outcome key against an expected value.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ObservableExpectation {
    pub key: String,
    pub expected_value: String,
}

impl ObservableExpectation {
    pub fn new(key: impl Into<String>, expected_value: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            expected_value: expected_value.into(),
        }
    }
}

/// Explicit behavioral, visual, structural, or native-compliance requirement.
///
/// Correctness in MAE is contract equivalence, never syntax similarity.
#[derive(Debug, Clone, PartialEq)]
pub struct Contract {
    pub id: ContractId,
    pub name: String,
    pub description: String,
    pub category: VerificationCategory,
    pub severity: ContractSeverity,
    pub preconditions: Vec<String>,
    pub stimulus: Option<ContractStimulus>,
    pub expectations: Vec<ObservableExpectation>,
    pub tolerance: TolerancePolicy,
}

impl Contract {
    pub fn new(
        id: impl Into<ContractId>,
        name: impl Into<String>,
        description: impl Into<String>,
        category: VerificationCategory,
        severity: ContractSeverity,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            description: description.into(),
            category,
            severity,
            preconditions: Vec::new(),
            stimulus: None,
            expectations: Vec::new(),
            tolerance: TolerancePolicy::Exact,
        }
    }

    pub fn with_preconditions(mut self, preconditions: Vec<String>) -> Self {
        self.preconditions = preconditions;
        self
    }

    pub fn with_stimulus(mut self, stimulus: ContractStimulus) -> Self {
        self.stimulus = Some(stimulus);
        self
    }

    pub fn with_expectation(mut self, expectation: ObservableExpectation) -> Self {
        self.expectations.push(expectation);
        self
    }

    pub fn with_expectations(mut self, expectations: Vec<ObservableExpectation>) -> Self {
        self.expectations = expectations;
        self
    }

    pub fn with_tolerance(mut self, tolerance: TolerancePolicy) -> Self {
        self.tolerance = tolerance;
        self
    }

    pub fn is_critical(&self) -> bool {
        self.severity.is_critical()
    }
}
