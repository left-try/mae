# Agent Tooling Integration Design

**Status:** revised design approved in chat; awaiting review of this written specification.

## Goal

Connect Ponytail to Codex and install the project authoring tools for the existing OKF knowledge bundle and OPF prompt collection in `universal-migration-engine-docs/`.

## Current state

- The target folder already has a curated `.okf/` bundle, an `AGENTS.md` protocol, five OPF prompts, and an `opf.yaml` registry.
- `.gitignore.fragment` already lists `.okf-index/`, but the target folder does not yet have its own active `.gitignore`.
- No `okf-agent` command or OPF CLI is currently available.
- The parent Git checkout has no commits and currently contains the documentation suite as untracked files. Work must preserve that state and proceed in place.
- OKF Agent's ordinary `index`/`update`/`validate` commands discover the Git root and initialize repository knowledge there. The Git root is the parent of the documentation suite, so those commands would target the wrong directory.

## Design

### Ponytail

Install the official Codex plugin from the DietrichGebert marketplace, pinned to release v5.3.2 / commit `16352ac229eb08a9598dcae48fe9f2b342567d86`. Inspect the installed plugin's lifecycle hooks and their scripts before asking the user to trust them in Codex. Keep project migration contracts and verification requirements authoritative; Ponytail review output remains advisory.

### OKF Agent

Install `okf-agent` with `pipx` from the user-provided repository, pinned to commit `ed3a192d0e44437a446d51d02781a3fa9a5cda93`. Use its standalone, read-only `bundle` profile to index only the authored `.okf/` Markdown files, with generated data stored under `.okf-index/`:

1. `okf-agent bundle index --root .okf --index-root .okf-index` to create or refresh the local index.
2. `okf-agent bundle search "<query>" --root .okf --index-root .okf-index` to retrieve context.
3. `okf-agent bundle update --root .okf --index-root .okf-index` after knowledge changes.
4. `okf-agent bundle validate --root .okf --index-root .okf-index` before completion.

Update the project `AGENTS.md` to use those explicit bundle commands. Do not run repository-profile `init`, `index`, `update`, `validate`, or `run` commands here: Git-root discovery would target the parent workspace. The bundle profile does not initialize Git, write repository instructions, or install hooks. Do not hand-edit generated `.okf-index/` output.

### Open Prompt Format

Install the Python CLI `open-prompt-format==0.2.0` using `uv tool`. Run `opf check .` from the target folder against the existing `opf.yaml` and prompts. Keep prompt IDs and prompt sources in place; make only compatibility fixes required by the checker and preserve the current project verification instructions. Record that OPF is experimental and keep the installed version pinned.

### Project documentation and ignore rules

- Add a target-folder `.gitignore` entry for `.okf-index/` so the generated index remains local and rebuildable.
- Add a concise `docs/TOOLING_SETUP.md` with the pinned installation commands and the four OKF bundle index/search/update/validate commands, plus OPF validation.
- Update the README's start-here section to link the setup guide.
- Update `AGENTS.md` to document the explicit OKF bundle profile commands and source/index roots.
- Keep the previous Ponytail assessment as rationale; update it only if implementation findings materially change its recommendations.

## Boundaries

- No AAMF runtime dependency, migration adapter, prompt provider, migration contract, or acceptance rule is added by this tooling setup.
- No `okf-agent init`, automatic Git hook installation, OPF model call, paid eval, or prompt release/publish is included.
- Global changes are limited to the explicitly requested user-level tool installations and Codex Ponytail plugin registration. Trusting Ponytail hooks remains a separate user action after review.
- Do not overwrite the existing untracked documentation suite or any current Codex marketplace entries.

## Verification

- Confirm `codex plugin list` reports Ponytail installed and inspect the installed hook definitions/scripts against the pinned upstream source.
- Confirm `okf-agent --help`, then run `okf-agent bundle index --root .okf --index-root .okf-index` and `okf-agent bundle validate --root .okf --index-root .okf-index` from the target folder.
- Confirm `uv tool list` reports `open-prompt-format v0.2.0` and run `opf check .` from the target folder. Treat warnings as advisory; require zero errors and do not rewrite prompts just to suppress heuristics.
- Confirm the `.okf/` source bundle is unchanged, only intended project files changed, and `.okf-index/` is ignored.
- Do not run application tests: this is a documentation/tooling setup and the user did not request a test suite.

## Source references

- [Ponytail Codex installation](https://github.com/DietrichGebert/ponytail/blob/main/INSTALL.md#codex), pinned to commit `16352ac229eb08a9598dcae48fe9f2b342567d86`.
- [OKF Agent README](https://github.com/left-try/okf-agent/blob/main/README.md), pinned to commit `ed3a192d0e44437a446d51d02781a3fa9a5cda93`.
- [Open Prompt Format README](https://github.com/stovo-team/open-prompt-format), [PyPI 0.2.0](https://pypi.org/project/open-prompt-format/0.2.0/).
