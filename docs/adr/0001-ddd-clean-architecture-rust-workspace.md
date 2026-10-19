# ADR 0001: DDD, SOLID, and Clean Architecture Enforced via Rust Cargo Workspace

- **Status:** Accepted
- **Date:** 2026-10-19

## Context

MAE (Migrate Absolutely Everything) requires a rigorous separation between:
1. **Migration Domain Core** (contracts, capabilities, versioned artifacts, task state machine, acceptance/completion/recovery policies, port traits).
2. **Application Layer** (orchestration use cases, plan hashing, wave/task scheduling, AAMF runtime bridge, OPF prompt provider, OKF knowledge bundle integration).
3. **Adapter Layer** (source analysis for React/TypeScript, target generation for Windows/Linux native Rust + Slint).
4. **Verification Layer** (independent build, native-compliance, differential behavior, and visual verifiers).

Additionally, v0.1 targets **Rust + Slint** native desktop applications, and our agentic TDD workflow benefits from fast, deterministic, offline-capable compilation and testing.

## Decision

We implement MAE v0.1 as a multi-crate **Rust Cargo Workspace** organized strictly along Clean Architecture (Hexagonal / Ports & Adapters) and Domain-Driven Design (DDD) boundaries:

```text
crates/
├── mae-domain/               # Enterprise Business Rules (Entities, Value Objects, Aggregates, Policies, Port Traits)
│                             # ZERO external or sibling dependencies.
├── mae-application/          # Application Business Rules (Use Cases, Orchestrator, OPF/OKF/AAMF Seams)
│                             # Depends ONLY on mae-domain.
├── mae-adapter-react/        # Source Adapter (React/TSX static semantic extractor, gap classifier, runtime trace merger)
│                             # Implements SourceAdapterPort from mae-domain.
├── mae-adapter-rust-slint/   # Target Adapter (Windows/Desktop Rust + Slint generator & scaffolding)
│                             # Implements TargetAdapterPort from mae-domain.
├── mae-verification/         # Independent Verifiers (Build, NativeCompliance, BehavioralDifferential, VisualParity)
│                             # Implements VerifierPluginPort from mae-domain.
└── mae-cli/                  # Composition Root & CLI Runner + E2E React -> Windows Rust + Slint pipeline
```

All workspace crates use standard-library-first Rust (`std` only, following Ponytail's zero-bloat principle), giving sub-second `cargo test --workspace` and `cargo clippy --workspace --all-targets -- -D warnings` feedback loops.

## Consequences

- **Compiler-enforced Dependency Inversion (SOLID DIP):** `mae-domain` cannot accidentally import filesystem, CLI, parser, browser, or target-framework code.
- **Deep Modules & Clear TDD Seams:** Each crate exposes a narrow, well-typed public interface tested via Red-Green-Refactor TDD before downstream crates consume it.
- **Native Target Synergy:** `mae-adapter-rust-slint` emits valid Rust + Slint target projects that can be inspected and verified directly by `mae-verification`.
