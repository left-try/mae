# Open Prompt Format Installation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Install the pinned OPF CLI and validate the project's existing prompt registry and prompt files.

**Architecture:** Install the Python package with uv's user-level tool environment. Validate the existing OPF registry and prompts in place, retaining prompt IDs and source ownership.

**Tech Stack:** uv, Python 3.12, Open Prompt Format CLI 0.2.0.

**Spec:** `docs/superpowers/specs/2026-10-17-agent-tooling-integration-design.md`

## Global Constraints

- Install the Python CLI `open-prompt-format==0.2.0` using `uv tool`.
- Run `opf check .` from the target folder against the existing `opf.yaml` and prompts.
- Keep prompt IDs and prompt sources in place; make only compatibility fixes required by the checker and preserve the current project verification instructions.
- Record that OPF is experimental and keep the installed version pinned.
- The parent Git checkout has no commits and currently contains the documentation suite as untracked files. Work must preserve that state and proceed in place.
- No application tests; use OPF validation only.

## Review Focus

- A different `opf` executable shadows the installed version: inspect `command -v opf` and `uv tool list` before and after install; this CLI does not implement `opf --version`, so use uv's version report.
- The registry has invalid prompt references: run `opf check .` and correct only reported source paths while preserving IDs.
- A prompt uses unsupported syntax: run the checker and preserve its intent, changing only syntax needed for valid `opf/0.2` rendering.
- Prompt IDs or source digest identity changes accidentally: compare frontmatter IDs before and after; all five existing IDs must remain unchanged.
- A checker result is heuristic or does not prove prompt behavior: report only structural validation; do not call models, run paid evals, or claim semantic correctness.

---

### Task 1: Install and validate the existing OPF collection

**Files:**
- Modify: existing prompt files under `universal-migration-engine-docs/prompts/` only if OPF reports a concrete compatibility error
- Read: `universal-migration-engine-docs/opf.yaml`

**Interfaces:**
- Consumes: five existing OPF prompts and the registry mappings in `opf.yaml`.
- Produces: user-level `opf` CLI pinned at version `0.2.0` and a successful structural check of the existing prompt collection.

- [x] **Step 1: Inspect the current CLI before installing**

Run: `command -v opf` and `uv tool list` (if available).

Expected: record the current executable and version; do not remove unrelated tools.

- [x] **Step 2: Install pinned OPF CLI**

Run: `uv tool install open-prompt-format==0.2.0`

Expected: uv installs the package in its isolated user tool environment and `uv tool list` reports `open-prompt-format v0.2.0`.

- [x] **Step 3: Check the current prompt collection**

From `universal-migration-engine-docs/`, run: `opf check .`

Expected: the checker reports zero errors. Record safety warnings as review findings; do not change prompt intent solely to suppress heuristics.

- [x] **Step 4: Repair only concrete compatibility findings if needed**

For any checker finding, preserve the existing prompt ID, migration behavior requirements, and independent-verification wording. Change only the syntax or registry reference named by the finding, then rerun `opf check .`.

Expected: the checker passes; if a finding cannot be resolved without changing intended behavior, stop and report the exact blocker rather than rewriting prompt semantics.

Actual: zero errors; six safety warnings retained as advisory review findings. No prompt changes were needed.

**Commit:** Not applicable; the parent checkout has no commits and is preserving an existing untracked documentation suite.
