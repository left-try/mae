# Local Agent Tooling Setup

This guide installs the tools used to maintain this documentation suite. Run project commands from `universal-migration-engine-docs/`.

## Install

Install OKF Agent from the reviewed upstream commit and Open Prompt Format CLI at version 0.2.0:

```sh
pipx install --force "git+https://github.com/left-try/okf-agent.git@ed3a192d0e44437a446d51d02781a3fa9a5cda93"
uv tool install open-prompt-format==0.2.0
```

Open Prompt Format is an experimental proposal. The CLI validates this suite's prompt registry and files; it does not establish semantic prompt quality.

## Ponytail Codex plugin

Ponytail is vendored at `vendor/ponytail/` and enabled through the repository-local plugin catalog. The catalog and setting are in the Git root, one level above this documentation suite:

- `../.agents/plugins/marketplace.json` registers the project catalog `mae-local` and points to `./universal-migration-engine-docs/vendor/ponytail`.
- `../.codex/config.toml` enables `ponytail@mae-local` for this trusted project.
- The vendored plugin is upstream v5.3.2, reviewed at commit `16352ac229eb08a9598dcae48fe9f2b342567d86`.

This keeps the Ponytail plugin registration and hook activation scoped to this repository. Open or restart Codex with this repository as the project and trust the project configuration when prompted. Review the hook commands in `vendor/ponytail/hooks/claude-codex-hooks.json` and their referenced scripts before approving hook execution. In Codex, the current mode flag lives in Ponytail's plugin data; `/ponytail default <mode>` writes Ponytail's user-level config under `~/.config/ponytail/config.json`, so use that command only if a default shared with other Ponytail-enabled projects is intended. Session transcript import is only started by the explicit `/ponytail band import` command (not supported by the Codex hook path).

To update the vendored plugin, replace `vendor/ponytail/` with the reviewed upstream release, update the pinned version and commit in this guide, and check that the project catalog still points to the vendored directory. Do not add it with `codex plugin marketplace add` or `codex plugin add`, since those commands configure a user-level installation.

## OKF knowledge bundle

Authored knowledge lives in `.okf/`. Its generated local index is `.okf-index/`, which is ignored by Git and can be rebuilt. Run these commands from the suite folder and keep both explicit roots:

```sh
okf-agent bundle index --root .okf --index-root .okf-index
okf-agent bundle search "<query>" --root .okf --index-root .okf-index
okf-agent bundle update --root .okf --index-root .okf-index
okf-agent bundle validate --root .okf --index-root .okf-index
```

Do not use repository-profile `okf-agent init`, `index`, `update`, `validate`, or `run` from this suite: Git-root discovery resolves to its parent workspace.

## OPF prompt collection

The prompt files and `opf.yaml` are the authored source. Validate them from this folder with:

```sh
opf check .
```

Use `uv tool list` to confirm the installed CLI version; this CLI does not implement `opf --version`. Keep existing prompt IDs stable when making compatibility fixes. Treat checker warnings as review findings, and do not change prompt intent solely to suppress heuristics. See `AGENTS.md` for migration-specific prompt and verification policies.
