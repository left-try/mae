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

**Completed in this MAE checkout:** The feature branch commit is present on its matching `origin` ref. TDD coverage now proves that an agent result with `success == false` fails the task before target emission or verification, and that the Codex, Claude Code, and Copilot provider placeholders return an error instead of claiming prompt execution. Full workspace tests and Clippy pass.

**Still required outside this checkout:** The lead-enrichment repository and its parity fixtures are not present here, so its submodule pointer cannot be updated and a representative source slice cannot be selected. No OS-level agent sandbox or real provider process is configured; containment must pass before a model run. The real-model pilot, pilot-driven adapter changes, and native Windows pilot evidence remain open.

---

### Task 1: Make the probe revision fetchable and reproducible

**Files / inputs:** MAE remote branch or tag; lead-enrichment repository submodule configuration and pointer; probe guide; run manifest. The external pipeline repository is not present in this workspace, so its exact paths must be resolved before implementation.

**Interfaces:**
- Consumes: MAE commit currently referenced by the pipeline repository’s submodule pointer.
- Produces: immutable, remotely reachable MAE revision and a recorded source revision / submodule pointer pair.

- [ ] Resolve and record the exact MAE commit SHA used by the probe; confirm it matches the submodule pointer.
- [ ] Publish that revision to an agreed remote ref and update the pipeline repository pointer to the reachable SHA.
- [ ] From a clean clone, initialize the submodule and follow the CLI probe guide through its documented offline run.
- [ ] Record the resolved SHAs, toolchain versions, commands, and results in the task report. Fail this task if a clean clone cannot fetch the pinned MAE revision.

**Acceptance:** A clean clone fetches the exact revision and reproduces the offline probe without relying on local-only commits or binaries.

### Task 2: Define the representative pipeline slice and contracts

**Files / inputs:** Existing Python CLI, deterministic parity fixtures, pipeline task report, and `docs/specs/python-to-rust-cli-spec.md`.

**Interfaces:**
- Consumes: working-tree Python source and baseline fixtures.
- Produces: reviewed pilot scope, explicit contract IDs, scenario matrix, and expected Python outcomes.

- [ ] Select one deterministic, representative stage with limited external side effects; identify its real invocation and required files/configuration.
- [ ] Declare contracts for valid input, boundary input, missing or malformed input, exit codes, stdout/stderr, filesystem effects, and stage result.
- [ ] Capture Python baseline outputs and fixture hashes for every scenario; keep credentials and live provider calls out of the baseline.
- [ ] Review the scenario matrix and explicitly label any behavior excluded from this first slice.

**Acceptance:** Each selected contract has an executable scenario and expected outcome; omitted behavior is disclosed and cannot be counted as passed.

### Task 3: Validate containment before running a real model

**Files / inputs:** Agent process launcher, child-process lifecycle controls, CI configuration, and probe/pilot guide.

**Interfaces:**
- Consumes: candidate pilot runner and approved security/resource policy.
- Produces: documented OS-level containment behavior suitable for a real model run.

- [ ] Use OS-level isolation for the real agent and descendants, enforcing the approved filesystem, network, CPU, memory, and timeout boundaries.
- [ ] Verify cancellation and timeout terminate child processes and leave no process able to mutate fixtures after the run.
- [ ] Record the containment setup and its limits in the pilot guide.

**Acceptance:** Descendants remain contained under success, failure, timeout, and cancellation. Do not start the real-model pilot until this gate passes.

### Task 4: Run the controlled real-model pilot with fresh evidence

**Files:** Pilot run manifest, generated Rust target, verifier configuration, and pilot evidence report in the pipeline migration workspace.

**Interfaces:**
- Consumes: Task 1’s pinned MAE revision, Task 2’s reviewed contract matrix, and Task 3’s containment evidence.
- Produces: model-run record, target artifact revision, independent differential evidence, coverage disclosure, and failure/retry record.

- [ ] Run the configured coding model against the selected slice with network disabled; record model, runner, prompt, and toolchain versions.
- [ ] Exercise at least one failed or incomplete generation and verify retry/recovery does not reuse a stale binary or stale evidence.
- [ ] Build and run the Rust target from a clean output directory; compare Python and Rust outcomes for every declared scenario.
- [ ] Bind each evidence record to source and target revisions and report declared, executed, passed, failed, and unexecuted scenarios.
- [ ] Reject acceptance on any `Fail`, `Inconclusive`, `NotRun`, stale evidence, or unsupported source construct that affects a required contract.

**Acceptance:** All required pilot contracts have fresh independent `Pass` evidence, the run is reproducible from its manifest, and agent completion claims are not used as verification.

### Task 5: Close concrete capability gaps revealed by the pilot

**Files:** Only the MAE domain, Python CLI adapter, Rust CLI adapter, verifier, prompts, and fixtures implicated by Task 4 findings. Update `GLOSSARY.md` / ADRs only if the domain language or decision changes; run `opf check .` after prompt edits.

**Interfaces:**
- Consumes: Task 4 failure evidence and reviewed recovery decisions.
- Produces: narrowly scoped adapter/verifier changes and regression scenarios for each accepted gap.

- [ ] Classify each pilot failure as unsupported source semantics, missing target dependency/module support, generation/recovery failure, or verifier coverage gap.
- [ ] Prioritize only gaps required by the selected pipeline contracts; disclose unsupported behavior rather than silently omitting it.
- [ ] Add a failing regression scenario at the owning seam before implementing each capability.
- [ ] Keep target behavior in the proper adapter or domain boundary; do not add dependencies to `mae-domain`.
- [ ] Re-run the pilot matrix and confirm evidence is regenerated for the new target revision.

**Acceptance:** Every addressed pilot failure has a regression check, and no old evidence is accepted after the target changes.

### Task 6: Validate native Windows behavior

**Files / inputs:** CI configuration, Rust toolchain/linker setup, and probe/pilot guide.

**Interfaces:**
- Consumes: pinned pilot artifact and contract matrix.
- Produces: native Windows build/run evidence.

- [ ] Run the same pinned pilot artifact and contract matrix on a native Windows runner with a configured Rust linker/toolchain.
- [ ] Record platform-specific results and differences; do not infer Windows success from Linux container runs.

**Acceptance:** Required Windows scenarios produce fresh passing evidence; Linux container runs do not count as Windows validation.

### Task 7: Decide readiness from the evidence

**Files:** Updated pilot report and readiness decision; no implementation changes unless a preceding task fails.

- [ ] Summarize passed contracts, unexecuted scenarios, platform coverage, model failures/retries, and remaining gaps.
- [ ] Keep the verdict at “under evaluation” if any required contract lacks fresh passing evidence or any declared critical scenario remains unexecuted.
- [ ] Expand to additional pipeline stages only after this slice passes and its remaining risks are reviewed.

**Acceptance:** The readiness statement matches the coverage disclosure and current evidence revisions; no claim exceeds the verified scope.
