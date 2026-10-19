# Roadmap

## Milestone 0 — Fork hygiene

Goal: establish an AAMF fork without semantic changes.

- fork AAMF;
- add upstream remote;
- run full existing test suite;
- initialize OKF;
- add this documentation;
- add upstream patch ledger;
- add CI for AAMF original tests.

Exit:

- fork builds;
- original tests pass;
- no migration behavior intentionally changed.

## Milestone 1 — Agent/prompt seams

- introduce `AgentBackend` abstraction without behavior change;
- preserve Copilot and Claude behavior;
- add Codex backend;
- introduce `ScenarioPromptProvider`;
- preserve legacy Markdown provider;
- add OPF provider;
- record prompt provenance.

Exit:

- same fixture works through legacy provider;
- same scenario can be loaded through OPF;
- Codex backend executes one scenario locally.

## Milestone 2 — Domain core

Implement pure domain types and policies:

- Migration;
- Contract;
- Capability;
- Artifact;
- Verification;
- AcceptancePolicy;
- CompletionPolicy;
- RecoveryDecision.

Tests must not require AAMF, LLMs, browser, or Slint.

Exit:

- domain invariants covered by unit/property tests.

## Milestone 3 — React static adapter

- project discovery;
- ts-morph analysis;
- ast-grep rules;
- module graph;
- component/state/effect/navigation artifacts;
- stable gap reason codes.

Exit:

- deterministic artifacts for fixture;
- source changes invalidate expected artifacts.

## Milestone 4 — Source runtime capture

- Playwright baseline runner;
- screenshots;
- scenarios;
- route/state observations;
- fixture network data.

Exit:

- reproducible baseline from clean checkout.

## Milestone 5 — Rust/Slint target adapter

- scaffold;
- theme/components;
- state/event boundary;
- build/test commands;
- native-compliance checks.

Exit:

- hand-authored reference target can pass adapter verification.

## Milestone 6 — AAMF artifact integration

- expose React analysis artifacts to planner/migrator;
- bridge AAMF tasks to domain tasks/contracts;
- external verifier hook;
- evidence freshness.

Exit:

- one AAMF task cannot be accepted when external evidence fails.

## Milestone 7 — End-to-end migration

- React fixture → Rust+Slint;
- repair loop;
- resume interruption test;
- behavior verification;
- visual verification;
- native compliance.

Exit:

- repeatable local run from source snapshot to accepted target.

## Milestone 8 — Benchmark and hardening

Measure:

- successful contract ratio;
- manual interventions;
- retries;
- cost/tokens;
- wall time;
- source coverage;
- visual delta;
- prompt versions;
- resume correctness.

Add adversarial fixtures.

## Milestone 9 — Second source/target

Only now validate generality.

Good choices:

- React → SwiftUI;
- React → Compose;
- Vue → Slint.

Use the second adapter to identify abstractions that were accidentally React/Slint-specific.

Do not start backend/game/system domain IRs before this step.
