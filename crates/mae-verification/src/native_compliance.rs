use std::fs;
use std::path::{Path, PathBuf};

use mae_domain::{
    ArtifactRevision, Contract, CoverageDisclosure, DomainError, EvidenceRecord, TargetPlatform,
    VerificationCategory, VerificationStatus, VerifierPluginPort,
};

const FORBIDDEN_WRAPPER_TOKENS: &[&str] = &[
    "wry",
    "tauri",
    "webview2",
    "webkit2gtk",
    "electron",
    "headless_chrome",
    "whole_app_proxy",
];

const FORBIDDEN_CLI_WRAPPER_TOKENS: &[&str] = &[
    "pyo3",
    "rustpython",
    "command::new(\"python",
    "command::new(\"python3",
    "whole_app_proxy",
];

/// Independent verifier proving the target is a genuine native Rust application
/// (either Slint GUI or native CLI), not a WebView wrapper, Electron shell,
/// Python subprocess wrapper, screenshot-as-UI, or whole-app proxy.
#[derive(Debug, Clone)]
pub struct NativeComplianceVerifier {
    pub target_dir: PathBuf,
    pub require_windows_subsystem: bool,
    pub require_linux_desktop_entry: bool,
    pub is_cli_target: bool,
    pub source_revision: ArtifactRevision,
}

impl NativeComplianceVerifier {
    pub fn new(target_dir: impl Into<PathBuf>, require_windows_subsystem: bool) -> Self {
        Self {
            target_dir: target_dir.into(),
            require_windows_subsystem,
            require_linux_desktop_entry: false,
            is_cli_target: false,
            source_revision: ArtifactRevision::initial(),
        }
    }

    pub fn for_platform(target_dir: impl Into<PathBuf>, platform: TargetPlatform) -> Self {
        Self {
            target_dir: target_dir.into(),
            require_windows_subsystem: platform.requires_windows_subsystem(),
            require_linux_desktop_entry: platform.requires_linux_desktop_entry(),
            is_cli_target: false,
            source_revision: ArtifactRevision::initial(),
        }
    }

    pub fn for_cli_target(target_dir: impl Into<PathBuf>) -> Self {
        Self {
            target_dir: target_dir.into(),
            require_windows_subsystem: false,
            require_linux_desktop_entry: false,
            is_cli_target: true,
            source_revision: ArtifactRevision::initial(),
        }
    }

    pub fn with_source_revision(mut self, rev: ArtifactRevision) -> Self {
        self.source_revision = rev;
        self
    }

    /// Evaluates native compliance on `self.target_dir` and returns `(VerificationStatus, Vec<String>)` violations.
    pub fn inspect_target(&self) -> Result<(VerificationStatus, Vec<String>), DomainError> {
        let mut violations = Vec::new();

        let cargo_toml_path = self.target_dir.join("Cargo.toml");
        let main_rs_path = self.target_dir.join("src/main.rs");

        if !cargo_toml_path.exists() {
            violations.push("Missing Cargo.toml in target directory".to_string());
        }
        if !main_rs_path.exists() {
            violations.push("Missing src/main.rs in target directory".to_string());
        }

        if self.is_cli_target {
            let cli_rs_path = self.target_dir.join("src/cli.rs");
            let core_mod_path = self.target_dir.join("src/core/mod.rs");
            if !cli_rs_path.exists() {
                violations.push("Missing src/cli.rs in CLI target directory".to_string());
            }
            if !core_mod_path.exists() {
                violations.push("Missing src/core/mod.rs in CLI target directory".to_string());
            }
            if let Ok(cargo_toml) = fs::read_to_string(&cargo_toml_path) {
                let lower = cargo_toml.to_lowercase();
                for token in FORBIDDEN_WRAPPER_TOKENS
                    .iter()
                    .chain(FORBIDDEN_CLI_WRAPPER_TOKENS.iter())
                {
                    if lower.contains(token) {
                        violations.push(format!(
                            "Forbidden wrapper dependency '{token}' detected in Cargo.toml"
                        ));
                    }
                }
            }
            scan_rs_files_for_tokens(
                &self.target_dir.join("src"),
                FORBIDDEN_WRAPPER_TOKENS,
                &mut violations,
            );
            scan_rs_files_for_tokens(
                &self.target_dir.join("src"),
                FORBIDDEN_CLI_WRAPPER_TOKENS,
                &mut violations,
            );

            return if violations.is_empty() {
                Ok((VerificationStatus::Pass, violations))
            } else {
                Ok((VerificationStatus::Fail, violations))
            };
        }

        let reducer_rs_path = self.target_dir.join("src/app/reducer.rs");
        let app_slint_path = self.target_dir.join("ui/app.slint");

        if !reducer_rs_path.exists() {
            violations.push(
                "Missing src/app/reducer.rs (Logic-UI separation violation: Rust reducer required)"
                    .to_string(),
            );
        }
        if !app_slint_path.exists() {
            violations.push("Missing ui/app.slint declarative Slint UI entrypoint".to_string());
        }

        if let Ok(cargo_toml) = fs::read_to_string(&cargo_toml_path) {
            let lower = cargo_toml.to_lowercase();
            for token in FORBIDDEN_WRAPPER_TOKENS {
                if lower.contains(token) {
                    violations.push(format!(
                        "Forbidden browser/wrapper dependency '{token}' detected in Cargo.toml"
                    ));
                }
            }
            if lower.contains("embedded_browser = true") {
                violations.push("Cargo.toml declares embedded_browser = true".to_string());
            }
        }

        if let Ok(main_rs) = fs::read_to_string(&main_rs_path) {
            let lower = main_rs.to_lowercase();
            for token in FORBIDDEN_WRAPPER_TOKENS {
                if lower.contains(token) {
                    violations.push(format!(
                        "Forbidden wrapper/proxy usage '{token}' detected in src/main.rs"
                    ));
                }
            }
            if self.require_windows_subsystem
                && !main_rs.contains("windows_subsystem = \"windows\"")
            {
                violations.push(
                    "Missing Windows GUI subsystem attribute `#![cfg_attr(target_os = \"windows\", windows_subsystem = \"windows\")]` in src/main.rs".to_string(),
                );
            }
        }

        if self.require_linux_desktop_entry {
            let packaging_dir = self.target_dir.join("packaging");
            let mut found_valid_desktop = false;
            if let Ok(entries) = fs::read_dir(&packaging_dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.extension().and_then(|e| e.to_str()) == Some("desktop") {
                        if let Ok(desktop_content) = fs::read_to_string(&path) {
                            if desktop_content.contains("[Desktop Entry]")
                                && desktop_content.contains("Type=Application")
                            {
                                found_valid_desktop = true;
                            }
                        }
                    }
                }
            }
            if !found_valid_desktop {
                violations.push(
                    "Missing Ubuntu/Linux XDG `[Desktop Entry]` file under packaging/*.desktop"
                        .to_string(),
                );
            }
        }

        if let Ok(app_slint) = fs::read_to_string(&app_slint_path) {
            if !app_slint.contains("inherits Window") {
                violations.push(
                    "ui/app.slint does not declare a native Slint Window component".to_string(),
                );
            }
            // Reject screenshot-as-UI cheat (static screenshot image with zero callbacks/properties)
            let has_screenshot_cheat = app_slint.to_lowercase().contains("screenshot_as_ui")
                || (app_slint.contains("Image {")
                    && !app_slint.contains("callback ")
                    && !app_slint.contains("in-out property"));
            if has_screenshot_cheat {
                violations.push(
                    "Screenshot-as-UI violation: ui/app.slint substitutes a static image for interactive Slint controls".to_string(),
                );
            }
        }

        // Also scan all .rs files under src/ for forbidden wrapper tokens
        scan_rs_files_for_tokens(
            &self.target_dir.join("src"),
            FORBIDDEN_WRAPPER_TOKENS,
            &mut violations,
        );

        if violations.is_empty() {
            Ok((VerificationStatus::Pass, violations))
        } else {
            Ok((VerificationStatus::Fail, violations))
        }
    }
}

fn scan_rs_files_for_tokens(dir: &Path, tokens: &[&str], violations: &mut Vec<String>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            scan_rs_files_for_tokens(&path, tokens, violations);
        } else if path.extension().and_then(|e| e.to_str()) == Some("rs") {
            if let Ok(content) = fs::read_to_string(&path) {
                let lower = content.to_lowercase();
                for token in tokens {
                    if lower.contains(token) {
                        let msg = format!(
                            "Forbidden wrapper/proxy token '{}' in {}",
                            token,
                            path.display()
                        );
                        if !violations.contains(&msg) {
                            violations.push(msg);
                        }
                    }
                }
            }
        }
    }
}

impl VerifierPluginPort for NativeComplianceVerifier {
    fn category(&self) -> VerificationCategory {
        VerificationCategory::NativeCompliance
    }

    fn verify(
        &self,
        contract: &Contract,
        target_rev: ArtifactRevision,
    ) -> Result<EvidenceRecord, DomainError> {
        let (status, violations) = self.inspect_target()?;
        let scenarios = if self.is_cli_target {
            vec![
                "native.cli_binary_entrypoint".to_string(),
                "native.no_python_subprocess_or_pyo3_wrapper".to_string(),
                "native.cli_core_separation".to_string(),
            ]
        } else {
            vec![
                "native.slint_window".to_string(),
                "native.no_webview_or_electron".to_string(),
                "native.logic_ui_separation".to_string(),
                "native.windows_subsystem".to_string(),
            ]
        };

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

        let summary = if violations.is_empty() {
            if self.is_cli_target {
                "Target passed native Rust CLI compliance gate (no Python subprocess/PyO3 wrapper)"
                    .to_string()
            } else {
                "Target passed native Rust + Slint + Windows subsystem compliance gate".to_string()
            }
        } else {
            format!("Native compliance failed: {}", violations.join("; "))
        };

        let mut record = EvidenceRecord::new(
            contract.id.clone(),
            "mae.verifier.native-compliance/0.1",
            VerificationCategory::NativeCompliance,
            self.source_revision,
            target_rev,
            status,
            coverage,
            summary,
        )
        .with_produced_file("src/main.rs");

        if self.is_cli_target {
            record = record.with_produced_file("src/cli.rs");
        } else {
            record = record.with_produced_file("ui/app.slint");
        }
        Ok(record)
    }
}
