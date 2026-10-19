# Component Tooling

This file separates tools that should be reused from code we should build.

## 1. Runtime/orchestration

### AAMF
https://github.com/jafreck/AAMF

Role:

- pipeline;
- DAG;
- scheduling;
- checkpoints/resume;
- retries;
- budget;
- observability;
- agent invocation.

**Decision:** base runtime.

### Lore
Already used by AAMF as `@jafreck/lore`.

Role:

- source structural index;
- files/symbols/calls/type refs;
- dependency graph;
- MCP access.

**Decision:** keep for the MVP.

### CADRE
Already used by AAMF as `@cadre-dev/framework`.

Role:

- flow DSL;
- steps;
- gates;
- parallel branches;
- loops;
- subflows.

**Decision:** keep.

---

## 2. React/TypeScript static analysis

### ts-morph
https://github.com/dsherret/ts-morph

Use for:

- TypeScript project loading;
- symbols;
- types;
- declarations;
- imports/exports;
- function/component structure;
- hooks and references with TypeScript semantics.

**Decision:** primary React/TS semantic analyzer.

### ast-grep
https://github.com/ast-grep/ast-grep

Use for:

- declarative AST patterns;
- unsupported-pattern detection;
- deterministic rewrites/codemods;
- rule packs for browser-only APIs, known React idioms, and risky patterns.

**Decision:** reusable rule engine; not the primary semantic model.

### dependency-cruiser
https://github.com/sverweij/dependency-cruiser

Use for:

- JS/TS module graph;
- cycle reporting;
- architecture-rule checks;
- cross-checking module dependency structure.

**Decision:** useful adapter helper; do not make the domain core depend on it.

### Tree-sitter
https://github.com/tree-sitter/tree-sitter

Use later for:

- language-independent syntax extraction;
- future source languages;
- target/source adapters without a rich compiler API.

**Decision:** defer until the second source-language adapter.

---

## 3. Runtime source capture

### Playwright
https://github.com/microsoft/playwright

Use on the React source side for:

- deterministic browser launch;
- representative viewports;
- screenshots;
- DOM snapshots;
- accessibility snapshots where available;
- interaction traces;
- navigation scenarios;
- computed runtime observations.

**Decision:** required for source behavioral/visual baseline.

Static analysis alone must not be considered enough for visual or dynamic behavior.

---

## 4. Rust + Slint target

### Slint
https://github.com/slint-ui/slint

Role:

- native target UI;
- Rust integration;
- headless testing backend;
- target screenshots/testing in CI.

Slint's repository contains a headless testing backend and tooling capable of taking screenshots without a display server when configured appropriately.

**Decision:** target UI and preferred target test surface.

### Rust toolchain

Use deterministic standard tools:

- `cargo build --locked`
- `cargo test --locked`
- `cargo clippy`
- `cargo fmt --check`

**Decision:** hard verification gates, not agent judgments.

---

## 5. Repository knowledge

### okf-agent
https://github.com/left-try/okf-agent

Role:

- committed `.okf/` curated knowledge;
- generated `.okf-index/` local index/wiki;
- repository instructions;
- MCP context retrieval;
- change-context update/validation workflow.

**Decision:** adopt for the engine repository.

Important:

```text
.okf/       commit
.okf-index/ do not author; local/rebuildable
```

A source project being migrated may optionally be initialized with OKF, but this must not be mandatory for migration.

---

## 6. Prompt source and evaluation

### Open Prompt Format
https://github.com/stovo-team/open-prompt-format

Role:

- prompt source;
- stable prompt IDs;
- inputs;
- digests;
- Git releases/channels;
- validation;
- optional Promptfoo evals;
- portable provider-neutral messages.

**Decision:** move AAMF scenario prompt text toward OPF.

Keep AAMF's canonical output JSON schemas in code.

---

## 7. What not to build ourselves initially

Do not implement:

- TypeScript parser;
- generic AST engine;
- module dependency visualizer;
- browser automation framework;
- Slint renderer;
- checkpoint database;
- agent scheduler;
- prompt registry format.

Spend custom engineering on:

- migration domain semantics;
- adapters;
- UI semantic capture;
- verification contracts;
- evidence freshness/provenance;
- integration seams.
