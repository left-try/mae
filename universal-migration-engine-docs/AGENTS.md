# Universal Migration Engine — Agent Protocol

This repository is an AAMF-based migration engine. Treat migration correctness as a contract-and-evidence problem, not a code-generation problem.

## 1. Repository context protocol

Run the OKF commands below from `universal-migration-engine-docs/`, the directory containing this `AGENTS.md`:

```sh
okf-agent bundle index --root .okf --index-root .okf-index
okf-agent bundle search "<query>" --root .okf --index-root .okf-index
okf-agent bundle update --root .okf --index-root .okf-index
okf-agent bundle validate --root .okf --index-root .okf-index
```

`.okf/` is the authored knowledge source. `.okf-index/` is generated local state, ignored by Git, and can be rebuilt. Use `bundle index` when creating the index, `bundle search` to retrieve context, `bundle update` after knowledge changes, and `bundle validate` before completion. Always pass both explicit roots.

Do not use the repository-profile `okf-agent init`, `index`, `update`, `validate`, or `run` commands from this suite. They discover the parent Git root and could modify knowledge outside this folder. Never hand-edit `.okf-index/`.

Before planning or editing:

1. Retrieve relevant OKF context with `bundle search` before making architectural assumptions.
2. Read the most relevant skill under `.agents/skills/`.
3. For AAMF-runtime work, inspect the current upstream-derived implementation before proposing changes.

After source changes, run focused tests for the modified subsystem and validate the OKF bundle with the explicit bundle command above.

## 2. Architectural boundaries

Keep these responsibilities separate:

- **Domain core:** contracts, capabilities, artifact provenance, task acceptance, migration completion, recovery decisions.
- **AAMF runtime:** DAG execution, scheduling, retries, checkpoints/resume, budgeting, observability, Git automation, agent invocation.
- **Source adapters:** extract source-specific semantics into versioned artifacts.
- **Target adapters:** map target-independent migration intent into target-specific code/build/test operations.
- **Verification:** independently collect evidence and evaluate contracts.
- **OKF:** repository knowledge and context retrieval.
- **OPF:** prompt/scenario source, identity, validation, release, and evals.

Do not move target/framework-specific logic into the domain core.

## 3. AAMF upstream patch rule

Prefer adapters and extension seams over editing upstream-derived AAMF modules.

Expected narrow patches:

- agent backend abstraction and Codex backend;
- prompt-provider seam for OPF;
- external analysis-artifact hooks;
- external verification hooks;
- domain acceptance/completion callback seam.

Avoid rewriting AAMF checkpointing, scheduler, observability, or task execution unless an experiment demonstrates a concrete incompatibility.

## 4. Migration invariants

A task must not become accepted when:

- a required dependency is not accepted;
- any required contract has current `FAIL`;
- required evidence is `INCONCLUSIVE` or `NOT_RUN` unless policy explicitly permits an exception;
- evidence was produced for an older target artifact version;
- target native-compliance checks fail;
- an agent asserts success without independent evidence.

Unknown compatibility is not treated as supported.

## 5. Prompt changes

AAMF scenario instructions should migrate toward OPF prompt files under `prompts/`.

For prompt edits:

1. change the OPF source, not provider-specific copies;
2. run `opf check .`;
3. preserve prompt IDs;
4. record prompt digest/release identity in invocation provenance;
5. update prompt eval fixtures when behavior changes.

Do not embed model choice into OPF prompt content. Model/provider selection belongs to execution configuration.

## 6. React → Rust + Slint MVP rules

The target must be a genuine native Rust + Slint application.

Forbidden as a successful migration result:

- WebView wrapper;
- Electron/Tauri web UI counted as native Slint;
- screenshot-as-UI;
- proxying the entire source app and declaring parity.

Preserve the source until independent verification passes.

## 7. Completion claims

Do not state that a migration, adapter, prompt, or skill is production-ready without the corresponding tests/evidence.

Skills in `.agents/skills/` are starter drafts. Before enforcing a skill as production process, pressure-test it against representative agent scenarios and revise it based on failures.
