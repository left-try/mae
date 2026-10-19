---
name: analyzing-react-source
description: Use when implementing or reviewing React/TypeScript source analysis for a migration, especially components, state, effects, routing, styles, or browser-specific behavior.
---

# Analyzing React Source

Status: starter draft; pressure-test before enforcing as production process.

## Core principle

Prefer deterministic compiler/runtime evidence over LLM inference.

## Required evidence

Use TypeScript-aware analysis for symbols/types, structured rules for known patterns, and runtime capture for rendered behavior.

Classify each discovered behavior as:

- deterministic static fact;
- runtime observation;
- inferred semantic fact;
- unknown/gap.

Never hide uncertainty.

## Output discipline

Emit versioned artifacts rather than prose-only findings:

- component graph;
- state/effect model;
- interaction/navigation model;
- capability gaps;
- provenance.

Do not put React-specific concepts into the migration domain core.
