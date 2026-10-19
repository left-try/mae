# Ponytail Integration Assessment

Checked 2026-10-17 against Ponytail's GitHub README, source prompt, installation guide, and the Universal Migration Engine documentation suite.

## Recommendation

Treat [Ponytail](https://github.com/DietrichGebert/ponytail) as an optional source of agent workflow guidance, not as an AAMF runtime dependency, migration adapter, or verifier. Its core product is a Markdown skill/ruleset with host-specific plugin adapters and review/audit commands. It does not define migration contracts or migration artifact schemas.

The best project seam is the provider-neutral prompt/agent policy layer already described in `ARCHITECTURE.md` and `KNOWLEDGE_AND_PROMPTS.md`. If adopted, port a small, attributed subset of Ponytail's implementation principles into an OPF prompt or a project skill. Keep migration-specific acceptance and verification policy authoritative.

## Useful contributions

- Encourage agents to read touched code and trace callers before editing.
- Prefer the smallest complete change, existing project patterns, and platform capabilities.
- Keep validation, error handling, security, accessibility, and required verification intact.
- Offer an optional report-only code review/audit step that identifies risky changes, missing checks, and unnecessary complexity.

Potential placement:

1. Add a provider-neutral prompt such as `prompts/migration.implementation-style.md`, or keep it as a reusable skill under `.agents/skills/`.
2. Reference it from implementation scenarios (planner/code-migrator/failure-resolver) where appropriate. Do not automatically apply a code-writing style prompt to the parity verifier, whose job and independence should remain clear.
3. If useful, model Ponytail review output as advisory review findings attached to a task/change-set. AAMF can route actionable findings to repair, while ordinary migration evidence still comes from independent build, behavioral, visual, accessibility, and native-compliance checks.
4. Keep `/ponytail-audit`, its UI/band, and host lifecycle hooks outside the migration engine core. They are developer-tool integrations, not migration capabilities.

## Boundaries and conflicts to resolve

- Never let a generation agent's review report count as independent verification or satisfy an acceptance contract.
- Do not weaken required test/evidence policy in the name of minimizing code. Ponytail recommends a small test/self-check for non-trivial logic; this project must follow its migration contract and verification policy.
- If adopted, put known shortcuts and risks into structured task/artifact reporting, not only in a short prose sign-off.
- The existing project protocol requires verification and knowledge-index maintenance after source changes. Ponytail's advice must not override those project-specific requirements.
- Do not copy all of Ponytail's always-on rules verbatim. Reconcile its mode controls, reply-tail convention, and test guidance with the engine's OPF provenance, evidence freshness, and independent-verification requirements.

## Adoption path

Start with a pinned, reviewed copy of only the useful skill/prompt content and a small set of representative agent scenarios. Compare resulting diffs and review findings against the current workflow. Keep it optional until those scenarios show that it reduces unnecessary code without reducing required tests, migration evidence, or target correctness. If using upstream content, retain the MIT attribution and record the upstream commit or release used.

Ponytail's Codex plugin is another option for a developer's local workflow. Its installation guide says the Codex setup has lifecycle hooks that the user reviews and trusts. That is separate from reproducible engine configuration, so avoid silently installing or invoking the full plugin in migration runs.

## Source notes

- Ponytail describes its core as a skill and documents the mode and review/audit commands in its [README](https://github.com/DietrichGebert/ponytail#how-it-works).
- Its [core skill](https://github.com/DietrichGebert/ponytail/blob/main/skills/ponytail/SKILL.md) covers minimal complete changes, existing-code reuse, preserving validation/security/accessibility, and checks for non-trivial logic.
- The [Codex installation guide](https://github.com/DietrichGebert/ponytail/blob/main/INSTALL.md#codex) documents installation and lifecycle-hook trust.
- The repository states that it is [MIT licensed](https://github.com/DietrichGebert/ponytail/blob/main/LICENSE). Its benchmark claims are maintainer-reported and should not be treated as independent evidence for this engine.
- Relevant local design references: `docs/ARCHITECTURE.md` (prompt, adapter, and verification seams), `docs/KNOWLEDGE_AND_PROMPTS.md` (OPF ownership), and `AGENTS.md` (repository-specific correctness and verification rules).

## Current status

This is an integration recommendation only. No Ponytail plugin, hook, prompt, or runtime dependency has been installed or added to migration execution. The local `okf-agent` command was unavailable during this assessment, so the generated `.okf-index/` was not built; the curated `.okf/architecture/index.md` and repository documentation were used as project context.
