# Change Knowledge

<!-- okf:generated:start -->
<!-- okf:generated:end -->

## Curated guidance

Record architecture-significant changes here or in linked change notes.

Each AAMF upstream patch should document:

- upstream file/module;
- reason;
- extension seam introduced;
- compatibility tests;
- whether the change could be upstreamed or removed later.

## MAE v0.1 (`React -> Native Windows Rust + Slint`) Changelog

- **Phase 0 (`3c4aa65`)**: Initialized repository, Ubiquitous Language (`GLOSSARY.md`), `AGENTS.md`, ADRs (`0001`, `0002`), OPF `0.2` prompt catalog (`opf.yaml`, `prompts/*.md`), OKF repository brain (`.okf/`), Matt Pocock skills, and Ponytail skills.
- **Phase 1 — PR #1 (`341a9fa`)**: Implemented pure DDD domain core (`crates/mae-domain`) with zero outward dependencies (`MigrationAggregate`, `SemanticUiModel`, `AcceptancePolicy`, `CompletionPolicy`, `HealthScorePolicy`, `RecoveryPolicy`, and Clean Architecture port traits; 27 Seam 1 TDD tests).
- **Phase 2 — PR #2 (`dbd4283`)**: Implemented Application & Integrations layer (`crates/mae-application`) including `OpfPromptProvider`, `OkfKnowledgeRepository`, `WaveSchedule`, `CheckpointManager`, `AgentBackendPort` adapters, and `MigrationOrchestrator` (14 Seam 2 TDD tests).
- **Phase 3 — PR #3 (`0ebc7b5`)**: Implemented `ReactSourceAdapter` (`crates/mae-adapter-react`) for Stage A–D discovery, TSX/CSS static semantic extraction, Sanka gap classification, and Playwright runtime trace baseline merging (18 Seam 3 TDD tests).
- **Phase 4 — PR #4 (`85da91c`)**: Implemented `RustSlintTargetAdapter` (`crates/mae-adapter-rust-slint`) with Windows GUI subsystem support (`windows_subsystem = "windows"`) and independent verifiers (`crates/mae-verification`: `NativeComplianceVerifier`, `BuildVerifier`, `BehavioralDifferentialVerifier`, `VisualParityVerifier`; 13 Seam 4 TDD tests).
- **Phase 5 — PR #5**: Implemented `crates/mae-cli` (`mae` binary + E2E pipeline) and multi-route React/TypeScript reference fixture (`fixtures/react-rust-slint/source`) with Seam 5 E2E verification across all 9 MVP acceptance criteria (76 total workspace tests).
