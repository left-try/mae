# MAE CLI Probe Assessment

## Assessment

The probe turns MAE’s CLI migration path from an untested idea into a reviewed, offline proof of execution. It does **not** demonstrate that MAE can migrate the lead-enrichment pipeline: no coding model was run, and the generated Rust program covers only a small greeting command.

**Quality: good for a bounded prototype; not ready for production pipeline migration.** The CLI adapter and its reported test evidence merit **7/10 as a prototype**. Evidence for production migration readiness remains low.

## What the probe demonstrates

- An initial probe found that MAE could report success while generating a Slint desktop application instead of a CLI. The new CLI path rejects this wrong-target output.
- The extension provides a Python-to-Rust CLI route with a bounded source inventory, a native Rust target scaffold, and a process-backed agent interface.
- Verification covers generation, Rust build and execution, and observable command behavior. Evidence is tied to source and target fingerprints, and a failed rerun invalidates prior evidence.
- The probe uses a checked-in fake agent, read-only fixture inputs, and network-disabled runs. It does not change the Python pipeline.
- The supplied report records 100 passing MAE workspace tests, Python verifier negative checks that remain active under `-O`, and matching Python/Rust outputs for four sample names.
- The supplied report records an independent review that confirmed fixes for stale-binary and disabled-assertion issues and found no new breakage.

These results validate MAE’s process and verification seam for this small case. They do not establish model-generated code quality or pipeline parity.

## Limits and risks

- The original CLI path produced a false positive by accepting a desktop app as a successful CLI migration. The new contract gate addresses this particular failure mode.
- A fake agent does not exercise real model behavior, including malformed generations, incomplete changes, retries, or recovery.
- MAE’s structural checks are not a security sandbox. A real agent and its child processes require OS-level isolation and containment.
- Native Windows validation was unavailable because `link.exe` was missing; successful workspace runs used Linux containers.
- The report says the MAE commit is local and unpublished while a submodule pointer refers to it. Until the exact commit is reachable from the remote or another durable source, a fresh clone cannot reproduce that state.
- The adapter is intentionally narrow: it inventories a restricted form of Python `argparse`, targets a dependency-free Rust CLI, and checks a small behavior contract.

## What production pipeline evidence still needs

A meaningful lead-enrichment migration would need to exercise the actual CLI and its contracts, multiple Rust modules and required dependencies, and the pipeline’s database lifecycle, file handling, provider integrations, caching, retries, and resume behavior. It also needs broader differential checks against the Python baseline, including errors and stage outcomes; a real coding-model run; native Windows validation; OS-level process containment; and a reachable, pinned MAE commit.

These are follow-up capabilities and validation tasks, not results of the current probe.

## Verdict

Keep MAE under evaluation as a migration tool; do not treat this probe as proof that the lead-enrichment pipeline has been migrated. The next decisive experiment is a controlled run on one representative, deterministic pipeline slice, using a real coding model and existing parity fixtures, after the MAE commit is fetchable and pinned. Expand adapter scope only where that experiment identifies a concrete gap.

## Source material

The supplied assessment refers to `E:/lead-sourcing-enrichment-module/.superpowers/sdd/2026-10-22-python-pipeline-rust-migration/task-1-report.md` and `E:/lead-sourcing-enrichment-module/tools/mae/docs/cli-probe.md`. Those external files were not available in this MAE workspace during this revision; reported test and review results above are attributed to the supplied assessment rather than independently re-run here.
