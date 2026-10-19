---
name: to-tickets
description: Break a plan or spec into tracer-bullet vertical-slice tickets/phases, each declaring its blocking edges and verification criteria.
license: MIT
metadata:
  source: https://github.com/mattpocock/skills
---

# To Tickets

Break a specification into tracer-bullet vertical slices, each declaring the phases/tickets that block it:

- Each slice is demoable and independently verifiable via `cargo test` and CLI/adapter checks.
- Each slice is sized to fit cleanly in a single subagent context window and PR.
- Wide refactors use expand-contract; greenfield Clean Architecture layers build from the inner Domain Core (`mae-domain`) outward to Application, Adapters, Verifiers, and CLI.
