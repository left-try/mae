---
name: to-spec
description: Turn the current conversation and architectural context into a structured specification with pre-agreed test seams.
license: MIT
metadata:
  source: https://github.com/mattpocock/skills
---

# To Spec

Synthesize the current architectural context and requirements into a specification document under `docs/specs/` using the project's `GLOSSARY.md` vocabulary and respecting `docs/adr/`.

## Required Sections

1. **Problem Statement**: From the user/system perspective.
2. **Solution & Architecture**: Clean Architecture layers, deep modules, and adapters.
3. **Pre-Agreed TDD Seams**: Public interfaces under test, what each seam catches, and what it misses.
4. **Tracer-Bullet Phases / Tickets**: Vertical slices with explicit blocking edges.
