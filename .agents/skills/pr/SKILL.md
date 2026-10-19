---
name: pr
description: Use when writing a Pull Request body with a concise visual summary, before/after verification evidence, and merge danger assessment.
license: MIT
metadata:
  source: https://github.com/mattpocock/skills
---

# Pull Request Template

Use this structure for every PR body:

```markdown
## Summary

<diagram, module tree, or call tree using GLOSSARY.md terminology>

## Evidence

- **Before:** <baseline / unimplemented phase state>
- **After:** <passing `cargo test --workspace`, `cargo clippy`, `opf check`, and `okf-agent bundle validate` output>

## Merge Danger

**Door:** <one-way or two-way>
**Blast Radius:** <crate/layer scope>
```
