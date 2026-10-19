# Verification and Contract Model

## 1. Principle

Generation and verification are separate authorities.

An agent may propose:

```text
"task complete"
```

but the domain core accepts only evidence.

## 2. Contract shape

Conceptual model:

```yaml
id: todo.create
category: behavioral
severity: critical
preconditions:
  - app_open
action:
  type: submit_form
expected:
  - todo_count_increases_by: 1
  - input_becomes_empty: true
```

Contract representation should be structured enough for deterministic tooling but not become a new programming language in the MVP.

## 3. Evidence shape

Every evidence record should identify:

- contract ID;
- verifier ID/version;
- source artifact revision;
- target artifact revision;
- fixture/scenario ID;
- status;
- measurements;
- produced files/logs/screenshots;
- timestamp;
- environment fingerprint where relevant.

Statuses:

```text
PASS
FAIL
INCONCLUSIVE
NOT_RUN
```

## 4. Categories

### Build

Examples:

- Cargo build;
- tests;
- clippy;
- Slint compile.

### Behavioral

Compare source and target scenario outcomes.

### Structural

Compare normalized UI/control/state structures where meaningful.

### Visual

Screenshot/geometry comparison.

### Native compliance

Prove the target is actually Rust+Slint, not a wrapper.

### Accessibility

Compare declared accessible names/roles/focus behavior when applicable.

### Performance

Measured only under controlled conditions.

## 5. Evidence freshness

Evidence is valid only for the artifact revision it tested.

If target code changes:

```text
target_rev_10 evidence != target_rev_11 evidence
```

Affected evidence becomes stale.

## 6. Independent verification

At least one critical verification path must be deterministic and external to the code-writing agent.

Examples:

- Cargo commands;
- browser/source replay;
- Slint headless replay;
- screenshot diff;
- plan/artifact hash validation.

LLM review may add findings but does not replace these gates.

## 7. Sanka-inspired coverage disclosure

Every verifier reports what it actually exercised.

Example:

```text
behavioral:
  declared_scenarios: 14
  executed: 12
  passed: 12
  unexecuted:
    - drag-reorder
    - offline-retry
```

Do not collapse that into “behavior verified.”

## 8. Crane-inspired completion gate

Final completion requires evidence for the current target revision.

Suggested completion:

```text
all critical contracts PASS
AND all required categories current
AND no critical capability gap
AND native compliance PASS
AND build PASS
AND accepted target revision == current target revision
```

## 9. No-regression health

A coarse health score may be useful for long migrations, but it must not hide critical failures.

Example:

```text
health = progress * correctness_gate
```

where `correctness_gate = 0` when a critical invariant fails.

Health is orchestration guidance, not the canonical contract model.
