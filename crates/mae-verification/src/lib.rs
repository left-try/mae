//! Independent Verification Suite (`crates/mae-verification`).
//!
//! Implements deterministic `VerifierPluginPort` verifiers separate from code generation:
//! - `NativeComplianceVerifier` (rejects WebView/Electron/Tauri/screenshot-as-UI/proxy cheats)
//! - `BuildVerifier` (runs Cargo build/test/clippy or structural target checks)
//! - `BehavioralDifferentialVerifier` (compares normalized React vs Slint traces with `CoverageDisclosure`)
//! - `VisualParityVerifier` (evaluates geometry, spacing, and pixel delta tolerances)

pub mod behavioral_verifier;
pub mod build_verifier;
pub mod native_compliance;
pub mod visual_verifier;

pub use behavioral_verifier::{BehavioralDifferentialVerifier, ScenarioTrace};
pub use build_verifier::BuildVerifier;
pub use native_compliance::NativeComplianceVerifier;
pub use visual_verifier::{VisualParityVerifier, VisualSnapshot};
