# Architecture

<!-- okf:generated:start -->
<!-- okf:generated:end -->

## Curated constraints

- AAMF is the execution/orchestration base.
- The migration domain core is framework/language independent.
- Source and target semantics enter through adapters and versioned artifacts.
- Correctness is contract-based and evidence-driven.
- Independent verification is mandatory for critical acceptance.
- IR stays small and composable; do not build a universal programming language.
- OKF and Lore serve different knowledge roles.
- OPF owns prompt text; AAMF owns scenario sequencing, capabilities, and output schemas.
- `.okf-index/` is local generated state and is not authored.
