# React → Rust + Slint MVP

## 1. Purpose

Validate the framework on a migration that is meaningfully harder than syntax translation:

```text
React web UI → native Linux Rust + Slint UI
```

This forces the system to handle:

- component hierarchy;
- state;
- events;
- navigation;
- CSS/layout semantics;
- assets;
- async effects;
- browser/platform gaps;
- visual parity;
- native target compliance.

## 2. Recommended source fixture

Start with a small but non-trivial application:

- two or three routes/views;
- reusable components;
- local state;
- form input;
- a list;
- filtering;
- async mocked data load;
- loading/error/success states;
- CSS layout;
- an image/icon asset;
- at least one responsive rule.

Avoid a simple counter.

## 3. Source analysis stages

### Stage A — Project discovery

Inputs:

- `package.json`;
- TypeScript config;
- source tree;
- routes;
- CSS/assets.

Outputs:

- `SourceInventory`;
- `ModuleDependencyGraph`.

Tools:

- Lore;
- ts-morph;
- dependency-cruiser.

### Stage B — React semantic capture

Extract:

- components;
- props;
- state variables;
- hooks;
- effects;
- event handlers;
- state transitions;
- conditional rendering;
- lists;
- context/store usage;
- router usage;
- platform/browser APIs.

Outputs:

- `ComponentGraph`;
- `StateModel`;
- `EffectModel`;
- `InteractionModel`;
- `NavigationModel`;
- `CapabilityReport`.

Tools:

- ts-morph;
- ast-grep rule packs;
- Codex only for ambiguous semantic interpretation.

### Stage C — Runtime capture

Run the source.

Capture:

- reference screenshots;
- representative viewport sizes;
- interaction scenarios;
- routes;
- runtime DOM structure;
- accessible names/roles where usable;
- computed dimensions/styles where needed;
- animation notes;
- network fixtures.

Tool:

- Playwright.

### Stage D — Assemble semantic UI artifacts

Combine static and runtime evidence.

Do not let runtime observations silently override source semantics; store provenance.

## 4. Gap classification

Borrow the Sanka idea of stable reason codes.

Examples:

```text
UI_BROWSER_ONLY_API
UI_UNSUPPORTED_CSS
UI_DYNAMIC_STYLE
UI_THIRD_PARTY_WIDGET
UI_CANVAS
UI_COMPLEX_ANIMATION
UI_UNRESOLVED_STATE_LIBRARY
UI_UNRESOLVED_ROUTING
UI_UNRESOLVED_EFFECT
UI_MANUAL_ASSET
UI_ACCESSIBILITY_GAP
```

Each gap has:

- source location;
- affected contract(s);
- severity;
- migration strategy;
- human decision if required.

## 5. Plan identity

Borrow Sanka's reviewed-plan principle.

Before applying a major migration plan, compute:

```text
plan_hash = hash(
  source_fingerprint
  + contract_set
  + capability_report
  + migration_plan
  + adapter_versions
)
```

Execution records the exact plan hash.

A changed source/contract/adapter invalidates the reviewed plan when it changes semantics.

## 6. Target structure

Recommended generated target:

```text
target/
├── Cargo.toml
├── Cargo.lock
├── build.rs
├── src/
│   ├── main.rs
│   ├── app/
│   │   ├── state.rs
│   │   ├── events.rs
│   │   ├── reducer.rs
│   │   ├── effects.rs
│   │   └── navigation.rs
│   └── services/
├── ui/
│   ├── app.slint
│   ├── components/
│   └── theme/
├── assets/
└── tests/
```

Exact layout is adapter policy, not domain law.

## 7. Logic separation

Prefer:

```text
Rust domain/state/effects
        ↕ callbacks/properties
      Slint UI
```

Do not encode all business behavior directly inside `.slint` files.

## 8. Target build gates

Hard gates:

```bash
cargo fmt --check
cargo build --locked
cargo test --locked
cargo clippy
```

A build failure is deterministic `FAIL`, not an agent opinion.

## 9. Slint verification

Use Slint's headless testing facilities when possible.

Target verification should support:

- launching without a display server;
- synthetic interactions;
- structural/state inspection;
- screenshots;
- deterministic fixture-driven state.

Keep source and target scenario definitions shared at the migration-contract level.

## 10. Differential behavior

For each contract scenario:

```text
same fixture
same logical action sequence
      ↓
source observation
target observation
      ↓
normalization
      ↓
comparison
```

Normalize only explicitly declared irrelevant differences.

Never normalize a mismatch simply to make a test pass.

## 11. Native-compliance gate

The migration fails native compliance if:

- UI is rendered by a browser engine;
- source app is embedded in a WebView;
- screenshot/video substitutes for real controls;
- target delegates all behavior back to the React runtime.

A bridge is allowed only when explicitly declared as a temporary migration strategy and must not count as full native completion.

## 12. Visual verification

Compare:

- geometry;
- spacing;
- typography;
- colors;
- borders;
- assets;
- visibility;
- state-specific screenshots.

Visual tolerance must be configuration, not hard-coded into prompts.

## 13. Performance phase

Only after functional acceptance:

- startup latency;
- idle memory;
- representative interaction latency;
- CPU during representative scenario.

Performance work creates new target revisions and therefore requires regression re-verification of behavioral contracts.

## 14. MVP acceptance

MVP is successful when:

1. source analysis produces versioned semantic artifacts;
2. AAMF creates/resumes an execution plan;
3. at least one real coding backend generates native Rust+Slint;
4. target builds locally on Linux;
5. independent behavioral verification passes for declared scenarios;
6. independent visual verification is operational;
7. native-compliance gate passes;
8. a deliberate failure causes repair/retry or block rather than false acceptance;
9. interrupted execution can resume.
