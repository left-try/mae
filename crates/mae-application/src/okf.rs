use std::fs;
use std::path::{Path, PathBuf};

use mae_domain::{DomainError, KnowledgeRepositoryPort};

/// Curated knowledge repository adapter managing `.okf/` repository brain bundles.
#[derive(Debug, Clone)]
pub struct OkfKnowledgeRepository {
    root_path: PathBuf,
}

impl OkfKnowledgeRepository {
    pub fn new(root_path: impl Into<PathBuf>) -> Self {
        Self {
            root_path: root_path.into(),
        }
    }

    pub fn root_path(&self) -> &Path {
        &self.root_path
    }

    /// Resolves the actual `.okf` bundle directory, accommodating either the repository root or `.okf/` itself.
    fn resolve_bundle_dir(&self) -> Option<PathBuf> {
        if self.root_path.join("manifest.yaml").exists() {
            Some(self.root_path.clone())
        } else if self.root_path.join(".okf").join("manifest.yaml").exists() {
            Some(self.root_path.join(".okf"))
        } else if self
            .root_path
            .join("../../.okf")
            .join("manifest.yaml")
            .exists()
        {
            Some(self.root_path.join("../../.okf"))
        } else {
            None
        }
    }

    /// Recursively collects markdown files and detects any prohibited `.okf-index/` artifacts.
    fn collect_md_files(
        dir: &Path,
        bundle_root: &Path,
        files: &mut Vec<(String, PathBuf)>,
        has_prohibited_index: &mut bool,
    ) -> Result<(), DomainError> {
        if !dir.exists() || !dir.is_dir() {
            return Ok(());
        }

        let entries = fs::read_dir(dir).map_err(|e| {
            DomainError::PortError(format!("Failed to read directory {}: {}", dir.display(), e))
        })?;

        for entry in entries {
            let entry = entry.map_err(|e| {
                DomainError::PortError(format!("Failed to read directory entry: {}", e))
            })?;
            let path = entry.path();
            let file_name = entry.file_name().to_string_lossy().to_string();

            if file_name == ".okf-index" {
                *has_prohibited_index = true;
                continue;
            }

            if path.is_dir() {
                Self::collect_md_files(&path, bundle_root, files, has_prohibited_index)?;
            } else if path.is_file() && path.extension().and_then(|ext| ext.to_str()) == Some("md")
            {
                let rel = path
                    .strip_prefix(bundle_root)
                    .unwrap_or(&path)
                    .to_string_lossy()
                    .replace('\\', "/");
                files.push((rel, path));
            }
        }

        Ok(())
    }
}

impl KnowledgeRepositoryPort for OkfKnowledgeRepository {
    fn validate_bundle(&self) -> Result<bool, DomainError> {
        let bundle_dir = match self.resolve_bundle_dir() {
            Some(dir) => dir,
            None => return Ok(false),
        };

        // 1. Invariant: Bundle must NOT contain generated .okf-index/ artifacts
        if bundle_dir.join(".okf-index").exists() {
            return Ok(false);
        }

        let mut md_files = Vec::new();
        let mut has_prohibited_index = false;
        Self::collect_md_files(
            &bundle_dir,
            &bundle_dir,
            &mut md_files,
            &mut has_prohibited_index,
        )?;
        if has_prohibited_index {
            return Ok(false);
        }

        // 2. Invariant: Presence of required bundle files
        let required_files = [
            "manifest.yaml",
            "index.md",
            "architecture/index.md",
            "application/index.md",
            "operations/index.md",
            "changes/index.md",
        ];

        for req in &required_files {
            let file_path = bundle_dir.join(req);
            if !file_path.exists() {
                return Ok(false);
            }
        }

        // 3. Invariant: manifest.yaml must specify schema_version: 1
        let manifest_content = fs::read_to_string(bundle_dir.join("manifest.yaml"))
            .map_err(|e| DomainError::PortError(format!("Failed to read manifest.yaml: {}", e)))?;

        let has_valid_schema = manifest_content.lines().any(|line| {
            let trimmed = line.trim();
            trimmed.starts_with("schema_version:")
                && trimmed
                    .strip_prefix("schema_version:")
                    .map(|v| v.trim() == "1")
                    .unwrap_or(false)
        });

        if !has_valid_schema {
            return Ok(false);
        }

        Ok(true)
    }

    fn search(&self, query: &str) -> Result<Vec<String>, DomainError> {
        let bundle_dir = match self.resolve_bundle_dir() {
            Some(dir) => dir,
            None => {
                return Err(DomainError::PortError(format!(
                    "OKF bundle root not found at {}",
                    self.root_path.display()
                )))
            }
        };

        let mut md_files = Vec::new();
        let mut has_prohibited_index = false;
        Self::collect_md_files(
            &bundle_dir,
            &bundle_dir,
            &mut md_files,
            &mut has_prohibited_index,
        )?;

        // Sort files deterministically by relative path
        md_files.sort_by(|a, b| a.0.cmp(&b.0));

        let query_lower = query.to_lowercase();
        let mut matches = Vec::new();

        for (rel_path, file_path) in md_files {
            let content = match fs::read_to_string(&file_path) {
                Ok(c) => c,
                Err(_) => continue,
            };

            for (line_idx, line) in content.lines().enumerate() {
                if line.to_lowercase().contains(&query_lower) {
                    matches.push(format!("{}:{}: {}", rel_path, line_idx + 1, line.trim()));
                }
            }
        }

        Ok(matches)
    }
}
