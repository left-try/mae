use std::fmt;

use crate::ids::{ArtifactId, ArtifactRevision, ContentHash};

/// Distinct kind of migration artifact or intermediate representation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ArtifactKind {
    SourceInventory,
    ModuleDependencyGraph,
    ComponentGraph,
    StateModel,
    EffectModel,
    InteractionModel,
    NavigationModel,
    LayoutModel,
    StyleTokenSet,
    AssetInventory,
    RuntimeTraceSet,
    ScreenshotBaseline,
    CapabilityReport,
    SemanticUiModel,
    TargetChangeSet,
    VerificationReport,
}

impl ArtifactKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::SourceInventory => "SOURCE_INVENTORY",
            Self::ModuleDependencyGraph => "MODULE_DEPENDENCY_GRAPH",
            Self::ComponentGraph => "COMPONENT_GRAPH",
            Self::StateModel => "STATE_MODEL",
            Self::EffectModel => "EFFECT_MODEL",
            Self::InteractionModel => "INTERACTION_MODEL",
            Self::NavigationModel => "NAVIGATION_MODEL",
            Self::LayoutModel => "LAYOUT_MODEL",
            Self::StyleTokenSet => "STYLE_TOKEN_SET",
            Self::AssetInventory => "ASSET_INVENTORY",
            Self::RuntimeTraceSet => "RUNTIME_TRACE_SET",
            Self::ScreenshotBaseline => "SCREENSHOT_BASELINE",
            Self::CapabilityReport => "CAPABILITY_REPORT",
            Self::SemanticUiModel => "SEMANTIC_UI_MODEL",
            Self::TargetChangeSet => "TARGET_CHANGE_SET",
            Self::VerificationReport => "VERIFICATION_REPORT",
        }
    }
}

impl fmt::Display for ArtifactKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Source of truth origin for an artifact, preserving evidence integrity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProvenanceOrigin {
    DeterministicStatic,
    RuntimeObservation,
    InferredSemantic { confidence_bps: u16 },
    UnknownGap,
}

impl ProvenanceOrigin {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::DeterministicStatic => "DETERMINISTIC_STATIC",
            Self::RuntimeObservation => "RUNTIME_OBSERVATION",
            Self::InferredSemantic { .. } => "INFERRED_SEMANTIC",
            Self::UnknownGap => "UNKNOWN_GAP",
        }
    }
}

/// Provenance metadata tracking who generated an artifact, how, and from what source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArtifactProvenance {
    pub origin: ProvenanceOrigin,
    pub generator_id: String,
    pub timestamp_iso: String,
    pub source_revision: ArtifactRevision,
}

impl ArtifactProvenance {
    pub fn new(
        origin: ProvenanceOrigin,
        generator_id: impl Into<String>,
        timestamp_iso: impl Into<String>,
        source_revision: ArtifactRevision,
    ) -> Self {
        Self {
            origin,
            generator_id: generator_id.into(),
            timestamp_iso: timestamp_iso.into(),
            source_revision,
        }
    }

    pub fn static_deterministic(
        generator_id: impl Into<String>,
        source_rev: ArtifactRevision,
    ) -> Self {
        Self::new(
            ProvenanceOrigin::DeterministicStatic,
            generator_id,
            "1970-01-01T00:00:00Z",
            source_rev,
        )
    }
}

/// Immutable, content-addressed, revision-tagged migration artifact.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VersionedArtifact {
    pub id: ArtifactId,
    pub kind: ArtifactKind,
    pub revision: ArtifactRevision,
    pub content_hash: ContentHash,
    pub provenance: ArtifactProvenance,
    pub content: Vec<u8>,
}

impl VersionedArtifact {
    pub fn new(
        id: impl Into<ArtifactId>,
        kind: ArtifactKind,
        revision: ArtifactRevision,
        provenance: ArtifactProvenance,
        content: Vec<u8>,
    ) -> Self {
        let content_hash = ContentHash::compute(&content);
        Self {
            id: id.into(),
            kind,
            revision,
            content_hash,
            provenance,
            content,
        }
    }

    pub fn from_string(
        id: impl Into<ArtifactId>,
        kind: ArtifactKind,
        revision: ArtifactRevision,
        provenance: ArtifactProvenance,
        text: impl Into<String>,
    ) -> Self {
        let text_str = text.into();
        Self::new(id, kind, revision, provenance, text_str.into_bytes())
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.content
    }

    pub fn as_str(&self) -> Result<&str, std::str::Utf8Error> {
        std::str::from_utf8(&self.content)
    }

    pub fn verify_integrity(&self) -> bool {
        let computed = ContentHash::compute(&self.content);
        computed == self.content_hash
    }
}
