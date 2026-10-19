---
name: implement-spec
description: Implement a specification across its task graph using subagents for vertical slices, TDD at pre-agreed seams, and two-axis code review.
license: MIT
metadata:
  source: https://github.com/mattpocock/skills
---

# Implement Spec

Execute the phases/tickets of a spec as a dependency-ordered task graph:

1. Identify the ready frontier of tickets/phases.
2. Dispatch implementation work with sparse context pointers (`GLOSSARY.md`, `docs/specs/v0.1-multiphase-tdd-plan.md`, `docs/adr/`).
3. Enforce TDD (`red -> green -> refactor`) at the pre-agreed seams and Ponytail minimal-complete-change discipline.
4. Verify with `cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, `opf check`, and `okf-agent bundle validate`.
5. Open a PR per phase using the `/pr` template and automerge once checks pass.
