use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use mae_domain::{
    ArtifactId, ArtifactKind, ArtifactProvenance, ArtifactRepositoryPort, ArtifactRevision,
    ContentHash, DomainError, ProvenanceOrigin, VersionedArtifact,
};

/// In-memory artifact repository with content-hash integrity verification.
#[derive(Debug, Clone, Default)]
pub struct InMemoryArtifactRepository {
    artifacts: BTreeMap<(ArtifactId, ArtifactRevision), VersionedArtifact>,
}

impl InMemoryArtifactRepository {
    pub fn new() -> Self {
        Self {
            artifacts: BTreeMap::new(),
        }
    }

    pub fn len(&self) -> usize {
        self.artifacts.len()
    }

    pub fn is_empty(&self) -> bool {
        self.artifacts.is_empty()
    }
}

impl ArtifactRepositoryPort for InMemoryArtifactRepository {
    fn store(&mut self, artifact: VersionedArtifact) -> Result<(), DomainError> {
        if !artifact.verify_integrity() {
            return Err(DomainError::InvalidArtifact(format!(
                "Content hash integrity check failed on store for artifact {}",
                artifact.id
            )));
        }
        self.artifacts
            .insert((artifact.id.clone(), artifact.revision), artifact);
        Ok(())
    }

    fn get(
        &self,
        id: &ArtifactId,
        revision: ArtifactRevision,
    ) -> Result<Option<VersionedArtifact>, DomainError> {
        match self.artifacts.get(&(id.clone(), revision)) {
            Some(artifact) => {
                if !artifact.verify_integrity() {
                    return Err(DomainError::InvalidArtifact(format!(
                        "Content hash integrity check failed on get for artifact {}",
                        artifact.id
                    )));
                }
                Ok(Some(artifact.clone()))
            }
            None => Ok(None),
        }
    }

    fn list_revisions(&self, id: &ArtifactId) -> Result<Vec<ArtifactRevision>, DomainError> {
        let mut revs: Vec<ArtifactRevision> = self
            .artifacts
            .keys()
            .filter(|(aid, _)| aid == id)
            .map(|(_, rev)| *rev)
            .collect();
        revs.sort();
        Ok(revs)
    }
}

/// Filesystem artifact repository storing content and metadata on disk with content-hash integrity checks.
#[derive(Debug, Clone)]
pub struct FileSystemArtifactRepository {
    base_dir: PathBuf,
}

impl FileSystemArtifactRepository {
    pub fn new(base_dir: impl Into<PathBuf>) -> Self {
        Self {
            base_dir: base_dir.into(),
        }
    }

    pub fn base_dir(&self) -> &Path {
        &self.base_dir
    }

    fn sanitize_id(id: &ArtifactId) -> String {
        id.as_str().replace(['/', '\\', ':', '*'], "_")
    }

    fn artifact_dir(&self, id: &ArtifactId) -> PathBuf {
        self.base_dir.join(Self::sanitize_id(id))
    }
}

fn parse_kind(s: &str) -> Result<ArtifactKind, DomainError> {
    match s {
        "SOURCE_INVENTORY" => Ok(ArtifactKind::SourceInventory),
        "MODULE_DEPENDENCY_GRAPH" => Ok(ArtifactKind::ModuleDependencyGraph),
        "COMPONENT_GRAPH" => Ok(ArtifactKind::ComponentGraph),
        "STATE_MODEL" => Ok(ArtifactKind::StateModel),
        "EFFECT_MODEL" => Ok(ArtifactKind::EffectModel),
        "INTERACTION_MODEL" => Ok(ArtifactKind::InteractionModel),
        "NAVIGATION_MODEL" => Ok(ArtifactKind::NavigationModel),
        "LAYOUT_MODEL" => Ok(ArtifactKind::LayoutModel),
        "STYLE_TOKEN_SET" => Ok(ArtifactKind::StyleTokenSet),
        "ASSET_INVENTORY" => Ok(ArtifactKind::AssetInventory),
        "RUNTIME_TRACE_SET" => Ok(ArtifactKind::RuntimeTraceSet),
        "SCREENSHOT_BASELINE" => Ok(ArtifactKind::ScreenshotBaseline),
        "CAPABILITY_REPORT" => Ok(ArtifactKind::CapabilityReport),
        "SEMANTIC_UI_MODEL" => Ok(ArtifactKind::SemanticUiModel),
        "TARGET_CHANGE_SET" => Ok(ArtifactKind::TargetChangeSet),
        "VERIFICATION_REPORT" => Ok(ArtifactKind::VerificationReport),
        other => Err(DomainError::PortError(format!(
            "Unknown artifact kind: {}",
            other
        ))),
    }
}

fn parse_origin(s: &str) -> Result<ProvenanceOrigin, DomainError> {
    if s == "DETERMINISTIC_STATIC" {
        Ok(ProvenanceOrigin::DeterministicStatic)
    } else if s == "RUNTIME_OBSERVATION" {
        Ok(ProvenanceOrigin::RuntimeObservation)
    } else if s == "UNKNOWN_GAP" {
        Ok(ProvenanceOrigin::UnknownGap)
    } else if let Some(bps) = s.strip_prefix("INFERRED_SEMANTIC:") {
        let val = bps.parse::<u16>().unwrap_or(0);
        Ok(ProvenanceOrigin::InferredSemantic {
            confidence_bps: val,
        })
    } else if s == "INFERRED_SEMANTIC" {
        Ok(ProvenanceOrigin::InferredSemantic {
            confidence_bps: 10000,
        })
    } else {
        Err(DomainError::PortError(format!(
            "Unknown provenance origin: {}",
            s
        )))
    }
}

impl ArtifactRepositoryPort for FileSystemArtifactRepository {
    fn store(&mut self, artifact: VersionedArtifact) -> Result<(), DomainError> {
        if !artifact.verify_integrity() {
            return Err(DomainError::InvalidArtifact(format!(
                "Content hash integrity check failed on store for artifact {}",
                artifact.id
            )));
        }

        let dir = self.artifact_dir(&artifact.id);
        fs::create_dir_all(&dir).map_err(|e| {
            DomainError::PortError(format!(
                "Failed to create artifact directory {}: {}",
                dir.display(),
                e
            ))
        })?;

        let bin_path = dir.join(format!("r{}.bin", artifact.revision.as_u64()));
        let meta_path = dir.join(format!("r{}.meta", artifact.revision.as_u64()));

        fs::write(&bin_path, &artifact.content).map_err(|e| {
            DomainError::PortError(format!(
                "Failed to write artifact payload {}: {}",
                bin_path.display(),
                e
            ))
        })?;

        let origin_str = match &artifact.provenance.origin {
            ProvenanceOrigin::InferredSemantic { confidence_bps } => {
                format!("INFERRED_SEMANTIC:{}", confidence_bps)
            }
            other => other.as_str().to_string(),
        };

        let meta_content = format!(
            "id={}\nkind={}\nrevision={}\ncontent_hash={}\norigin={}\ngenerator={}\ntimestamp={}\nsource_rev={}\n",
            artifact.id.as_str(),
            artifact.kind.as_str(),
            artifact.revision.as_u64(),
            artifact.content_hash.as_str(),
            origin_str,
            artifact.provenance.generator_id,
            artifact.provenance.timestamp_iso,
            artifact.provenance.source_revision.as_u64(),
        );

        fs::write(&meta_path, meta_content).map_err(|e| {
            DomainError::PortError(format!(
                "Failed to write artifact metadata {}: {}",
                meta_path.display(),
                e
            ))
        })?;

        Ok(())
    }

    fn get(
        &self,
        id: &ArtifactId,
        revision: ArtifactRevision,
    ) -> Result<Option<VersionedArtifact>, DomainError> {
        let dir = self.artifact_dir(id);
        let bin_path = dir.join(format!("r{}.bin", revision.as_u64()));
        let meta_path = dir.join(format!("r{}.meta", revision.as_u64()));

        if !bin_path.exists() || !meta_path.exists() {
            return Ok(None);
        }

        let content = fs::read(&bin_path).map_err(|e| {
            DomainError::PortError(format!(
                "Failed to read artifact payload {}: {}",
                bin_path.display(),
                e
            ))
        })?;

        let meta_content = fs::read_to_string(&meta_path).map_err(|e| {
            DomainError::PortError(format!(
                "Failed to read artifact metadata {}: {}",
                meta_path.display(),
                e
            ))
        })?;

        let mut kind = None;
        let mut expected_hash = None;
        let mut origin = None;
        let mut generator = String::new();
        let mut timestamp = String::new();
        let mut source_rev = 1u64;

        for line in meta_content.lines() {
            let line = line.trim();
            if let Some(val) = line.strip_prefix("kind=") {
                kind = Some(parse_kind(val)?);
            } else if let Some(val) = line.strip_prefix("content_hash=") {
                expected_hash = Some(ContentHash::new(val));
            } else if let Some(val) = line.strip_prefix("origin=") {
                origin = Some(parse_origin(val)?);
            } else if let Some(val) = line.strip_prefix("generator=") {
                generator = val.to_string();
            } else if let Some(val) = line.strip_prefix("timestamp=") {
                timestamp = val.to_string();
            } else if let Some(val) = line.strip_prefix("source_rev=") {
                if let Ok(num) = val.parse::<u64>() {
                    source_rev = num;
                }
            }
        }

        let kind = kind.ok_or_else(|| {
            DomainError::PortError("Missing 'kind=' in artifact metadata".to_string())
        })?;
        let expected_hash = expected_hash.ok_or_else(|| {
            DomainError::PortError("Missing 'content_hash=' in artifact metadata".to_string())
        })?;
        let origin = origin.ok_or_else(|| {
            DomainError::PortError("Missing 'origin=' in artifact metadata".to_string())
        })?;

        let provenance = ArtifactProvenance::new(
            origin,
            generator,
            timestamp,
            ArtifactRevision::new(source_rev),
        );

        let artifact = VersionedArtifact {
            id: id.clone(),
            kind,
            revision,
            content_hash: expected_hash,
            provenance,
            content,
        };

        if !artifact.verify_integrity() {
            return Err(DomainError::InvalidArtifact(format!(
                "Content hash integrity check failed on get for artifact {}",
                id
            )));
        }

        Ok(Some(artifact))
    }

    fn list_revisions(&self, id: &ArtifactId) -> Result<Vec<ArtifactRevision>, DomainError> {
        let dir = self.artifact_dir(id);
        if !dir.exists() {
            return Ok(Vec::new());
        }

        let entries = fs::read_dir(&dir).map_err(|e| {
            DomainError::PortError(format!("Failed to read directory {}: {}", dir.display(), e))
        })?;

        let mut revs = Vec::new();
        for entry in entries {
            let entry = entry.map_err(|e| {
                DomainError::PortError(format!("Failed to read directory entry: {}", e))
            })?;
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with('r') && name.ends_with(".bin") {
                let num_str = &name[1..name.len() - 4];
                if let Ok(num) = num_str.parse::<u64>() {
                    revs.push(ArtifactRevision::new(num));
                }
            }
        }

        revs.sort();
        Ok(revs)
    }
}
