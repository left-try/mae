# Competitor Audit and Reuse Decision

## Decision

Use **AAMF as the runtime base**.

Borrow design patterns from **Sanka** and **Crane**, but do not use either as the central engine for the first implementation.

## AAMF

Repository: https://github.com/jafreck/AAMF  
License: MIT.

### What exists in the real repository

AAMF defines a 9-phase deterministic migration pipeline and uses:

- `@jafreck/lore` for code indexing and symbol/dependency graph construction;
- `@cadre-dev/framework` for flow orchestration;
- `MigrationRuntime`;
- `MigrationFlowContext`;
- `AamfFlowCheckpointAdapter`;
- `migrationFlow`;
- a task graph builder;
- agent launcher;
- checkpoint manager;
- budget/cost tracking;
- observability;
- Git automation.

Its task planner contracts SCCs, bounds task sizes, and emits dependency-ordered tasks. Phase 4 supports `per-task`, `wave-barrier`, and `sync-epoch` execution.

The repository has substantial tests grouped under `tests/agents`, `budget`, `config`, `core`, `e2e`, `execution`, `flow`, `lore`, `observability`, and more.

### What we reuse

- flow runtime;
- deterministic task graph;
- retries;
- checkpoint/resume;
- task/wave scheduling;
- target change tracking;
- Git commits;
- budgets;
- logs/metrics;
- agent process lifecycle;
- Lore KB + MCP;
- existing scenario registry/output schema discipline.

### Gaps for us

- current public backend choices are Copilot CLI and Claude Code, not Codex;
- task graph is code-symbol centric, not UI-semantic;
- scenario prompt catalog directly loads Markdown templates;
- parity-verifier is still part of the AAMF agent workflow; our acceptance must also consume independent deterministic evidence;
- there is no universal source/target adapter SDK matching our intended architecture.

### Decision

**Fork/base + narrow patches.**

Do not rewrite AAMF orchestration.

---

## Sanka

Repository: https://github.com/sankaHQ/sanka  
Licensing is split: Apache-2.0 tooling/extension SDK areas and AGPL-3.0 migration engine areas.

### Strong ideas

Sanka's local migration lifecycle is:

```text
scan → plan → apply → test → verify
```

Useful properties:

- immutable/reviewed plan hash;
- machine-readable CLI;
- extension isolation;
- local execution;
- independent verify phase;
- explicit disclosure of unverified coverage;
- extension gap reporting.

### Particularly relevant experimental work

Sanka has an experimental TypeScript → Rust extension with:

- static TypeScript capture;
- deterministic target generation;
- plan hash;
- cargo tests;
- source-vs-target replay verification;
- explicit unsupported/gap reporting.

Its experimental React Native → native work captures:

- screens;
- navigation;
- styles;
- local state;
- data loading;
- forms;
- structural tree representations;
- stable manual-adaptation reason codes.

That is highly relevant to our React adapter even though the target is Slint rather than SwiftUI/Compose.

### What we borrow

- capture/gap model;
- stable reason codes;
- reviewed plan identity;
- “apply exactly what was planned” principle;
- differential source/target verification;
- native-target compliance gate;
- disclosure of unverified surfaces.

### Why not use as the base

- mature public paths remain bounded recipes;
- relevant UI and TypeScript→Rust paths are experimental;
- licensing of the engine is less convenient than AAMF MIT;
- our desired orchestration and artifact domain model are already closer to AAMF.

### Decision

**Reference + selective design reuse.**

---

## Crane

Repository: https://github.com/githubnext/crane

### Strong ideas

Crane defines each migration in Markdown/issue form with:

- source;
- target;
- strategy;
- verification command;
- health score;
- deterministic completion gate.

Each iteration must not regress the health score. Final completion requires evidence on the current PR head.

State is human-readable on a `memory/crane` branch.

### What we borrow

- migration definition ergonomics;
- monotonic/non-regressing health concept;
- deterministic completion gate;
- current-head/current-artifact evidence requirement;
- parity corpus idea;
- in-place vs greenfield strategy vocabulary.

### Why not use as the base

Crane is tightly coupled to:

- GitHub Agentic Workflows;
- GitHub scheduling;
- GitHub repository memory;
- GitHub Copilot.

Our runtime needs first-class local execution and provider independence.

### Decision

**Reference architecture only.**

---

## Combined architecture

```text
AAMF
  = runtime, DAG, resume, scheduling, agent orchestration

Sanka ideas
  = capture, immutable plan identity, gap codes, differential verify

Crane ideas
  = acceptance health, no-regression gate, deterministic completion evidence

Our engine
  = contracts, artifacts, adapters, capability semantics, acceptance/completion policy

OKF
  = repo knowledge

OPF
  = prompt source/versioning/evals
```
