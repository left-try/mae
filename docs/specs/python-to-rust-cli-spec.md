# Specification & TDD Plan: Python CLI to Native Rust CLI Migration

**Domain Slice:** Systems & Developer Tooling (`Python CLI` → `Native Rust CLI`)  
**Architecture:** DDD, SOLID, Clean Architecture with Independent Verification  
**Standard:** Contract-and-Evidence Equivalence, Logic-CLI Separation  

---

## 1. Problem Statement

Migrating a production Python command-line utility to Rust is commonly treated as line-by-line syntax transpilation or an LLM prompt that dumps all logic into a single monolithic `main.rs`. This results in:
- **CLI Flag/Argument Drift:** Differences in flag naming (long vs short), argument ordering, type coercion errors, default values, and `--help` formatting.
- **Exit Code & Stream Desynchronization:** Scripts expecting exit code `2` on missing files or structured JSON on `stdout` with progress on `stderr` break downstream pipelines when the Rust binary exits with `1` or pollutes `stdout`.
- **Logic Entanglement:** Business logic tightly coupled to argument parsing, preventing isolated unit testing.
- **Self-Certifying LLM Hallucinations:** AI agents declaring the Rust binary "done" without executing black-box differential runs against the original Python source.

MAE solves this by treating migration as **contract equivalence validated by independent differential evidence**.

---

## 2. Working Set Criteria & Definition of Done

A migration from a Python CLI to a native Rust CLI is considered **Working & Complete** when:

1. **Deterministic Static Extraction:** `mae-adapter-python-cli` extracts all commands, subcommands, flags, options, positional arguments, choices, types, defaults, and env fallbacks into a `CliSemanticModel` without syntax errors.
2. **Capability & Gap Disclosure:** Any unsupported Python dynamic constructs (`eval`, dynamic module loading, unmapped PyPI C-bindings) are surfaced in `CapabilityReport` with explicit `CliGapReasonCode`s.
3. **Idiomatic Clean Target Scaffolding:** `mae-adapter-rust-cli` generates:
   - `Cargo.toml` with release profiles and minimal required dependencies (`clap` or standard library).
   - `src/cli.rs`: Pure CLI argument definition and command routing.
   - `src/core/`: Pure, testable Rust domain business logic modules decoupled from CLI parsing.
   - `src/main.rs`: Application entrypoint wiring arguments, delegating to core logic, and translating errors into deterministic exit codes.
4. **Target Compilation & Unit Tests:** The target Rust project compiles cleanly (`cargo check`) and passes all generated unit tests (`cargo test`).
5. **Differential Behavioral Oracle Parity (`CliDifferentialVerifier`):**
   - For every scenario in a declared test matrix (e.g. 15+ varied CLI invocations):
     - Exit code equivalence ($E_{\text{rust}} = E_{\text{python}}$)
     - Standard output equivalence (exact match or normalized JSON match for structured commands)
     - Standard error equivalence (non-empty descriptive error reporting on failures)
   - Emits a complete `CoverageDisclosure` detailing passed, failed, and unexecuted scenarios.
6. **Freshness & Acceptance Policy:** Migration task transitions to `Accepted` only when all contracts have fresh `Pass` evidence against the latest target `ArtifactRevision`.

---

## 3. Pre-Agreed TDD Seams

Following `codebase-design` and `tdd` skills:

### Seam 1: CLI Domain Modeling & Port Contracts (`mae-domain`)
- **Public API:** `CliSemanticModel`, `CliCommandNode`, `CliArgument`, `CliArgumentType`, `CliIoContract`, `CliExitCode`, `CliGapReasonCode`, `CliSourceAdapterPort`, `CliTargetAdapterPort`, `CliVerifierPort`.
- **What it catches:** Invariant violations, invalid argument types, missing required arguments, non-deterministic model hashing, and state machine transition bugs.
- **What it misses:** Real Python AST parsing and real cargo compilation.

### Seam 2: Python Source AST Extraction (`mae-adapter-python-cli`)
- **Public API:** `PythonCliSourceAdapter::discover()` and `extract_semantics()`.
- **What it catches:** Missing subcommands, dropped flags, unextracted choices/defaults/env variables, failure to detect `argparse` / `click` patterns, and silent omission of unsupported Python modules.
- **What it misses:** Target Rust syntax generation.

### Seam 3: Rust CLI Target Scaffolding & Code Generation (`mae-adapter-rust-cli`)
- **Public API:** `RustCliTargetAdapter::scaffold_target()` and `emit_target_code()`.
- **What it catches:** Malformed Rust syntax, missing `Cargo.toml` dependencies, logic leakage into argument parsing structs, missing exit code mappings, and non-monotonic `ArtifactRevision` increments.
- **What it misses:** Differential runtime comparison against Python.

### Seam 4: Differential Behavioral Oracle Verifier (`mae-verification`)
- **Public API:** `CliDifferentialVerifier::verify()`.
- **What it catches:** Discrepancies between Python and Rust execution (exit codes, stdout output, stderr error formats), unexecuted scenario omissions, and stale evidence.
- **What it misses:** Source Python AST analysis nuances.

---

## 4. Reference Fixture: `dataforge` Python CLI

To ensure rigorous validation, our reference Python fixture (`fixtures/python-rust-cli/source/dataforge.py`) must be non-trivial:
- **Subcommands:**
  - `stats`: Analyzes a JSON or CSV dataset (computes row count, column stats, mean/median/stddev, null counts). Supports `--format=json|table|summary`, `--output=<path>`, and `--round=<int>`.
  - `filter`: Filters records by predicate column expression (`--where "age > 30"`), `--limit=<int>`, `--sort-by=<col>`, and `--descending`.
  - `transform`: Transforms columns (e.g. upper, lower, scale, hash), reading from `--input` or `stdin`, writing to `--output` or `stdout`.
  - `inspect`: Fast metadata inspection of schema and file size.
- **Global Flags & Config:**
  - `--verbose` / `-v`, `--quiet` / `-q`, `--version`.
  - Environment variable fallback: `DATAFORGE_DEFAULT_FORMAT`.
- **Deterministic Exit Codes:**
  - `0`: Success
  - `1`: Validation / Argument / Query syntax error
  - `2`: File not found / I/O error
  - `3`: Processing / parse / schema error
- **Stream Discipline:**
  - Data output on `stdout`.
  - Informational / error / progress messages on `stderr`.
  - Supports UNIX pipeline chaining via `stdin`.
