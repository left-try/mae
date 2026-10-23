# ADR 0004: Concurrency Mapping and Performance Optimization for CLI Migrations

## Status
Accepted

## Context
A primary motivation for migrating CLI utilities from Python to Native Rust is computational performance and multi-core resource utilization.
In Python:
1. **The Global Interpreter Lock (GIL)** prevents threads from executing CPU-bound bytecode in parallel. Developers must resort to `multiprocessing.Pool` or `ProcessPoolExecutor`.
2. **IPC & Serialization Overhead**: Python multiprocessing requires serializing (`pickle`) inputs and outputs over IPC pipes or shared memory, incurring heavy serialization CPU overhead, high memory duplication, and process spawn latency.
3. **Async IO vs Compute**: Python's `asyncio` is single-threaded and easily starves during CPU-bound bursts.

In Rust:
1. **True Data Parallelism**: Rust provides fearlessly concurrent, native multi-threading.
2. **Zero-Copy Scoped Threads**: `std::thread::scope` enables worker threads to borrow immutable chunks of memory (`&[Record]`) concurrently without IPC, serialization, or atomic reference counting overhead.
3. **Algorithmic & Compiler Optimizations**: Rust's standard library, cache-aligned struct representations, loop vectorization, and LTO enable 10x-100x speedups for CPU-heavy tasks.

## Decision
1. **Domain Model Extension**:
   - Define `CliConcurrencyModel` (`Sequential`, `ParallelThreads`, `MultiProcessingEmulation`, `AsyncPipeline`) and `CliPerformanceProfile` in `mae-domain`.
   - Add capability classification: `CLI_CONCURRENCY_PYTHON_GIL_OVERHEAD` mapped to `DirectNativeMapping` via Rust native scoped threads (`std::thread::scope`).
2. **Source Adapter (`mae-adapter-python-cli`)**:
   - Detect imports and usages of `multiprocessing`, `concurrent.futures` (`ProcessPoolExecutor`, `ThreadPoolExecutor`), and `asyncio`.
   - Extract CLI concurrency parameters (`--workers`, `--mode`, `--chunk-size`).
3. **Target Adapter (`mae-adapter-rust-cli`)**:
   - Transform Python multiprocessing/threading pools into idiomatic Rust scoped multi-threading using `std::thread::scope` and `std::thread::available_parallelism`.
   - Implement zero-allocation chunk partitioning: divide read-only record slices evenly among hardware threads.
   - Implement CPU-intensive routines (e.g. iterative cryptographic hashing, statistical variance, batch transforms) in cache-efficient, vectorizable Rust loops.
4. **Differential Parity & Performance Gate (`mae-verification`)**:
   - Verify that results produced under parallel execution (multiprocessing, threading, or sequential) match **identically** between Python and Rust.
   - Introduce benchmark metrics into verification evidence recording execution time and speedup ratio.

## Consequences
- Maintains Ponytail zero-dependency discipline (`std`-only Rust, no external crate bloat).
- High-performance, multi-threaded Rust code that replaces heavy Python multiprocessing pipelines without external runtime dependencies.
- Independent differential verifier proves both semantic equivalence and significant performance gains.
