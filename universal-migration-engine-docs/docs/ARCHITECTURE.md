# Architecture

## 1. Objective

Build a framework that can migrate software between technology stacks while preserving explicitly declared observable contracts.

The first path is:

```text
React + TypeScript + browser
            ↓
      source analysis
            ↓
   semantic/artifact layer
            ↓
      AAMF task graph
            ↓
       coding agents
            ↓
    Rust + Slint + Linux
            ↓
 independent verification
```

The architecture must not hard-code React, Rust, Slint, or frontend semantics into the migration domain core.

## 2. Definition of migration success

Let the source program produce a required contract set:

```text
C_required = {c1, c2, ... cn}
```

The target is accepted when there is current evidence that:

```text
Target |= C_required
```

This deliberately avoids claiming whole-program semantic equivalence. The engine guarantees only the contracts it can define and verify.

Examples:

- pressing `Send` appends exactly one message;
- navigation from list → details preserves the selected entity;
- required pixels/layout fall within a visual tolerance;
- target uses Slint rendering rather than a browser;
- startup and memory stay inside declared thresholds.

## 3. System layers

```text
┌──────────────────────────────────────────────────────────┐
│                    Application Layer                     │
│ CLI · API · migration commands · experiment harness     │
└───────────────────────────┬──────────────────────────────┘
                            │
┌───────────────────────────▼──────────────────────────────┐
│                  Migration Domain Core                   │
│ Migration · Contract · Capability · Artifact            │
│ MigrationTask · MigrationPlan · Verification            │
│ AcceptancePolicy · CompletionPolicy · RecoveryDecision  │
└─────────────┬───────────────────────────────┬────────────┘
              │                               │
┌─────────────▼────────────┐      ┌──────────▼─────────────┐
│     Adapter Layer        │      │ Verification Layer    │
│ source / target / IR     │      │ build / behavior /    │
│ platform bindings       │      │ visual / native / perf│
└─────────────┬────────────┘      └──────────┬─────────────┘
              │                               │
┌─────────────▼───────────────────────────────▼────────────┐
│                   AAMF Runtime Base                      │
│ Lore · CADRE · DAG · scheduler · retries · checkpoints  │
│ budget · observability · agent launching · git          │
└─────────────┬───────────────────────────────┬────────────┘
              │                               │
┌─────────────▼────────────┐      ┌──────────▼─────────────┐
│ Agent Backends           │      │ Repository Knowledge  │
│ Codex/Claude/Copilot/... │      │ OKF + Lore MCP        │
└──────────────────────────┘      └────────────────────────┘

Prompt source is orthogonal:
OPF → ScenarioPromptProvider → AAMF agent backend
```

## 4. Why AAMF is the base

AAMF already implements expensive orchestration concerns that should not be rewritten for the MVP:

- deterministic multi-phase migration flow;
- Lore-backed source indexing;
- SCC contraction and bounded migration tasks;
- dependency-aware task execution;
- per-task, wave-barrier, and sync-epoch scheduling;
- build/test synchronization gates;
- retries and failure recovery;
- task blocking policies;
- checkpoints and deterministic resume;
- token/cost tracking;
- observability;
- per-task/per-wave Git automation;
- agent scenario lifecycle.

We should extend it, not make those concerns part of our domain core.

## 5. What remains ours

AAMF's task graph is primarily symbol/dependency oriented. That is useful but insufficient for UI migration.

We add:

### Domain model

- `Migration`
- `Contract`
- `Capability`
- `Artifact`
- `MigrationTask` domain projection
- `Verification`
- `AcceptancePolicy`
- `CompletionPolicy`
- `RecoveryDecision`

### Artifact model

Artifacts are versioned, hashed, and provenance-aware.

Initial artifact types:

- `SourceInventory`
- `ModuleDependencyGraph`
- `ComponentGraph`
- `StateModel`
- `EffectModel`
- `InteractionModel`
- `NavigationModel`
- `LayoutModel`
- `StyleTokenSet`
- `AssetInventory`
- `RuntimeTraceSet`
- `ScreenshotBaseline`
- `CapabilityReport`
- `SemanticUIModel`
- `TargetChangeSet`
- `VerificationReport`

IR is one artifact family, not the only representation in the system.

## 6. Common semantic core vs domain IR

Do not build one giant language-independent AST.

Use composable layers:

### Common semantic primitives

- type/data shape;
- state;
- events;
- state transitions;
- effects;
- errors;
- resources;
- concurrency markers;
- external contracts.

### UI domain artifacts

- component hierarchy;
- layout constraints;
- style tokens;
- navigation;
- interaction;
- accessibility;
- animations;
- assets;
- responsive rules.

Future domains can add:

- Service IR;
- Simulation IR;
- Systems IR;
- Data Pipeline IR.

## 7. Capability model

Capabilities are contextual, not booleans.

```text
SUPPORTED
PARTIAL
UNSUPPORTED
UNKNOWN
```

Each capability assessment records:

- source requirement;
- target support;
- selected strategy;
- evidence/provenance;
- risk;
- manual adaptation reason code if needed.

Strategies:

- direct native mapping;
- semantic reconstruction;
- replacement component;
- custom implementation;
- compatibility wrapper;
- hybrid bridge;
- manual adaptation;
- unsupported.

## 8. Task model

A migration task describes an outcome, not necessarily one source file.

Bad task identity:

```text
migrate src/components/Chat.tsx
```

Better:

```text
Implement chat-list behavior and rendering contracts in the native target.
```

A task can consume several artifacts and emit several target artifacts.

## 9. State machine

Recommended task states:

```text
PENDING → READY → RUNNING → VERIFYING → ACCEPTED
                        ↘              ↗
                          NEEDS_REPAIR
                               ↓
                              READY

Additional:
BLOCKED
FAILED
CANCELLED
```

AAMF execution state remains the operational source of truth for running/resuming tasks. The domain state records acceptance semantics.

## 10. Acceptance invariant

A task is accepted only if:

```text
dependencies accepted
AND required contract evidence current
AND no required verification is FAIL
AND native-target policy passes
AND no critical blocker remains
```

Evidence has an artifact/version identity. A new target change invalidates affected evidence.

## 11. Prompt architecture

AAMF currently compiles repository Markdown templates into immutable `ScenarioPrompt` objects. Replace the direct template-loading boundary with:

```text
ScenarioPromptProvider
    ├── BundledMarkdownPromptProvider   # compatibility
    └── OpfPromptProvider               # preferred
```

OPF owns:

- prompt source;
- prompt ID;
- inputs;
- digest;
- release/channel;
- prompt checks;
- eval fixtures.

AAMF still owns:

- scenario registry;
- output JSON schemas;
- capabilities/tool policy;
- sequencing;
- invocation;
- retry/recovery.

Do not move AAMF output contracts into free-form prompts.

## 12. Knowledge architecture

Two knowledge systems coexist for different reasons.

### Lore

Run-specific machine index used by AAMF for:

- files;
- symbols;
- resolved call edges;
- type-reference edges;
- migration task graph;
- agent MCP queries.

### OKF

Repository-owned engineering memory used by humans and coding agents for:

- architectural constraints;
- component responsibilities;
- decisions;
- operating procedures;
- migration invariants;
- known gaps;
- change knowledge.

Do not attempt to replace Lore with OKF in the MVP.

AAMF agents may receive both MCP sources:

```text
lore-kb
okf-repository-brain
```

Lore answers “what code/symbol depends on what?”
OKF answers “what does this repository intend and what constraints must I preserve?”

## 13. Provider independence

Target shape:

```text
AgentBackend
    execute(ScenarioInvocation) -> AgentResult
```

Implement:

- `CopilotBackend` — existing behavior;
- `ClaudeCodeBackend` — existing behavior;
- `CodexBackend` — first new backend;
- future process/API backends.

OPF messages must not assume a provider. Provider-specific system/user delivery is an AgentBackend concern.

## 14. Source/target adapter SDK

```text
SourceAdapter
  discover()
  analyzeStatic()
  captureRuntime()
  emitArtifacts()
  assessCapabilities()

TargetAdapter
  scaffold()
  mapCapabilities()
  generateTargetPlan()
  build()
  test()
  exposeVerificationDriver()
```

For MVP:

```text
ReactSourceAdapter
RustSlintTargetAdapter
```

## 15. Verification architecture

Verification must be independently runnable.

Categories:

- `BUILD`
- `BEHAVIOR`
- `STRUCTURAL`
- `VISUAL`
- `NATIVE_COMPLIANCE`
- `ACCESSIBILITY`
- `PERFORMANCE`
- later `SECURITY`, `PROTOCOL`, `DATA`

Verification result:

```text
PASS
FAIL
INCONCLUSIVE
NOT_RUN
```

The verifier emits evidence. The domain core decides whether that evidence is sufficient.

## 16. Repository layout

Recommended fork layout:

```text
.
├── AGENTS.md
├── agents/                       # upstream AAMF templates during transition
├── prompts/                      # OPF prompt source
├── opf.yaml
├── src/
│   ├── agents/                   # upstream AAMF scenario registry/catalog
│   ├── budget/                   # upstream AAMF
│   ├── config/                   # upstream + small extensions
│   ├── core/                     # upstream AAMF
│   ├── execution/                # upstream AAMF
│   ├── flow/                     # upstream AAMF
│   ├── logging/                  # upstream AAMF
│   ├── observability/            # upstream AAMF
│   ├── domain/                   # ours
│   ├── artifacts/                # ours
│   ├── adapters/
│   │   ├── source/react/
│   │   └── target/rust-slint/
│   ├── verification/             # ours
│   └── integrations/
│       ├── aamf/
│       ├── okf/
│       └── opf/
├── .okf/                         # committed curated repository knowledge
├── .okf-index/                   # generated local state; ignored
├── .agents/skills/
├── tests/
│   ├── upstream/                 # optional logical grouping, not required initially
│   ├── domain/
│   ├── adapters/
│   ├── verification/
│   └── e2e/
├── fixtures/
│   └── react-rust-slint/
└── docs/
```

Do not physically reorganize all upstream AAMF files on day one. Preserve upstream paths first; add new modules alongside them.

## 17. Fork maintenance rule

Track changes as:

```text
UPSTREAM-UNCHANGED
UPSTREAM-PATCHED
OURS
```

Document each patch to AAMF in `docs/AAMF_EXTENSION_PLAN.md`.

Prefer one explicit compatibility layer to scattered modifications.

## 18. Non-goals for MVP

- perfect migration of arbitrary JavaScript;
- universal full-program semantic IR;
- backend/game/systems adapters;
- distributed execution;
- marketplace;
- automatic model training;
- automatic acceptance of unsupported behavior;
- guaranteed pixel-perfect parity for every browser CSS behavior.

The MVP succeeds when the framework can migrate a representative React slice into a real Rust + Slint application, resume safely, and independently prove declared parity.
