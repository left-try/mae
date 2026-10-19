# AAMF Extension Plan

## Goal

Use AAMF as the base while preserving the ability to pull upstream changes.

The safest strategy is:

> **fork AAMF, preserve its existing directory structure, add new modules alongside it, and make only narrow compatibility patches to upstream-owned code.**

## 1. Upstream-owned zones

Treat these as upstream-first:

```text
src/core/
src/flow/
src/execution/
src/budget/
src/logging/
src/observability/
```

Do not reorganize them in the first milestone.

## 2. New zones

Add:

```text
src/domain/
src/artifacts/
src/adapters/
src/verification/
src/integrations/
```

Suggested:

```text
src/domain/
  migration.ts
  contract.ts
  capability.ts
  artifact.ts
  verification.ts
  acceptance-policy.ts
  completion-policy.ts
  recovery-policy.ts

src/adapters/
  source/react/
  target/rust-slint/

src/verification/
  build/
  behavior/
  visual/
  native-compliance/
  performance/

src/integrations/
  aamf/
  okf/
  opf/
```

## 3. Required narrow AAMF patches

### Patch A — AgentBackend seam

Current AAMF publicly supports Copilot CLI and Claude Code.

Target:

```text
AgentBackend
  id
  buildInvocation(...)
  parseResult(...)
  parseUsage(...)
  buildMcpConfig(...)
```

Implement Codex as a third backend without placing Codex conditions throughout the runtime.

### Patch B — PromptProvider seam

AAMF's `ScenarioPromptCatalog` currently reads bundled Markdown template files and compiles them into immutable prompts.

Refactor boundary:

```text
ScenarioPromptProvider
  load(agentName, vars) -> ScenarioPromptSource
```

Implement:

- `LegacyTemplatePromptProvider`;
- `OpfPromptProvider`.

AAMF continues to append the canonical scenario contract and canonical output JSON schema.

### Patch C — Analysis artifact hook

After Lore indexing and before migration planning, allow registered analyzers to emit additional artifacts.

For React:

```text
ReactStaticAnalyzer
RuntimeCaptureAnalyzer
UiSemanticAssembler
```

Do not replace Lore task derivation at first. Enrich task planning with extra artifacts.

### Patch D — External verifier hook

Allow task/wave verification to call deterministic verifier plugins.

Example:

```text
VerifierPlugin
  verify(task, targetRevision, contractSet) -> VerificationEvidence[]
```

AAMF agent parity verification may remain as an additional signal but cannot be the only acceptance evidence.

### Patch E — Domain decision bridge

Before marking a task/domain milestone accepted:

```text
DomainAcceptance.evaluate(...)
```

AAMF controls operational completion.
The domain core controls semantic acceptance.

## 4. Do not patch initially

Avoid changing:

- checkpoint file semantics;
- FlowRunner lifecycle;
- retry executor;
- budget tracking;
- metrics JSONL;
- Git commit automation;
- run locking;
- task queue implementation.

If one of these becomes incompatible, first write a failing integration test showing why.

## 5. Prompt migration plan

AAMF has a scenario catalog including:

- knowledge-builder;
- migration-planner;
- adjudicator;
- code-migrator;
- parity-verifier;
- test-writer;
- parity-failure-resolver;
- final-parity-checker;
- e2e-test-crafter;
- documentation-writer;
- idiomatic-reviewer;
- idiomatic-planner;
- idiomatic-refactorer.

Migrate content incrementally to OPF while preserving scenario IDs.

Phase 1:

```text
code-migrator
parity-verifier
migration-planner
```

Phase 2: remaining scenarios.

Do not change all prompt behavior simultaneously with the runtime fork.

## 6. Checkpoint compatibility

AAMF checkpoints are valuable and should remain the primary runtime-resume state.

Our domain artifacts must be independently recoverable from content-addressed files.

Checkpoint stores references to artifact IDs/hashes, not duplicate copies of all semantic content.

## 7. AAMF KB + OKF

Keep both:

- Lore/AAMF KB: machine source graph for a migration run;
- OKF: curated project/repository knowledge.

Potential MCP configuration:

```text
servers:
  lore:
    purpose: source code graph
  okf:
    purpose: repository intent/architecture/history
  slint:
    purpose: target UI inspection/testing
```

Do not require OKF for arbitrary third-party source repos.

## 8. Testing fork compatibility

Create a dedicated upstream compatibility suite.

Minimum tests:

- existing AAMF tests still pass;
- legacy bundled prompt provider behaves identically;
- Codex backend can be selected without affecting Claude/Copilot;
- OPF provider produces the same AAMF canonical output contract;
- checkpoint resume works with domain artifacts;
- external verifier failure blocks domain acceptance;
- source analyzer failure is recoverable and does not corrupt Lore state.

## 9. Upstream sync procedure

For every upstream merge:

1. merge/rebase upstream AAMF;
2. run AAMF original test suite;
3. run extension compatibility tests;
4. run React→Rust/Slint fixture;
5. run `okf-agent update`;
6. run `okf-agent validate`;
7. record meaningful architecture changes in `.okf/changes/`.
