---
name: generating-rust-slint-target
description: Use when implementing or reviewing the native Rust and Slint target produced from a web or React source.
---

# Generating Rust + Slint Target

Status: starter draft; pressure-test before enforcing as production process.

## Native rule

The accepted target uses Rust application logic and Slint UI.

Do not count WebView/browser embedding, screenshot-as-UI, or whole-source proxying as native completion.

## Structure

Prefer a clear state/event/effect boundary in Rust and presentation/callback bindings in Slint.

## Verification expectations

Keep the target buildable continuously where the execution mode requires it.

Run deterministic Cargo and Slint checks after changes. Do not weaken verifier thresholds to make generated code pass.
