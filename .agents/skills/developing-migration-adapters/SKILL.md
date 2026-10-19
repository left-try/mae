---
name: developing-migration-adapters
description: Use when adding or changing a source adapter, target adapter, capability mapping, or adapter-specific artifact format.
---

# Developing Migration Adapters

Status: starter draft; pressure-test before enforcing as production process.

## Boundary

Adapters translate framework/platform semantics into or out of domain artifacts.

They may depend on compilers, AST tools, browsers, build tools, and platform SDKs.

The domain core must not depend on those libraries.

## Rules

- version every artifact schema;
- preserve provenance;
- emit explicit UNKNOWN/UNSUPPORTED gaps;
- avoid source-target pair logic when a source-only or target-only responsibility is sufficient;
- add capability tests and fixture-based integration tests;
- keep target code generation separate from target verification.
