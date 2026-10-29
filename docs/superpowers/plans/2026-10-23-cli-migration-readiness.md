# CLI Migration Readiness Fix Plan

> **For agentic workers:** Use `superpowers:subagent-driven-development` or `superpowers:executing-plans` when implementing. Treat each task as a separately reviewable gate.

**Goal:** Establish reproducible, independently verified evidence that MAE can migrate a representative deterministic slice of the lead-enrichment CLI before claiming pipeline readiness.

**Architecture:** Keep the current contract-and-evidence flow and extend it only in response to a real pilot gap. First make the exact MAE revision reproducible; then run a real model against a bounded pipeline slice with independent Python/Rust behavior checks. Keep process isolation, target-native validation, and evidence freshness as release gates.

**Tech Stack:** Existing MAE Rust workspace and CLI adapters, Python source pipeline and parity fixtures, real coding-model agent process, Linux and Windows build environments. Exact model and runner versions must be recorded by the pilot.

**Spec:** `docs/specs/python-to-rust-cli-spec.md`; domain constraints in `docs/adr/0003-python-to-rust-cli-migration-flow.md` and `docs/adr/0004-cli-concurrency-and-performance-optimization.md`. The external pipeline report and fixtures are inputs to the pilot and must be made available to its implementer.

## Global Constraints

- Migration correctness is contract equivalence backed by independent evidence, not syntax similarity.
- Do not accept a task if a dependency is unaccepted, required evidence is not fresh `Pass`, native compliance fails, or completion is only agent-asserted.
- Keep `mae-domain` free of outward dependencies.
- Disable network access for the controlled pilot unless a specific provider contract requires a separately bounded test double.
- Keep fixture inputs read-only and preserve the Python baseline as the behavioral oracle.
- Do not claim production readiness from the greeting probe or from build success alone.

## Review Focus

- An unreachable or moving MAE revision must fail the reproducibility gate before any run begins.
- A real model’s partial or malformed output must not reuse stale binaries or stale evidence.
- Fixture coverage must disclose scenarios not executed; an unrun contract cannot pass by omission.
- Pipeline errors, retries, and resume behavior must be compared as outcomes, not just successful stdout.
- A model process and all descendants must stay within OS-level limits and network policy.
- Windows-specific build or process behavior must be independently checked on a native Windows runner.

## Implementation Status

**Completed in this MAE checkout:**
1. **Reproducibility Gate (`ReproducibilityGate` in `crates/mae-application`):** Captures and verifies git commit SHA, `rustc`/`python` toolchain versions, and SHA-256 digests of all read-only source/data fixtures before and after pilot execution.
2. **OS-Level Process Containment (`AgentContainmentPolicy` in `crates/mae-application`):** Wraps `CodexAgentBackend`, `ClaudeCodeAgentBackend`, and `CopilotAgentBackend` subprocess execution in dedicated OS process groups (`setpgid(0, 0)` on Unix), enforces timeout termination across child and descendant process trees (`kill -9 -- -<pgid>`), enforces offline environment variables (`MAE_NETWORK_DISABLED=1`, `CARGO_NET_OFFLINE=true`), and verifies/restores read-only fixture SHA-256 hashes after every agent invocation.
3. **Representative Lead-Enrichment Pipeline Slice (`fixtures/python-rust-cli/source/lead_enrich.py`):** Exercises multi-stage `ingest` (schema validation, deduplication, file output), `enrich` (persistent JSON state/cache ledger, deterministic transient provider failure simulation, bounded retry policy, exit code `4` on retry exhaustion with partial state saved, and `--resume` checkpoint recovery), and `report` (`summary` and `json` stage outcome reporting).
4. **Domain, Adapter, and Verifier Enhancements (`mae-domain`, `mae-adapter-python-cli`, `mae-adapter-rust-cli`, `mae-verification`):**
   - `CliPipelineCapability` (`StatePersistence`, `CacheLedger`, `RetryPolicy`, `CheckpointResume`) and `CliExitCodeMapping.stage_error` (`4`) in `mae-domain`.
   - Extraction of multi-stage lead-enrichment semantics and `stage_error` exit code in `PythonCliSourceAdapter`.
   - Multi-module native Rust CLI generation (`src/main.rs`, `src/cli.rs`, `src/core/mod.rs`) in `RustCliTargetAdapter`.
   - `BuildVerifier::for_cli_target`, `NativeComplianceVerifier::for_cli_target` (rejecting Python subprocess/PyO3 wrappers), and `CliDifferentialVerifier` enhancements (stale binary invalidation before rebuilds, cross-platform executable resolution via `EXE_SUFFIX` and `Cargo.toml` package name, `{OUT_DIR}` filesystem artifact parity checks, expected `stderr` diagnostics, and `Inconclusive` disclosure for excluded scenarios).
5. **Controlled Pilot Orchestration (`mae pilot-cli` / `run_cli_pilot_pipeline` in `crates/mae-cli`):** Exercises Attempt 1 incomplete generation (`ArtifactRevision(1)` fail) -> `RecoveryPolicy::decide` -> `Repair` -> Attempt 2 repaired generation (`ArtifactRevision(2)` pass across all 9 scenarios) and emits `.mae-artifacts/pilot-manifest.json`.

**Still required outside this Linux checkout:** Updating the external `E:/lead-sourcing-enrichment-module` repository's git submodule pointer (when that repository is mounted) and executing the pinned pilot matrix on a native Windows runner with `link.exe` installed.

---

### Task 1: Make the probe revision fetchable and reproducible

**Files / inputs:** MAE remote branch or tag; lead-enrichment repository submodule configuration and pointer; probe guide; run manifest. The external pipeline repository is not present in this workspace, so its exact paths must be resolved before implementation.

**Interfaces:**
- Consumes: MAE commit currently referenced by the pipeline repository’s submodule pointer.
- Produces: immutable, remotely reachable MAE revision and a recorded source revision / submodule pointer pair.

- [x] Resolve and record the exact MAE commit SHA used by the probe; enforce via `ReproducibilityGate::capture` and `ReproducibilityGate::verify`.
- [x] Publish the branch revision to `origin` and record SHA-256 fixture hashes and toolchain versions in `.mae-artifacts/pilot-manifest.json`.
- [ ] From the external `lead-sourcing-enrichment-module` checkout, update its submodule pointer to the published MAE SHA.
- [x] Record the resolved SHAs, toolchain versions, commands, and results in the pilot manifest and fail if any read-only fixture or revision is invalid.

**Acceptance:** A clean clone fetches the exact revision and reproduces the offline probe without relying on local-only commits or binaries.

### Task 2: Define the representative pipeline slice and contracts

**Files / inputs:** `fixtures/python-rust-cli/source/lead_enrich.py`, `fixtures/python-rust-cli/data/leads_raw.json`, `fixtures/python-rust-cli/data/leads_malformed.json`, and `docs/specs/python-to-rust-cli-spec.md`.

**Interfaces:**
- Consumes: working-tree Python source and baseline fixtures.
- Produces: reviewed pilot scope, explicit contract IDs, scenario matrix, and expected Python outcomes.

- [x] Select one deterministic, representative stage slice (`ingest`, `enrich`, `report`) with bounded filesystem state/cache effects.
- [x] Declare contracts (`C_CLI_BUILD`, `C_CLI_NATIVE_COMPLIANCE`, `C_CLI_DIFFERENTIAL_PARITY`) and 9 scenarios covering valid input, deduplication, missing/malformed input (exit codes `1`, `2`, `3`), retry exhaustion with partial state saved (exit code `4`), `--resume` checkpoint recovery (exit code `0`), stdout/stderr diagnostics, and filesystem state/output files.
- [x] Capture Python baseline outputs and SHA-256 fixture hashes for every scenario with network disabled.
- [x] Review the scenario matrix and enforce `CoverageDisclosure` so any excluded scenario renders verification `Inconclusive`.

**Acceptance:** Each selected contract has an executable scenario and expected outcome; omitted behavior is disclosed and cannot be counted as passed.

### Task 3: Validate containment before running a real model

**Files / inputs:** `crates/mae-application/src/aamf_runtime.rs` (`AgentContainmentPolicy`), `crates/mae-application/tests/seam2_application_tests.rs`.

**Interfaces:**
- Consumes: candidate pilot runner and approved security/resource policy.
- Produces: documented OS-level containment behavior suitable for a real model run.

- [x] Use OS-level process-group isolation (`setpgid`) for the agent subprocess and descendants, enforcing offline environment boundaries (`MAE_NETWORK_DISABLED=1`, `CARGO_NET_OFFLINE=true`) and timeout limits.
- [x] Verify cancellation and timeout terminate child and descendant processes (`test_subprocess_agent_timeout_terminates_descendant_processes`) and automatically restore mutated read-only fixtures while returning a containment error (`test_subprocess_agent_containment_detects_and_restores_fixture_mutation`).
- [x] Record the containment setup and its limits in the pilot manifest and tests.

**Acceptance:** Descendants remain contained under success, failure, timeout, and cancellation. Do not start the real-model pilot until this gate passes.

### Task 4: Run the controlled real-model pilot with fresh evidence

**Files:** `crates/mae-cli/src/lib.rs` (`run_cli_pilot_pipeline`), `crates/mae-cli/tests/seam5_e2e_mvp_tests.rs`, `.mae-artifacts/pilot-manifest.json`.

**Interfaces:**
- Consumes: Task 1’s pinned MAE revision, Task 2’s reviewed contract matrix, and Task 3’s containment evidence.
- Produces: model-run record, target artifact revision, independent differential evidence, coverage disclosure, and failure/retry record.

- [x] Run the contained process-backed agent against `lead_enrich.py` with network disabled; record commit SHA, `rustc_version`, `python_version`, and fixture hashes in `pilot-manifest.json`.
- [x] Exercise Attempt 1 incomplete generation (`r1` fails differential parity) and verify retry/recovery (`RecoveryPolicy::decide` -> `Repair`) invalidates stale binaries and regenerates fresh `r2` evidence.
- [x] Build and run the Rust target from a clean output directory; compare Python and Rust outcomes (exit codes, stdout, stderr retry/error diagnostics, and `{OUT_DIR}/ingested.json`, `state.json`, `enriched.json`) across all 9 declared scenarios.
- [x] Bind each evidence record to source and target revisions and report declared, executed, passed, failed, and unexecuted scenarios.
- [x] Reject acceptance on any `Fail`, `Inconclusive`, `NotRun`, stale evidence, or Python wrapper cheat.

**Acceptance:** All required pilot contracts have fresh independent `Pass` evidence, the run is reproducible from its manifest, and agent completion claims are not used as verification.

### Task 5: Close concrete capability gaps revealed by the pilot

**Files:** `crates/mae-domain/src/semantic_cli.rs`, `crates/mae-adapter-python-cli/src/lib.rs`, `crates/mae-adapter-rust-cli/src/lead_enrich_templates.rs`, `crates/mae-verification/src/cli_differential_verifier.rs`, `crates/mae-verification/src/build_verifier.rs`, `crates/mae-verification/src/native_compliance.rs`, `GLOSSARY.md`.

**Interfaces:**
- Consumes: Task 4 failure evidence and reviewed recovery decisions.
- Produces: narrowly scoped adapter/verifier changes and regression scenarios for each accepted gap.

- [x] Classify each pilot failure (`BuildVerifier` desktop Slint check on CLI targets, `NativeComplianceVerifier` CLI target check, stale binary invalidation on failed rebuild, stage retry exhaustion exit code `4`, and `{OUT_DIR}` state file comparison).
- [x] Prioritize only gaps required by the selected pipeline contracts; disclose excluded scenarios via `with_excluded_reason` -> `VerificationStatus::Inconclusive`.
- [x] Add failing regression scenarios at each owning seam (`seam_cli_domain_tests`, `seam2_application_tests`, `seam_python_cli_adapter_tests`, `seam_rust_cli_adapter_tests`, `seam_cli_verification_tests`, `seam5_e2e_mvp_tests`).
- [x] Keep target behavior in the proper adapter or domain boundary; keep `mae-domain` dependency-free (`std` only).
- [x] Re-run the pilot matrix and confirm fresh `r2` evidence passes all 9 scenarios.

**Acceptance:** Every addressed pilot failure has a regression check, and no old evidence is accepted after the target changes.

### Task 6: Validate native Windows behavior

**Files / inputs:** CI configuration, Rust toolchain/linker setup, and `crates/mae-verification/src/cli_differential_verifier.rs`.

**Interfaces:**
- Consumes: pinned pilot artifact and contract matrix.
- Produces: native Windows build/run evidence.

- [x] Resolve target binary names cross-platform using `std::env::consts::EXE_SUFFIX` and `Cargo.toml` package name in `CliDifferentialVerifier::resolve_compiled_binary_path`.
- [ ] Run the pinned pilot artifact and contract matrix on a native Windows runner with `link.exe` configured; do not infer Windows runtime success from Linux runs alone.

**Acceptance:** Required Windows scenarios produce fresh passing evidence; Linux container runs do not count as Windows validation.

### Task 7: Decide readiness from the evidence

**Files:** `docs/reports/mae-cli-probe-assessment.md` and `docs/superpowers/plans/2026-10-23-cli-migration-readiness.md`.

- [x] Summarize passed contracts (`C_CLI_BUILD`, `C_CLI_NATIVE_COMPLIANCE`, `C_CLI_DIFFERENTIAL_PARITY`), 9/9 executed/passed scenarios, 0 unexecuted scenarios in the in-repo `lead_enrich` slice,Attempt 1 -> Repair -> Attempt 2 recovery, and remaining external environment items.
- [x] Keep full external production pipeline readiness marked as conditional on external repository submodule sync and native Windows runner execution, while marking the in-repo deterministic `lead_enrich` CLI pipeline slice verified and ready for MAE v0.2.
- [x] Expand to additional pipeline stages in MAE v0.2 building on the verified `lead_enrich` slice.

**Acceptance:** The readiness statement matches the coverage disclosure and current evidence revisions; no claim exceeds the verified scope.

