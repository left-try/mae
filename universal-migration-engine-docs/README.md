# Universal Migration Engine — Documentation Suite

This repository skeleton documents a proposed universal software migration engine built on an **AAMF runtime base**, with a separate domain core, source/target adapters, independent verification, repository knowledge through **OKF Repository Brain**, and provider-neutral prompt assets through **Open Prompt Format (OPF)**.

The first executable target is intentionally narrow:

> **React / TypeScript web application → native Linux application in Rust + Slint**

The design is meant to expand later to other frontend stacks and then, with domain-specific IRs and validators, to backend, games, data pipelines, and systems software.

## Current architectural decisions

1. **AAMF is the execution/orchestration base.**
   - Reuse its deterministic DAG/task construction, checkpoints/resume, retry/recovery, observability, budgeting, Git automation, and agent lifecycle.
   - Keep modifications to upstream AAMF narrow and explicit.

2. **The migration domain core is ours.**
   - AAMF does not define migration success.
   - Our core owns contracts, artifacts, capabilities, task acceptance, completion, and recovery policy.

3. **Intermediate representation stays small and composable.**
   - Do not create a universal programming language.
   - Use a small common semantic core plus domain artifacts such as UI state, interaction, layout, navigation, assets, and effects.

4. **Correctness means contract equivalence, not source-code similarity.**
   - The source and target may use completely different architectures.
   - A task is accepted only when required contracts have current evidence.

5. **Verification is independent from generation.**
   - The agent that writes code does not get to certify it.
   - Build, behavior, native-target, visual, accessibility, and performance evidence are separate artifacts.

6. **Optimization is a second phase.**
   - First reach behavioral/visual acceptance.
   - Then optimize while preserving the same contracts.

7. **OKF and OPF have separate jobs.**
   - OKF: repository-owned curated knowledge + rebuildable local index + agent protocol.
   - OPF: versioned prompts/scenarios, prompt identity, validation, release/eval workflow.
   - `.okf/` is committed; `.okf-index/` is generated locally and must not be treated as authored documentation.

## Start here

- `docs/ARCHITECTURE.md` — complete system design.
- `docs/COMPETITOR_AUDIT.md` — AAMF vs Sanka vs Crane and what we reuse.
- `docs/AAMF_EXTENSION_PLAN.md` — exact fork strategy and patch boundaries.
- `docs/COMPONENT_TOOLING.md` — suggested GitHub tools by subsystem.
- `docs/REACT_RUST_SLINT_MVP.md` — first migration path.
- `docs/VERIFICATION.md` — contract/evidence model.
- `docs/KNOWLEDGE_AND_PROMPTS.md` — OKF + OPF + skills.
- `docs/DECISIONS.md` — agreed architectural decisions.
- `docs/ROADMAP.md` — staged implementation roadmap.
- `docs/TOOLING_SETUP.md` — pinned OKF Agent and OPF CLI setup and maintenance commands.
- `AGENTS.md` — repository instructions for coding agents.

## Evidence base checked

Architecture decisions in this suite were informed by the public repositories and source files checked on 2026-10-16:

- https://github.com/jafreck/AAMF
- https://github.com/sankaHQ/sanka
- https://github.com/githubnext/crane
- https://github.com/left-try/okf-agent
- https://github.com/stovo-team/open-prompt-format
- https://github.com/dsherret/ts-morph
- https://github.com/ast-grep/ast-grep
- https://github.com/sverweij/dependency-cruiser
- https://github.com/tree-sitter/tree-sitter
- https://github.com/microsoft/playwright
- https://github.com/slint-ui/slint

This is a design/documentation starter, not proof that the full React → Rust + Slint pipeline already works end-to-end.
