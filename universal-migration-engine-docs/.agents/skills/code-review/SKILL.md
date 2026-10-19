---
name: code-review
description: Two-axis review of changes since a fixed point: Standards (DDD, SOLID, Clean Architecture, Ponytail, Fowler smells) and Spec (faithfulness to the originating spec and contracts).
license: MIT
metadata:
  source: https://github.com/mattpocock/skills
---

# Two-Axis Code Review

Review every phase diff (`git diff main...HEAD`) along two axes before merging:

1. **Standards Axis**:
   - Conforms to `GLOSSARY.md`, `AGENTS.md`, DDD, OOP, SOLID, and Clean Architecture dependency direction (`mae-domain` has zero outward dependencies).
   - Follows `ponytail` discipline: smallest complete change, no unused abstractions or bloat, explicit validation at boundaries.
   - Zero compiler or Clippy warnings.
2. **Spec Axis**:
   - Faithfully implements every requirement and test seam in `docs/specs/v0.1-multiphase-tdd-plan.md` for the phase.
   - All required unit/integration tests, `opf check`, and `okf-agent bundle validate` pass.
