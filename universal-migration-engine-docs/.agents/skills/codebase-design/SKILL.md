---
name: codebase-design
description: Shared vocabulary for designing deep modules. Use when designing or improving a module's interface, finding deepening opportunities, deciding where a seam goes, or making code testable.
license: MIT
metadata:
  source: https://github.com/mattpocock/skills
---

# Codebase Design

Design **deep modules**: a lot of behaviour behind a small interface, placed at a clean seam, testable through that interface.

## Glossary

- **Module**: Anything with an interface and an implementation.
- **Interface**: Everything a caller must know to use the module correctly: type signature, invariants, ordering constraints, error modes, required configuration, and performance characteristics.
- **Implementation**: What is inside a module, its body of code.
- **Depth**: Leverage at the interface—the amount of behaviour a caller or test can exercise per unit of interface they have to learn.
- **Seam**: A place where you can alter behaviour without editing in that place; the location at which a module's interface lives (`trait` ports in `mae-domain`).
- **Adapter**: A concrete implementation satisfying an interface at a seam.
- **Leverage**: More capability per unit of interface learned.
- **Locality**: Change, bugs, knowledge, and verification concentrate in one place rather than spreading across callers.
