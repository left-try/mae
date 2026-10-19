---
name: domain-modeling
description: Build and sharpen a project's domain model. Use when discussing codebase terminology, writing or editing a GLOSSARY.md, or recording or editing an ADR.
license: MIT
metadata:
  source: https://github.com/mattpocock/skills
---

# Domain Modeling

Actively build and sharpen the project's domain model as you design. This is the *active* discipline: challenging terms, inventing edge-case scenarios, and writing the glossary and decisions down the moment they crystallise.

## File structure

```text
/
├── GLOSSARY.md
├── docs/
│   └── adr/
│       ├── 0001-ddd-clean-architecture-rust-workspace.md
│       └── 0002-v01-react-to-windows-rust-slint-frontend-scope.md
└── crates/
```

## Rules

1. Every domain concept in `mae-domain` must match `GLOSSARY.md` verbatim.
2. When introducing or refining an architectural constraint, record or update an ADR in `docs/adr/` and sync `.okf/` knowledge.
3. Challenge vague terms with concrete edge-case scenarios (e.g., stale evidence after a target edit, partial scenario execution, browser-only API gaps).
