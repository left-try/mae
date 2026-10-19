---
name: modifying-aamf-runtime
description: Use when a change touches AAMF-derived core, flow, execution, checkpoint, agent-launcher, prompt-catalog, budget, or observability code.
---

# Modifying AAMF Runtime

Status: starter draft; pressure-test before enforcing as production process.

## Default stance

Preserve upstream behavior and create extension seams.

Before changing upstream-derived code, identify whether the requirement can live in:

- domain;
- adapter;
- verifier;
- integration layer.

## Allowed early patch classes

- AgentBackend abstraction/Codex;
- PromptProvider abstraction/OPF;
- external analysis-artifact hook;
- external verifier hook;
- domain acceptance callback.

Do not casually rewrite checkpoints, scheduler, retry logic, or observability.

Run the original AAMF test surface plus compatibility tests.
