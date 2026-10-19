# Architecture Decisions

## D-001 — Intermediate representation

**Decision:** small common semantic model + extensible artifacts/domain IRs.

Rejected for MVP:

- one complete universal executable IR;
- one direct N×M translator per source/target pair.

Reason: a full language-neutral representation quickly becomes a programming language/compiler project and still cannot erase target runtime semantics.

---

## D-002 — Domain core

**Decision:** domain core stays separate from infrastructure.

It must not directly:

- parse React;
- invoke Codex;
- run Cargo;
- launch browsers;
- write Git commits.

It receives facts/evidence and returns decisions.

---

## D-003 — Equivalence

**Decision:** preserve explicitly agreed observable contracts.

Do not promise universal semantic equivalence.

---

## D-004 — Performance

**Decision:** migrate first, optimize second.

Optimization is allowed only under the same behavioral acceptance gates.

---

## D-005 — Runtime base

**Decision:** AAMF.

Why:

- strongest reusable DAG/runtime;
- checkpoints/resume;
- local CLI;
- retries;
- cost/observability;
- permissive MIT license;
- significant test surface.

---

## D-006 — Sanka reuse

**Decision:** borrow concepts, not runtime core.

Borrow:

- scan/capture separation;
- plan hash;
- gap reason codes;
- test/verify split;
- differential verification;
- native-target compliance.

---

## D-007 — Crane reuse

**Decision:** borrow acceptance concepts.

Borrow:

- health/no-regression;
- deterministic completion gate;
- current-head/current-artifact evidence;
- human-readable migration definition.

---

## D-008 — Repository knowledge

**Decision:** OKF.

`.okf/` is source-controlled curated knowledge.
`.okf-index/` is generated local output.

---

## D-009 — Prompt source

**Decision:** Open Prompt Format.

Prompt provider must be pluggable so legacy AAMF templates remain usable during migration.

---

## D-010 — First adapter pair

**Decision:** React/TypeScript → Rust/Slint/Linux.

Reason: sufficiently different source/target models to validate the architecture while remaining narrow enough for an MVP.

---

## D-011 — Static analysis

For first React adapter:

- ts-morph: primary semantic analysis;
- ast-grep: rules/patterns;
- dependency-cruiser: module graph helper;
- Lore: AAMF graph/index;
- Tree-sitter: deferred until additional languages.

---

## D-012 — Runtime capture

**Decision:** Playwright for source React runtime.

Static analysis is not enough for rendered behavior/visual evidence.

---

## D-013 — Target verification

**Decision:** Cargo gates + Slint headless testing facilities + independent scenario comparison.

Agent review is supplementary.
