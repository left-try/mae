---
name: tdd
description: Test-driven development. Use when building features or fixing bugs test-first with a red-green-refactor loop at pre-agreed seams.
license: MIT
metadata:
  source: https://github.com/mattpocock/skills
---

# Test-Driven Development

TDD is the **red → green → refactor** loop. Every vertical slice follows this discipline:

1. Read `GLOSSARY.md` and `docs/adr/` so test names and public interfaces match the project's ubiquitous language.
2. **Test only at pre-agreed seams**: verify behavior through public interfaces and domain port traits, never against internal private details.
3. **Red**: Write a failing test that reads like a specification of observable capability or domain invariant.
4. **Green**: Write the smallest complete implementation (following `ponytail`) that makes the test pass.
5. **Refactor**: Deepen modules, remove duplication, and run `cargo test --workspace` and `cargo clippy --workspace --all-targets -- -D warnings`.
