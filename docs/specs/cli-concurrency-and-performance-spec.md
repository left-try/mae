# Specification: CLI Concurrency Mapping & Performance Optimization

## 1. Problem Statement
Python command-line utilities tackling CPU-intensive batch operations rely on complex concurrency constructs (`multiprocessing.Pool`, `concurrent.futures.ProcessPoolExecutor`, `ThreadPoolExecutor`, and `asyncio`). These workarounds suffer from high GIL contention, process initialization overhead, and memory copying due to IPC pickling.

The MAE pipeline must:
1. Recognize Python concurrency patterns (`multiprocessing`, `threading`, `asyncio`).
2. Scaffold native, zero-cost Rust concurrency using `std::thread::scope` with work chunking.
3. Eliminate IPC and serialization overhead by sharing borrowed slices across threads.
4. Verify both exact numerical/digest differential parity and measurable speedup gains.

## 2. Test Seams & Deliverables

### Seam 1: Domain Concurrency Types (`crates/mae-domain`)
- Add `CliConcurrencyModel` enum:
  - `Sequential`: Single-threaded execution.
  - `ParallelThreads`: Native multi-threaded execution.
  - `MultiProcessing`: Process-isolated task mapping.
  - `AsyncEventLoop`: Asynchronous cooperative event loop.
- Add `CliPerformanceContract`:
  - Enforces maximum duration tolerance and expected speedup factor.
- Pre-agreed test: `crates/mae-domain/tests/seam_cli_domain_tests.rs`.

### Seam 2: Source Concurrency Extractor (`crates/mae-adapter-python-cli`)
- Inspect Python AST/code for:
  - `import multiprocessing`, `ProcessPoolExecutor`, `ThreadPoolExecutor`, `import asyncio`.
  - Extract `--workers`, `--chunk-size`, and concurrency modes.
- Classify `CLI_CONCURRENCY_PYTHON_GIL_OVERHEAD` as `Supported` with strategy `DirectNativeMapping`.
- Pre-agreed test: `crates/mae-adapter-python-cli/tests/seam_python_cli_adapter_tests.rs`.

### Seam 3: Native Rust Scoped Concurrency Generator (`crates/mae-adapter-rust-cli`)
- Generate `src/core/crunch.rs` and `src/core/parallel.rs` with `std::thread::scope`:
  - Partition slices across `workers` (or `std::thread::available_parallelism`).
  - Eliminate IPC/serialization: threads read immutable memory references `&[Record]` directly.
  - Implement CPU-heavy batch workloads: multi-round PBKDF/SHA256 crypto proof-of-work, statistical variance, and bulk record calculations.
- Pre-agreed test: `crates/mae-adapter-rust-cli/tests/seam_rust_cli_adapter_tests.rs`.

### Seam 4: Differential Parity & Benchmark Verifier (`crates/mae-verification`)
- Execute differential tests comparing Python `multiprocessing`/`async` runs against Rust native multi-threading:
  - Verify exact stdout tree parity across multi-core runs.
  - Verify speedup factor is recorded in verification evidence.
- Pre-agreed test: `crates/mae-verification/tests/seam_cli_verification_tests.rs`.

### Seam 5: CLI Pipeline Integration (`crates/mae-cli`)
- Full pipeline run of `analyze-cli` and `migrate-cli` on concurrent CPU-heavy workloads.
- Pre-agreed test: `crates/mae-cli/tests/seam5_e2e_mvp_tests.rs`.
