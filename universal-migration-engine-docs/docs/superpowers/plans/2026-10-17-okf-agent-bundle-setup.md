# OKF Agent Bundle Setup Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Install the pinned OKF Agent CLI, create a local index for the existing `.okf/` bundle without touching the parent Git repository, and document the project workflow.

**Architecture:** Use the standalone, read-only `okf-agent bundle` profile with explicit source and generated-index roots. Keep authored knowledge in `.okf/`, local generated state in ignored `.okf-index/`, and record exact setup commands in the documentation suite.

**Tech Stack:** pipx, OKF Agent at commit `ed3a192d0e44437a446d51d02781a3fa9a5cda93`, Markdown.

**Spec:** `docs/superpowers/specs/2026-10-17-agent-tooling-integration-design.md`

## Global Constraints

- Install `okf-agent` with `pipx` from the user-provided repository, pinned to commit `ed3a192d0e44437a446d51d02781a3fa9a5cda93`.
- Run only `okf-agent bundle` commands, always with `--root .okf --index-root .okf-index`, from `universal-migration-engine-docs/`.
- Do not run repository-profile `init`, `index`, `update`, `validate`, or `run`: Git-root discovery would target the parent workspace.
- Do not install Git hooks, hand-edit generated `.okf-index/`, or alter authored `.okf/` content.
- Add `.okf-index/` to the target folder's active `.gitignore`.
- Update `AGENTS.md` to use bundle commands, add `docs/TOOLING_SETUP.md`, and link it from README's Start here list.
- The parent Git checkout has no commits and contains the suite as untracked files. Preserve this state and proceed in place.
- Do not add or run application tests; use the requested OKF CLI validation and ignore-rule check.

## Review Focus

- A global `okf-agent` executable already exists: inspect `command -v okf-agent` and `okf-agent --help`; replace it only if it is not the pinned version/source.
- The normal CLI commands may discover the parent Git root: keep every command in the `bundle` namespace with both explicit roots.
- The bundle has unsupported frontmatter or structure: run `bundle validate`; repair authored content only for concrete findings and preserve knowledge meaning.
- Generated index is accidentally tracked: verify `git check-ignore .okf-index/` resolves to the target folder's `.gitignore`.
- The initial index may be stale after documenting commands: run `bundle update` after content changes, then validate.

---

### Task 1: Install and index the existing bundle

**Files:**
- Generated locally: `universal-migration-engine-docs/.okf-index/` (ignored; do not hand-edit)
- Read: `universal-migration-engine-docs/.okf/`

**Interfaces:**
- Consumes: existing curated OKF Markdown under `.okf/`.
- Produces: pinned user-level `okf-agent` CLI and a rebuildable local index under `.okf-index/`.

- [x] **Step 1: Inspect the current CLI**

Run: `command -v okf-agent` and `okf-agent --help` (if available).

Expected: record whether a current installation exists; do not remove unrelated tools.

- [x] **Step 2: Install the pinned CLI if missing or mismatched**

Run: `pipx install --force "git+https://github.com/left-try/okf-agent.git@ed3a192d0e44437a446d51d02781a3fa9a5cda93"`.

Expected: `okf-agent --help` lists the `bundle` command.

- [x] **Step 3: Add a local ignore rule and build the bundle index**

Create `universal-migration-engine-docs/.gitignore` containing:

```gitignore
.okf-index/
```

From `universal-migration-engine-docs/`, run:

```sh
okf-agent bundle index --root .okf --index-root .okf-index
```

Expected: index output is created under `.okf-index/`; authored `.okf/` remains unchanged.

### Task 2: Wire the bundle workflow into project instructions

**Files:**
- Modify: `universal-migration-engine-docs/AGENTS.md`
- Modify: `universal-migration-engine-docs/README.md`
- Create: `universal-migration-engine-docs/docs/TOOLING_SETUP.md`
- Modify: `universal-migration-engine-docs/.gitignore` if needed

**Interfaces:**
- Consumes: pinned OKF Agent and OPF CLI versions from the approved spec.
- Produces: setup and maintenance instructions that stay scoped to this documentation suite.

- [x] **Step 1: Replace unsafe repository-profile instructions in `AGENTS.md`**

Document the following commands, run from the suite folder:

```sh
okf-agent bundle index --root .okf --index-root .okf-index
okf-agent bundle search "<query>" --root .okf --index-root .okf-index
okf-agent bundle update --root .okf --index-root .okf-index
okf-agent bundle validate --root .okf --index-root .okf-index
```

State briefly that the bundle is the source of truth, `.okf-index/` is generated local state, and repository-profile commands must not be used from this suite because Git-root discovery resolves to its parent. Preserve all unrelated migration and prompt policy.

- [x] **Step 2: Create the tooling setup guide**

Include the pinned install commands:

```sh
pipx install --force "git+https://github.com/left-try/okf-agent.git@ed3a192d0e44437a446d51d02781a3fa9a5cda93"
uv tool install open-prompt-format==0.2.0
```

Document the four OKF bundle commands above, `opf check .`, and the OPF project's experimental status. Explain that generated `.okf-index/` can be rebuilt and is ignored by Git.

- [x] **Step 3: Link the guide from README**

Add `docs/TOOLING_SETUP.md` to the README's Start here list.

Expected: setup guidance is discoverable without altering architecture decisions.

### Task 3: Refresh and validate the final bundle state

**Files:**
- Generated locally: `universal-migration-engine-docs/.okf-index/`
- Read: `universal-migration-engine-docs/.okf/`
- Verify: `universal-migration-engine-docs/.gitignore`, `AGENTS.md`, `README.md`, `docs/TOOLING_SETUP.md`

- [x] **Step 1: Refresh after documentation edits**

From `universal-migration-engine-docs/`, run:

```sh
okf-agent bundle update --root .okf --index-root .okf-index
```

- [x] **Step 2: Validate the source bundle**

Run:

```sh
okf-agent bundle validate --root .okf --index-root .okf-index
```

Expected: validation succeeds. Resolve only concrete structural findings while preserving authored knowledge intent.

- [x] **Step 3: Verify index isolation and intended edits**

Run `git check-ignore .okf-index/` from the suite folder and compare the `.okf/` source tree with its pre-index state.

Expected: `.okf-index/` is ignored, `.okf/` is unchanged, and only the planned project docs/ignore file plus ignored generated index were added or changed.

**Commit:** Not applicable; the parent checkout has no commits and the existing suite remains untracked.
