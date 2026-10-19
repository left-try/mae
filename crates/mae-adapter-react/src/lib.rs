use std::sync::Mutex;

use mae_domain::{
    ArtifactRevision, CapabilityReport, DomainError, SemanticUiModel, SourceAdapterPort,
    TargetPlatform, VersionedArtifact,
};

pub mod discovery;
pub mod gap_classifier;
pub mod json;
pub mod runtime_capture;
pub mod static_analyzer;

pub use discovery::ProjectInventory;
pub use runtime_capture::{
    DomAccessibilityObservation, RouteTransitionObservation, RuntimeTraceBaseline,
    ScreenshotObservation,
};

/// Pure Clean Architecture Source Adapter for React/TypeScript web applications.
///
/// Implements `SourceAdapterPort` for:
/// - Stage A: Deterministic source inventory and module dependency graph discovery.
/// - Stage B: Static semantic extraction of components, state, effects, interactions, and styles.
/// - Stage B/D: Capability assessment and Sanka-inspired gap reason code classification.
/// - Stage C & D: Playwright runtime baseline merging with strict provenance preservation.
pub struct ReactSourceAdapter {
    pub target_platform: TargetPlatform,
    last_source_path: Mutex<Option<String>>,
}

impl ReactSourceAdapter {
    /// Creates a new `ReactSourceAdapter` targeting the specified platform.
    pub fn new(target_platform: TargetPlatform) -> Self {
        Self {
            target_platform,
            last_source_path: Mutex::new(None),
        }
    }

    /// Default constructor targeting native Windows desktop (`WindowsDesktop`).
    pub fn default_windows() -> Self {
        Self::new(TargetPlatform::WindowsDesktop)
    }

    /// Discovers project assets and module dependency edges into a structured `ProjectInventory`.
    pub fn discover_project(&self, source_path: &str) -> Result<ProjectInventory, DomainError> {
        discovery::discover_project(source_path)
    }

    /// Extracts and emits a deterministic `ArtifactKind::ModuleDependencyGraph` artifact.
    pub fn extract_module_dependency_graph(
        &self,
        source_path: &str,
    ) -> Result<VersionedArtifact, DomainError> {
        let inv = self.discover_project(source_path)?;
        Ok(inv.to_module_dependency_graph_artifact(ArtifactRevision::new(1)))
    }

    /// Directly classifies capability gaps for a source directory and semantic model.
    pub fn classify_source_gaps(
        &self,
        source_path: &str,
        model: &SemanticUiModel,
    ) -> Result<CapabilityReport, DomainError> {
        gap_classifier::classify_source_gaps(source_path, model)
    }

    /// Merges runtime trace observations into the semantic model while preserving static truth.
    pub fn merge_runtime_observations(
        &self,
        model: &mut SemanticUiModel,
        baseline: &RuntimeTraceBaseline,
    ) -> Result<Vec<VersionedArtifact>, DomainError> {
        runtime_capture::merge_runtime_observations(model, baseline)
    }
}

impl Default for ReactSourceAdapter {
    fn default() -> Self {
        Self::default_windows()
    }
}

impl SourceAdapterPort for ReactSourceAdapter {
    fn discover(&self, source_path: &str) -> Result<VersionedArtifact, DomainError> {
        if let Ok(mut lock) = self.last_source_path.lock() {
            *lock = Some(source_path.to_string());
        }
        let inv = discovery::discover_project(source_path)?;
        Ok(inv.to_source_inventory_artifact(ArtifactRevision::new(1)))
    }

    fn extract_semantics(&self, source_path: &str) -> Result<SemanticUiModel, DomainError> {
        if let Ok(mut lock) = self.last_source_path.lock() {
            *lock = Some(source_path.to_string());
        }
        static_analyzer::extract_semantics(source_path, self.target_platform)
    }

    fn assess_capabilities(
        &self,
        model: &SemanticUiModel,
    ) -> Result<CapabilityReport, DomainError> {
        let source_path_opt = self.last_source_path.lock().ok().and_then(|g| g.clone());
        if let Some(path) = source_path_opt {
            gap_classifier::classify_source_gaps(&path, model)
        } else {
            gap_classifier::classify_model_gaps(model)
        }
    }
}
