# MAE Ubiquitous Language & Domain Glossary

> Maintained in accordance with `domain-modeling` and `codebase-design` skills (`mattpocock/skills`). Use these exact terms across code, tests, specs, ADRs, OKF knowledge, and OPF prompts.

## Core Architectural Vocabulary (`codebase-design`)

- **Module**: Anything with an interface and an implementation (a function, struct/trait pair, or workspace crate).
- **Interface**: Everything a caller must know to use a module correctly: type signature, domain invariants, state ordering constraints, error modes, and performance guarantees.
- **Implementation**: The internal body of code behind an interface.
- **Depth**: Leverage at the interface—the amount of domain or migration behavior a caller (or test) exercises per unit of interface complexity. Prefer **deep modules** over shallow pass-through wrappers.
- **Seam**: A boundary (`trait` / port in `mae-domain`) where behavior can be altered or verified without editing the caller. All TDD tests target pre-agreed seams.
- **Adapter**: A concrete struct satisfying a Domain Port at a seam (e.g., `ReactSourceAdapter`, `RustSlintTargetAdapter`, `OpfPromptProvider`, `InMemoryArtifactRepository`).

---

## Migration Domain Core Vocabulary (`mae-domain`)

- **Migration (`MigrationAggregate`)**: The aggregate root governing a single source-to-target software migration run, owning its required `Contract` set, `CapabilityReport`, reviewed `PlanHash`, `MigrationTask` DAG, and `EvidenceRecord` ledger.
- **Contract**: An explicitly declared, observable requirement (`C_required`) that the migrated target must satisfy (defined by `ContractId`, `VerificationCategory`, `ContractSeverity`, preconditions, stimulus/action, expected observable outcomes, and optional quantitative tolerance). Correctness in MAE means **contract equivalence**, never syntax similarity.
- **Capability Assessment (`CapabilityAssessment`)**: Contextual classification (`Supported`, `Partial`, `Unsupported`, `Unknown`) of a source requirement against target platform capabilities, paired with a selected `MigrationStrategy`, risk level, provenance, and optional `GapReasonCode`.
- **Gap Reason Code (`GapReasonCode`)**: Sanka-inspired stable identifier classifying why a source UI construct cannot be trivially mapped (`UI_BROWSER_ONLY_API`, `UI_UNSUPPORTED_CSS`, `UI_DYNAMIC_STYLE`, `UI_THIRD_PARTY_WIDGET`, `UI_CANVAS`, `UI_COMPLEX_ANIMATION`, `UI_UNRESOLVED_STATE_LIBRARY`, `UI_UNRESOLVED_ROUTING`, `UI_UNRESOLVED_EFFECT`, `UI_MANUAL_ASSET`, `UI_ACCESSIBILITY_GAP`).
- **Migration Strategy (`MigrationStrategy`)**: The approved approach for a capability (`DirectNativeMapping`, `SemanticReconstruction`, `ReplacementComponent`, `CustomImplementation`, `CompatibilityWrapper`, `HybridBridge`, `ManualAdaptation`, `Unsupported`).
- **Versioned Artifact (`VersionedArtifact`)**: An immutable, content-addressed (`ContentHash`), revision-tagged (`ArtifactRevision`) piece of migration evidence or intermediate representation with explicit provenance.
- **Semantic UI Model (`SemanticUiModel`)**: Composable frontend domain IR combining `ComponentGraph`, `StateModel`, `EffectModel`, `InteractionModel`, `NavigationModel`, `LayoutModel`, `StyleTokenSet`, and `AssetInventory` without inventing a universal programming language.
- **Plan Hash (`PlanHash`)**: Deterministic cryptographic/canonical digest of `(source_fingerprint + contract_set + capability_report + migration_plan + adapter_versions)`. Any semantic change invalidates the reviewed plan.
- **Migration Task (`MigrationTask`)**: A bounded outcome-oriented unit of migration work in the DAG with lifecycle states: `Pending -> Ready -> Running -> Verifying -> Accepted`, plus recovery states `NeedsRepair -> Ready`, `Blocked`, `Failed`, and `Cancelled`.
- **Evidence Record (`EvidenceRecord`)**: Independent verification output bound to a specific `ContractId`, verifier ID, source `ArtifactRevision`, and target `ArtifactRevision`, with status `Pass`, `Fail`, `Inconclusive`, or `NotRun`. Target edits increment the target `ArtifactRevision` and automatically render prior evidence **stale**.
- **Coverage Disclosure (`CoverageDisclosure`)**: Sanka-inspired accounting of `declared_scenarios`, `executed`, `passed`, and `unexecuted` scenario IDs so partial test runs never masquerade as full behavioral parity.
- **Acceptance Policy (`AcceptancePolicy`)**: Pure domain specification enforcing that a `MigrationTask` becomes `Accepted` if and only if all dependencies are `Accepted`, all required contracts have fresh `Pass` evidence on the current target revision, native compliance passes, and no critical blockers remain.
- **Completion Policy & Health Score (`CompletionPolicy`, `HealthScorePolicy`)**: Crane-inspired gate ensuring overall migration completion requires current-revision evidence across all critical contracts, with `health = progress * correctness_gate` (`correctness_gate = 0` whenever any critical invariant fails).
- **Recovery Decision (`RecoveryDecision`)**: Deterministic domain policy output (`Repair`, `Replan`, `Escalate`, `Block`) when verification fails.

---

## v0.1 Frontend Target Vocabulary (`React -> Windows Rust + Slint`)

- **Windows Slint Target (`WindowsDesktop`)**: A genuine native desktop application written in Rust + Slint configured for Windows desktop execution (`#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]`, native window geometry/DPI/theming) while supporting headless verification on Linux/Windows CI.
- **Logic-UI Separation**: Architectural rule in generated targets keeping state, events, reducers, async effects, and navigation in pure Rust (`src/app/`) and declarative view layout, styling, and property/callback bindings in Slint (`ui/*.slint`).
- **Native Compliance Gate (`NativeComplianceVerifier`)**: Independent verifier that rejects fake migrations (WebView wrappers such as `wry`/`tauri`/`webview2`, Electron, screenshot-as-UI, or whole-app proxying to the React runtime).

---

## CLI Target Vocabulary (`Python CLI -> Native Rust CLI`)

- **CLI Semantic Model (`CliSemanticModel`)**: Pure domain IR capturing command hierarchy (`CliCommandNode`), argument definitions (`CliArgument`), I/O streaming contracts (`CliIoContract`), deterministic exit code mappings (`CliExitCodeMapping`), and concurrency profiles (`CliConcurrencyModel`).
- **CLI Concurrency Model (`CliConcurrencyModel`)**: Classification of source concurrency patterns (`Sequential`, `ParallelThreads`, `MultiProcessing`, `AsyncEventLoop`) mapped to zero-cost, zero-serialization native Rust execution.
- **Scoped Data Parallelism**: Rust concurrency pattern using `std::thread::scope` enabling worker threads to borrow immutable slices of records (`&[Record]`) without IPC, pickling, or heap allocation overhead, replacing Python `multiprocessing.Pool`.
- **CLI Pipeline Capability (`CliPipelineCapability`)**: Multi-stage CLI operational invariants (`StatePersistence`, `CacheLedger`, `RetryPolicy`, `CheckpointResume`) extracted from source CLI pipelines and enforced by target generation and differential verification.
- **Logic-CLI Separation**: Architectural rule in generated Rust CLI utilities keeping argument parsing, validation, and flag dispatch in `src/cli.rs` and pure, testable domain business logic in `src/core/`.
- **Differential Behavioral Oracle (`CliDifferentialVerifier`)**: Independent verifier executing identical argument vectors and stdin streams across both the Python source and the compiled Rust binary, asserting exact exit code equality, stdout tree parity, diagnostic stderr behavior, filesystem output/state artifact equality, and performance acceleration.
- **Reproducibility Gate (`ReproducibilityGate`)**: Pre- and post-pilot verification gate recording git commit SHA, `rustc`/`python` toolchain versions, and read-only fixture SHA-256 digests so any silent source mutation fails the migration immediately.
- **Agent Containment Policy (`AgentContainmentPolicy`)**: OS-level subprocess containment boundary (`mae-application`) enforcing dedicated process-group isolation (`setpgid`), descendant process-tree termination on timeout, offline environment isolation (`MAE_NETWORK_DISABLED=1`, `CARGO_NET_OFFLINE=true`), and read-only fixture hash verification/restoration.

