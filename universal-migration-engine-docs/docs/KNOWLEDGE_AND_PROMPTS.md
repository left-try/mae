# OKF, OPF, AGENTS.md and Skills

## 1. Why use both OKF and OPF

They solve different persistence problems.

### OKF

Repository knowledge:

- architecture;
- boundaries;
- constraints;
- operating procedures;
- decisions;
- change knowledge;
- local search/MCP.

### OPF

Prompt assets:

- prompt/scenario source;
- IDs;
- inputs;
- digest;
- release/channel identity;
- validation;
- evals;
- provider-neutral message rendering.

Using one for both responsibilities would create unnecessary coupling.

## 2. OKF layout

Commit:

```text
.okf/
  manifest.yaml
  index.md
  architecture/
  application/
  operations/
  changes/
```

Generate locally:

```text
.okf-index/
```

`.okf-index/` should be ignored and rebuildable.

Suggested workflow:

```bash
okf-agent init .
okf-agent update .
okf-agent validate .
```

Before agent work, retrieve change context through OKF MCP or the CLI/index.

## 3. AAMF Lore vs OKF

Do not merge them conceptually.

| System | Scope | Main content |
|---|---|---|
| Lore/AAMF KB | migration run/source code | symbols, calls, types, file graph |
| OKF | engine repository / optional project repo | intent, architecture, constraints, curated knowledge |

They can both be exposed to agents over MCP.

## 4. OPF migration

AAMF currently has scenario templates in `agents/templates/*.md`.

Target state:

```text
prompts/
  migration.knowledge-builder.md
  migration.planner.md
  migration.code-migrator.md
  migration.parity-verifier.md
  migration.failure-resolver.md
  ...
opf.yaml
```

The AAMF scenario registry still owns:

- active scenario IDs;
- canonical output JSON schema;
- tool capabilities.

OPF owns the message text.

## 5. Prompt provenance

For every agent invocation, record:

- OPF prompt ID;
- source digest;
- optional OPF release;
- rendered input digest;
- provider;
- model;
- AAMF scenario ID;
- migration task ID.

This makes prompt changes auditable alongside code changes.

## 6. Provider independence

OPF emits ordered messages.

Do not make prompt files encode:

- Codex CLI flags;
- Claude Code flags;
- Copilot flags.

That mapping belongs in `AgentBackend`.

## 7. Prompt evals

Add eval fixtures for high-risk scenarios:

- planner must not silently remove unsupported features;
- code migrator must not use WebView when native Slint is required;
- verifier must report `INCONCLUSIVE` when evidence is missing;
- failure resolver must not mark a mismatch fixed without rerunning the signal;
- analyzer must disclose uncertain semantic interpretation.

Evals complement deterministic tests; they do not replace them.

## 8. AGENTS.md

`AGENTS.md` should remain short enough to be loaded frequently.

It should define:

- OKF retrieval/update/validate protocol;
- subsystem boundaries;
- migration invariants;
- prompt rules;
- native target rules;
- completion-claim discipline.

Detailed workflows belong in skills/docs.

## 9. Skills

Use project skills for judgment-heavy recurring work.

Initial drafts in this suite:

- `analyzing-react-source`;
- `developing-migration-adapters`;
- `defining-migration-contracts`;
- `generating-rust-slint-target`;
- `verifying-migration-parity`;
- `authoring-opf-migration-prompts`;
- `modifying-aamf-runtime`.

These are starter drafts. Before relying on them as enforced agent process, pressure-test representative scenarios and revise them based on actual failure modes.
