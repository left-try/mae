# ADR 0003: Python CLI to Native Rust CLI Migration Flow & CLI Domain Model

- **Status:** Accepted
- **Date:** 2026-10-21

## Context

Following the success of MAE v0.1 (Frontend React to Native Windows Rust + Slint), MAE expands its contract-and-evidence migration engine to command-line software: **Python CLI utilities → Native Rust CLI binaries**.

CLI migration has distinct architectural advantages:
1. **Observable, Deterministic Contracts**: The interface of a CLI tool consists of `(argv, stdin, env, cwd, files) -> (exit_code, stdout, stderr, filesystem_mutations)`.
2. **Deterministic Differential Testing**: Both the Python script and the compiled Rust binary can be invoked headlessly with identical test matrices (valid invocations, boundary cases, missing arguments, invalid types, `--help`, `--version`).
3. **No Display/Windowing Dependency**: Runs in fast, sub-millisecond offline test suites.

However, in MAE v0.1, `SourceAdapterPort` and `TargetAdapterPort` in `crates/mae-domain/src/ports.rs` were tied to `SemanticUiModel`. To preserve Clean Architecture without falling into the "universal AST" trap, MAE must model CLI semantics directly.

## Decision

1. **CLI Semantic Domain Model (`CliSemanticModel`):**
   - We define `crates/mae-domain/src/semantic_cli.rs` containing:
     - `CliCommandGraph` & `CliCommandNode` (hierarchical commands and subcommands)
     - `CliArgument` (positional args, flags, named options, choices, defaults, environment variable fallbacks)
     - `CliIoContract` (stdin consumption, stdout format [JSON, PlainText, Tabular], stderr logging)
     - `CliExitCodeMapping` (standardized exit codes for success, validation error, I/O error, execution error)
     - `CliCapabilityReport` & `CliGapReasonCode` (e.g. `CLI_DYNAMIC_EVAL`, `CLI_UNSUPPORTED_PYPI_DEP`, `CLI_ASYNC_EVENT_LOOP`, `CLI_NATIVE_C_EXTENSION`).

2. **Clean Architecture Adapters:**
   - **`crates/mae-adapter-python-cli`**: Source adapter implementing `CliSourceAdapterPort`. Analyzes Python AST (`argparse`, `sys.argv`, `click`), extracting command structure, help messages, argument constraints, type coercions, and business logic entry points.
   - **`crates/mae-adapter-rust-cli`**: Target adapter implementing `CliTargetAdapterPort`. Generates an idiomatic Rust CLI project adhering to **Logic-CLI Separation**:
     - `src/cli.rs`: Command line parsing using `clap` (derive mode) or standard library parsing.
     - `src/core/`: Pure, testable Rust domain business logic.
     - `src/main.rs`: Entrypoint mapping arguments, invoking core logic, and exiting with exact exit codes.
     - `Cargo.toml`: Standard Rust package manifest.
   - **`crates/mae-verification`**: Independent `CliDifferentialVerifier` plugin running both Python and Rust targets with identical argument vectors, comparing exit codes, normalized stdout, and stderr.

3. **Generalization of Ports:**
   - Define `CliSourceAdapterPort` and `CliTargetAdapterPort` in `mae-domain` alongside the UI ports, adhering to Interface Segregation Principle (ISP).

## Consequences

- Extends MAE to systems and developer tooling.
- Keeps domain models deep and expressive: UI semantics remain pure in `SemanticUiModel`, while CLI semantics are captured in `CliSemanticModel`.
- Guarantees true behavioral equivalence: parity is proven through black-box differential execution of real inputs and outputs.
