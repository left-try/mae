---
name: authoring-opf-migration-prompts
description: Use when creating or changing OPF prompts used by migration analyzers, planners, code migrators, verifiers, or recovery agents.
---

# Authoring OPF Migration Prompts

Status: starter draft; pressure-test before enforcing as production process.

## Source of truth

Prompt text lives in OPF files with stable IDs.

AAMF code owns scenario output schemas and tool capabilities.

## Rules

- do not encode provider CLI flags in prompt text;
- keep scope explicit and bounded;
- require uncertainty disclosure;
- never tell the verifier to assume success;
- run `opf check .` after prompt edits;
- update eval cases when prompt behavior changes;
- record prompt digest/release in invocation provenance.
