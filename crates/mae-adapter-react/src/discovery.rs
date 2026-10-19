use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use mae_domain::{
    ArtifactId, ArtifactKind, ArtifactProvenance, ArtifactRevision, DomainError, VersionedArtifact,
};

use crate::json::JsonValue;

/// Structured inventory representing discovered project assets and topology.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectInventory {
    pub project_name: String,
    pub version: String,
    pub dependencies: BTreeMap<String, String>,
    pub dev_dependencies: BTreeMap<String, String>,
    pub entry_files: Vec<String>,
    pub tsx_files: Vec<String>,
    pub ts_files: Vec<String>,
    pub style_files: Vec<String>,
    pub asset_files: Vec<String>,
    pub module_dependencies: Vec<(String, String)>,
    pub has_tsconfig: bool,
}

impl ProjectInventory {
    /// Formats inventory to deterministic JSON representation.
    pub fn to_source_inventory_json(&self) -> String {
        let mut deps_obj = Vec::new();
        for (k, v) in &self.dependencies {
            deps_obj.push((k.clone(), JsonValue::String(v.clone())));
        }

        let mut dev_deps_obj = Vec::new();
        for (k, v) in &self.dev_dependencies {
            dev_deps_obj.push((k.clone(), JsonValue::String(v.clone())));
        }

        let entries_arr: Vec<JsonValue> = self
            .entry_files
            .iter()
            .map(|s| JsonValue::String(s.clone()))
            .collect();
        let tsx_arr: Vec<JsonValue> = self
            .tsx_files
            .iter()
            .map(|s| JsonValue::String(s.clone()))
            .collect();
        let ts_arr: Vec<JsonValue> = self
            .ts_files
            .iter()
            .map(|s| JsonValue::String(s.clone()))
            .collect();
        let style_arr: Vec<JsonValue> = self
            .style_files
            .iter()
            .map(|s| JsonValue::String(s.clone()))
            .collect();
        let asset_arr: Vec<JsonValue> = self
            .asset_files
            .iter()
            .map(|s| JsonValue::String(s.clone()))
            .collect();

        let root = JsonValue::Object(vec![
            (
                "project_name".to_string(),
                JsonValue::String(self.project_name.clone()),
            ),
            (
                "version".to_string(),
                JsonValue::String(self.version.clone()),
            ),
            (
                "has_tsconfig".to_string(),
                JsonValue::Bool(self.has_tsconfig),
            ),
            ("dependencies".to_string(), JsonValue::Object(deps_obj)),
            (
                "dev_dependencies".to_string(),
                JsonValue::Object(dev_deps_obj),
            ),
            ("entry_files".to_string(), JsonValue::Array(entries_arr)),
            ("tsx_files".to_string(), JsonValue::Array(tsx_arr)),
            ("ts_files".to_string(), JsonValue::Array(ts_arr)),
            ("style_files".to_string(), JsonValue::Array(style_arr)),
            ("asset_files".to_string(), JsonValue::Array(asset_arr)),
        ]);

        root.to_pretty_json(0)
    }

    /// Formats module dependency edges to deterministic JSON representation.
    pub fn to_module_graph_json(&self) -> String {
        let mut edges_arr = Vec::new();
        for (from, to) in &self.module_dependencies {
            edges_arr.push(JsonValue::Object(vec![
                ("from".to_string(), JsonValue::String(from.clone())),
                ("to".to_string(), JsonValue::String(to.clone())),
            ]));
        }

        let root = JsonValue::Object(vec![
            (
                "project_name".to_string(),
                JsonValue::String(self.project_name.clone()),
            ),
            ("edges".to_string(), JsonValue::Array(edges_arr)),
        ]);

        root.to_pretty_json(0)
    }

    /// Emits a deterministic `VersionedArtifact` for `ArtifactKind::SourceInventory`.
    pub fn to_source_inventory_artifact(&self, source_rev: ArtifactRevision) -> VersionedArtifact {
        let json_str = self.to_source_inventory_json();
        let artifact_id = ArtifactId::new(format!("inventory:{}", self.project_name));
        let provenance =
            ArtifactProvenance::static_deterministic("mae-adapter-react:discovery", source_rev);
        VersionedArtifact::from_string(
            artifact_id,
            ArtifactKind::SourceInventory,
            source_rev,
            provenance,
            json_str,
        )
    }

    /// Emits a deterministic `VersionedArtifact` for `ArtifactKind::ModuleDependencyGraph`.
    pub fn to_module_dependency_graph_artifact(
        &self,
        source_rev: ArtifactRevision,
    ) -> VersionedArtifact {
        let json_str = self.to_module_graph_json();
        let artifact_id = ArtifactId::new(format!("module-graph:{}", self.project_name));
        let provenance =
            ArtifactProvenance::static_deterministic("mae-adapter-react:discovery", source_rev);
        VersionedArtifact::from_string(
            artifact_id,
            ArtifactKind::ModuleDependencyGraph,
            source_rev,
            provenance,
            json_str,
        )
    }
}

/// Inspects a React/TypeScript source project directory and constructs a `ProjectInventory`.
pub fn discover_project(source_path: &str) -> Result<ProjectInventory, DomainError> {
    let root = PathBuf::from(source_path);
    if !root.exists() {
        return Err(DomainError::PortError(format!(
            "Source path does not exist: {}",
            source_path
        )));
    }

    let mut project_name = root
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("react-app")
        .to_string();
    let mut version = "0.1.0".to_string();
    let mut dependencies = BTreeMap::new();
    let mut dev_dependencies = BTreeMap::new();

    // Check package.json
    let package_json_path = root.join("package.json");
    if package_json_path.exists() {
        if let Ok(pkg_content) = fs::read_to_string(&package_json_path) {
            if let Ok(json_val) = JsonValue::parse(&pkg_content) {
                if let Some(n) = json_val.get_str("name") {
                    project_name = n.to_string();
                }
                if let Some(v) = json_val.get_str("version") {
                    version = v.to_string();
                }
                if let Some(deps) = json_val.get_object("dependencies") {
                    for (k, v) in deps {
                        if let Some(ver) = v.as_str() {
                            dependencies.insert(k.clone(), ver.to_string());
                        }
                    }
                }
                if let Some(dev_deps) = json_val.get_object("devDependencies") {
                    for (k, v) in dev_deps {
                        if let Some(ver) = v.as_str() {
                            dev_dependencies.insert(k.clone(), ver.to_string());
                        }
                    }
                }
            }
        }
    }

    let has_tsconfig = root.join("tsconfig.json").exists();

    // Traverse files recursively
    let mut relative_files = Vec::new();
    walk_directory(&root, &root, &mut relative_files)?;
    relative_files.sort();

    let mut entry_files = Vec::new();
    let mut tsx_files = Vec::new();
    let mut ts_files = Vec::new();
    let mut style_files = Vec::new();
    let mut asset_files = Vec::new();
    let mut module_dependencies = Vec::new();

    for rel_path in &relative_files {
        let lower = rel_path.to_lowercase();
        if lower.ends_with(".tsx") || lower.ends_with(".jsx") {
            tsx_files.push(rel_path.clone());
        } else if lower.ends_with(".ts") || lower.ends_with(".js") {
            ts_files.push(rel_path.clone());
        } else if lower.ends_with(".css")
            || lower.ends_with(".scss")
            || lower.ends_with(".sass")
            || lower.ends_with(".less")
        {
            style_files.push(rel_path.clone());
        } else if lower.ends_with(".svg")
            || lower.ends_with(".png")
            || lower.ends_with(".jpg")
            || lower.ends_with(".jpeg")
            || lower.ends_with(".ico")
            || lower.ends_with(".webp")
            || lower.ends_with(".gif")
        {
            asset_files.push(rel_path.clone());
        }

        // Detect entry files
        if is_likely_entry_file(rel_path) {
            entry_files.push(rel_path.clone());
        }

        // Extract module dependencies from code files
        if lower.ends_with(".tsx")
            || lower.ends_with(".ts")
            || lower.ends_with(".jsx")
            || lower.ends_with(".js")
        {
            let full_path = root.join(rel_path);
            if let Ok(content) = fs::read_to_string(&full_path) {
                let edges = extract_import_edges(rel_path, &content);
                module_dependencies.extend(edges);
            }
        }
    }

    entry_files.sort();
    tsx_files.sort();
    ts_files.sort();
    style_files.sort();
    asset_files.sort();
    module_dependencies.sort();
    module_dependencies.dedup();

    Ok(ProjectInventory {
        project_name,
        version,
        dependencies,
        dev_dependencies,
        entry_files,
        tsx_files,
        ts_files,
        style_files,
        asset_files,
        module_dependencies,
        has_tsconfig,
    })
}

fn walk_directory(current: &Path, root: &Path, out: &mut Vec<String>) -> Result<(), DomainError> {
    if !current.is_dir() {
        return Ok(());
    }

    let entries = fs::read_dir(current).map_err(|e| {
        DomainError::PortError(format!("Failed to read dir {}: {}", current.display(), e))
    })?;

    for entry in entries {
        let entry = entry.map_err(|e| {
            DomainError::PortError(format!(
                "Directory entry error in {}: {}",
                current.display(),
                e
            ))
        })?;
        let path = entry.path();
        let file_name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or_default();

        // Skip ignored directories
        if path.is_dir() {
            if file_name == "node_modules"
                || file_name == ".git"
                || file_name == "target"
                || file_name == "dist"
                || file_name == "build"
                || file_name == ".next"
                || file_name == ".turbo"
                || file_name == ".okf-index"
            {
                continue;
            }
            walk_directory(&path, root, out)?;
        } else if let Ok(rel) = path.strip_prefix(root) {
            let rel_str = rel.to_string_lossy().replace('\\', "/");
            out.push(rel_str);
        }
    }

    Ok(())
}

fn is_likely_entry_file(path: &str) -> bool {
    matches!(
        path,
        "src/main.tsx"
            | "src/main.ts"
            | "src/main.jsx"
            | "src/main.js"
            | "src/index.tsx"
            | "src/index.ts"
            | "src/index.jsx"
            | "src/index.js"
            | "src/App.tsx"
            | "src/App.jsx"
            | "main.tsx"
            | "index.tsx"
            | "App.tsx"
    )
}

/// Extracts `(from_file, imported_specifier)` edges from source code lines.
pub fn extract_import_edges(from_file: &str, content: &str) -> Vec<(String, String)> {
    let mut edges = Vec::new();

    for line in content.lines() {
        let trimmed = line.trim();
        // Check for import ... from '...' or import '...'
        if trimmed.starts_with("import ") || trimmed.starts_with("import{") {
            if let Some(specifier) = extract_specifier_from_import(trimmed) {
                edges.push((from_file.to_string(), specifier));
            }
        } else if trimmed.starts_with("export ") && trimmed.contains(" from ") {
            if let Some(specifier) = extract_specifier_from_export(trimmed) {
                edges.push((from_file.to_string(), specifier));
            }
        }
    }

    edges
}

fn extract_specifier_from_import(line: &str) -> Option<String> {
    // Case 1: import ... from 'specifier' or "specifier"
    if let Some(pos) = line.rfind("from") {
        let after_from = line[pos + 4..].trim();
        return extract_quoted_string(after_from);
    }

    // Case 2: import 'specifier' or import "specifier"
    let after_import = line[6..].trim();
    if after_import.starts_with('\'') || after_import.starts_with('"') {
        return extract_quoted_string(after_import);
    }

    None
}

fn extract_specifier_from_export(line: &str) -> Option<String> {
    if let Some(pos) = line.rfind("from") {
        let after_from = line[pos + 4..].trim();
        return extract_quoted_string(after_from);
    }
    None
}

fn extract_quoted_string(s: &str) -> Option<String> {
    let trimmed = s.trim_matches(';').trim();
    if ((trimmed.starts_with('"') && trimmed.ends_with('"'))
        || (trimmed.starts_with('\'') && trimmed.ends_with('\'')))
        && trimmed.len() >= 2
    {
        return Some(trimmed[1..trimmed.len() - 1].to_string());
    }
    None
}
