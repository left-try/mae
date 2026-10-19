# Proposed Repository Layout

```text
universal-migrator/
├── AGENTS.md
├── README.md
├── opf.yaml
├── prompts/
│   ├── migration.planner.md
│   ├── migration.code-migrator.md
│   ├── migration.parity-verifier.md
│   ├── migration.failure-resolver.md
│   └── migration.ui-semantic-analyzer.md
├── .agents/
│   └── skills/
│       ├── analyzing-react-source/
│       ├── authoring-opf-migration-prompts/
│       ├── defining-migration-contracts/
│       ├── developing-migration-adapters/
│       ├── generating-rust-slint-target/
│       ├── modifying-aamf-runtime/
│       └── verifying-migration-parity/
├── .okf/
│   ├── manifest.yaml
│   ├── index.md
│   ├── architecture/index.md
│   ├── application/index.md
│   ├── operations/index.md
│   └── changes/index.md
├── .okf-index/                    # local, generated, ignored
├── agents/                        # AAMF legacy templates during transition
├── src/
│   ├── agents/                    # AAMF
│   ├── budget/                    # AAMF
│   ├── config/                    # AAMF + narrow patches
│   ├── core/                      # AAMF
│   ├── execution/                 # AAMF
│   ├── flow/                      # AAMF
│   ├── logging/                   # AAMF
│   ├── observability/             # AAMF
│   ├── domain/
│   ├── artifacts/
│   ├── adapters/
│   │   ├── source/
│   │   │   └── react/
│   │   └── target/
│   │       └── rust-slint/
│   ├── verification/
│   └── integrations/
│       ├── aamf/
│       ├── okf/
│       └── opf/
├── tests/
│   ├── domain/
│   ├── adapters/
│   ├── verification/
│   └── e2e/
├── fixtures/
│   └── react-rust-slint/
└── docs/
```

## Ownership convention

Add a small ownership note to major directories:

```text
upstream-aamf
ours
generated
```

Avoid copying the same knowledge into README, OKF, skills, and prompts. Each has a different job:

- README/docs: human reference;
- OKF: curated machine-retrievable repository knowledge;
- skills: reusable agent procedures;
- OPF: scenario prompt source;
- AAMF config/checkpoints: execution state.
