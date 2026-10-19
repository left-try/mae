---
name: verifying-migration-parity
description: Use when implementing or reviewing source-target parity checks, evidence freshness, native-compliance gates, screenshot comparison, or migration acceptance logic.
---

# Verifying Migration Parity

Status: starter draft; pressure-test before enforcing as production process.

## Authority

Verification emits evidence. Domain policy decides acceptance.

## Rules

- bind evidence to exact source/target artifact revisions;
- report PASS, FAIL, INCONCLUSIVE, or NOT_RUN;
- disclose unexecuted scenarios;
- rerun failed signals after repair;
- keep code-writing and critical verification responsibilities separate;
- never normalize away an unexplained mismatch.

A current critical FAIL blocks acceptance.
