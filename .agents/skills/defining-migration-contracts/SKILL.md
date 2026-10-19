---
name: defining-migration-contracts
description: Use when creating, changing, prioritizing, or evaluating behavioral, visual, structural, native-compliance, accessibility, or performance migration contracts.
---

# Defining Migration Contracts

Status: starter draft; pressure-test before enforcing as production process.

## Principle

A contract describes observable required behavior, not implementation similarity.

A critical contract must be independently verifiable.

## Contract quality

A good contract states:

- preconditions;
- action/stimulus;
- observable expectation;
- severity;
- required verifier category;
- acceptable tolerance when quantitative.

Avoid requirements such as “same architecture” unless architecture itself is a target constraint.

Missing evidence is never a pass.
