# Operations

<!-- okf:generated:start -->
<!-- okf:generated:end -->

## Repository protocol

Before planning/editing, retrieve OKF context.

After changes:

```bash
okf-agent update . --changed-files <paths>
okf-agent validate .
```

For prompt changes:

```bash
opf check .
```

For AAMF changes, run the original AAMF test suite plus extension compatibility tests.

Do not edit `.okf-index/` manually.
