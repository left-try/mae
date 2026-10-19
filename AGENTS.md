# MAE — Agent Protocol & Engineering Standards

Treat migration correctness as a **contract-and-evidence** problem, never a syntax-similarity problem. Follow **DDD, OOP, SOLID, and Clean Architecture** strictly.

## 1. Ubiquitous Language & Skills

1. Read `GLOSSARY.md` and `docs/adr/` before designing or editing code.
2. Active skills live in `.agents/skills/`:
   - **Matt Pocock Engineering Skills**: `domain-modeling`, `codebase-design`, `tdd`, `to-spec`, `to-tickets`, `implement-spec`, `pr`, `code-review`.
   - **Ponytail Skills**: `ponytail`, `ponytail-review`, `ponytail-audit`, `ponytail-debt`, `ponytail-gain`, `ponytail-help`.
   - **Migration Engine Skills**: `defining-migration-contracts`, `analyzing-react-source`, `generating-rust-slint-target`, `verifying-migration-parity`, `developing-migration-adapters`, `authoring-opf-migration-prompts`, `modifying-aamf-runtime`.

## 2. OKF Repository Brain Protocol

Run explicit `bundle` commands (never hand-edit `.okf-index/`):

```sh
okf-agent bundle index --root .okf --index-root .okf-index
okf-agent bundle search "<query>" --root .okf --index-root .okf-index
okf-agent bundle update --root .okf --index-root .okf-index
okf-agent bundle validate --root .okf --index-root .okf-index
```

## 3. Open Prompt Format (OPF) Protocol

Prompt sources live in `prompts/*.md` and are registered in `opf.yaml`. After any prompt edit, run:

```sh
opf check .
```

## 4. Clean Architecture & Migration Invariants

- `crates/mae-domain` has **zero outward dependencies**. It defines Entities, Value Objects, Aggregates, Policies (`AcceptancePolicy`, `CompletionPolicy`, `HealthScorePolicy`, `RecoveryPolicy`), and Port traits.
- A `MigrationTask` must **never** transition to `Accepted` if:
  1. Any required dependency is not `Accepted`;
  2. Any required contract has status `Fail`, `Inconclusive`, or `NotRun`;
  3. Evidence was produced for an older target `ArtifactRevision` (stale evidence);
  4. `NativeComplianceVerifier` fails (e.g., WebView wrapper, Electron, screenshot-as-UI, or whole-app proxy);
  5. An agent asserts completion without independent deterministic evidence.
