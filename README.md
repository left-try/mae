# MAE (Migrate Absolutely Everything) — Universal Migration Engine v0.1

MAE is a contract-driven, evidence-verified software migration engine built with **Domain-Driven Design (DDD), OOP, SOLID, and Clean Architecture** principles, integrating:

- **AAMF Orchestration Seams** (deterministic task DAGs, wave-barrier execution, checkpoint/resume, agent backend abstraction)
- **Open Prompt Format (`opf/0.2`)** ([`stovo-team/open-prompt-format`](https://github.com/stovo-team/open-prompt-format)) for portable, versioned, digest-verified scenario prompts (`opf.yaml`, `prompts/*.md`)
- **OKF Repository Brain (`okf-agent`)** ([`left-try/okf-agent`](https://github.com/left-try/okf-agent)) for curated repository engineering knowledge (`.okf/`)
- **Ponytail Minimal-Complete-Change Discipline** ([`DietrichGebert/ponytail`](https://github.com/DietrichGebert/ponytail))
- **Matt Pocock Engineering Skills** ([`mattpocock/skills`](https://github.com/mattpocock/skills): `domain-modeling`, `codebase-design`, `tdd`, `to-spec`, `to-tickets`, `implement-spec`, `pr`, `code-review`)

## Version 0.1 Target Slice (Frontend Migration)

> **React / TypeScript Web Application → Native Windows / Desktop Application in Rust + Slint**

## Clean Architecture Crate Layout

```text
.
├── GLOSSARY.md                                # Ubiquitous language & domain glossary
├── AGENTS.md                                  # Agent protocol, OKF/OPF/Ponytail/TDD rules
├── opf.yaml                                   # Open Prompt Format registry
├── prompts/                                   # OPF 0.2 scenario prompts
├── .okf/                                      # Curated OKF Repository Brain knowledge bundle
├── .agents/skills/                            # Matt Pocock + Ponytail + Migration skills
├── docs/
│   ├── adr/                                   # Architecture Decision Records (0001, 0002)
│   └── specs/v0.1-multiphase-tdd-plan.md      # Multiphase TDD specification & seam plan
├── universal-migration-engine-docs/           # Comprehensive architecture & audit documentation suite
├── crates/
│   ├── mae-domain/                            # Phase 1: Pure DDD Core (Entities, VOs, Aggregates, Policies, Ports)
│   ├── mae-application/                       # Phase 2: Use Cases, Orchestrator, OPF, OKF & AAMF Runtime Seams
│   ├── mae-adapter-react/                     # Phase 3: React/TypeScript Static & Runtime Semantic Source Adapter
│   ├── mae-adapter-rust-slint/                # Phase 4A: Native Windows/Desktop Rust + Slint Target Adapter
│   ├── mae-verification/                      # Phase 4B: Independent Build, NativeCompliance, Behavioral & Visual Verifiers
│   └── mae-cli/                               # Phase 5: CLI Runner & End-to-End Migration Pipeline
└── fixtures/
    └── react-rust-slint/                      # Non-trivial multi-route React -> Windows Rust + Slint reference fixture
```

## Quick Commands

```bash
# Run full TDD test suite across all Clean Architecture layers
cargo test --workspace

# Run strict linter and formatting checks
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all -- --check

# Validate Open Prompt Format (OPF) prompt registry
opf check .

# Index, search, update, and validate OKF knowledge bundle
okf-agent bundle index --root .okf --index-root .okf-index
okf-agent bundle validate --root .okf --index-root .okf-index
```
