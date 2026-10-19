# Ponytail Codex Plugin Installation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Enable the official Ponytail Codex plugin only for this repository at a reviewed, pinned upstream release.

**Architecture:** Vendor Ponytail v5.3.2 from its reviewed commit under the docs suite, register it in the Git root's local plugin catalog, and enable it through project config. Inspect the hook definitions and scripts before presenting the separate Codex project-trust step to the user.

**Tech Stack:** Codex CLI 0.146.0, Ponytail v5.3.2, repository-local Codex plugin marketplace.

**Spec:** `docs/superpowers/specs/2026-10-17-agent-tooling-integration-design.md`

**Implementation note:** The remote Git marketplace fetch stalled. Ponytail was first staged from the official npm `5.3.2` package after `npm view` confirmed its `gitHead` equals `16352ac229eb08a9598dcae48fe9f2b342567d86`. Per the user's follow-up, the package is now vendored at `universal-migration-engine-docs/vendor/ponytail`, registered in the Git root as `mae-local`, and enabled in the Git root's `.codex/config.toml`. The user-level `ponytail-local` installation and marketplace were removed. The hooks were reviewed but not trusted.

## Global Constraints

- Install the official Codex plugin from the DietrichGebert marketplace, pinned to release v5.3.2 / commit `16352ac229eb08a9598dcae48fe9f2b342567d86`.
- Inspect the installed plugin's lifecycle hooks and their scripts before asking the user to trust them in Codex.
- Keep project migration contracts and verification requirements authoritative; Ponytail review output remains advisory.
- Ponytail registration and enablement must stay repository-scoped. Trusting the project's hooks remains a separate user action after review.
- Do not overwrite the existing untracked documentation suite or any current Codex marketplace entries.

## Review Focus

- The Ponytail marketplace is already configured: inspect `codex plugin marketplace list` and reuse the existing source rather than adding a duplicate.
- Ponytail is already installed at the reviewed version: inspect `codex plugin list` and avoid reinstalling it.
- The marketplace source resolves to a different commit: verify installed plugin metadata against `16352ac229eb08a9598dcae48fe9f2b342567d86` and stop if it does not match.
- Installed hook scripts differ from the reviewed upstream commit: compare the installed cache files and source repository before presenting trust.
- The Codex CLI reports install or hook-review commands differently from the documented flow: follow current `codex plugin --help` and do not silently use obsolete syntax.

---

### Task 1: Enable project-local Ponytail and prepare hook review

**Files:**
- `../.agents/plugins/marketplace.json`
- `../.codex/config.toml`
- `vendor/ponytail/` (vendored upstream plugin)

**Interfaces:**
- Consumes: the approved upstream Ponytail repository and commit.
- Produces: one repository-local Ponytail plugin entry and an inspected hook set; no lifecycle hook is trusted automatically.

- [x] **Step 1: Inspect current Codex plugin state**

Run: `codex plugin marketplace list` and `codex plugin list`.

Expected: record existing marketplace and Ponytail installation state; retain unrelated entries.

- [x] **Step 2: Confirm current install command syntax**

Run: `codex plugin marketplace --help`, `codex plugin add --help`.

Expected: the CLI supports the official marketplace registration and plugin installation commands documented by current Codex.

- [x] **Step 3: Inspect upstream lifecycle hook definitions and scripts at the pinned commit**

Read the `.codex-plugin` manifest, hook configuration, and referenced scripts from `16352ac229eb08a9598dcae48fe9f2b342567d86`.

Expected: list each hook, command, timeout, and file it runs; do not trust or execute the scripts during review.

- [x] **Step 4: Vendor Ponytail and register a repository-local marketplace**

Copy the verified upstream v5.3.2 contents under `vendor/ponytail/`, then register it in the Git root's `.agents/plugins/marketplace.json` using the repository-relative source path.

Expected: the project catalog `mae-local` points to the vendored package; no user-level marketplace is required.

- [x] **Step 5: Enable Ponytail in repository-local Codex config**

Set `[plugins."ponytail@mae-local"] enabled = true` in the Git root's `.codex/config.toml`.

Expected: Codex can load Ponytail for this trusted repository from the local marketplace. The `codex plugin list` CLI only reports the user-level installation and may not show project catalogs.

- [x] **Step 6: Review installed hooks and prepare user trust handoff**

Inspect the vendored plugin's manifest, hook configuration, and referenced scripts. Compare them to the pinned upstream files from Step 3.

Expected: provide the user a concise hook inventory and confirmation that installed scripts match the reviewed source. Do not trust hooks programmatically; the user reviews and trusts them through Codex's supported `/hooks` interface.

**Commit:** Not created; project-local files are left for review in the working tree.
