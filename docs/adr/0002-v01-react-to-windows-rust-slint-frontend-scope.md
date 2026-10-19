# ADR 0002: Version 0.1 Scope — Frontend React to Native Windows Rust + Slint

- **Status:** Accepted
- **Date:** 2026-10-19

## Context

The long-term vision of MAE is universal software migration across frontend, backend, games, data pipelines, and systems software. Attempting all domains simultaneously risks building an overly abstract, unverified universal AST.

For **Version 0.1**, the user specified narrowing scope strictly to **frontend migration: React/TypeScript web application → native Rust + Slint desktop application for Windows** (with cross-platform headless testability so Linux/Windows development and CI both run deterministically).

## Decision

1. **Frontend-Only Semantic IR (`SemanticUiModel`):**
   - Version 0.1 extracts and models frontend semantics: `ComponentGraph`, `StateModel`, `EffectModel`, `InteractionModel`, `NavigationModel`, `LayoutModel`, `StyleTokenSet`, `AssetInventory`, and `CapabilityReport` with Sanka-inspired `UI_*` gap reason codes.
2. **Windows-Ready Native Rust + Slint Target (`RustSlintTargetAdapter`):**
   - Generated applications configure the Windows GUI subsystem (`#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]`), Windows window sizing/DPI/title metadata, and clean separation between pure Rust state/reducer/effects (`src/app/`) and declarative Slint UI (`ui/app.slint`).
   - Business state and reducer transitions remain testable in headless `cargo test` on both Linux and Windows without requiring a live display server.
3. **Hard Native-Compliance Gate:**
   - Embedding a WebView (`wry`, `tauri`, `webview2`), Electron shell, screenshot-as-UI, or proxying UI logic back to a Node/React runtime is a hard `FAIL` in `NativeComplianceVerifier`.
4. **Multiphase PR Workflow:**
   - Each architectural phase (Phase 1: Domain Core, Phase 2: Application & OPF/OKF/AAMF Integrations, Phase 3: React Source Adapter, Phase 4: Rust+Slint Windows Target Adapter & Verifiers, Phase 5: CLI & End-to-End Reference Migration) is developed via TDD on a dedicated branch, opened as a GitHub Pull Request via `gh pr create`, verified, and automerged via `gh pr merge`.

## Consequences

- Keeps v0.1 focused, verifiable, and honest about capability gaps.
- Proves the end-to-end architecture (Contracts -> Source Artifacts -> PlanHash -> Target Generation -> Independent Evidence -> Acceptance/Repair/Resume) on a non-trivial React frontend fixture.
